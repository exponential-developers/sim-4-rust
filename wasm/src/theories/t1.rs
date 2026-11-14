/**
 * Theory 1, ported from sim-3 `src/Theories/T1-T8/T1.ts`.
 *
 * Variable order: q1, q2, c1, c2, c3, c4
 */

use std::f64::consts::LOG10_E;

use crate::api::config::TheoryType;
use crate::theories::traditional::*;
use crate::utils::{
    fastmath,
    cost::{Cost, ExponentialCost, FirstFreeCost},
    lognum::LogNum,
    result::SimResult,
    value::Value,
    variable::Variable,
};

const Q1: usize = 0;
const Q2: usize = 1;
const C1: usize = 2;
const C2: usize = 3;
const C3: usize = 4;
const C4: usize = 5;

/** Order in which milestones are assigned: [logterm, c1exp, c3term, c4term] */
const MILESTONE_PRIORITY: [usize; 4] = [2, 3, 0, 1];

const CONVERTER: Converter = Converter {
    r9_affected: true,
    tau_factor: 1.,
    mult_exponent: 0.164,
    // -log10(3)
    mult_factor: -0.47712125471966244,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum T1Strat {
    T1,
    T1Coast,
    T1C34,
    T1C34Coast,
    T1C4,
    T1C4Coast,
    T1Ratio,
    T1RatioCoast,
    T1SolarXLII,
    T1SolarXLIIOld,
    T1SolarXLIIOldCoast,
}

impl T1Strat {
    pub const ALL: [T1Strat; 11] = [
        T1Strat::T1,
        T1Strat::T1Coast,
        T1Strat::T1C34,
        T1Strat::T1C34Coast,
        T1Strat::T1C4,
        T1Strat::T1C4Coast,
        T1Strat::T1Ratio,
        T1Strat::T1RatioCoast,
        T1Strat::T1SolarXLII,
        T1Strat::T1SolarXLIIOld,
        T1Strat::T1SolarXLIIOldCoast,
    ];

    pub fn name(self) -> &'static str {
        match self {
            T1Strat::T1 => "T1",
            T1Strat::T1Coast => "T1Coast",
            T1Strat::T1C34 => "T1C34",
            T1Strat::T1C34Coast => "T1C34Coast",
            T1Strat::T1C4 => "T1C4",
            T1Strat::T1C4Coast => "T1C4Coast",
            T1Strat::T1Ratio => "T1Ratio",
            T1Strat::T1RatioCoast => "T1RatioCoast",
            T1Strat::T1SolarXLII => "T1SolarXLII",
            T1Strat::T1SolarXLIIOld => "T1SolarXLIIOld",
            T1Strat::T1SolarXLIIOldCoast => "T1SolarXLIIOldCoast",
        }
    }

    pub fn from_name(name: &str) -> Option<T1Strat> {
        T1Strat::ALL.into_iter().find(|s| s.name() == name)
    }

    fn is_coast(self) -> bool {
        self.name().contains("Coast")
    }

    fn is_old(self) -> bool {
        self.name().contains("Old")
    }

    /** Strats using the "new" pub cycle and the two-pass coasting setup */
    fn is_new_cycle(self) -> bool {
        matches!(self, T1Strat::T1SolarXLII | T1Strat::T1C34Coast | T1Strat::T1C4Coast)
    }

    /** Strat with "Coast" removed (used for the first pass of coasting strats) */
    fn without_coast(self) -> T1Strat {
        match self {
            T1Strat::T1Coast => T1Strat::T1,
            T1Strat::T1C34Coast => T1Strat::T1C34,
            T1Strat::T1C4Coast => T1Strat::T1C4,
            T1Strat::T1RatioCoast => T1Strat::T1Ratio,
            T1Strat::T1SolarXLIIOldCoast => T1Strat::T1SolarXLIIOld,
            s => s,
        }
    }
}

