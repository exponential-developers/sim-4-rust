/*!
 * Theory EF, ported from sim-3 `src/Theories/CTs/EF.ts`.
 *
 * Variable order: tdot, q1, q2, b1, b2, c1, c2, a1, a2, a3
 *
 * Currencies: rho (tdot, q1, q2, a1), R (b1, b2, a2) and I (c1, c2, a3). Most strats look ahead:
 * when a purchase brings a variable close to a milestone or to the publication point, the sim
 * forks to also try coasting (not buying it any more), like sim-3's `confirmPurchase`.
 */

use std::sync::OnceLock;

use crate::api::config::TheoryType;
use crate::theories::pubtable::{CodedTable, IntTable, js_round};
use crate::theories::traditional::*;
use crate::utils::{
    cost::{Cost, FirstFreeCost},
    fastmath,
    lognum::LogNum,
    result::SimResult,
    value::Value,
    variable::Variable,
};

const TDOT: usize = 0;
const Q1: usize = 1;
const Q2: usize = 2;
const B1: usize = 3;
const B2: usize = 4;
const C1: usize = 5;
const C2: usize = 6;
const A1: usize = 7;
const A2: usize = 8;
const A3: usize = 9;

const VARIABLE_COUNT: usize = 10;

/** Order in which milestones are assigned */
const MILESTONE_PRIORITY: [usize; 5] = [0, 1, 2, 3, 4];

/** Rho at which each milestone is unlocked */
const MILESTONE_UNLOCKS: [f64; 14] = [
    10., 20., 30., 40., 50., 70., 90., 110., 130., 150., 250., 275., 300., 325.,
];

const CONVERTER: Converter = Converter {
    r9_affected: false,
    tau_factor: 1.6,
    mult_exponent: 0.09675,
    mult_factor: 0.,
};

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EfStrat {
        EF,
        EFSnax,
        EFd,
        EFAI,
        EFPlay,
    }
}

/** Publication table: the rho (times 32) to publish at for each last pub rho (times 32) */
fn pub_table() -> &'static IntTable {
    static TABLE: OnceLock<IntTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        IntTable::decode_format2(&CodedTable::parse(include_str!(
            "data/EFpubtable_coded.json"
        )))
    })
}

/** What EFPlay takes from the EFAI run: the a2 and a3 levels it reached (sim-3 `lasta23` ...) */
#[derive(Debug, Clone, Copy)]
struct PlayData {
    last_strong_a2: f64,
    last_strong_a2_cost: f64,
    last_a2_cost: f64,
    last_a3_cost: f64,
}

/** Simulates EF with the given strat (sim-3 `ef` function) */
pub fn simulate_ef(strat: EfStrat, input: &TheoryInput) -> SimResult {
    if strat != EfStrat::EFPlay {
        return EfSim::new(strat, input).simulate();
    }
    let initial = EfSim::new(EfStrat::EFAI, input).simulate();
    let mut sim = EfSim::new(strat, input);
    let a2 = get_last_level("a2", &initial.bought_vars) as f64;
    let a3 = get_last_level("a3", &initial.bought_vars) as f64;
    let last_strong_a2 = 10. * ((a2 - 1.) / 10.).floor() + 1.;
    sim.play = Some(PlayData {
        last_strong_a2,
        last_strong_a2_cost: l10(500.) + 2.2 * (last_strong_a2 - 1.) * l10(2.),
        last_a2_cost: l10(500.) + 2.2 * (a2 - 1.) * l10(2.),
        last_a3_cost: l10(500.) + 2.2 * (a3 - 1.) * l10(2.),
    });
    sim.simulate()
}

/** sim-3 `getBestResult(first, second)` where the first may be missing */
fn best_of(first: Option<SimResult>, second: SimResult) -> SimResult {
    best_result(first.unwrap_or_else(default_result), Some(second))
}

/** log10 constants used by the buying conditions, computed once per sim */
#[derive(Debug, Clone)]
struct EfConsts {
    l10_1_25: f64,
    l10_2: f64,
    l10_2_5: f64,
    l10_5: f64,
    /** log10(10 + n) for n = q1 level % 10 */
    l10_10_plus: [f64; 10],
    /** log10(4 + n / 2) for n = a1 level % 10 */
    l10_4_plus_half: [f64; 10],
}

#[derive(Debug, Clone)]
struct EfSim {
    core: TraditionalCore,
    strat: EfStrat,
    vars: Vec<Variable>,
    /** Currencies R and I (log10) */
    r: f64,
    i: f64,
    q: f64,
    t_var: f64,
    next_milestone_cost: f64,
    /** Only for EFPlay */
    play: Option<PlayData>,
    /** Rho at which the pub is forced (from the pub table); infinity: not forced */
    forced_pub_rho: f64,
    /** Variables that are no longer bought */
    coasting: [bool; VARIABLE_COUNT],
    /** Best result of the forks this sim started (not copied to forks) */
    best_res: Option<SimResult>,
    do_continuity_fork: bool,
    /** Number of forks above this sim */
    depth: i32,
    /** Current exponents of b2 and c2, which depend on milestones */
    b2_power: f64,
    c2_power: f64,
    consts: EfConsts,
}

