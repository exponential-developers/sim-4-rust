/*!
 * Theory BaP (Basel problem), ported from sim-3 `src/Theories/CTs/BaP.ts`.
 *
 * Variable order: tdot, c1, c2, c3, c4, c5, c6, c7, c8, c9, c10, n
 */

use std::f64::consts::PI;
use std::sync::OnceLock;

use crate::api::config::TheoryType;
use crate::theories::pubtable::{CodedTable, PubTable, js_round, js_to_fixed};
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
const C1: usize = 1;
const C2: usize = 2;
const C3: usize = 3;
const C10: usize = 10;
const N: usize = 11;

const VARIABLE_COUNT: usize = 12;

/** Rho at which each milestone is unlocked */
const MILESTONE_UNLOCKS: [f64; 20] = [
    10., 15., 20., 25., 30., 40., 50., 70., 90., 120., 150., 200., 250., 300., 400., 500., 600.,
    700., 800., 1000.,
];

/** Rho at which the a and q milestones get another level */
const A_POINTS: [f64; 9] = [20., 30., 50., 80., 140., 240., 400., 600., 800.];
const Q_POINTS: [f64; 8] = [25., 40., 60., 100., 180., 300., 500., 700.];

/** Rho at which the "coast" strats stop buying until the next milestone */
const COAST_POINTS: [f64; 22] = [
    10., 15., 20., 25., 30., 40., 50., 60., 70., 80., 100., 140., 150., 180., 200., 240., 300.,
    400., 500., 600., 700., 1000.,
];

const A_PRIORITY: [usize; 5] = [0, 1, 2, 3, 4];
const Q_PRIORITY: [usize; 5] = [0, 1, 3, 2, 4];

const CONVERTER: Converter = Converter {
    r9_affected: false,
    tau_factor: 0.4,
    mult_exponent: 0.132075,
    // log10(5)
    mult_factor: 0.6989700043360189,
};

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum BapStrat {
        BaP,
        BaPcoast,
        BaPAI,
        BaPAIMS,
        BaPAIMS2,
        BaPd,
        BaPdMS,
    }
}

impl BapStrat {
    /** Strats that alternate the milestone priority every 100 seconds */
    fn alternates_milestones(self) -> bool {
        matches!(self, BapStrat::BaPdMS | BapStrat::BaPAIMS)
    }

    /** Strats buying the cheapest variable by weight instead of by conditions */
    fn uses_weights(self) -> bool {
        self.name().contains("AI")
    }
}

/** Publication table: the rho to publish at for each last pub rho */
fn pub_table() -> &'static PubTable {
    static TABLE: OnceLock<PubTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        PubTable::decode_format1(&CodedTable::parse(include_str!(
            "data/BaPpubtable_coded.json"
        )))
    })
}

/** Simulates BaP with the given strat (sim-3 `bap` function) */
pub fn simulate_bap(strat: BapStrat, input: &TheoryInput) -> SimResult {
    BapSim::new(strat, input).simulate()
}

/** sim-3 `getBestResult(first, second)` where the first may be missing */
fn best_of(first: Option<SimResult>, second: SimResult) -> SimResult {
    best_result(first.unwrap_or_else(default_result), Some(second))
}

/** log10 constants used by the sim, computed once per sim */
#[derive(Debug, Clone)]
struct BapConsts {
    l10_2: f64,
    l10_4: f64,
    l10_25: f64,
    l10_pi2_6: f64,
    /** 2^0.25, the cost ratio of c1 */
    p: f64,
}

#[derive(Debug, Clone)]
struct BapSim {
    core: TraditionalCore,
    strat: BapStrat,
    vars: Vec<Variable>,
    q: [f64; 9],
    r: f64,
    t_var: f64,
    forced_pub_rho: f64,
    do_continuity_fork: bool,
    best_res: Option<SimResult>,
    consts: BapConsts,
}

impl BapSim {
    fn new(strat: BapStrat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::BaP, false, input, CONVERTER, 7.);
        core.milestone_unlocks = MILESTONE_UNLOCKS.to_vec();

