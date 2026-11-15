/*!
 * Theory 6, ported from sim-3 `src/Theories/T1-T8/T6.ts`.
 *
 * Variable order: q1, q2, r1, r2, c1, c2, c3, c4, c5
 *
 * The "IdleRecovery" strats take an idle recovery stop (sim-3's strat specific input
 * `idleRcvStop`). Here it is part of the strat name: `T6SnaxIdleRecoveryM3` is
 * `T6SnaxIdleRecovery` with a stop of 3, and the result is named like sim-3 names it.
 */

use crate::api::config::TheoryType;
use crate::theories::traditional::*;
use crate::utils::{
    cost::{Cost, FirstFreeCost},
    lognum::LogNum,
    result::SimResult,
    value::Value,
    variable::Variable,
};

const Q1: usize = 0;
const Q2: usize = 1;
const R1: usize = 2;
const R2: usize = 3;
const C1: usize = 4;
const C2: usize = 5;
const C3: usize = 6;
const C4: usize = 7;
const C5: usize = 8;

const CONVERTER: Converter = Converter {
    r9_affected: true,
    tau_factor: 1.,
    mult_exponent: 0.196,
    // -log10(50)
    mult_factor: -1.6989700043360187,
};

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum T6Strat {
        T6,
        T6C3,
        T6C4,
        T6C125,
        T6C12,
        T6C5,
        T6C5Coast,
        T6Snax,
        T6C125dStopC12,
        T6C125dStopC12Coast,
        T6SnaxCoast,
        T6SnaxIdleRecovery,
        T6SnaxIdleRecoveryCoast,
        T6SnaxIdleRecoveryM1,
        T6SnaxIdleRecoveryM1Coast,
        T6SnaxIdleRecoveryM2,
        T6SnaxIdleRecoveryM2Coast,
        T6SnaxIdleRecoveryM3,
        T6SnaxIdleRecoveryM3Coast,
        T6SnaxIdleRecoveryM4,
        T6SnaxIdleRecoveryM4Coast,
        T6SnaxIdleRecoveryM5,
        T6SnaxIdleRecoveryM5Coast,
        T6C3d,
        T6C4d,
        T6C125d,
        T6C12d,
        T6C5d,
        T6C5dCoast,
        T6C5dIdleRecovery,
        T6C5dIdleRecoveryCoast,
        T6AI,
        T6AICoast,
    }
}

impl T6Strat {
    fn is_coast(self) -> bool {
        self.name().contains("Coast")
    }

    /** Strat with "Coast" removed (used for the first pass of coasting strats) */
    fn without_coast(self) -> T6Strat {
        T6Strat::from_name(&self.name().replace("Coast", "")).unwrap_or(self)
    }

    /** Idle recovery stop: the number in T6SnaxIdleRecoveryM3, 0 for the other strats */
    fn idle_recovery_stop(self) -> i32 {
        self.name()
            .split_once("IdleRecoveryM")
            .and_then(|(_, rest)| rest.chars().next()?.to_digit(10))
            .map_or(0, |d| d as i32)
    }

    /** The strat name sim-3 gives the result (it appends the stop as "M3" itself) */
    fn sim3_name(self) -> String {
        match self.idle_recovery_stop() {
            0 => self.name().to_owned(),
            stop => self.name().replacen(&format!("M{stop}"), "", 1),
        }
    }

    /** Order in which milestones are assigned (sim-3 `getMilestonePriority`) */
    fn milestone_priority(self, last_pub_rho: f64, max_rho: f64) -> &'static [usize] {
        use T6Strat::*;
        let count = (last_pub_rho.max(max_rho) / 25.).floor().min(6.);
        match self.without_coast() {
            T6 => {
                if count >= 4. {
                    &[0, 3, 1, 2]
                } else {
                    &[1, 0, 3, 2]
                }
            }
            T6C3 | T6C3d => &[0],
            T6C4 | T6C4d => &[1, 0],
            T6C125 | T6C125d => &[0, 2, 3],
            T6C12 | T6C12d => &[0, 3],
            T6C5 | T6C5d | T6C5dIdleRecovery => &[0, 2],
            // Snax, StopC12, SnaxIdleRecovery (with any stop) and AI
            _ => &[0, 3, 2],
        }
    }
}

