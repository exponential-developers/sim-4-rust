/*!
 * Theory CSR2, ported from sim-3 `src/Theories/CTs/CSR2.ts`.
 *
 * Variable order: q1, q2, c1, n, c2
 *
 * CSR2XL first simulates the publication once to find the best time to stop buying ("coast"),
 * then simulates again buying only until then. The "PT" strats publish at the point given by a
 * table and fork to try coasting variables near it.
 */

use std::sync::OnceLock;

use crate::api::config::TheoryType;
use crate::theories::pubtable::{CodedTable, IntTable, js_round, js_to_fixed};
use crate::theories::traditional::*;
use crate::utils::{
    cost::{Cost, FirstFreeCost},
    fastmath,
    lognum::LogNum,
    result::SimResult,
    value::Value,
    variable::Variable,
};

const Q1: usize = 0;
const Q2: usize = 1;
const C1: usize = 2;
const N: usize = 3;
const C2: usize = 4;

/** Order in which milestones are assigned */
const C2_PRIORITY: [usize; 3] = [1, 2, 0];
const Q1_PRIORITY: [usize; 3] = [0, 1, 2];

/** Rho at which each milestone is unlocked */
const MILESTONE_UNLOCKS: [f64; 6] = [10., 45., 80., 115., 220., 500.];

const CONVERTER: Converter = Converter {
    r9_affected: false,
    tau_factor: 0.4,
    mult_exponent: 0.55075,
    // -log10(200)
    mult_factor: -2.3010299956639813,
};

/** Bounds, per variable, for stopping to buy near the forced publication point */
const LOW_BOUNDS_ACTIVE: [f64; 5] = [0.65, 0.15, 0.85, 0., 0.];
const HIGH_BOUNDS_ACTIVE: [f64; 5] = [1.45, 0.5, 1.8, 1.2, 1.2];
const LOW_BOUNDS_PASSIVE: [f64; 5] = [1., 0.15, 1.35, 0., 0.];
const HIGH_BOUNDS_PASSIVE: [f64; 5] = [3.85, 0.5, 3.8, 1.2, 1.2];

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Csr2Strat {
        CSR2,
        CSR2PT,
        CSR2d,
        CSR2dPT,
        CSR2XL,
        CSR2XLPT,
    }
}

impl Csr2Strat {
    fn uses_pub_table(self) -> bool {
        self.name().contains("PT")
    }

    /** CSR2XL and CSR2XLPT use the active bounds */
    fn has_active_bounds(self) -> bool {
        self.name().contains("XL")
    }
}

/** Publication table: the rho (times 16) to publish at for each last pub rho (times 16) */
fn pub_table() -> &'static IntTable {
    static TABLE: OnceLock<IntTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        IntTable::decode_format2(&CodedTable::parse(include_str!(
            "data/CSR2pubtable_coded.json"
        )))
    })
}

/** Simulates CSR2 with the given strat (sim-3 `csr2` function) */
pub fn simulate_csr2(strat: Csr2Strat, input: &TheoryInput) -> SimResult {
    Csr2Sim::new(strat, input).simulate(true)
}

/** sim-3 `getCoastLen` */
fn coast_len(r: f64) -> f64 {
    use fastmath::pow;
    if r < 45. {
        pow(r, 2.1) / 10.
    } else if r < 80. {
        pow(r, 2.22) / 40.
    } else if r < 220. {
        pow(r, 2.7) / 3.3e4 + 40.
    } else if r < 500. {
        pow(r, 2.8) / 9.2e4 + 40.
    } else {
        pow(1.5, pow(r, 0.8475) / 20.) * 5.
    }
}

/** Cost increase of each variable (as in sim-3), for the CSR2XL result */
fn cost_increases() -> [f64; 5] {
    [
        5.,
        128.,
        16.,
        fastmath::pow(2., fastmath::log2(256.) * 3.346),
        fastmath::pow(10., 5.65),
    ]
}

/** log10 constants used by the sim, computed once per sim */
#[derive(Debug, Clone)]
struct Csr2Consts {
    l10_2: f64,
    l10_4: f64,
    l10_200: f64,
    l10_1_8: f64,
    l10_1_3: f64,
    l10_root8: f64,
    l10_root8_plus_3: f64,
    /** log10(7 + n) for n = q1 level % 10 and log10(15 + n) for n = c1 level % 10 */
    l10_7_plus: [f64; 10],
    l10_15_plus: [f64; 10],
}