impl EfSim {
    fn new(strat: EfStrat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::EF, false, input, CONVERTER, 10.);
        core.milestone_unlocks = MILESTONE_UNLOCKS.to_vec();
        core.milestones_max = vec![2, 3, 5, 2, 2];

        let stepwise = |offset: f64| Value::new_stepwise(2., 10, offset);
        let first_free = |cost: crate::utils::cost::ExponentialCost, value: Value, name| {
            let mut v = Variable::new(name, Cost::new(FirstFreeCost::new(cost)), value);
            // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
            v.buy();
            v
        };
        let exp = crate::utils::cost::ExponentialCost::new;

        let vars = vec![
            Variable::new("tdot", exp_cost(1e6, 1e6), Value::new_exponential(10.)),
            first_free(exp(10., 1.61328), stepwise(0.), "q1"),
            Variable::new(
                "q2",
                Cost::new_exponential(5., 60.),
                Value::new_exponential(2.),
            ),
            first_free(exp(20., 200.), stepwise(1.), "b1"),
            Variable::new(
                "b2",
                Cost::new_exponential(100., 2.),
                Value::new_exponential(1.1),
            ),
            first_free(exp(20., 200.), stepwise(1.), "c1"),
            Variable::new(
                "c2",
                Cost::new_exponential(100., 2.),
                Value::new_exponential(1.1),
            ),
            first_free(exp_cost_log2_model(2000., 2.2), stepwise(1.), "a1"),
            Variable::new(
                "a2",
                exp_cost_log2(500., 2.2),
                Value::new_stepwise(40., 10, 1.),
            ),
            Variable::new("a3", exp_cost_log2(500., 2.2), Value::new_exponential(2.)),
        ];

        let lpr = core.last_pub_rho;
        let mut forced_pub_rho = f64::INFINITY;
        if lpr < 374. && strat != EfStrat::EF {
            // a missing entry gives NaN, which never ends the pub (as in sim-3)
            forced_pub_rho = pub_table()
                .get(js_round(lpr * 32.) as i64)
                .map_or(f64::NAN, |v| v as f64 / 32.);
        }

