/*!
 * Theory FP (fractal patterns), ported from sim-3 `src/Theories/CTs/FP.ts`.
 *
 * Variable order: tdot, c1, c2, q1, q2, r1, n, s
 */

use std::sync::OnceLock;

use crate::api::config::TheoryType;
use crate::theories::pubtable::{CodedTable, IntTable, js_round};
use crate::theories::traditional::*;
use crate::utils::{
    cost::{CompositeCost, Cost, ExponentialCost, FirstFreeCost},
    fastmath,
    lognum::LogNum,
    result::SimResult,
    value::{Value, ValueTrait},
    variable::Variable,
};

const TDOT: usize = 0;
const C1: usize = 1;
const C2: usize = 2;
const Q1: usize = 3;
const Q2: usize = 4;
const R1: usize = 5;
const N: usize = 6;
const S: usize = 7;

const VARIABLE_COUNT: usize = 8;

/** Order in which milestones are assigned */
const MILESTONE_PRIORITY: [usize; 6] = [0, 1, 2, 3, 4, 5];

const CONVERTER: Converter = Converter {
    r9_affected: false,
    tau_factor: 0.3,
    mult_exponent: 0.331,
    // log10(5)
    mult_factor: 0.6989700043360189,
};

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FpStrat {
        FP,
        FPcoast,
        FPd,
        FPdMS,
        FPmodBurstC1,
        FPmodBurstC1MS,
    }
}

impl FpStrat {
    /** Strats where milestone 4 is switched on and off while s is small */
    fn is_ms(self) -> bool {
        self.name().contains("MS")
    }

    fn is_active(self) -> bool {
        matches!(self, FpStrat::FPd | FpStrat::FPdMS)
    }
}

/** Publication tables: the rho (times 8) to publish at for each last pub rho (times 8) */
fn pub_table(extended: bool) -> &'static IntTable {
    static NORMAL: OnceLock<IntTable> = OnceLock::new();
    static EXTENDED: OnceLock<IntTable> = OnceLock::new();
    if extended {
        EXTENDED.get_or_init(|| {
            IntTable::decode_format2(&CodedTable::parse(include_str!(
                "data/FPextendedPT_coded.json"
            )))
        })
    } else {
        NORMAL.get_or_init(|| {
            IntTable::decode_format2(&CodedTable::parse(include_str!(
                "data/FPpubtable_coded.json"
            )))
        })
    }
}

/** sim-3's `VariableSValue`: a plain number growing by 0.15 per level, a bit faster in a middle range */
#[derive(Debug, Clone, Copy)]
struct SValue;

impl SValue {
    fn get_s(level: i32) -> f64 {
        const CUTOFFS: [i32; 2] = [32, 39];
        if level < CUTOFFS[0] {
            return 1. + level as f64 * 0.15;
        }
        if level < CUTOFFS[1] {
            return Self::get_s(CUTOFFS[0] - 1) + 0.15 + (level - CUTOFFS[0]) as f64 * 0.2;
        }
        Self::get_s(CUTOFFS[1] - 1) + 0.2 + (level - CUTOFFS[1]) as f64 * 0.15
    }
}

impl ValueTrait for SValue {
    fn compute_from_zero(&self, level: i32) -> LogNum {
        LogNum {
            value: Self::get_s(level),
            sign: 1,
        }
    }

    fn compute_next(&self, _old_value: LogNum, current_level: i32) -> LogNum {
        LogNum {
            value: Self::get_s(current_level + 1),
            sign: 1,
        }
    }
}

/** sim-3 `stepwiseSum` */
fn stepwise_sum(level: f64, base: f64, length: f64) -> f64 {
    if level <= length {
        return level;
    }
    let level = level - length;
    let cycles = (level / length).floor();
    let rest = level - cycles * length;
    base * (cycles + 1.) * ((length * cycles) / 2. + rest) + length + level
}