#[derive(Debug, Clone)]
struct Csr2Sim {
    core: TraditionalCore,
    /** The input this sim was made with (the CSR2XL first pass starts from the same input) */
    input: TheoryInput,
    strat: Csr2Strat,
    vars: Vec<Variable>,
    q: f64,
    update_error_flag: bool,
    error: f64,
    /** [time to stop buying at, 1 if that time was searched for (0 while searching)] */
    recursion_value: [f64; 2],
    /** [best tau/h found, time] while searching for the best coast */
    best_coast: [f64; 2],
    forced_pub_rho: f64,
    coasting: [bool; 5],
    best_res: Option<SimResult>,
    do_continuity_fork: bool,
    low_bounds: &'static [f64; 5],
    high_bounds: &'static [f64; 5],
    consts: Csr2Consts,
}

/** sim-3 `getBestResult(first, second)` where the first may be missing */
fn best_of(first: Option<SimResult>, second: SimResult) -> SimResult {
    best_result(first.unwrap_or_else(default_result), Some(second))
}

impl Csr2Sim {
    fn new(strat: Csr2Strat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::CSR2, false, input, CONVERTER, 10.);
        core.milestone_unlocks = MILESTONE_UNLOCKS.to_vec();
        core.milestones_max = vec![3, 1, 2];

        let mut q1 = Variable::new(
            "q1",
            Cost::new(FirstFreeCost::new(exp_cost_model(10., 5.))),
            Value::new_stepwise(2., 10, 0.),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        q1.buy();

        let vars = vec![
            q1,
            Variable::new("q2", exp_cost(15., 128.), Value::new_exponential(2.)),
            Variable::new("c1", exp_cost(1e6, 16.), Value::new_stepwise(2., 10, 1.)),
            Variable::new(
                "n",
                exp_cost(50., cost_increases()[3]),
                Value::new(PlainLinearValue {
                    power: 1.,
                    offset: 1.,
                }),
            ),
            Variable::new(
                "c2",
                exp_cost(1e3, cost_increases()[4]),
                Value::new_exponential(2.),
            ),
        ];

        let lpr = core.last_pub_rho;
        let mut forced_pub_rho = f64::INFINITY;
        if strat.uses_pub_table() && (500. ..1499.5).contains(&lpr) {
            // a missing entry gives NaN, which never ends the pub (as in sim-3)
            forced_pub_rho = pub_table()
                .get(js_round(lpr * 16.) as i64)
                .map_or(f64::NAN, |v| v as f64 / 16.);
        }
        let (low_bounds, high_bounds) = if strat.has_active_bounds() {
            (&LOW_BOUNDS_ACTIVE, &HIGH_BOUNDS_ACTIVE)
        } else {
            (&LOW_BOUNDS_PASSIVE, &HIGH_BOUNDS_PASSIVE)
        };

        let root8 = 8f64.sqrt();
        let mut sim = Csr2Sim {
            core,
            input: input.clone(),
            strat,
            vars,
            q: 0.,
            update_error_flag: true,
            error: 0.,
            recursion_value: [f64::INFINITY, 0.],
            best_coast: [0., 0.],
            forced_pub_rho,
            coasting: [false; 5],
            best_res: None,
            do_continuity_fork: true,
            low_bounds,
            high_bounds,
            consts: Csr2Consts {
                l10_2: l10(2.),
                l10_4: l10(4.),
                l10_200: l10(200.),
                l10_1_8: l10(1.8),
                l10_1_3: l10(1.3),
                l10_root8: l10(root8),
                l10_root8_plus_3: l10(root8 + 3.),
                l10_7_plus: std::array::from_fn(|n| l10(7. + n as f64)),
                l10_15_plus: std::array::from_fn(|n| l10(15. + n as f64)),
            },
        };
        sim.update_milestones();
        sim
    }

    /** sim-3 `copy()`: copyFrom keeps the milestones, but not the best result or continuity fork */
    fn copy(&self) -> Csr2Sim {
        let mut fork = self.clone();
        fork.best_res = None;
        fork.do_continuity_fork = true;
        fork
    }

    /** Whether this sim is the one searching for the best coast (the first pass of CSR2XL) */
    fn searching_coast(&self) -> bool {
        self.strat == Csr2Strat::CSR2XL && self.recursion_value[1] == 0.
    }

