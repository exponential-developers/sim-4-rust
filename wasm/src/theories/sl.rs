/*!
 * Theory SL, ported from sim-3 `src/Theories/CTs/SL.ts`.
 *
 * Variable order: a1, a2, b1, b2
 */

use std::f64::consts::{LOG10_E, PI};

use crate::api::config::TheoryType;
use crate::theories::traditional::*;
use crate::utils::{
    cost::{Cost, FirstFreeCost},
    fastmath,
    lognum::LogNum,
    result::SimResult,
    value::Value,
    variable::Variable,
};

const A1: usize = 0;
const A2: usize = 1;
const B1: usize = 2;
const B2: usize = 3;

const CONVERTER: Converter = Converter {
    r9_affected: false,
    tau_factor: 0.4,
    mult_exponent: 0.375,
    mult_factor: 0.,
};

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SlStrat {
        SL,
        SLStopA,
        SLCoast,
        SLStopAd,
        SLdCoast,
        SLMS,
        SLMSd,
    }
}

impl SlStrat {
    fn is_coast(self) -> bool {
        matches!(self, SlStrat::SLCoast | SlStrat::SLdCoast)
    }

    fn uses_milestone_swaps(self) -> bool {
        matches!(self, SlStrat::SLMS | SlStrat::SLMSd)
    }
}

/** Simulates SL with the given strat (sim-3 `sl` function) */
pub fn simulate_sl(strat: SlStrat, input: &TheoryInput) -> SimResult {
    if !strat.is_coast() {
        return SlSim::new(strat, input).simulate();
    }
    let first_pass = if strat == SlStrat::SLCoast {
        SlStrat::SLStopA
    } else {
        SlStrat::SLStopAd
    };
    let res1 = SlSim::new(first_pass, input).simulate();
    let mut sim = SlSim::new(strat, input);
    for (id, name) in [(A1, "a1"), (A2, "a2"), (B1, "b1"), (B2, "b2")] {
        sim.vars[id].set_original_cap(get_last_level(name, &res1.bought_vars));
    }

    let rho = input.last_pub_rho;
    if strat == SlStrat::SLCoast {
        // These are based on sim results:
        let a1 = if rho <= 10. {
            5
        } else if rho <= 110. {
            4
        } else if rho <= 200. {
            3
        } else {
            2
        };
        sim.vars[A1].configure_cap(a1);
        sim.vars[A2].configure_cap(1);
        let b1 = if rho <= 50. {
            3
        } else if rho <= 100. {
            2
        } else {
            1
        };
        sim.vars[B1].configure_cap(b1);
        sim.vars[B2].configure_cap(if rho <= 150. || rho >= 1400. { 1 } else { 0 });
    } else {
        let a1 = if rho <= 110. {
            4
        } else if rho <= 220. {
            3
        } else {
            2
        };
        sim.vars[A1].configure_cap(a1);
        sim.vars[A2].configure_cap(1);
        let b1 = if rho <= 50. {
            3
        } else if rho <= 120. {
            2
        } else {
            1
        };
        sim.vars[B1].configure_cap(b1);
        sim.vars[B2].configure_cap(if rho <= 120. || rho >= 1450. { 1 } else { 0 });
    }
    sim.simulate()
}

/** Order in which milestones are assigned when no swap applies */
const DEFAULT_PRIORITY: [usize; 4] = [3, 2, 0, 1];

/** log10 constants, computed once per sim */
#[derive(Debug, Clone)]
struct SlConsts {
    l10_2: f64,
    l10_6: f64,
    l10_pi: f64,
    /** log10(2 * n + 1) for n = a1 level % 3 */
    l10_odd: [f64; 3],
    /** log10(1 + n) for n = b1 level % 4 */
    l10_1_plus: [f64; 4],
}

#[derive(Debug, Clone)]
struct SlSim {
    core: TraditionalCore,
    strat: SlStrat,
    coast: bool,
    vars: Vec<Variable>,
    rho2: f64,
    rho3: f64,
    inverse_e_gamma: f64,
    consts: SlConsts,
}

impl SlSim {
    fn new(strat: SlStrat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::SL, false, input, CONVERTER, 10.);
        core.milestone_unlock_steps = 25.;
        core.milestones_max = vec![3, 5, 2, 2];
        // The sim also ends when the pub multiplier gets too large
        core.sim_end_cur_mult = Some(15.);