        let exp = |value: f64| Value::new_exponential(value);
        let log2_cost = |layer: i32, exponent: i32| {
            // c(layer) costs layer^exponent and grows by 2^(exponent * log2(layer)) per level
            let layer = layer as f64;
            exp_cost_log2(
                fastmath::pow(layer, exponent as f64),
                exponent as f64 * fastmath::log2(layer),
            )
        };
        let mut c1 = Variable::new(
            "c1",
            Cost::new(FirstFreeCost::new(exp_cost_log2_model(0.0625, 0.25))),
            Value::new_stepwise(65536., 64, 0.),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        c1.buy();

        let vars = vec![
            Variable::new("tdot", exp_cost(1e6, 1e6), Value::new_stepwise(2., 10, 0.)),
            c1,
            Variable::new("c2", exp_cost_log2(16., 4.), exp(2.)),
            Variable::new("c3", exp_cost(19683., 19683.), exp(3.)),
            Variable::new("c4", exp_cost_log2(fastmath::pow(4., 16.), 32.), exp(4.)),
            Variable::new("c5", log2_cost(5, 25), exp(5.)),
            Variable::new("c6", log2_cost(6, 36), exp(6.)),
            Variable::new("c7", log2_cost(7, 49), exp(7.)),
            Variable::new("c8", log2_cost(8, 64), exp(8.)),
            Variable::new("c9", log2_cost(9, 81), exp(9.)),
            Variable::new("c10", log2_cost(10, 100), exp(10.)),
            Variable::new(
                "n",
                exp_cost_log2(fastmath::pow(10., 40.), 60. * fastmath::log2(10.)),
                Value::new_stepwise(6., 16, 1.),
            ),
        ];

        let lpr = core.last_pub_rho;
        let mut forced_pub_rho = f64::INFINITY;
        if lpr < 1480. {
            let seek = if lpr < 100. {
                js_round(lpr * 4.) / 4.
            } else {
                js_round(lpr)
            };
            // a missing entry gives NaN, which never ends the pub (as in sim-3)
            forced_pub_rho = pub_table().get_exact(&js_to_fixed(seek, 2));
        }

        let mut sim = BapSim {
            core,
            strat,
            vars,
            q: [-1e60; 9],
            r: -1e60,
            t_var: 0.,
            forced_pub_rho,
            do_continuity_fork: true,
            best_res: None,
            consts: BapConsts {
                l10_2: l10(2.),
                l10_4: l10(4.),
                l10_25: l10(25.),
                l10_pi2_6: l10(PI * PI / 6.),
                p: fastmath::pow(2., 0.25),
            },
        };
        sim.update_milestones(0., 0.);
        sim
    }

    /** sim-3 `copy()`: the copy's constructor sets milestones from the last pub rho alone */
    fn copy(&self) -> BapSim {
        let mut fork = self.clone();
        fork.best_res = None;
        fork.do_continuity_fork = true;
        let priority = fork.set_milestone_limits(0., 0.);
        fork.core
            .set_milestones_at(fork.core.last_pub_rho.max(0.), priority);
        fork
    }

    /** sim-3 `getMilestonePriority`: also sets the maximum level of each milestone */
    fn set_milestone_limits(&mut self, max_rho: f64, t: f64) -> &'static [usize] {
        let rho = self.core.last_pub_rho.max(max_rho);
        let a_max = binary_insertion_search(&A_POINTS, rho) as i32;
        let q_max = binary_insertion_search(&Q_POINTS, rho) as i32;
        let use_n = i32::from(rho >= 1000. && max_rho >= 940.);
        self.core.milestones_max = vec![1, 1, a_max, q_max, use_n];