        let mut sim = EfSim {
            core,
            strat,
            vars,
            r: f64::NEG_INFINITY,
            i: f64::NEG_INFINITY,
            q: 0.,
            t_var: 0.,
            next_milestone_cost: f64::INFINITY,
            play: None,
            forced_pub_rho,
            coasting: [false; VARIABLE_COUNT],
            best_res: None,
            do_continuity_fork: true,
            depth: 0,
            b2_power: 1.1,
            c2_power: 1.1,
            consts: EfConsts {
                l10_1_25: l10(1.25),
                l10_2: l10(2.),
                l10_2_5: l10(2.5),
                l10_5: l10(5.),
                l10_10_plus: std::array::from_fn(|n| l10(10. + n as f64)),
                l10_4_plus_half: std::array::from_fn(|n| l10(4. + n as f64 / 2.)),
            },
        };
        sim.update_milestones();
        sim
    }

    /** sim-3 `copy()`: a copy of this sim whose milestones are recalculated */
    fn copy(&self) -> EfSim {
        let mut fork = self.clone();
        fork.best_res = None;
        fork.do_continuity_fork = true;
        fork.depth = self.depth + 1;
        fork.update_milestones();
        fork
    }

    /** sim-3 `updateMilestones`: also tracks the next milestone and the b2/c2 exponents */
    fn update_milestones(&mut self) {
        let rho = self.core.last_pub_rho.max(self.core.max_rho);
        let stage = binary_insertion_search(&MILESTONE_UNLOCKS, rho);
        self.next_milestone_cost = MILESTONE_UNLOCKS
            .get(stage)
            .copied()
            .unwrap_or(f64::INFINITY);
        self.core.update_milestones(&MILESTONE_PRIORITY);

        let (ms3, ms4) = (self.core.milestones[3], self.core.milestones[4]);
        let b2_power = 1.1 + 0.01 * ms3 as f64;
        if self.b2_power != b2_power {
            self.b2_power = b2_power;
            self.recalculate(B2, b2_power);
        }
        let c2_power = 1.1 + 0.0125 * ms4 as f64;
        if self.c2_power != c2_power {
            self.c2_power = c2_power;
            self.recalculate(C2, c2_power);
        }
    }

    /** Changes the exponent of an exponential variable and recomputes its value */
    fn recalculate(&mut self, id: usize, power: f64) {
        let v = &mut self.vars[id];
        v.value_model = Value::new_exponential(power);
        v.value = v.value_model.compute_from_zero(v.level);
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    /** Value of the currency variable `i` is bought with */
    fn currency(&self, i: usize) -> f64 {
        match i {
            B1 | B2 | A2 => self.r,
            C1 | C2 | A3 => self.i,
            _ => self.core.rho.value,
        }
    }

    fn symbol(i: usize) -> &'static str {
        match i {
            B1 | B2 | A2 => "R",
            C1 | C2 | A3 => "I",
            _ => "rho",
        }
    }

    fn pay(&mut self, i: usize, cost: LogNum) {
        match i {
            B1 | B2 | A2 => self.r = sub_log(self.r, cost.value),
            C1 | C2 | A3 => self.i = sub_log(self.i, cost.value),
            _ => self.core.rho -= cost,
        }
    }

    fn buying_condition(&self, i: usize) -> bool {
        use EfStrat::*;
        let cm = self.core.cur_mult;
        let ms = &self.core.milestones;
        let c = |j: usize| self.cost(j);
        let k = &self.consts;
        let q1_level = (self.vars[Q1].level % 10) as usize;
        let a1_level = (self.vars[A1].level % 10) as usize;
        let a1_ok = || c(A1) + k.l10_4_plus_half[a1_level] < c(Q2) || self.coasting[2];
        match self.strat {
            EF => true,
            EFSnax => match i {
                Q1 | B1 | B2 | C1 | C2 => cm < 1.,
                A1 => cm < 1. || self.core.last_pub_rho > 150.,
                _ => true,
            },
            EFd => match i {
                Q1 => c(Q1) + 1. < c(Q2),
                B1 | B2 | C1 => cm < 1.,
                C2 => cm < 1.,
                A1 => c(C2) + k.l10_2_5 < c(Q2),
                _ => true,
            },
            EFAI => match i {
                Q1 => c(Q1) + k.l10_10_plus[q1_level] < c(Q2),
                B1 | B2 => c(i) + k.l10_5 < c(A2) || ms[1] < 2 || cm < 1.,
                C1 | C2 => c(i) + k.l10_5 < c(A3) || ms[1] < 2 || cm < 1.,
                A1 => a1_ok(),
                _ => true,
            },
            EFPlay => {
                let play = self.play;
                match i {
                    Q1 => c(Q1) + k.l10_10_plus[q1_level] < c(Q2),
                    B1 | B2 => match play {
                        None => true,
                        Some(p) => {
                            if (self.vars[A2].level as f64) < p.last_strong_a2 {
                                c(i) + k.l10_5 < p.last_strong_a2_cost
                            } else if c(i) + k.l10_5 < p.last_a2_cost {
                                c(i) + k.l10_2 < c(A2)
                            } else {
                                false
                            }
                        }
                    },
                    C1 | C2 => match play {
                        None => true,
                        Some(p) => {
                            if c(i) + k.l10_5 < p.last_a3_cost {
                                c(i) + k.l10_1_25 < c(A3)
                            } else {
                                false
                            }
                        }
                    },
                    A1 => a1_ok(),
                    A2 => match play {
                        None => true,
                        Some(p) => c(B1).min(c(B2)) + k.l10_5 >= p.last_strong_a2_cost,
                    },
                    _ => true,
                }
            }
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        let ms = &self.core.milestones;
        match i {
            TDOT => self.vars[TDOT].level < 4,
            B1 | B2 => ms[0] > 0,
            C1 | C2 => ms[0] > 1,
            A1 => ms[1] > 0,
            A2 => ms[1] > 1,
            A3 => ms[1] > 2,
            _ => true,
        }
    }

    /** sim-3 `getDynamicCoastingConditions` */
    fn dynamic_coasting(&self, i: usize) -> bool {
        let cm = self.core.cur_mult;
        match i {
            Q1 => cm > 1.2,
            Q2 => cm > 1.6,
            A1 => cm > 1.4,
            _ => false,
        }
    }

    /** sim-3 `getForcedDynamicCoastingConditions` */
    fn forced_dynamic_coasting(&self, i: usize) -> bool {
        i == Q1 && (self.coasting[2] || self.coasting[7])
    }

    /** sim-3 `confirmPurchase`: decides to stop buying, or forks a sim that stops buying */
    fn confirm_purchase(&mut self, id: usize) -> bool {
        const LOW_BOUNDS: [f64; VARIABLE_COUNT] = [0., 0.6, 0.2, 0., 0., 0., 0., 0.3, 0., 0.];
        const HIGH_BOUNDS: [f64; VARIABLE_COUNT] = [0., 1.8, 1.5, 0., 0., 0., 0., 1.5, 0., 0.];
        let next_coast = js_min(self.forced_pub_rho, self.next_milestone_cost);
        let do_dynamic_coasting = self.forced_pub_rho == f64::INFINITY && self.strat != EfStrat::EF;
        let cost = self.cost(id);

        if self.forced_pub_rho - cost <= LOW_BOUNDS[id]
            || (do_dynamic_coasting && self.forced_dynamic_coasting(id))
        {
            self.coasting[id] = true;
            return false;
        }
        if next_coast - cost < HIGH_BOUNDS[id] || (do_dynamic_coasting && self.dynamic_coasting(id))
        {
            // sim-3 throws an error at a depth of 100; here the sim just doesn't fork any more
            if self.depth <= 100 {
                let mut fork = self.copy();
                fork.coasting[id] = true;
                let fork_res = fork.simulate();
                self.best_res = Some(best_of(self.best_res.take(), fork_res));
            }
        }
        true
    }

    fn tick(&mut self) {
        let logbonus = l10(self.core.dt) + self.core.tot_mult;

        self.q = add_log(self.q, self.val(Q1) + self.val(Q2) + logbonus);
        self.t_var += self.core.dt * (self.vars[TDOT].level as f64 / 5. + 0.2);

        let b = self.val(B1) + self.val(B2);
        let c = self.val(C1) + self.val(C2);
        let r = b + l10(fastmath::cos(self.t_var).abs());
        let i = c + l10(fastmath::sin(self.t_var).abs());
        let ms = &self.core.milestones;
        let (ms0, ms1, ms2) = (ms[0], ms[1], ms[2]);
        if ms0 > 0 {
            self.r = add_log(self.r, logbonus + r * 2.);
        }
        if ms0 > 1 {
            self.i = add_log(self.i, logbonus + i * 2.);
        }

        let a = if ms1 > 0 {
            (self.val(A1) + self.val(A2) + self.val(A3)) * (0.1 * ms2 as f64 + 1.)
        } else {
            0.
        };
        let t_term = l10(self.t_var) + self.q * 2.;
        let rhodot = match ms0 {
            0 => logbonus + t_term / 2.,
            1 => logbonus + add_log(t_term, self.r * 2.) / 2.,
            2 => logbonus + a + add_log(add_log(t_term, self.r * 2.), self.i * 2.) / 2.,
            _ => return,
        };
        self.core.rho += LogNum {
            value: rhodot,
            sign: 1,
        };
    }

    /** sim-3 `buyVariablesFork` */
    fn buy_variables_fork(&mut self) {
        // EF has no onAnyVariablePurchased step, so whether something was bought is not tracked
        for i in (0..VARIABLE_COUNT).rev() {
            while self.currency(i) > self.cost(i)
                && self.buying_condition(i)
                && self.variable_available(i)
                && !self.coasting[i]
            {
                if !self.confirm_purchase(i) {
                    break;
                }
                self.core.record_purchase_in(&self.vars[i], Self::symbol(i));
                let cost = self.vars[i].cost;
                self.pay(i, cost);
                self.vars[i].buy();
            }
        }
    }

    fn simulate(&mut self) -> SimResult {
        // The pub is forced at the table's rho; then the sim doesn't end because of time
        self.core.pub_target =
            (self.forced_pub_rho != f64::INFINITY).then_some(self.forced_pub_rho);
        self.core.do_sim_end_conditions = self.forced_pub_rho == f64::INFINITY;

        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            let previous_next_milestone_cost = self.next_milestone_cost;
            if self.core.last_pub_rho <= 325. {
                self.update_milestones();
            }
            if self.next_milestone_cost > previous_next_milestone_cost {
                self.coasting = [false; VARIABLE_COUNT];
            }
            self.buy_variables_fork();
            if self.forced_pub_rho == 375. && self.core.max_rho >= 371. && self.do_continuity_fork {
                self.do_continuity_fork = false;
                let mut fork = self.copy();
                fork.forced_pub_rho = f64::INFINITY;
                let res = fork.simulate();
                if res.pub_rho.value > 375. {
                    self.best_res = Some(best_of(self.best_res.take(), res));
                }
            }
            self.core.collect_pub_table();
        }
        self.core.trim_bought_vars();

        let level = |i: usize| get_last_level(self.vars[i].name, &self.core.bought_vars);
        let mut extra = String::new();
        if self.strat != EfStrat::EF {
            extra = format!(" q1: {} q2: {} a1: {}", level(Q1), level(Q2), level(A1));
            if self.core.params.show_a23 {
                extra += &format!(" a2: {} a3: {}", level(A2), level(A3));
            }
        }
        let result = self.core.create_result(self.strat.name(), &extra);
        best_result(result, self.best_res.take())
    }
}