/** Simulates T6 with the given strat (sim-3 `t6` function) */
pub fn simulate_t6(strat: T6Strat, input: &TheoryInput) -> SimResult {
    if !strat.is_coast() {
        return T6Sim::new(strat, input).simulate();
    }
    let base = strat.without_coast();
    let res1 = T6Sim::new(base, input).simulate();
    let mut sim = T6Sim::new(strat, input);
    sim.vars[Q1].set_original_cap(get_last_level("q1", &res1.bought_vars));
    let name = strat.name();
    if name.contains("T6C5d")
        || base.name().contains("IdleRecovery")
        || base.name().contains("T6AI")
    {
        sim.vars[Q1].configure_cap(4);
    } else {
        // This is the observed max for best idle strat.
        sim.vars[Q1].configure_cap(10);
    }
    sim.vars[R1].set_original_cap(get_last_level("r1", &res1.bought_vars));
    sim.vars[R1].configure_cap(1);
    sim.simulate()
}

/** log10 constants used by the buying conditions, computed once per sim */
#[derive(Debug, Clone)]
struct T6Consts {
    l10_2: f64,
    l10_3: f64,
    l10_5: f64,
    l10_8: f64,
    l10_10: f64,
    /** log10(7 + n), log10(5 + n) and log10(8 + n) for n = level % 10 */
    l10_7_plus: [f64; 10],
    l10_5_plus: [f64; 10],
    l10_8_plus: [f64; 10],
}

#[derive(Debug, Clone)]
struct T6Sim {
    core: TraditionalCore,
    strat: T6Strat,
    /** Strat with "Coast" removed: the one whose conditions apply */
    base: T6Strat,
    coast: bool,
    idle_stop: f64,
    vars: Vec<Variable>,
    q: f64,
    r: f64,
    k: f64,
    /** rho at which c1 and c2 stopped being worth buying (sim-3 `stopC12[0]`) */
    stop_c12_rho: f64,
    /** Ticks in a row with k > 0.3 (`stopC12[1]`) */
    stop_c12_count: i32,
    /** Whether c1 and c2 are still bought (`stopC12[2]`) */
    stop_c12_active: bool,
    consts: T6Consts,
}

fn min3(a: f64, b: f64, c: f64) -> f64 {
    a.min(b).min(c)
}

impl T6Sim {
    fn new(strat: T6Strat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::T6, true, input, CONVERTER, 12.);
        core.milestone_unlock_steps = 25.;
        core.milestones_max = vec![1, 1, 1, 3];
        // T6 never reads the publication multiplier during the sim
        core.track_cur_mult = false;

