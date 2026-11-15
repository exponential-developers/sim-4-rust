/*!
 * Theory 7, ported from sim-3 `src/Theories/T1-T8/T7.ts`.
 *
 * Variable order: q1, c1, c2, c3, c4, c5, c6. Only rho_1 is spent; rho_2 just grows.
 */

use crate::api::config::TheoryType;
use crate::theories::traditional::*;
use crate::utils::{
    cost::{Cost, ExponentialCost, FirstFreeCost},
    lognum::LogNum,
    result::SimResult,
    value::Value,
    variable::Variable,
};

const Q1: usize = 0;
const C1: usize = 1;
const C2: usize = 2;
const C3: usize = 3;
const C4: usize = 4;
const C5: usize = 5;
const C6: usize = 6;

const CONVERTER: Converter = Converter {
    r9_affected: true,
    tau_factor: 1.,
    mult_exponent: 0.152,
    mult_factor: 0.,
};

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum T7Strat {
        T7,
        T7Coast,
        T7C12,
        T7C12Coast,
        T7C3,
        T7C3Coast,
        T7noC12,
        T7noC12Coast,
        T7noC123,
        T7noC123Coast,
        T7noC1234,
        T7noC1234Coast,
        T7C12d,
        T7C12dCoast,
        T7C3d,
        T7C3dCoast,
        T7PlaySpqcey,
        T7PlaySpqceyCoast,
    }
}

impl T7Strat {
    fn is_coast(self) -> bool {
        self.name().contains("Coast")
    }

    /** Strat with "Coast" removed (used for the first pass of coasting strats) */
    fn without_coast(self) -> T7Strat {
        T7Strat::from_name(&self.name().replace("Coast", "")).unwrap_or(self)
    }

    /** Order in which milestones are assigned */
    fn milestone_priority(self) -> &'static [usize] {
        use T7Strat::*;
        match self.without_coast() {
            T7 | T7PlaySpqcey => &[1, 0, 2, 3, 4],
            T7C12 | T7C12d => &[4],
            T7C3 | T7C3d => &[1],
            T7noC12 => &[1, 0, 2, 3],
            T7noC123 | T7noC1234 => &[0, 2, 3],
            _ => unreachable!("handled by without_coast"),
        }
    }
}

/** Simulates T7 with the given strat (sim-3 `t7` function) */
pub fn simulate_t7(strat: T7Strat, input: &TheoryInput) -> SimResult {
    if !strat.is_coast() {
        return T7Sim::new(strat, input).simulate();
    }
    let res1 = T7Sim::new(strat.without_coast(), input).simulate();
    let mut sim = T7Sim::new(strat, input);
    sim.vars[Q1].set_original_cap(get_last_level("q1", &res1.bought_vars));
    sim.vars[Q1].configure_cap(13);
    sim.simulate()
}

/** log10 constants used by the sim, computed once per sim */
#[derive(Debug, Clone)]
struct T7Consts {
    l10_1_5: f64,
    l10_0_5: f64,
    l10_4: f64,
    l10_8: f64,
    /** log10(c2ratio) */
    l10_c2ratio: f64,
}

#[derive(Debug, Clone)]
struct T7Sim {
    core: TraditionalCore,
    strat: T7Strat,
    /** Strat with "Coast" removed: the one whose conditions apply */
    base: T7Strat,
    /** Order in which milestones are assigned */
    priority: &'static [usize],
    coast: bool,
    vars: Vec<Variable>,
    /** rho_1 is `core.rho` */
    rho2: f64,
    drho13: f64,
    drho23: f64,
    /** Ratio used by T7PlaySpqcey (infinity: not used) */
    c2ratio: f64,
    consts: T7Consts,
}

impl T7Sim {
    fn new(strat: T7Strat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::T7, true, input, CONVERTER, 10.);
        core.milestone_unlock_steps = 25.;
        core.milestones_max = vec![1, 1, 1, 1, 3];
        // T7 never reads the publication multiplier during the sim
        core.track_cur_mult = false;