        let l2_10 = fastmath::log2(10.);
        let mut a1 = Variable::new(
            "a1",
            Cost::new(FirstFreeCost::new(exp_cost_log2_model(1., 0.369 * l2_10))),
            Value::new_stepwise(3.5, 3, 0.),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        a1.buy();

        let vars = vec![
            a1,
            Variable::new("a2", exp_cost(175., 10.), Value::new_exponential(2.)),
            Variable::new(
                "b1",
                exp_cost_log2(500., 0.649 * l2_10),
                Value::new_stepwise(6.5, 4, 0.),
            ),
            Variable::new(
                "b2",
                exp_cost_log2(1000., 0.926 * l2_10),
                Value::new_exponential(2.),
            ),
        ];

        let mut sim = SlSim {
            core,
            strat,
            coast: strat.is_coast(),
            vars,
            rho2: 0.,
            rho3: 0.,
            inverse_e_gamma: 0.,
            consts: SlConsts {
                l10_2: l10(2.),
                l10_6: l10(6.),
                l10_pi: l10(PI),
                l10_odd: std::array::from_fn(|n| l10(2. * n as f64 + 1.)),
                l10_1_plus: std::array::from_fn(|n| l10(1. + n as f64)),
            },
        };
        sim.update_milestones();
        sim
    }

    /** sim-3 `getMilestonePriority` (the swaps only apply to the MS strats) */
    fn milestone_priority(&self) -> &'static [usize] {
        let max_val = self.core.last_pub_rho.max(self.core.max_rho);
        if self.strat.uses_milestone_swaps() && (25. ..=300.).contains(&max_val) {
            // when to swap to a3exp (increasing rho2dot) before b1b2
            let (emg_before_b1b2, r2exp_before_b1b2) = if max_val < 50. {
                (5., 4.)
            } else if max_val < 75. {
                (7., 6.)
            } else if max_val < 100. {
                (12., 10.)
            } else if max_val < 150. {
                (20., 15.)
            } else if max_val < 175. {
                (8., 6.)
            } else if max_val < 200. {
                (1.5, 1.)
            } else if max_val < 275. {
                (3., 3.)
            } else if max_val < 300. {
                (2., 2.)
            } else {
                // exactly 300: the defaults
                (5., 4.)
            };
            let min_cost = self.cost(B1).min(self.cost(B2));
            let rho = self.core.rho.value;
            if rho + l10(emg_before_b1b2) < min_cost {
                // b12 exp
                return &[3, 2, 0, 1];
            } else if self.core.cur_mult > 4.5 || rho + l10(r2exp_before_b1b2) > min_cost {
                // rho2 exp
                return &[0, 1, 3, 2];
            } else if rho + l10(emg_before_b1b2) > min_cost
                && rho + l10(r2exp_before_b1b2) < min_cost
            {
                // a3 boost
                return &[1, 0, 3, 2];
            }
        }
        &DEFAULT_PRIORITY
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

    /** a1's cost relative to a2's (the "d" strats) */
    fn a1_cheap(&self) -> bool {
        let lvl = (self.vars[A1].level % 3) as usize;
        self.cost(A1) + self.consts.l10_odd[lvl] < self.cost(A2)
    }

    /** b1's cost relative to b2's (the "d" strats) */
    fn b1_cheap(&self) -> bool {
        let lvl = (self.vars[B1].level % 4) as usize;
        self.cost(B1) + self.consts.l10_1_plus[lvl] < self.cost(B2)
    }

    fn buying_condition(&self, i: usize) -> bool {
        use SlStrat::*;
        let cm = self.core.cur_mult;
        let ok = |j: usize| self.vars[j].should_buy;
        match self.strat {
            SL => true,
            SLStopA => match i {
                A1 | A2 => cm < 4.5,
                _ => cm < 6.,
            },
            SLCoast => ok(i),
            SLStopAd => match i {
                A1 => cm < 4.5 && self.a1_cheap(),
                A2 => cm < 4.5,
                B1 => cm < 6. && self.b1_cheap(),
                _ => cm < 6.,
            },
            SLdCoast => match i {
                A1 => ok(A1) && self.a1_cheap(),
                B1 => ok(B1) && self.b1_cheap(),
                _ => ok(i),
            },
            SLMS => match i {
                A1 | A2 => cm < 4.,
                _ => cm < 7.5,
            },
            SLMSd => match i {
                A1 => cm < 4. && self.a1_cheap(),
                A2 => cm < 4.,
                B1 => cm < 7.5 && self.b1_cheap(),
                _ => cm < 7.5,
            },
        }
    }