        let stepwise = || Value::new_stepwise(2., 10, 0.);
        let exp2 = || Value::new_exponential(2.);
        let mut q1 = Variable::new(
            "q1",
            Cost::new(FirstFreeCost::new(exp_cost_model(15., 3.))),
            stepwise(),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        q1.buy();

        let vars = vec![
            q1,
            Variable::new("q2", exp_cost(500., 100.), exp2()),
            Variable::new("r1", exp_cost(1e25, 1e5), stepwise()),
            Variable::new("r2", exp_cost(1e30, 1e10), exp2()),
            Variable::new("c1", exp_cost(10., 2.), Value::new_stepwise(2., 10, 1.)),
            Variable::new("c2", exp_cost(100., 5.), exp2()),
            Variable::new("c3", exp_cost(1e7, 1.255), stepwise()),
            Variable::new("c4", exp_cost(1e25, 5e5), exp2()),
            Variable::new("c5", exp_cost(15., 3.9), exp2()),
        ];

        let base = strat.without_coast();
        let mut sim = T6Sim {
            core,
            strat,
            base,
            coast: strat.is_coast(),
            idle_stop: strat.idle_recovery_stop() as f64,
            vars,
            q: f64::NEG_INFINITY,
            r: 0.,
            k: 0.,
            stop_c12_rho: 0.,
            stop_c12_count: 0,
            stop_c12_active: true,
            consts: T6Consts {
                l10_2: l10(2.),
                l10_3: l10(3.),
                l10_5: l10(5.),
                l10_8: l10(8.),
                l10_10: l10(10.),
                l10_7_plus: std::array::from_fn(|n| l10(7. + n as f64)),
                l10_5_plus: std::array::from_fn(|n| l10(5. + n as f64)),
                l10_8_plus: std::array::from_fn(|n| l10(8. + n as f64)),
            },
        };
        sim.update_milestones(0.);
        sim
    }