/** sim-3 `approx`: log10 of the number of segments of the fractal at a level */
fn approx(l10_2: f64, level: i32) -> f64 {
    let n = level as f64 + 1.;
    l10(1. / 6.) + add_log(l10_2 * (2. * n), l10_2)
}

/** Simulates FP with the given strat (sim-3 `fp` function) */
pub fn simulate_fp(strat: FpStrat, input: &TheoryInput) -> SimResult {
    FpSim::new(strat, input).simulate()
}

/** sim-3 `getBestResult(first, second)` where the first may be missing */
fn best_of(first: Option<SimResult>, second: SimResult) -> SimResult {
    best_result(first.unwrap_or_else(default_result), Some(second))
}

/** Values the sim remembers while the level they come from doesn't change */
#[derive(Debug, Clone, Copy)]
struct Memo {
    level: i32,
    value: f64,
}

impl Memo {
    const EMPTY: Memo = Memo {
        level: -1,
        value: 0.,
    };

    fn get(&mut self, level: i32, compute: impl FnOnce() -> f64) -> f64 {
        if self.level != level {
            *self = Memo {
                level,
                value: compute(),
            };
        }
        self.value
    }
}

#[derive(Debug, Clone)]
struct FpSim {
    core: TraditionalCore,
    strat: FpStrat,
    vars: Vec<Variable>,
    q: f64,
    r: f64,
    t_var: f64,
    /** Fractal values: T(n), U(n), S(floor(sqrt(n))) and n */
    t_n: f64,
    u_n: f64,
    s_n: f64,
    n: f64,
    update_n_flag: bool,
    /** log10 of t_n, u_n, n and u_n * 2 / 2, updated together with them */
    l10_t_n: f64,
    l10_u_n: f64,
    l10_n: f64,
    l10_u_n_2_half: f64,
    forced_pub_rho: f64,
    coasting: [bool; VARIABLE_COUNT],
    best_res: Option<SimResult>,
    do_continuity_fork: bool,
    /** Remembered vq1, vr1 and approx(q2 level) (they only change when a variable is bought) */
    memo_vq1: Memo,
    memo_vr1: Memo,
    memo_a: Memo,
    l10_2: f64,
}

impl FpSim {
    fn new(strat: FpStrat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::FP, false, input, CONVERTER, 12.);
        core.milestone_unlocks = vec![
            l10(5e22),
            95.,
            175.,
            300.,
            385.,
            420.,
            550.,
            600.,
            700.,
            1500.,
        ];
        core.milestones_max = vec![2, 2, 3, 1, 1, 1];

        let log = |value: f64| LogNum { value, sign: 1 };
        let mut c1 = Variable::new(
            "c1",
            Cost::new(FirstFreeCost::new(exp_cost_model(10., 1.4))),
            Value::new_stepwise(150., 100, 0.),
        );
        c1.buy();
        let mut q1 = Variable::new(
            "q1",
            Cost::new(FirstFreeCost::new(exp_cost_model(1e35, 12.))),
            Value::new_stepwise(10., 10, 0.),
        );
        q1.buy();
        let mut r1 = Variable::new(
            "r1",
            Cost::new(FirstFreeCost::new(CompositeCost::new(
                exp_cost_model(1e80, 25.),
                // "1e480" is too large for a number
                ExponentialCost::new(log(480.), log(parse_value(150.))),
                285,
            ))),
            Value::new_stepwise(2., 5, 0.),
        );
        r1.buy();

        let vars = vec![
            Variable::new("tdot", exp_cost(1e4, 1e4), Value::new_exponential(10.)),
            c1,
            Variable::new(
                "c2",
                Cost::new(CompositeCost::new(
                    exp_cost_model(1e15, 40.),
                    exp_cost_model(1e37, 16.42),
                    15,
                )),
                Value::new_exponential(2.),
            ),
            q1,
            Variable::new("q2", exp_cost(1e76, 1e3), Value::new_exponential(10.)),
            r1,
            Variable::new("n", exp_cost(1e4, 3e6), Value::new_exponential(10.)),
            Variable::new(
                "s",
                // "1e730" is too large for a number
                Cost::Exponential(ExponentialCost::new(log(730.), log(parse_value(1e30)))),
                Value::new(SValue),
            ),
        ];