        let exp2 = || Value::new_exponential(2.);
        let mut q1 = Variable::new(
            "q1",
            Cost::new(FirstFreeCost::new(ExponentialCost::new(500., 1.51572))),
            Value::new_stepwise(2., 10, 0.),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        q1.buy();

        let vars = vec![
            q1,
            Variable::new(
                "c1",
                Cost::new_exponential(10., 1.275),
                Value::new_stepwise(2., 10, 1.),
            ),
            Variable::new("c2", Cost::new_exponential(40., 8.), exp2()),
            Variable::new("c3", Cost::new_exponential(1e5, 63.), exp2()),
            Variable::new("c4", Cost::new_exponential(10., 2.82), exp2()),
            Variable::new("c5", Cost::new_exponential(1e8, 60.), exp2()),
            Variable::new("c6", Cost::new_exponential(1e2, 2.81), exp2()),
        ];

        let lpr = core.last_pub_rho;
        let mut c2ratio = f64::INFINITY;
        if lpr >= 100. {
            c2ratio = 100.;
        }
        if lpr >= 175. {
            c2ratio = 10.;
        }
        if lpr >= 250. {
            c2ratio = 20.;
        }
        if lpr >= 275. {
            c2ratio = 50.;
        }
        if lpr >= 300. {
            c2ratio = f64::INFINITY;
        }

        let mut sim = T7Sim {
            core,
            strat,
            base: strat.without_coast(),
            priority: strat.milestone_priority(),
            coast: strat.is_coast(),
            vars,
            rho2: f64::NEG_INFINITY,
            drho13: 0.,
            drho23: 0.,
            c2ratio,
            consts: T7Consts {
                l10_1_5: l10(1.5),
                l10_0_5: l10(0.5),
                l10_4: l10(4.),
                l10_8: l10(8.),
                l10_c2ratio: l10(c2ratio),
            },
        };
        sim.core.update_milestones(sim.priority);
        sim
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    fn buying_condition(&self, i: usize) -> bool {
        use T7Strat::*;
        let c = |j: usize| self.cost(j);
        let k = &self.consts;
        // Only q1 is ever stopped (by coasting forks), so this is true for every other variable
        // and for every strat that isn't coasting
        if !self.vars[i].should_buy {
            return false;
        }
        match self.base {
            T7 => true,
            T7C12 => matches!(i, Q1 | C1 | C2),
            T7C3 => matches!(i, Q1 | C3),
            T7noC12 => matches!(i, Q1 | C3 | C4 | C5 | C6),
            T7noC123 => matches!(i, Q1 | C4 | C5 | C6),
            T7noC1234 => matches!(i, Q1 | C5 | C6),
            T7C12d => match i {
                Q1 => c(Q1) + 1. < c(C2),
                C1 => c(C1) + k.l10_8 < c(C2),
                C2 => true,
                _ => false,
            },
            T7C3d => match i {
                Q1 => c(Q1) + 1. < c(C3),
                C3 => true,
                _ => false,
            },
            T7PlaySpqcey => match i {
                Q1 => c(Q1) + k.l10_4 < c(C6),
                C1 => c(C1) + l10(10. + self.vars[C2].level as f64) < c(C2),
                C2 => c(C2) + k.l10_c2ratio < c(C6),
                C3 | C4 => c(i) + 1. < c(C6),
                C5 => c(C5) + k.l10_4 < c(C6),
                _ => true,
            },
            _ => unreachable!("handled by without_coast"),
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        let ms = &self.core.milestones;
        match i {
            C3 => ms[1] > 0,
            C4 => ms[0] > 0,
            C5 => ms[2] > 0,
            C6 => ms[3] > 0,
            _ => true,
        }
    }

    fn tick(&mut self) {
        let ms = &self.core.milestones;
        let vc1 = self.val(C1) * (1. + 0.05 * ms[4] as f64);
        let rho = js_max(self.core.rho.value, 0.);
        let rho2 = js_max(self.rho2, 0.);
        let k = &self.consts;

        let drho11 = vc1 + self.val(C2);
        let drho12 = if ms[1] > 0 {
            k.l10_1_5 + self.val(C3) + rho / 2.
        } else {
            0.
        };
        let drho21 = if ms[0] > 0 { self.val(C4) } else { 0. };
        let drho22 = if ms[2] > 0 {
            k.l10_1_5 + self.val(C5) + rho2 / 2.
        } else {
            0.
        };
        let v6 = self.val(C6);
        let (drho13, drho23) = if ms[3] > 0 {
            (
                js_min(
                    self.drho13 + 2.,
                    js_min(k.l10_0_5 + v6 + rho2 / 2. - rho / 2., rho + 2.),
                ),
                js_min(
                    self.drho23 + 2.,
                    js_min(k.l10_0_5 + v6 + rho / 2. - rho2 / 2., rho2 + 2.),
                ),
            )
        } else {
            (0., 0.)
        };
        self.drho13 = drho13;
        self.drho23 = drho23;
        let dtq1bonus = l10(self.core.dt) + self.val(Q1) + self.core.tot_mult;

        self.core.rho += LogNum {
            value: dtq1bonus + add_log(add_log(drho11, drho12), drho13),
            sign: 1,
        };
        self.rho2 = add_log(
            self.rho2,
            dtq1bonus + add_log(add_log(drho21, drho22), drho23),
        );
    }

    fn buy_variables(&mut self) {
        // T7 has no onAnyVariablePurchased step, so whether something was bought is not tracked
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
        if id == Q1 && self.coast {
            let v = &mut self.vars[id];
            if v.should_buy && v.coasting_cap_reached() {
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
            .set_milestones_at(fork.core.last_pub_rho.max(0.), fork.priority);
        fork.vars[id].stop_buying();
        let res = fork.simulate();
        self.core.best_fork_res = Some(best_result(res, self.core.best_fork_res.take()));
    }

    fn simulate(&mut self) -> SimResult {
        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            if self.core.last_pub_rho < 175. {
                self.core.update_milestones(self.priority);
            }
            self.buy_variables();
            if self.vars[Q1].should_fork {
                self.do_fork_variable(Q1);
            }
            self.core.collect_pub_table();
        }
        self.core.trim_bought_vars();

        let mut extra = String::new();
        if self.strat.name().contains("T7PlaySpqcey") && self.c2ratio != f64::INFINITY {
            // JavaScript prints the number: 100, 10, 20 or 50
            extra = format!("{}", self.c2ratio);
        }
        if self.coast {
            extra +=
                &self.vars[Q1].prepare_extra_for_cap(get_last_level("q1", &self.core.bought_vars));
        }
        let res = self.core.create_result(self.strat.name(), &extra);
        best_result(res, self.core.best_fork_res.take())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(rho: f64, sigma: i64) -> TheoryInput {
        TheoryInput {
            sigma,
            last_pub_rho: rho,
            rho_cap: f64::INFINITY,
            params: SimParams::default(),
        }
    }

    #[test]
    fn c2_ratio_depends_on_the_last_pub_rho() {
        for (rho, ratio) in [
            (50., f64::INFINITY),
            (120., 100.),
            (200., 10.),
            (260., 20.),
            (280., 50.),
            (320., f64::INFINITY),
        ] {
            assert_eq!(
                T7Sim::new(T7Strat::T7PlaySpqcey, &input(rho, 100)).c2ratio,
                ratio,
                "{rho}"
            );
        }
    }

    #[test]
    fn spqcey_strats_report_the_c2_ratio() {
        let res = simulate_t7(T7Strat::T7PlaySpqcey, &input(200., 100));
        assert!(res.strat.starts_with("T7PlaySpqcey10"), "{}", res.strat);
        let res = simulate_t7(T7Strat::T7PlaySpqceyCoast, &input(200., 100));
        assert!(res.strat.contains("Coast10 q1: "), "{}", res.strat);
        let res = simulate_t7(T7Strat::T7PlaySpqcey, &input(320., 100));
        assert_eq!(res.strat, "T7PlaySpqcey");
    }

    #[test]
    fn variables_need_their_milestones() {
        // priority [1, 0, 2, 3, 4], one milestone per e25: c3, then c4, then c5, then c6
        let sim = T7Sim::new(T7Strat::T7, &input(30., 40));
        assert!(sim.variable_available(C3) && !sim.variable_available(C4));
        let sim = T7Sim::new(T7Strat::T7, &input(60., 60));
        assert!(sim.variable_available(C4) && !sim.variable_available(C5));
        let sim = T7Sim::new(T7Strat::T7, &input(80., 60));
        assert!(sim.variable_available(C5) && !sim.variable_available(C6));
        let sim = T7Sim::new(T7Strat::T7, &input(100., 60));
        assert!(sim.variable_available(C6));
    }

    #[test]
    fn coasting_strats_report_q1_and_never_lose_to_nothing() {
        let res = simulate_t7(T7Strat::T7C12dCoast, &input(150., 90));
        assert!(res.tau_h > 0. && res.strat.contains(" q1: "), "{res:?}");
        assert_eq!(T7Strat::T7noC1234Coast.without_coast(), T7Strat::T7noC1234);
        assert_eq!(T7Strat::T7.without_coast(), T7Strat::T7);
    }
}