    fn update_milestones(&mut self, max_rho: f64) {
        let priority = self
            .strat
            .milestone_priority(self.core.last_pub_rho, max_rho);
        self.core.update_milestones(priority);
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    fn level_mod_10(&self, i: usize) -> usize {
        (self.vars[i].level % 10) as usize
    }

    /** sim-3 `calculateIntegral`; also updates `k` */
    fn calculate_integral(&mut self, vc1: f64, vc2: f64, vc3: f64, vc4: f64, vc5: f64) -> f64 {
        let ms = &self.core.milestones;
        let (q, r) = (self.q, self.r);
        let term1 = vc1 + vc2 + q + r;
        let term2 = vc3 + q * 2. + r - self.consts.l10_2;
        let term3 = if ms[1] > 0 {
            vc4 + q * 3. + r - self.consts.l10_3
        } else {
            f64::NEG_INFINITY
        };
        let term4 = if ms[2] > 0 {
            vc5 + q + r * 2. - self.consts.l10_2
        } else {
            f64::NEG_INFINITY
        };
        self.k = term4 - term1;
        self.core.tot_mult + add_log(add_log(add_log(term1, term2), term3), term4)
    }

    fn buying_condition(&self, i: usize) -> bool {
        use T6Strat::*;
        // Only q1 and r1 are ever stopped (by coasting forks), so this is true for every other
        // variable and for every strat that isn't coasting
        if !self.vars[i].should_buy {
            return false;
        }
        let c = |j: usize| self.cost(j);
        let k = &self.consts;
        let ms = &self.core.milestones;
        let inf = f64::INFINITY;
        let c5_if_ms = if ms[2] > 0 { c(C5) } else { inf };
        let r2_if_ms = if ms[0] > 0 { c(R2) } else { inf };
        // "Idle recovery": buy everything until rho is back above the last pub (minus the stop)
        let recovering = self.core.max_rho <= self.core.last_pub_rho - self.idle_stop;
        let stop_c12 = self.stop_c12_active;
        match self.base {
            T6 => true,
            T6C3 => match i {
                C1 | C2 => self.vars[C3].level == 0,
                C4 | C5 => false,
                _ => true,
            },
            T6C4 => matches!(i, Q1 | Q2 | R1 | R2 | C4),
            T6C125 => matches!(i, Q1 | Q2 | R1 | R2 | C1 | C2 | C5),
            T6C12 => matches!(i, Q1 | Q2 | R1 | R2 | C1 | C2),
            T6C5 => matches!(i, Q1 | Q2 | R1 | R2 | C5),
            T6Snax => match i {
                C1 | C2 => stop_c12,
                C3 | C4 => false,
                _ => true,
            },
            T6C125dStopC12 => {
                let m = min3(c(Q2), c(R2), c5_if_ms);
                match i {
                    Q1 => c(Q1) + k.l10_7_plus[self.level_mod_10(Q1)] < m,
                    R1 => c(R1) + k.l10_5 < m,
                    C1 => stop_c12 && (c(C1) + k.l10_5 < m),
                    C2 => stop_c12,
                    C3 | C4 => false,
                    _ => true,
                }
            }
            T6SnaxIdleRecovery | T6SnaxIdleRecoveryM1 | T6SnaxIdleRecoveryM2
            | T6SnaxIdleRecoveryM3 | T6SnaxIdleRecoveryM4 | T6SnaxIdleRecoveryM5 => {
                let m = min3(c(Q2), c(R2), c5_if_ms);
                match i {
                    Q1 => recovering || c(Q1) + k.l10_7_plus[self.level_mod_10(Q1)] < m,
                    R1 => recovering || c(R1) + k.l10_5 < m,
                    C1 | C2 => stop_c12,
                    C3 | C4 => false,
                    _ => true,
                }
            }
            T6C3d => {
                let m = min3(c(Q2), r2_if_ms, c(C3));
                match i {
                    Q1 => c(Q1) + k.l10_3 < m,
                    R1 => c(R1) + k.l10_3 < min3(c(Q2), c(R2), c(C3)),
                    C1 => self.vars[C3].level == 0 && c(C1) + k.l10_3 < min3(c(Q2), c(R2), c(C3)),
                    C2 => self.vars[C3].level == 0,
                    C3 | Q2 | R2 => true,
                    _ => false,
                }
            }
            T6C4d => match i {
                Q1 => {
                    let l = if ms[0] > 0 { k.l10_5 } else { k.l10_10 };
                    c(Q1) + l < min3(c(Q2), r2_if_ms, c(C4))
                }
                R1 => c(R1) + k.l10_5 < min3(c(Q2), c(R2), c(C4)),
                Q2 | R2 | C4 => true,
                _ => false,
            },
            T6C125d => {
                let m = c(Q2).min(c(R2)).min(c(C2)).min(c5_if_ms);
                match i {
                    Q1 | R1 | C1 => c(i) + k.l10_8 < m,
                    Q2 | R2 | C2 | C5 => true,
                    _ => false,
                }
            }
            T6C12d => {
                let m = min3(c(Q2), c(R2), c(C2));
                match i {
                    Q1 | R1 | C1 => c(i) + k.l10_8 < m,
                    Q2 | R2 | C2 => true,
                    _ => false,
                }
            }
            T6C5d => {
                let m = min3(c(Q2), c(R2), c5_if_ms);
                match i {
                    Q1 => c(Q1) + k.l10_7_plus[self.level_mod_10(Q1)] < m,
                    R1 => c(R1) + k.l10_5 < m,
                    Q2 | R2 | C5 => true,
                    _ => false,
                }
            }
            T6C5dIdleRecovery => {
                let m = min3(c(Q2), c(R2), c5_if_ms);
                match i {
                    Q1 => recovering || c(Q1) + k.l10_7_plus[self.level_mod_10(Q1)] < m,
                    R1 => recovering || c(R1) + k.l10_5 < m,
                    Q2 | R2 | C5 => true,
                    _ => false,
                }
            }
            // T6AI buys with weights instead
            _ => false,
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        let ms = &self.core.milestones;
        match i {
            R1 | R2 => ms[0] > 0,
            C5 => ms[2] > 0,
            _ => true,
        }
    }

    fn tick(&mut self) {
        let ms3 = self.core.milestones[3];
        let vc1 = self.val(C1) * (1. + 0.05 * ms3 as f64);
        let (v2, v3, v4, v5) = (self.val(C2), self.val(C3), self.val(C4), self.val(C5));

        let before = self.calculate_integral(vc1, v2, v3, v4, v5);
        let mut c = sub_log(before, self.core.rho.value);

        let l10dt = l10(self.core.dt);
        self.q = add_log(self.q, self.val(Q1) + self.val(Q2) + l10dt);
        self.r = if self.core.milestones[0] > 0 {
            add_log(self.r, self.val(R1) + self.val(R2) + l10dt - 3.)
        } else {
            0.
        };

        let new_currency = self.calculate_integral(vc1, v2, v3, v4, v5);
        if c > new_currency {
            c = new_currency;
        }
        self.core.rho = LogNum {
            value: js_max(0., sub_log(new_currency, c)),
            sign: 1,
        };

        if self.k > 0.3 {
            self.stop_c12_count += 1;
        } else {
            self.stop_c12_count = 0;
        }
        if self.stop_c12_count > 30 && self.stop_c12_active {
            self.stop_c12_rho = self.core.max_rho;
            self.stop_c12_active = false;
        }
    }

    /** sim-3 `getVariableWeights` (used by the T6AI strats) */
    fn variable_weights(&self) -> [f64; 9] {
        let k = &self.consts;
        let kk = self.k;
        let mut weights = [
            k.l10_7_plus[self.level_mod_10(Q1)],
            0.,
            k.l10_5_plus[self.level_mod_10(R1)],
            0.,
            js_max(0., kk) + k.l10_8_plus[self.level_mod_10(C1)],
            js_max(0., kk),
            f64::INFINITY,
            f64::INFINITY,
            -js_min(0., kk),
        ];
        for i in [Q1, R1] {
            if !self.vars[i].should_buy {
                weights[i] = f64::INFINITY;
            }
        }
        weights
    }

    /** sim-3 `buyVariablesWeight`: always buys the variable with the lowest cost + weight */
    fn buy_variables_weight(&mut self) {
        loop {
            let weights = self.variable_weights();
            let mut min_cost = f64::MAX;
            let mut min_index = None;
            for i in (0..self.vars.len()).rev() {
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
                    self.on_variable_purchased(i);
                }
                _ => break,
            }
        }
    }

    fn buy_variables(&mut self) {
        if matches!(self.base, T6Strat::T6AI) {
            return self.buy_variables_weight();
        }
        // T6 has no onAnyVariablePurchased step, so whether something was bought is not tracked
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
        if matches!(id, Q1 | R1) && self.coast {
            let v = &mut self.vars[id];
            // For this theory, going above the original cap is mostly counterproductive
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
        // last pub rho only (max rho is still 0 then); copyFrom does not copy milestones.
        let priority = fork.strat.milestone_priority(fork.core.last_pub_rho, 0.);
        fork.core
            .set_milestones_at(fork.core.last_pub_rho.max(0.), priority);
        fork.vars[id].stop_buying();
        let res = fork.simulate();
        self.core.best_fork_res = Some(best_result(res, self.core.best_fork_res.take()));
    }

    fn simulate(&mut self) -> SimResult {
        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            if self.core.last_pub_rho < 150. {
                self.update_milestones(self.core.max_rho);
            }
            self.buy_variables();
            self.core.ticks += 1;
            for id in [Q1, R1] {
                if self.vars[id].should_fork {
                    self.do_fork_variable(id);
                }
            }
            self.core.collect_pub_table();
        }
        self.core.trim_bought_vars();

        let name = self.strat.name();
        let mut extra = String::new();
        if name.contains("IdleRecovery") && self.idle_stop > 0. {
            extra += &format!("M{}", self.idle_stop);
        }
        if name.contains("Snax") || name.contains("StopC12") {
            extra += &format!(" {}", log_to_exp(self.stop_c12_rho, 1));
        }
        if self.coast {
            let bv = &self.core.bought_vars;
            extra += &self.vars[Q1].prepare_extra_for_cap(get_last_level("q1", bv));
            extra += &self.vars[R1].prepare_extra_for_cap(get_last_level("r1", bv));
        }
        let res = self.core.create_result(&self.strat.sim3_name(), &extra);
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
            last_pub_tau: None,
            rho_cap: f64::INFINITY,
            params: SimParams::default(),
        }
    }