        let lpr = core.last_pub_rho;
        let mut forced_pub_rho = f64::INFINITY;
        if (1200. ..3490.).contains(&lpr) && strat != FpStrat::FP {
            // a missing entry gives NaN, which never ends the pub (as in sim-3)
            forced_pub_rho = pub_table(lpr >= 1990.)
                .get(js_round(lpr * 8.) as i64)
                .map_or(f64::NAN, |v| v as f64 / 8.);
        }

        let mut sim = FpSim {
            core,
            strat,
            vars,
            q: 0.,
            r: 0.,
            t_var: 0.,
            t_n: 1.,
            u_n: 1.,
            s_n: 0.,
            n: 1.,
            update_n_flag: true,
            l10_t_n: l10(1.),
            l10_u_n: l10(1.),
            l10_n: l10(1.),
            l10_u_n_2_half: l10(2.) / 2.,
            forced_pub_rho,
            coasting: [false; VARIABLE_COUNT],
            best_res: None,
            do_continuity_fork: true,
            memo_vq1: Memo::EMPTY,
            memo_vr1: Memo::EMPTY,
            memo_a: Memo::EMPTY,
            l10_2: l10(2.),
        };
        sim.core.update_milestones(&MILESTONE_PRIORITY);
        sim
    }

    /** sim-3 `copy()`: copyFrom keeps the milestones, but not the best result or continuity fork */
    fn copy(&self) -> FpSim {
        let mut fork = self.clone();
        fork.best_res = None;
        fork.do_continuity_fork = true;
        fork
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    /** sim-3 `T` */
    fn t_of(n: f64) -> f64 {
        if n == 0. {
            return 0.;
        }
        let log2_n = fastmath::log2(n);
        if log2_n % 1. == 0. {
            return (fastmath::pow(2., 2. * log2_n + 1.) + 1.) / 3.;
        }
        let i = n - fastmath::pow(2., log2_n.floor());
        Self::t_of(fastmath::pow(2., log2_n.floor())) + 2. * Self::t_of(i) + Self::t_of(i + 1.) - 1.
    }

    /** sim-3 `V` */
    fn v_of(n: f64) -> f64 {
        if n == 0. {
            return 0.;
        }
        let log2_n = fastmath::log2(n);
        if log2_n % 1. == 0. {
            return fastmath::pow(2., 2. * log2_n);
        }
        let i = n - fastmath::pow(2., log2_n.floor());
        fastmath::pow(2., 2. * log2_n.floor()) + 3. * Self::v_of(i)
    }

    /** sim-3 `U` */
    fn u_of(n: f64) -> f64 {
        (4. / 3.) * Self::v_of(n) - (1. / 3.)
    }

    /** sim-3 `S` */
    fn s_of(&self, n: f64) -> f64 {
        if n == 0. {
            return 0.;
        }
        if self.core.milestones[3] == 0 {
            return l10(3.) * (n - 1.);
        }
        l10(1. / 3.) + sub_log(self.l10_2 + l10(3.) * n, l10(3.))
    }

    /** sim-3 `updateN` */
    fn update_n(&mut self) {
        self.t_n = Self::t_of(self.n);
        self.u_n = Self::u_of(self.n);
        self.s_n = self.s_of(self.n.sqrt().floor());
        self.l10_t_n = l10(self.t_n);
        self.l10_u_n = l10(self.u_n);
        self.l10_n = l10(self.n);
        self.l10_u_n_2_half = l10(self.u_n * 2.) / 2.;
    }

    fn buying_condition(&self, i: usize) -> bool {
        let c = |j: usize| self.cost(j);
        let ms4 = self.core.milestones[4];
        match self.strat {
            FpStrat::FP | FpStrat::FPcoast => true,
            s if s.is_active() => {
                i != C1 || c(C1) + l10((self.vars[C1].level % 100 + 1) as f64) < c(C2)
            }
            // FPmodBurstC1(MS)
            _ => match i {
                C1 => {
                    let level = self.vars[C1].level;
                    let mod100 = level % 100;
                    if mod100 > 85 {
                        let level_minus_mod = level - mod100;
                        let total_cost = self.cost_for_levels(
                            C1,
                            level_minus_mod + mod100,
                            level_minus_mod + 101,
                        );
                        if total_cost < c(C2) && (ms4 == 0 || total_cost < c(S)) {
                            return true;
                        }
                    }
                    let next = c(C1) + l10((mod100 + 1) as f64);
                    next < c(C2) && (ms4 == 0 || next < c(S))
                }
                C2 => ms4 == 0 || c(C2) + 0.1 < c(S),
                Q1 => c(Q1) + l10(((self.vars[Q1].level % 10 + 1) * 2) as f64) < c(Q2),
                Q2 => ms4 == 0 || c(Q2) + 0.1 < c(S),
                _ => true,
            },
        }
    }

    /** sim-3 `getCostForLevels`: the cost of buying levels `from` to `to - 1` */
    fn cost_for_levels(&self, id: usize, from: i32, to: i32) -> f64 {
        let model = &self.vars[id].cost_model;
        let mut total = model.get_cost(from).value;
        for level in (from + 1)..to {
            total = add_log(total, model.get_cost(level).value);
        }
        total
    }

    fn variable_available(&self, i: usize) -> bool {
        let ms = &self.core.milestones;
        match i {
            TDOT => self.vars[TDOT].level < 4,
            Q1 | Q2 => ms[0] > 0,
            R1 => ms[0] > 1,
            S => ms[4] > 0,
            _ => true,
        }
    }

    fn tick(&mut self) {
        if self.update_n_flag {
            let level = self.vars[N].level as f64;
            let ms1 = self.core.milestones[1];
            let term1 = stepwise_sum(level, 1., 40.);
            let term2 = if ms1 > 0 {
                (stepwise_sum((level - 30.).max(0.), 1., 35.) * 2.).floor()
            } else {
                0.
            };
            let term3 = if ms1 > 1 {
                (stepwise_sum((level - 69.).max(0.), 1., 30.) * 2.4).floor()
            } else {
                0.
            };
            self.n = (1. + term1 + term2 + term3).min(20000.);
            self.update_n();
            self.update_n_flag = false;
        }

        if self.strat.is_ms() && self.core.last_pub_rho > 700. && self.val(S) < 2. {
            self.core.milestones[4] = 1;
            if ((self.core.ticks % 20) as f64) < 10. / self.val(S) {
                self.core.milestones[4] = 0;
            }
        }

        let (ms0, ms2, ms4, ms5) = {
            let ms = &self.core.milestones;
            (ms[0], ms[2], ms[4], ms[5])
        };
        let (q1_level, r1_level) = (self.vars[Q1].level, self.vars[R1].level);
        let q1_value = self.val(Q1);
        let r1_value = self.val(R1);
        let vq1 = self.memo_vq1.get(q1_level, || {
            q1_value - l10(1. + 1000. / fastmath::pow(q1_level as f64, 1.5))
        });
        let vr1 = self.memo_vr1.get(r1_level, || {
            r1_value - l10(1. + 1e9 / fastmath::pow(r1_level as f64, 4.))
        });
        let q2_level = self.vars[Q2].level;
        let l10_2 = self.l10_2;
        let a = self.memo_a.get(q2_level, || approx(l10_2, q2_level));

        let l10dt = l10(self.core.dt);
        self.t_var += (self.vars[TDOT].level as f64 / 5. + 0.2) * self.core.dt;

        let s_value = if ms4 > 0 { self.val(S) } else { 0. };
        let qdot = vq1 + a + self.l10_u_n * (7. + s_value) - 3.;
        if ms0 > 0 {
            self.q = add_log(self.q, qdot + l10dt);
        }

        let vsn = self.s_n * (1. + 0.6 * ms2 as f64);
        let rdot = if ms5 < 1 {
            vr1 + (self.l10_t_n + self.l10_u_n) * self.l10_n + vsn
        } else {
            vr1 + (self.l10_t_n + self.l10_u_n) * self.l10_u_n_2_half + vsn
        };
        if self.core.milestones[0] > 1 {
            self.r = add_log(self.r, rdot + l10dt);
        }

        let s_term = if ms4 > 0 { self.val(S) - 2. } else { 0. };
        let mut rhodot = self.core.tot_mult
            + self.val(C1)
            + self.val(C2)
            + self.l10_t_n * (7. + s_term)
            + l10(self.t_var);
        rhodot += if ms0 > 0 { self.q } else { 0. };
        rhodot += if self.core.milestones[0] > 1 {
            self.r
        } else {
            0.
        };

        self.core.rho += LogNum {
            value: rhodot + l10dt,
            sign: 1,
        };
    }

    /** sim-3 `confirmPurchase`: near the forced pub, stops buying or forks a sim that stops */
    fn confirm_purchase(&mut self, id: usize) -> bool {
        const LOW_BOUNDS: [f64; VARIABLE_COUNT] = [0., 0.3, 0.15, 0.3, 0.3, 0.1, 0., 0.];
        const HIGH_BOUNDS: [f64; VARIABLE_COUNT] = [0., 3.5, 0.5, 3.5, 1., 3.5, 1.5, 0.];
        if self.forced_pub_rho != f64::INFINITY {
            let distance = self.forced_pub_rho - self.cost(id);
            if distance <= LOW_BOUNDS[id] {
                self.coasting[id] = true;
                return false;
            }
            if distance < HIGH_BOUNDS[id] {
                let mut fork = self.copy();
                fork.coasting[id] = true;
                let fork_res = fork.simulate();
                self.best_res = Some(best_of(self.best_res.take(), fork_res));
            }
        }
        true
    }

    /** sim-3 `buyVariablesFork` */
    fn buy_variables_fork(&mut self) {
        // FP has no onAnyVariablePurchased step, so whether something was bought is not tracked
        for i in (0..VARIABLE_COUNT).rev() {
            while self.core.rho.value > self.cost(i)
                && self.buying_condition(i)
                && self.variable_available(i)
                && !self.coasting[i]
            {
                if !self.confirm_purchase(i) {
                    break;
                }
                self.core.record_purchase(&self.vars[i]);
                let cost = self.vars[i].cost;
                self.core.rho -= cost;
                self.vars[i].buy();
                if i == N {
                    self.update_n_flag = true;
                }
            }
        }
    }

    fn simulate(&mut self) -> SimResult {
        if self.forced_pub_rho != f64::INFINITY {
            self.core.pub_target = Some(self.forced_pub_rho);
            self.core.sim_end_cur_mult = None;
        } else {
            self.core.pub_target = None;
            self.core.sim_end_cur_mult = Some(10000.);
        }
        self.core.do_sim_end_conditions = self.forced_pub_rho == f64::INFINITY;

        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            self.core.update_milestones(&MILESTONE_PRIORITY);
            self.buy_variables_fork();
            if self.forced_pub_rho == 2000. && self.core.max_rho >= 1996. && self.do_continuity_fork
            {
                self.do_continuity_fork = false;
                let mut fork = self.copy();
                fork.forced_pub_rho = pub_table(true)
                    .get(js_round(self.core.last_pub_rho * 8.) as i64)
                    .map_or(f64::NAN, |v| v as f64 / 8.);
                if fork.forced_pub_rho > 2000. {
                    let res = fork.simulate();
                    self.best_res = Some(best_of(self.best_res.take(), res));
                }
            }
            self.core.collect_pub_table();
        }
        self.core.trim_bought_vars();

        let result = self.core.create_result(self.strat.name(), "");
        best_result(result, self.best_res.take())
    }
}