    /** sim-3 `getMilestonePriority` */
    fn milestone_priority(&self) -> &'static [usize] {
        if self.core.last_pub_rho < 500. && self.strat == Csr2Strat::CSR2XL {
            let lpr = self.core.last_pub_rho;
            let mut ms_cond = 0.;
            if lpr > 45. {
                ms_cond = 4.;
            }
            if lpr > 80. {
                ms_cond = 8.;
            }
            if lpr > 115. {
                ms_cond = 20.;
            }
            if lpr > 220. {
                ms_cond = 40.;
            }
            let c = |i: usize| self.vars[i].cost.value;
            let rho = self.core.rho.value;
            // milestones are not set yet in the constructor
            let ms1 = self.core.milestones.get(1).copied().unwrap_or(0);
            if ((rho + l10(ms_cond * 0.5) > c(N)
                || (rho + l10(ms_cond) > c(C2) && ms1 > 0)
                || (self.core.cur_mult > 1. && rho + self.consts.l10_2 > c(Q2)))
                && rho < c(N).min(c(C2)))
                || self.core.t > self.recursion_value[0]
            {
                return &Q1_PRIORITY;
            }
        }
        &C2_PRIORITY
    }

    fn update_milestones(&mut self) {
        let priority = self.milestone_priority();
        self.core.update_milestones(priority);
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    fn update_error(&mut self, n: f64) {
        let k = &self.consts;
        let x = n * k.l10_root8_plus_3;
        self.error = (if n % 2. == 0. {
            sub_log(x, 0.)
        } else {
            add_log(x, 0.)
        }) - k.l10_root8;
    }

    /** sim-3 `searchCoast`: remembers the time at which stopping to buy gives the best tau/h */
    fn search_coast(&mut self, rhodot: f64) {
        let promising = self.core.cur_mult > 0.7;
        if !promising {
            return;
        }
        let k = &self.consts;
        let lpr = self.core.last_pub_rho;
        let mut i = coast_len(lpr);
        let max_multi = ((self.core.tot_mult + k.l10_4 + k.l10_200) / 2.203) * 10.;
        if lpr < 500. {
            let max_rho = self.core.max_rho;
            let q1_bonus = self.val(Q1)
                * (if max_rho >= 10. {
                    if max_rho >= 45. {
                        if max_rho >= 80. { 1.15 } else { 1.1 }
                    } else {
                        1.05
                    }
                } else {
                    1.
                });
            let rho = self.core.rho.value;
            let t = self.core.t;
            let s = |i: f64, best: &mut [f64; 2]| {
                let end_rho = add_log(rho, rhodot + q1_bonus + l10(i * 1.5));
                let end_tau_h = (js_min(max_multi, end_rho) - lpr) / ((t + i) / 3600.);
                if best[0] < end_tau_h {
                    best[0] = end_tau_h;
                    best[1] = t;
                }
            };
            let mut best = self.best_coast;
            s(i, &mut best);
            i *= 0.8;
            s(i, &mut best);
            i /= fastmath::pow(0.8, 2.);
            s(i, &mut best);
            self.best_coast = best;
        } else {
            let ms0 = self.core.milestones[0] as f64;
            let rhodot =
                self.core.tot_mult + self.val(Q1) * (1. + 0.05 * ms0) + self.val(Q2) + self.q;
            let qdot = self.core.tot_mult + self.val(C1) + self.val(C2) * 1.15 + self.error;
            let avg_q = add_log(self.q + k.l10_2, qdot + l10(i * 1.5)) - k.l10_2;
            let end_rho = add_log(self.core.rho.value, rhodot - self.q + avg_q + l10(i * 1.5));
            let end_tau_h = (end_rho - lpr) / ((self.core.t + i) / 3600.);
            if self.best_coast[0] < end_tau_h && end_rho < max_multi {
                self.best_coast = [end_tau_h, self.core.t];
            }
        }
    }

    fn buying_condition(&self, i: usize) -> bool {
        use Csr2Strat::*;
        let c = |j: usize| self.cost(j);
        let k = &self.consts;
        let min_others = c(Q2).min(c(N)).min(c(C2));
        match self.strat {
            CSR2 | CSR2PT => true,
            CSR2d | CSR2dPT => match i {
                Q1 => c(Q1) + k.l10_7_plus[(self.vars[Q1].level % 10) as usize] < min_others,
                C1 => c(C1) + k.l10_15_plus[(self.vars[C1].level % 10) as usize] < min_others,
                _ => true,
            },
            CSR2XL | CSR2XLPT => match i {
                Q1 => c(Q1) + k.l10_7_plus[(self.vars[Q1].level % 10) as usize] < min_others,
                Q2 => c(Q2) + k.l10_1_8 < c(C2),
                C1 => c(C1) + k.l10_15_plus[(self.vars[C1].level % 10) as usize] < min_others,
                N => c(N) + k.l10_1_3 < c(C2),
                _ => true,
            },
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        i != C2 || self.core.milestones[1] > 0
    }

    fn tick(&mut self) {
        let ms = &self.core.milestones;
        let (ms0, ms1, ms2) = (ms[0], ms[1], ms[2]);
        let vq1 = self.val(Q1) * (1. + 0.05 * ms0 as f64);
        let vc2 = if ms1 > 0 {
            self.val(C2) * (1. + 0.5 * ms2 as f64)
        } else {
            0.
        };

        if self.update_error_flag {
            let c2_level = if ms1 > 0 {
                self.vars[C2].level as f64
            } else {
                0.
            };
            let vn = self.val(N) + c2_level;
            self.update_error(vn);
            self.update_error_flag = false;
        }

        // Only the first pass of CSR2XL uses what it finds
        if self.core.last_pub_rho < 500. && self.searching_coast() {
            self.search_coast(self.core.tot_mult + self.val(Q2) + self.q);
        }

        let qdot = self.val(C1) + vc2 + self.error;
        self.q = add_log(self.q, self.core.tot_mult + l10(self.core.dt) + qdot);
        let rhodot = self.core.tot_mult + vq1 + self.val(Q2) + self.q;
        self.core.rho += LogNum {
            value: rhodot + l10(self.core.dt),
            sign: 1,
        };
    }

    /** sim-3 `confirmPurchase`: near the forced pub, stops buying or forks a sim that stops */
    fn confirm_purchase(&mut self, id: usize) -> bool {
        if self.forced_pub_rho != f64::INFINITY {
            let distance = self.forced_pub_rho - self.cost(id);
            if distance <= self.low_bounds[id] {
                self.coasting[id] = true;
                return false;
            }
            if distance < self.high_bounds[id] {
                let mut fork = self.copy();
                fork.coasting[id] = true;
                let fork_res = fork.simulate(true);
                self.best_res = Some(best_of(self.best_res.take(), fork_res));
            }
        }
        true
    }

    /** sim-3 `buyVariablesFork` */
    fn buy_variables_fork(&mut self) {
        let mut bought = false;
        for i in (0..self.vars.len()).rev() {
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
                bought = true;
                if i > 2 {
                    self.update_error_flag = true;
                }
            }
        }
        if bought && self.searching_coast() {
            self.search_coast(self.core.tot_mult + self.val(Q2) + self.q);
        }
    }

    /**
     * `first_pass_allowed`: whether CSR2XL may start with a first pass to find the best coast
     * (sim-3: the `data.recursionValue` it was started with is not set yet)
     */
    fn simulate(&mut self, first_pass_allowed: bool) -> SimResult {
        self.core.pub_target =
            (self.forced_pub_rho != f64::INFINITY).then_some(self.forced_pub_rho);
        self.core.do_sim_end_conditions = self.forced_pub_rho == f64::INFINITY;

        if self.core.last_pub_rho >= 10. && first_pass_allowed && self.strat == Csr2Strat::CSR2XL {
            let mut first_pass = Csr2Sim::new(self.strat, &self.input);
            first_pass.simulate(false);
            self.recursion_value = [first_pass.best_coast[1], 1.];
        }
        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            if self.core.t < self.recursion_value[0]
                || self.core.cur_mult < 0.7
                || self.recursion_value[1] == 0.
            {
                self.buy_variables_fork();
            }
            if self.core.last_pub_rho < 500. {
                self.update_milestones();
            }
            if self.forced_pub_rho == 1500. && self.core.max_rho >= 1497. && self.do_continuity_fork
            {
                self.do_continuity_fork = false;
                let mut fork = self.copy();
                fork.forced_pub_rho = f64::INFINITY;
                let res = fork.simulate(true);
                if res.pub_rho.value > 1500. {
                    self.best_res = Some(best_of(self.best_res.take(), res));
                }
            }
            self.core.collect_pub_table();
        }
        if self.recursion_value[1] == 1. || self.strat != Csr2Strat::CSR2XL {
            self.core.trim_bought_vars();
        }

        let mut extra = String::from(" ");
        if self.strat == Csr2Strat::CSR2XL {
            let mut last_buy = 0f64;
            for (i, increase) in cost_increases().iter().enumerate() {
                last_buy = js_max(last_buy, self.cost(i) - l10(*increase));
            }
            let multi = fastmath::pow(
                10.,
                self.core
                    .tot_mult_from_rho(js_min(last_buy, self.core.pub_rho))
                    - self.core.tot_mult,
            );
            extra += &js_to_fixed(multi, 2);
        }
        if self.strat.uses_pub_table() {
            let level = |i: usize| get_last_level(self.vars[i].name, &self.core.bought_vars);
            extra += &format!("q1: {} q2: {} c1: {}", level(Q1), level(Q2), level(C1));
        }
        let result = self.core.create_result(self.strat.name(), &extra);
        best_result(result, self.best_res.take())
    }
}