    #[test]
    fn idle_recovery_stop_is_part_of_the_strat_name() {
        assert_eq!(T6Strat::T6SnaxIdleRecovery.idle_recovery_stop(), 0);
        assert_eq!(T6Strat::T6SnaxIdleRecoveryM3.idle_recovery_stop(), 3);
        assert_eq!(T6Strat::T6SnaxIdleRecoveryM5Coast.idle_recovery_stop(), 5);
        assert_eq!(T6Strat::T6C5d.idle_recovery_stop(), 0);
        // results are named like sim-3 names them: strat name, then "M<stop>"
        assert_eq!(
            T6Strat::T6SnaxIdleRecoveryM3Coast.sim3_name(),
            "T6SnaxIdleRecoveryCoast"
        );
        assert_eq!(
            T6Strat::T6SnaxIdleRecoveryM2.sim3_name(),
            "T6SnaxIdleRecovery"
        );
        assert_eq!(T6Strat::T6AI.sim3_name(), "T6AI");
        let res = simulate_t6(T6Strat::T6SnaxIdleRecoveryM2, &input(300., 120));
        assert!(
            res.strat.starts_with("T6SnaxIdleRecoveryM2 "),
            "{}",
            res.strat
        );
        let res = simulate_t6(T6Strat::T6SnaxIdleRecoveryM2Coast, &input(300., 120));
        assert!(
            res.strat.starts_with("T6SnaxIdleRecoveryCoastM2 "),
            "{}",
            res.strat
        );
        assert!(res.strat.contains(" q1: ") && res.strat.contains(" r1: "));
    }