        if self.strat.alternates_milestones() && t % 300. < 100. {
            &Q_PRIORITY
        } else {
            &A_PRIORITY
        }
    }

    fn update_milestones(&mut self, max_rho: f64, t: f64) {
        let priority = self.set_milestone_limits(max_rho, t);
        self.core.update_milestones(priority);
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    /** sim-3 `getNextCoast`: the next rho at which buying stops for a while */
    fn next_coast(&self) -> f64 {
        let mut next_coast = self.forced_pub_rho;
        let rho = self.core.max_rho.max(self.core.last_pub_rho);
        for point in COAST_POINTS {
            if next_coast > point && rho < point {
                next_coast = point;
            }
        }
        next_coast
    }

    /** sim-3 `getRdot` */
    fn r_dot(&self, c1: f64, r_ms: bool) -> f64 {
        if c1 <= 2. {
            // exact computation
            let c1 = fastmath::pow(10., c1);
            let mut sum = 0.;
            let mut i = 1.;
            while i < c1 + 0.001 {
                sum += 1. / (i * i);
                i += 1.;
            }
            if r_ms {
                return l10(1. / ((PI * PI) / 6. - sum));
            }
            return l10(sum + (1. / (c1 * c1)));
        }

        let approx_sum = add_log(-c1, -self.consts.l10_2 - 2. * c1);
        if r_ms {
            return if c1 <= 10. {
                // higher accuracy estimate
                -approx_sum
            } else {
                // discard higher order terms to avoid div by 0
                c1
            };
        }
        add_log(sub_log(self.consts.l10_pi2_6, approx_sum), -2. * c1)
    }

    /** sim-3 `getA` */
    fn a_value(level: i32, n_unlocked: bool, n_value: f64) -> f64 {
        if n_unlocked {
            let mut partial_sum = 0.;
            if n_value <= 100. {
                // exact computation
                let mut i = 1.;
                while i <= n_value + 0.01 {
                    partial_sum += 1. / (i * i);
                    i += 1.;
                }
            } else {
                partial_sum = (PI * PI) / 6.
                    - (1. / (n_value + 1.) + 1. / (2. * ((n_value + 1.) * (n_value + 1.))));
            }
            12. / (PI * PI) - 1.0 / partial_sum
        } else {
            let mut a = 0.3;
            let mut i = 9;
            while i > 9 - level {
                a += (i * i) as f64 / 1000.;
                i -= 1;
            }
            a
        }
    }

    fn buying_condition(&self, i: usize) -> bool {
        match self.strat {
            BapStrat::BaP => true,
            BapStrat::BaPcoast => self.core.max_rho < self.next_coast() - self.consts.l10_25,
            BapStrat::BaPd | BapStrat::BaPdMS => {
                i != C1 || {
                    let tdot_level = self.vars[TDOT].level as f64;
                    self.cost(C1) + l10((0.5 * tdot_level) % 64.) < self.cost(C2)
                        && (self.core.milestones[0] > 0 || self.vars[C1].level < 65)
                }
            }
            // The AI strats buy with weights
            _ => false,
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        let ms = &self.core.milestones;
        match i {
            TDOT => self.vars[TDOT].level < 4,
            C1 | C2 => true,
            N => ms[4] > 0,
            // c3 to c10
            _ => ms[3] > i as i32 - 3,
        }
    }

    fn tick(&mut self) {
        let l10dt = l10(self.core.dt);
        self.t_var += (1 + self.vars[TDOT].level) as f64 * self.core.dt;

        let (ms0, ms1, ms2, ms3, ms4) = {
            let ms = &self.core.milestones;
            (ms[0], ms[1], ms[2], ms[3], ms[4])
        };
        if ms3 > 7 {
            self.q[8] = add_log(self.q[8], self.val(C10) + l10dt);
        }
        for i in (2..=9).rev() {
            if ms3 > i as i32 - 3 {
                let above = if ms3 > i as i32 - 2 {
                    self.q[i - 1]
                } else {
                    0.
                };
                self.q[i - 2] = add_log(self.q[i - 2], self.val(i) + above + l10dt);
            }
        }

        self.r = add_log(self.r, self.r_dot(self.val(C1), ms0 > 0) + l10dt);
        let vn = if ms4 > 0 {
            fastmath::pow(10., self.val(N))
        } else {
            0.
        };

        let a = Self::a_value(ms2, ms4 > 0, vn);
        let rhodot = if ms1 == 0 {
            self.core.tot_mult + (l10(self.t_var) + self.q[0] + self.r) * a
        } else {
            self.core.tot_mult + l10(self.t_var) + (self.q[0] + self.r) * a
        };
        self.core.rho += LogNum {
            value: rhodot + l10dt,
            sign: 1,
        };
    }

    /** sim-3 `getVariableWeights` (used by the AI strats) */
    fn variable_weights(&self) -> [f64; VARIABLE_COUNT] {
        let k = &self.consts;
        let ms = &self.core.milestones;
        let next_coast = self.next_coast();
        // costs of c2 and, once n is unlocked, c3
        let min_layer_cost = if ms[4] > 0 {
            self.cost(C2).min(self.cost(C3))
        } else {
            self.cost(C2)
        };
        let next_m64_levels = (64 - ((self.vars[C1].level - 1) % 64)) as f64;
        let next_m64_cost =
            self.cost(C1) + l10((fastmath::pow(k.p, next_m64_levels) - 1.) / (k.p - 1.));
        let coast64 = next_m64_cost < min_layer_cost + k.l10_2 && ms[0] > 0;

        if self.core.max_rho > next_coast - k.l10_25 {
            [f64::INFINITY; VARIABLE_COUNT]
        } else if coast64 {
            let mut weights = [k.l10_4; VARIABLE_COUNT];
            weights[TDOT] = 0.;
            weights[C1] = 0.;
            weights[N] = 0.;
            weights
        } else {
            let mut weights = [0.; VARIABLE_COUNT];
            weights[C1] = if ms[0] > 0 {
                l10((self.vars[C1].level % 64) as f64)
            } else if self.vars[C1].level < 65 {
                k.l10_2
            } else {
                f64::INFINITY
            };
            weights
        }
    }

    /** sim-3 `buyVariablesWeight`: always buys the variable with the lowest cost + weight */
    fn buy_variables_weight(&mut self) {
        loop {
            let weights = self.variable_weights();
            let mut min_cost = f64::MAX;
            let mut min_index = None;
            for i in (0..VARIABLE_COUNT).rev() {
                let weighted = self.cost(i) + weights[i];
                if weighted < min_cost && self.variable_available(i) {
                    min_cost = weighted;
                    min_index = Some(i);
                }
            }
            match min_index {
                Some(i) if self.cost(i) < self.core.rho.value => {
                    let cost = self.vars[i].cost;
                    self.core.rho -= cost;
                    self.core.record_purchase(&self.vars[i]);
                    self.vars[i].buy();
                }
                _ => break,
            }
        }
    }

    fn buy_variables(&mut self) {
        if self.strat.uses_weights() {
            return self.buy_variables_weight();
        }
        for i in (0..VARIABLE_COUNT).rev() {
            while self.core.rho.value > self.cost(i)
                && self.buying_condition(i)
                && self.variable_available(i)
            {
                self.core.record_purchase(&self.vars[i]);
                let cost = self.vars[i].cost;
                self.core.rho -= cost;
                self.vars[i].buy();
            }
        }
    }

    fn simulate(&mut self) -> SimResult {
        self.core.pub_target =
            (self.forced_pub_rho != f64::INFINITY).then_some(self.forced_pub_rho);
        self.core.do_sim_end_conditions = self.forced_pub_rho == f64::INFINITY;

        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            self.update_milestones(self.core.max_rho, self.core.t);
            self.buy_variables();
            if self.forced_pub_rho == 1500. && self.core.max_rho >= 1495. && self.do_continuity_fork
            {
                self.do_continuity_fork = false;
                let mut fork = self.copy();
                fork.forced_pub_rho = f64::INFINITY;
                let res = fork.simulate();
                if res.pub_rho.value >= 1500. {
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