/** Simulates T1 with the given strat (sim-3 `t1` function) */
pub fn simulate_t1(strat: T1Strat, input: &TheoryInput) -> SimResult {
    if strat.is_new_cycle() {
        let mut initial = T1Sim::new(strat, input);
        initial.do_coasting = false;
        let initial_res = initial.simulate();
        let last_q1 = get_last_level("q1", &initial_res.bought_vars);
        let last_q2 = get_last_level("q2", &initial_res.bought_vars);
        let last_c3 = get_last_level("c3", &initial_res.bought_vars);

        let mut sim = T1Sim::new(strat, input);
        sim.do_coasting = true;
        sim.vars[Q1].set_original_cap(last_q1);
        sim.vars[Q2].set_original_cap(last_q2);
        sim.vars[C3].set_original_cap(last_c3);
        sim.vars[Q1].configure_cap(if strat == T1Strat::T1SolarXLII { 5 } else { 18 });
        sim.vars[Q2].configure_cap(1);
        sim.vars[C3].configure_cap(3);
        sim.simulate()
    } else if strat.is_coast() {
        let res1 = T1Sim::new(strat.without_coast(), input).simulate();
        let last_q1 = get_last_level("q1", &res1.bought_vars);
        let last_c3 = get_last_level("c3", &res1.bought_vars);

        let mut sim = T1Sim::new(strat, input);
        // T1Coast force-skips 2 levels due to a better pub cycle; active strats don't.
        sim.vars[Q1].set_original_cap(if strat == T1Strat::T1Coast { last_q1 - 2 } else { last_q1 });
        sim.vars[Q1].configure_cap(18);
        sim.vars[C3].set_original_cap(last_c3);
        sim.vars[C3].configure_cap(3);
        sim.simulate()
    } else {
        T1Sim::new(strat, input).simulate()
    }
}

/** log10 constants used by the buying conditions, computed once per sim */
#[derive(Debug, Clone)]
struct T1Consts {
    l10_5: f64,
    l10_1_11: f64,
    l10_c3_ratio: f64,
    /** log10(6 + n) and log10(15 + n) for n = q1 level % 10 */
    l10_6_plus: [f64; 10],
    l10_15_plus: [f64; 10],
}

impl T1Consts {
    fn new(c3_ratio: f64) -> Self {
        T1Consts {
            l10_5: l10(5.),
            l10_1_11: l10(1.11),
            l10_c3_ratio: l10(c3_ratio),
            l10_6_plus: std::array::from_fn(|n| l10(6. + n as f64)),
            l10_15_plus: std::array::from_fn(|n| l10(15. + n as f64)),
        }
    }
}

#[derive(Debug, Clone)]
struct T1Sim {
    core: TraditionalCore,
    strat: T1Strat,
    vars: Vec<Variable>,
    term1: f64,
    term2: f64,
    term3: f64,
    term_ratio: f64,
    consts: T1Consts,
    do_coasting: bool,
    /** Only used by the "Old" strats */
    coast: f64,
}

impl T1Sim {
    fn new(strat: T1Strat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::T1, true, input, CONVERTER, 10.);
        core.milestone_unlock_steps = 25.;
        core.milestones_max = vec![1, 3, 1, 1];
        // T1 never reads the publication multiplier during the sim
        core.track_cur_mult = false;

        // Costs are built from log10 values the same way sim-3's ExponentialCost parses them,
        // so both sims compare identical floating point numbers.
        let log_cost = |v: f64| LogNum { value: v, sign: 1 };
        let log2_inc = |inc: f64| log_cost(l10(2.) * fastmath::exp10(parse_value(inc)));
        let l2_10 = fastmath::log2(10.);

        let mut q1 = Variable::new(
            "q1",
            Cost::new(FirstFreeCost::new(ExponentialCost::new(5., 2.))),
            Value::new_stepwise(2., 10, 0.),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        q1.buy();

        let vars = vec![
            q1,
            Variable::new("q2", Cost::new_exponential(100., 10.), Value::new_exponential(2.)),
            Variable::new("c1", Cost::new_exponential(15., 2.), Value::new_stepwise(2., 10, 1.)),
            Variable::new("c2", Cost::new_exponential(3000., 10.), Value::new_exponential(2.)),
            Variable::new(
                "c3",
                Cost::Exponential(ExponentialCost::new(log_cost(4.), log2_inc(4.5 * l2_10))),
                Value::new_exponential(10.),
            ),
            Variable::new(
                "c4",
                Cost::Exponential(ExponentialCost::new(log_cost(10.), log2_inc(8. * l2_10))),
                Value::new_exponential(10.),
            ),
        ];

        let lpr = core.last_pub_rho;
        let c3_ratio = if lpr < 300. { 1. } else if lpr < 450. { 1.1 } else if lpr < 550. { 2. } else if lpr < 655. { 5. } else { 10. };

        let mut sim = T1Sim {
            core,
            strat,
            vars,
            term1: 0.,
            term2: 0.,
            term3: 0.,
            term_ratio: 0.,
            consts: T1Consts::new(c3_ratio),
            do_coasting: strat.is_coast(),
            coast: f64::INFINITY,
        };
        sim.core.update_milestones(&MILESTONE_PRIORITY);
        sim
    }

