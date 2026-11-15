/*!
 * Weierstrass Sine Product, ported from sim-3 `src/Theories/CTs/WSP.ts`.
 *
 * Variable order: q1, q2, n, c1, c2
 */

use std::f64::consts::{LOG10_E, PI};
use std::sync::OnceLock;

use crate::api::config::TheoryType;
use crate::theories::pubtable::{CodedTable, PubTable};
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
const N: usize = 2;
const C1: usize = 3;
const C2: usize = 4;

/** Order in which milestones are assigned: [q1exp, c2 unlock, n-based c1 exponent] */
const MILESTONE_PRIORITY: [usize; 3] = [2, 1, 0];

/** Rho at which each milestone is unlocked */
const MILESTONE_UNLOCKS: [f64; 8] = [10., 25., 40., 55., 70., 100., 140., 200.];

const CONVERTER: Converter = Converter {
    r9_affected: false,
    tau_factor: 0.4,
    mult_exponent: 0.375,
    mult_factor: 0.,
};

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum WspStrat {
        WSP,
        WSPStopC1,
        WSPStopC1Coast,
        WSPPTStopC1Coast,
        WSPdStopC1,
        WSPdStopC1Coast,
        WSPdPTStopC1Coast,
        WSPPostRecoveryStopC1Coast,
    }
}

impl WspStrat {
    fn is_coast(self) -> bool {
        self.name().contains("Coast")
    }

    /** Active ("d") strats */
    fn is_active(self) -> bool {
        self.name().contains("WSPd")
    }

    fn uses_pub_table(self) -> bool {
        matches!(
            self,
            WspStrat::WSPPTStopC1Coast | WspStrat::WSPdPTStopC1Coast
        )
    }

    /** Strat of the first pass of coasting strats ("PT", "Coast" and "PostRecovery" removed) */
    fn first_pass(self) -> WspStrat {
        let name = self
            .name()
            .replacen("PT", "", 1)
            .replacen("Coast", "", 1)
            .replacen("PostRecovery", "", 1);
        WspStrat::from_name(&name).unwrap_or(self)
    }
}

/** Publication tables: the active one for the "d" strats, the passive one for the others */
fn pub_table(active: bool) -> &'static PubTable {
    static ACTIVE: OnceLock<PubTable> = OnceLock::new();
    static PASSIVE: OnceLock<PubTable> = OnceLock::new();
    if active {
        ACTIVE.get_or_init(|| {
            PubTable::decode_format1(&CodedTable::parse(include_str!(
                "data/table_wsp_0_1_active_coast_coded.json"
            )))
        })
    } else {
        PASSIVE.get_or_init(|| {
            PubTable::decode_format1(&CodedTable::parse(include_str!(
                "data/table_wsp_0_1_passive_coast_coded.json"
            )))
        })
    }
}

/** Simulates WSP with the given strat (sim-3 `wsp` function) */
pub fn simulate_wsp(strat: WspStrat, input: &TheoryInput) -> SimResult {
    if !strat.is_coast() {
        return WspSim::new(strat, input).simulate();
    }
    let res1 = WspSim::new(strat.first_pass(), input).simulate();
    let mut sim = WspSim::new(strat, input);
    sim.vars[Q1].set_original_cap(get_last_level("q1", &res1.bought_vars));
    if strat.is_active() {
        // For WSPd, it is always either skip 0, 1 or 2.
        sim.vars[Q1].configure_cap(2);
    } else if input.last_pub_rho >= 300. {
        sim.vars[Q1].configure_cap(10);
    } else {
        sim.vars[Q1].configure_cap(19);
    }
    sim.simulate()
}

/** sim-3 `srK_helper` */
fn sr_k_helper(x: f64) -> f64 {
    let x2 = x * x;
    fastmath::ln(x2 + 1. / 6. + 1. / 120. / x2 + 1. / 810. / x2 / x2) / 2. - 1.
}

/** sim-3 `sineRatioK` (K = 5) */
fn sine_ratio_k(n: f64, x: f64) -> f64 {
    const K: f64 = 5.;
    if n < 1. || x >= n + 1. {
        return 0.;
    }
    let big_n = n + 1. + K;
    let x2 = x * x;
    let l1 = sr_k_helper(big_n + x);
    let l2 = sr_k_helper(big_n - x);
    let l3 = sr_k_helper(big_n);
    let mut result =
        big_n * (l1 + l2 - 2. * l3) + x * (l1 - l2) - fastmath::ln(1. - x2 / big_n / big_n) / 2.;
    let mut k = n + 1.;
    while k < big_n {
        result -= fastmath::ln(1. - x2 / k / k);
        k += 1.;
    }
    LOG10_E * result
}