    /** sim-3 `updateInverseE_Gamma` */
    fn update_inverse_e_gamma(&mut self, x: f64) {
        let k = &self.consts;
        let y = l10(k.l10_2 / LOG10_E + x / LOG10_E + k.l10_pi / LOG10_E) - (k.l10_2 + x);
        self.inverse_e_gamma =
            0. - LOG10_E - add_log(sub_log(y, y + y - k.l10_2), y + y + y + k.l10_6);
    }

    fn tick(&mut self) {
        let ms = &self.core.milestones;
        let rho3dot =
            self.val(B1) * (1. + 0.02 * ms[2] as f64) + self.val(B2) * (1. + 0.02 * ms[3] as f64);
        let l10dt = l10(self.core.dt);
        self.rho3 = add_log(self.rho3, rho3dot + l10dt);
        self.update_inverse_e_gamma(js_max(1., self.rho3));

        let ms = &self.core.milestones;
        let rho2dot = LOG10_E
            * (self.val(A1) / LOG10_E + self.val(A2) / LOG10_E
                - fastmath::ln(2. - 0.008 * ms[1] as f64) * (js_max(1., self.rho3) / LOG10_E));
        self.rho2 = add_log(self.rho2, js_max(0., rho2dot) + l10dt);

        let rhodot = self.rho2 * (1. + ms[0] as f64 * 0.02) * 0.5 + self.inverse_e_gamma;
        self.core.rho += LogNum {
            value: rhodot + self.core.tot_mult + l10dt,
            sign: 1,
        };
    }

    fn buy_variables(&mut self) {
        // SL has no onAnyVariablePurchased step, so whether something was bought is not tracked
        for i in (0..self.vars.len()).rev() {
            while self.core.rho.value > self.cost(i) && self.buying_condition(i) {
                self.core.record_purchase(&self.vars[i]);
                let cost = self.vars[i].cost;
                self.core.rho -= cost;
                self.vars[i].buy();
                self.on_variable_purchased(i);
            }
        }
    }

    fn on_variable_purchased(&mut self, id: usize) {
        if self.coast && self.vars[id].should_buy && self.vars[id].coasting_cap_reached() {
            let extra = if self.strat == SlStrat::SLCoast {
                if id == A1 { 4 } else { 2 }
            } else if id == A1 {
                3
            } else {
                2
            };
            let v = &mut self.vars[id];
            if (v.level as f64) < v.original_cap + extra as f64 {
                v.should_fork = true;
            } else {
                v.stop_buying();
            }
        }
    }

    /** Simulates a fork where variable `id` is no longer bought (sim-3 `doForkVariable`) */
    fn do_fork_variable(&mut self, id: usize) {
        self.vars[id].should_fork = false;
        let mut fork = self.clone();
        // sim-3 builds the fork with its constructor, which computes milestones from the last pub
        // rho with rho still at -infinity: the first branch of the priority always applies
        fork.core
            .set_milestones_at(fork.core.last_pub_rho.max(0.), &DEFAULT_PRIORITY);
        fork.vars[id].stop_buying();
        let res = fork.simulate();
        self.core.best_fork_res = Some(best_result(res, self.core.best_fork_res.take()));
    }

    fn simulate(&mut self) -> SimResult {
        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            if self.core.last_pub_rho < 300. {
                self.update_milestones();
            }
            self.buy_variables();
            for id in 0..self.vars.len() {
                if self.vars[id].should_fork {
                    self.do_fork_variable(id);
                }
            }
            self.core.collect_pub_table();
        }
        self.core.trim_bought_vars();

        let mut extra = String::new();
        if self.coast {
            let bv = &self.core.bought_vars;
            for (id, name) in [(A1, "a1"), (A2, "a2"), (B1, "b1"), (B2, "b2")] {
                extra += &self.vars[id].prepare_extra_for_cap(get_last_level(name, bv));
            }
        }
        let res = self.core.create_result(self.strat.name(), &extra);
        best_result(res, self.core.best_fork_res.take())
    }
}