    fn rho(&self) -> f64 {
        self.core.rho.value
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn c4_cost_or_1000(&self) -> f64 {
        if self.core.milestones[3] > 0 { self.cost(C4) } else { 1000. }
    }

    fn solar_q1(&self) -> bool {
        let lvl = (self.vars[Q1].level % 10) as usize;
        self.cost(Q1) + self.consts.l10_5 <= self.rho()
            && self.cost(Q1) + self.consts.l10_6_plus[lvl] <= self.cost(Q2)
            && self.cost(Q1) + self.consts.l10_15_plus[lvl] < self.c4_cost_or_1000()
    }

    /** Ratio-style conditions shared by T1Ratio(Coast) and T1SolarXLII* */
    fn ratio_cond(&self, i: usize) -> bool {
        match i {
            Q2 => self.cost(Q2) + self.consts.l10_1_11 < self.rho(),
            C1 => self.cost(C1) + self.term_ratio + 1. <= self.rho(),
            C2 => self.cost(C2) + self.term_ratio <= self.rho(),
            C3 => self.cost(C3) + self.consts.l10_c3_ratio < self.rho(),
            _ => true,
        }
    }

    fn buying_condition(&self, i: usize) -> bool {
        use T1Strat::*;
        let coast_ok = |id: usize| self.vars[id].should_buy;
        match self.strat {
            T1 => true,
            T1Coast => match i { Q1 | C3 => coast_ok(i), _ => true },
            T1C34 => !matches!(i, C1 | C2),
            T1C34Coast => match i { Q1 | Q2 | C3 => coast_ok(i), C1 | C2 => false, _ => true },
            T1C4 => !matches!(i, C1 | C2 | C3),
            T1C4Coast => match i { Q1 | Q2 => coast_ok(i), C1 | C2 | C3 => false, _ => true },
            T1Ratio => match i { Q1 => self.cost(Q1) + 1. < self.rho(), _ => self.ratio_cond(i) },
            T1RatioCoast => match i {
                Q1 => coast_ok(Q1) && self.cost(Q1) + 1. < self.rho(),
                C3 => coast_ok(C3) && self.ratio_cond(C3),
                _ => self.ratio_cond(i),
            },
            // T1SolarXLII and T1SolarXLIIOldCoast use the coast version of the conditions
            T1SolarXLII | T1SolarXLIIOldCoast => match i {
                Q1 => coast_ok(Q1) && self.solar_q1(),
                Q2 => coast_ok(Q2) && self.ratio_cond(Q2),
                C3 => coast_ok(C3) && self.ratio_cond(C3),
                _ => self.ratio_cond(i),
            },
            T1SolarXLIIOld => match i { Q1 => self.solar_q1(), _ => self.ratio_cond(i) },
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        match i {
            C3 => self.core.milestones[2] > 0,
            C4 => self.core.milestones[3] > 0,
            _ => true,
        }
    }

    fn tick(&mut self) {
        let rho = self.rho();
        let ms = &self.core.milestones;
        self.term1 = self.vars[C1].value.value * (1. + 0.05 * ms[1] as f64)
            + self.vars[C2].value.value
            + if ms[0] > 0 { fastmath::log10(1. + rho.max(0.) / LOG10_E / 100.) } else { 0. };
        self.term2 = add_log(
            self.vars[C3].value.value + rho * 0.2,
            self.vars[C4].value.value + rho * 0.3,
        );
        self.term3 = self.vars[Q1].value.value + self.vars[Q2].value.value;

        let rhodot = add_log(self.term1, self.term2) + self.term3 + self.core.tot_mult + fastmath::log10(self.core.dt);
        self.core.rho += LogNum { value: rhodot, sign: 1 };
    }

    fn buy_variables(&mut self) {
        let mut bought = false;
        for i in (0..self.vars.len()).rev() {
            while self.rho() > self.cost(i) && self.buying_condition(i) && self.variable_available(i) {
                self.core.record_purchase(&self.vars[i]);
                let cost = self.vars[i].cost;
                self.core.rho -= cost;
                self.vars[i].buy();
                bought = true;
                self.on_variable_purchased(i);
            }
        }
        if bought {
            self.on_any_variable_purchased();
        }
    }

    fn on_variable_purchased(&mut self, id: usize) {
        if matches!(id, Q1 | Q2 | C3) && self.do_coasting {
            let v = &mut self.vars[id];
            // For this theory, we don't want to go above the original cap
            if v.should_buy && v.coasting_cap_reached() && !v.above_original_cap() {
                v.should_fork = true;
            }
        }
    }

    fn on_any_variable_purchased(&mut self) {
        self.term_ratio = if self.core.last_pub_rho < 350. {
            let m3 = if self.core.milestones[3] > 0 { 1. } else { 0. };
            js_max(self.consts.l10_5, (self.term2 - self.term1) * m3)
        } else {
            f64::INFINITY
        };
    }

    /** Simulates a fork where variable `id` is no longer bought (sim-3 `doForkVariable`) */
    fn do_fork_variable(&mut self, id: usize) {
        self.vars[id].should_fork = false;
        let mut fork = self.clone();
        // sim-3 builds the fork with its constructor, which computes milestones from the
        // last pub rho only; copyFrom does not copy milestones.
        fork.core.set_milestones_at(fork.core.last_pub_rho.max(0.), &MILESTONE_PRIORITY);
        fork.vars[id].stop_buying();
        let res = fork.simulate();
        self.core.best_fork_res = Some(best_result(res, self.core.best_fork_res.take()));
    }

    /** Sets up the theory-specific publication point */
    fn setup_pub_cycle(&mut self) {
        let lpr = self.core.last_pub_rho;
        let next_c4 = ((lpr - 10.) / 8.).ceil() * 8. + 10.;

        if self.strat.is_old() {
            let dist = next_c4 - lpr;
            let pub_point = if dist < 3. {
                next_c4 + 2.
            } else if dist < 5. {
                next_c4 - 2. + l10(1.5)
            } else {
                next_c4 - 4. + l10(1.4)
            };
            let coast = (if dist < 3. { next_c4 } else { lpr.floor() }) + l10(30.);
            self.coast = (8. + l10(30.)).max(coast + (pub_point - coast).floor());
            self.core.do_sim_end_conditions = false;
            self.core.pub_target = Some(pub_point);
        }

        if self.strat.is_new_cycle() {
            let c4dist = next_c4 - lpr;
            let (prev_point, next_point) = if self.strat == T1Strat::T1SolarXLII {
                if c4dist > 7.5 { (next_c4 - 10. + l10(5.5), next_c4 - 6. + l10(1.5)) }
                else if c4dist > 4.5 { (next_c4 - 6. + l10(1.5), next_c4 - 4. + l10(5.5)) }
                else if c4dist > 2.25 { (next_c4 - 4. + l10(5.5), next_c4 - 2. + l10(5.5)) }
                else { (next_c4 - 2. + l10(5.5), next_c4 + 2. + l10(1.5)) }
            } else if c4dist > 7.5 { (next_c4 - 10. + l10(6.5), next_c4 - 6.) }
            else if c4dist > 4.5 { (next_c4 - 6., next_c4 - 4. + l10(6.5)) }
            else if c4dist > 2.25 { (next_c4 - 4. + l10(6.5), next_c4 - 2. + l10(6.5)) }
            else { (next_c4 - 2. + l10(6.5), next_c4 + 2.) };

            let mut misalignment = lpr - prev_point;
            if misalignment.abs() < l10(2.) {
                misalignment = 0.;
            }
            self.core.do_sim_end_conditions = false;
            self.core.pub_target = Some(next_point + misalignment * 0.5);
        }
    }

    fn simulate(&mut self) -> SimResult {
        self.setup_pub_cycle();

        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            if self.core.last_pub_rho < 176. {
                self.core.update_milestones(&MILESTONE_PRIORITY);
            }
            if !self.strat.is_old() || self.rho() < self.coast {
                self.buy_variables();
            }
            for id in [Q1, Q2, C3] {
                if self.vars[id].should_fork {
                    self.do_fork_variable(id);
                }
            }
            self.core.collect_pub_table();
        }
        self.core.trim_bought_vars();

        let mut extra = String::new();
        if self.strat.is_old() {
            extra.push(' ');
            if self.core.last_pub_rho >= 50. {
                extra += &log_to_exp(self.core.pub_rho.min(self.coast), 2);
            }
        }
        if self.do_coasting {
            let bv = &self.core.bought_vars;
            extra += &self.vars[Q1].prepare_extra_for_cap(get_last_level("q1", bv));
            extra += &self.vars[Q2].prepare_extra_for_cap(get_last_level("q2", bv));
            if self.vars[C3].level != 0 {
                extra += &self.vars[C3].prepare_extra_for_cap(get_last_level("c3", bv));
            }
        }
        let res = self.core.create_result(self.strat.name(), &extra);
        best_result(res, self.core.best_fork_res.take())
    }
}