    #[test]
    fn coasting_removes_coast_from_the_first_pass() {
        assert_eq!(T6Strat::T6AICoast.without_coast(), T6Strat::T6AI);
        assert_eq!(
            T6Strat::T6C5dIdleRecoveryCoast.without_coast(),
            T6Strat::T6C5dIdleRecovery
        );
        assert_eq!(T6Strat::T6.without_coast(), T6Strat::T6);
    }

    #[test]
    fn t6_milestones_depend_on_rho() {
        // below e100 the first milestones go to q (index 1 first)
        assert_eq!(T6Strat::T6.milestone_priority(60., 0.), [1, 0, 3, 2]);
        assert_eq!(T6Strat::T6.milestone_priority(60., 120.), [0, 3, 1, 2]);
        assert_eq!(T6Strat::T6C5d.milestone_priority(500., 0.), [0, 2]);
    }

    #[test]
    fn snax_strats_stop_c12_after_k_stays_high() {
        let res = simulate_t6(T6Strat::T6Snax, &input(400., 150));
        // "T6Snax <rho where c1 and c2 stopped>"
        let rest = res.strat.strip_prefix("T6Snax ").unwrap();
        assert!(rest.contains('e'), "{}", res.strat);
        let mut sim = T6Sim::new(T6Strat::T6Snax, &input(400., 150));
        assert!(sim.stop_c12_active);
        for _ in 0..20000 {
            sim.tick();
            sim.core.update_sim_status();
            sim.buy_variables();
        }
        assert!(
            !sim.stop_c12_active && sim.stop_c12_rho > 400.,
            "{}",
            sim.stop_c12_rho
        );
    }

    #[test]
    fn ai_strats_buy_with_weights() {
        let mut sim = T6Sim::new(T6Strat::T6AICoast, &input(300., 120));
        for _ in 0..5000 {
            sim.tick();
            sim.core.update_sim_status();
            sim.buy_variables();
        }
        assert!(sim.vars[Q1].level > 1 && sim.vars[C1].level > 0);
        // coasting weights: a stopped variable is never the cheapest
        sim.vars[Q1].stop_buying();
        assert_eq!(sim.variable_weights()[Q1], f64::INFINITY);
        assert!(sim.variable_weights()[R1].is_finite());
    }
}