/** log10 constants used by the buying conditions, computed once per sim */
#[derive(Debug, Clone)]
struct WspConsts {
    l10_3: f64,
    /** Weight of c1's cost, by last pub rho */
    c1_weight: f64,
    /** log10(6 + n) for n = q1 level % 10 */
    l10_6_plus: [f64; 10],
}

#[derive(Debug, Clone)]
struct WspSim {
    core: TraditionalCore,
    strat: WspStrat,
    coast: bool,
    vars: Vec<Variable>,
    q: f64,
    /** Sine ratio, updated when n or c2 is bought */
    s: f64,
    update_s_flag: bool,
    consts: WspConsts,
}

impl WspSim {
    fn new(strat: WspStrat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::WSP, false, input, CONVERTER, 8.);
        core.milestone_unlocks = MILESTONE_UNLOCKS.to_vec();
        core.milestones_max = vec![4, 1, 3];

        let mut q1 = Variable::new(
            "q1",
            Cost::new(FirstFreeCost::new(exp_cost_log2_model(10., 3.38 / 4.))),
            Value::new_stepwise(2., 10, 0.),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        q1.buy();

        let vars = vec![
            q1,
            Variable::new(
                "q2",
                exp_cost_log2(1000., 3.38 * 3.),
                Value::new_exponential(2.),
            ),
            Variable::new("n", exp_cost_log2(20., 3.38), Value::new_exponential(10.)),
            Variable::new(
                "c1",
                exp_cost_log2(50., 3.38 / 1.5),
                Value::new_stepwise(2., 50, 1.),
            ),
            Variable::new(
                "c2",
                exp_cost_log2(1e10, 3.38 * 10.),
                Value::new_exponential(2.),
            ),
        ];

        let lpr = core.last_pub_rho;
        let mut c1_weight = 0.;
        if lpr >= 25. {
            c1_weight = l10(3.);
        }
        if lpr >= 40. {
            c1_weight = 1.;
        }
        if lpr >= 200. {
            c1_weight = l10(50.);
        }
        if lpr >= 400. {
            c1_weight = 3.;
        }
        if lpr >= 700. {
            c1_weight = 10000.;
        }

        if strat.uses_pub_table() && lpr < 1499. {
            core.do_sim_end_conditions = false;
            core.pub_target = Some(pub_table(strat.is_active()).target(lpr, 10.));
        }
        // The sim also ends when the pub multiplier gets too large
        core.sim_end_cur_mult = Some(15.);

        let mut sim = WspSim {
            core,
            strat,
            coast: strat.is_coast(),
            vars,
            q: 0.,
            s: 0.,
            update_s_flag: false,
            consts: WspConsts {
                l10_3: l10(3.),
                c1_weight,
                l10_6_plus: std::array::from_fn(|n| l10(6. + n as f64)),
            },
        };
        sim.core.update_milestones(&MILESTONE_PRIORITY);
        sim
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    /** q1's cost is low enough compared to the other variables (the "d" strats) */
    fn q1_cheap(&self) -> bool {
        let c2 = if self.core.milestones[1] > 0 {
            self.cost(C2)
        } else {
            f64::INFINITY
        };
        let lvl = (self.vars[Q1].level % 10) as usize;
        self.cost(Q1) + self.consts.l10_6_plus[lvl] < self.cost(Q2).min(self.cost(N)).min(c2)
    }

    /** c1's cost is low enough compared to the other variables (the "d" strats) */
    fn c1_cheap(&self) -> bool {
        let c2 = if self.core.milestones[1] > 0 {
            self.cost(C2)
        } else {
            f64::INFINITY
        };
        self.cost(C1) + self.consts.c1_weight < self.cost(Q2).min(self.cost(N)).min(c2)
            || self.core.t < 15.
    }

    /** c1 is bought early in the publication or while the last pub is low (the passive strats) */
    fn c1_early(&self) -> bool {
        self.core.last_pub_rho < 450. || self.core.t < 15.
    }

    fn buying_condition(&self, i: usize) -> bool {
        use WspStrat::*;
        let q1_ok = self.vars[Q1].should_buy;
        match self.strat {
            WSP => true,
            WSPStopC1 => i != C1 || self.c1_early(),
            WSPStopC1Coast | WSPPTStopC1Coast => match i {
                Q1 => q1_ok,
                C1 => self.c1_early(),
                _ => true,
            },
            WSPPostRecoveryStopC1Coast => {
                let recovering = self.core.max_rho <= self.core.last_pub_rho;
                match i {
                    Q1 => q1_ok && (recovering || self.q1_cheap()),
                    C1 => {
                        if recovering {
                            self.c1_early()
                        } else {
                            self.c1_cheap()
                        }
                    }
                    _ => true,
                }
            }
            WSPdStopC1 => match i {
                Q1 => self.q1_cheap(),
                C1 => self.c1_cheap(),
                _ => true,
            },
            WSPdStopC1Coast | WSPdPTStopC1Coast => match i {
                Q1 => q1_ok && self.q1_cheap(),
                C1 => self.c1_cheap(),
                _ => true,
            },
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        i != C2 || self.core.milestones[1] > 0
    }

    fn update_s(&mut self) {
        let vn = l10(self.val(N));
        let vc1 = self.val(C1);
        let chi = fastmath::exp10(
            l10(PI) + vc1 + vn
                - add_log(vc1, vn - self.consts.l10_3 * self.core.milestones[2] as f64),
        ) + 1.;
        self.s = sine_ratio_k(self.val(N), chi / PI);
    }

    fn tick(&mut self) {
        if self.update_s_flag {
            self.update_s_flag = false;
            self.update_s();
        }

        let vq1 = self.val(Q1) * (1. + 0.01 * self.core.milestones[0] as f64);
        let l10dt = l10(self.core.dt);
        let qdot = js_max(0., l10dt + self.s + self.val(C2));
        self.q = add_log(self.q, qdot);

        let rhodot = self.core.tot_mult + vq1 + self.val(Q2) + self.q + l10dt;
        self.core.rho += LogNum {
            value: rhodot,
            sign: 1,
        };
    }

    fn buy_variables(&mut self) {
        // WSP has no onAnyVariablePurchased step, so whether something was bought is not tracked
        for i in (0..self.vars.len()).rev() {
            while self.core.rho.value > self.cost(i)
                && self.buying_condition(i)
                && self.variable_available(i)
            {
                self.core.record_purchase(&self.vars[i]);
                let cost = self.vars[i].cost;
                self.core.rho -= cost;
                self.vars[i].buy();
                self.on_variable_purchased(i);
            }
        }
    }

    fn on_variable_purchased(&mut self, id: usize) {
        if id == N || id == C2 {
            self.update_s_flag = true;
        }
        if id == Q1 && self.coast {
            let v = &mut self.vars[id];
            // For WSP: no need to go over original cap
            if v.should_buy && v.coasting_cap_reached() && !v.above_original_cap() {
                v.should_fork = true;
            }
        }
    }

    /** Simulates a fork where variable `id` is no longer bought (sim-3 `doForkVariable`) */
    fn do_fork_variable(&mut self, id: usize) {
        self.vars[id].should_fork = false;
        let mut fork = self.clone();
        // sim-3 builds the fork with its constructor, which computes milestones from the
        // last pub rho only; copyFrom does not copy milestones.
        fork.core
            .set_milestones_at(fork.core.last_pub_rho.max(0.), &MILESTONE_PRIORITY);
        fork.vars[id].stop_buying();
        let res = fork.simulate();
        self.core.best_fork_res = Some(best_result(res, self.core.best_fork_res.take()));
    }

    fn simulate(&mut self) -> SimResult {
        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            if self.core.last_pub_rho < 200. {
                self.core.update_milestones(&MILESTONE_PRIORITY);
            }
            self.buy_variables();
            if self.vars[Q1].should_fork {
                self.do_fork_variable(Q1);
            }
            self.core.collect_pub_table();
        }
        self.core.trim_bought_vars();

        let mut extra = String::new();
        if self.coast {
            extra +=
                &self.vars[Q1].prepare_extra_for_cap(get_last_level("q1", &self.core.bought_vars));
        }
        let res = self.core.create_result(self.strat.name(), &extra);
        best_result(res, self.core.best_fork_res.take())
    }
}
