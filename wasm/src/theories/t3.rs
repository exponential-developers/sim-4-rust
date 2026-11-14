/*!
 * Theory 3, ported from sim-3 `src/Theories/T1-T8/T3.ts`.
 *
 * Variable order: b1, b2, b3, c11, c12, c13, c21, c22, c23, c31, c32, c33.
 * Variable `i` is bought with rho_(i % 3 + 1): b1, c11, c21, c31 with rho_1, and so on.
 */

use crate::api::config::TheoryType;
use crate::theories::traditional::*;
use crate::utils::{
    cost::{Cost, ExponentialCost, FirstFreeCost},
    lognum::{self, LogNum},
    result::SimResult,
    value::Value,
    variable::Variable,
};

const B1: usize = 0;
const B2: usize = 1;
const B3: usize = 2;
const C11: usize = 3;
const C12: usize = 4;
const C13: usize = 5;
const C21: usize = 6;
const C22: usize = 7;
const C23: usize = 8;
const C31: usize = 9;
const C32: usize = 10;
const C33: usize = 11;

/** Currency symbols, by variable index % 3 (sim-3 records them with each purchase) */
const SYMBOLS: [&str; 3] = ["rho_1", "rho_2", "rho_3"];

/** Order in which milestones are assigned: [dimensions, b1exp, b2exp, b3exp] */
const MILESTONE_PRIORITY: [usize; 4] = [1, 2, 0, 3];

const CONVERTER: Converter = Converter {
    r9_affected: true,
    tau_factor: 1.,
    mult_exponent: 0.147,
    // log10(3)
    mult_factor: 0.47712125471966244,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum T3Strat {
    T3Play2,
    T3Play,
    T3Snax,
    T3SnaxCoast,
    T3SnaxdC12,
    T3Snax2,
    T3P2C23d,
    T3P2C23C33d,
    T3P2C23,
    T3P2C23C33,
    T3P2C23C33Coast,
    T3noC11C13C21C33d,
    T3noC11C13C21C33,
    T3noC13C33d,
    T3noC13C33,
    T3noC11C13C33d,
    T3noC11C13C33,
    T3noC13C32C33d,
    T3noC13C32C33,
    T3C11C12C21d,
    T3C11C12C21,
    T3,
}

impl T3Strat {
    /** In data.json order */
    pub const ALL: [T3Strat; 22] = [
        T3Strat::T3Play2,
        T3Strat::T3Play,
        T3Strat::T3Snax,
        T3Strat::T3SnaxCoast,
        T3Strat::T3SnaxdC12,
        T3Strat::T3Snax2,
        T3Strat::T3P2C23d,
        T3Strat::T3P2C23C33d,
        T3Strat::T3P2C23,
        T3Strat::T3P2C23C33,
        T3Strat::T3P2C23C33Coast,
        T3Strat::T3noC11C13C21C33d,
        T3Strat::T3noC11C13C21C33,
        T3Strat::T3noC13C33d,
        T3Strat::T3noC13C33,
        T3Strat::T3noC11C13C33d,
        T3Strat::T3noC11C13C33,
        T3Strat::T3noC13C32C33d,
        T3Strat::T3noC13C32C33,
        T3Strat::T3C11C12C21d,
        T3Strat::T3C11C12C21,
        T3Strat::T3,
    ];

    pub fn name(self) -> &'static str {
        use T3Strat::*;
        match self {
            T3Play2 => "T3Play2",
            T3Play => "T3Play",
            T3Snax => "T3Snax",
            T3SnaxCoast => "T3SnaxCoast",
            T3SnaxdC12 => "T3SnaxdC12",
            T3Snax2 => "T3Snax2",
            T3P2C23d => "T3P2C23d",
            T3P2C23C33d => "T3P2C23C33d",
            T3P2C23 => "T3P2C23",
            T3P2C23C33 => "T3P2C23C33",
            T3P2C23C33Coast => "T3P2C23C33Coast",
            T3noC11C13C21C33d => "T3noC11C13C21C33d",
            T3noC11C13C21C33 => "T3noC11C13C21C33",
            T3noC13C33d => "T3noC13C33d",
            T3noC13C33 => "T3noC13C33",
            T3noC11C13C33d => "T3noC11C13C33d",
            T3noC11C13C33 => "T3noC11C13C33",
            T3noC13C32C33d => "T3noC13C32C33d",
            T3noC13C32C33 => "T3noC13C32C33",
            T3C11C12C21d => "T3C11C12C21d",
            T3C11C12C21 => "T3C11C12C21",
            T3 => "T3",
        }
    }

    pub fn from_name(name: &str) -> Option<T3Strat> {
        T3Strat::ALL.into_iter().find(|s| s.name() == name)
    }

    fn is_coast(self) -> bool {
        self.name().contains("Coast")
    }

    /** Strat with "Coast" removed (used for the first pass of coasting strats) */
    fn without_coast(self) -> T3Strat {
        match self {
            T3Strat::T3SnaxCoast => T3Strat::T3Snax,
            T3Strat::T3P2C23C33Coast => T3Strat::T3P2C23C33,
            s => s,
        }
    }
}

/** Simulates T3 with the given strat (sim-3 `t3` function) */
pub fn simulate_t3(strat: T3Strat, input: &TheoryInput) -> SimResult {
    if !strat.is_coast() {
        return T3Sim::new(strat, input).simulate();
    }
    let res1 = T3Sim::new(strat.without_coast(), input).simulate();
    let last_b2 = get_last_level("b2", &res1.bought_vars);
    let last_b3 = get_last_level("b3", &res1.bought_vars);

    let mut sim = T3Sim::new(strat, input);
    sim.vars[B2].set_original_cap(last_b2);
    sim.vars[B2].configure_cap(9);
    sim.vars[B3].set_original_cap(last_b3);
    sim.vars[B3].configure_cap(9);
    sim.simulate()
}

/** log10 constants used by the buying conditions, computed once per sim */
#[derive(Debug, Clone)]
struct T3Consts {
    l10_2: f64,
    l10_3: f64,
    l10_4: f64,
    l10_5: f64,
    l10_7: f64,
    l10_8: f64,
    l10_9: f64,
    l10_10: f64,
    /** log10(1 / (2 / 5)), written like sim-3 */
    l10_2_5: f64,
}

impl T3Consts {
    fn new() -> Self {
        T3Consts {
            l10_2: l10(2.),
            l10_3: l10(3.),
            l10_4: l10(4.),
            l10_5: l10(5.),
            l10_7: l10(7.),
            l10_8: l10(8.),
            l10_9: l10(9.),
            l10_10: l10(10.),
            l10_2_5: l10(1. / (2. / 5.)),
        }
    }
}

/** Buying conditions of the strats that only turn variables on or off */
fn fixed_conditions(strat: T3Strat) -> Option<[bool; 12]> {
    use T3Strat::*;
    const T: bool = true;
    const F: bool = false;
    Some(match strat {
        T3P2C23 => [F, T, T, F, T, F, F, T, T, F, T, F],
        T3P2C23C33 => [F, T, T, F, T, F, F, T, T, F, T, T],
        T3noC11C13C21C33 => [T, T, T, F, T, F, F, T, T, T, T, F],
        T3noC13C33 => [T, T, T, T, T, F, T, T, T, T, T, F],
        T3noC11C13C33 => [T, T, T, F, T, F, T, T, T, T, T, F],
        T3noC13C32C33 => [T, T, T, T, T, F, T, T, T, T, F, F],
        T3C11C12C21 => [T, T, F, T, T, F, T, F, F, F, F, F],
        T3 => [T; 12],
        _ => return None,
    })
}

fn min3(a: f64, b: f64, c: f64) -> f64 {
    a.min(b).min(c)
}

#[derive(Debug, Clone)]
struct T3Sim {
    core: TraditionalCore,
    strat: T3Strat,
    vars: Vec<Variable>,
    /** rho_1 is `core.rho` */
    rho2: LogNum,
    rho3: LogNum,
    fixed: Option<[bool; 12]>,
    consts: T3Consts,
}

impl T3Sim {
    fn new(strat: T3Strat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::T3, true, input, CONVERTER, 9.);
        core.milestone_unlock_steps = 25.;
        core.milestones_max = vec![1, 2, 2, 2];

        let stepwise = || Value::new_stepwise(2., 10, 0.);
        let exp2 = || Value::new_exponential(2.);
        let mut b1 = Variable::new(
            "b1",
            Cost::new(FirstFreeCost::new(ExponentialCost::new(10., 1.18099))),
            stepwise(),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        b1.buy();

        let vars = vec![
            b1,
            Variable::new("b2", Cost::new_exponential(10., 1.308), stepwise()),
            Variable::new("b3", Cost::new_exponential(3000., 1.675), stepwise()),
            Variable::new("c11", Cost::new_exponential(20., 6.3496), exp2()),
            Variable::new("c12", Cost::new_exponential(10., 2.74), exp2()),
            Variable::new("c13", Cost::new_exponential(1000., 1.965), exp2()),
            Variable::new("c21", Cost::new_exponential(500., 18.8343), exp2()),
            Variable::new("c22", Cost::new_exponential(1e5, 3.65), exp2()),
            Variable::new("c23", Cost::new_exponential(1e5, 2.27), exp2()),
            Variable::new("c31", Cost::new_exponential(1e4, 1248.27), exp2()),
            Variable::new("c32", Cost::new_exponential(1e3, 6.81744), exp2()),
            Variable::new("c33", Cost::new_exponential(1e5, 2.98), exp2()),
        ];

        let mut sim = T3Sim {
            core,
            strat,
            vars,
            rho2: lognum::ZERO,
            rho3: lognum::ZERO,
            fixed: fixed_conditions(strat),
            consts: T3Consts::new(),
        };
        sim.core.update_milestones(&MILESTONE_PRIORITY);
        sim
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    /** Currency that variable `i` is bought with */
    fn currency(&self, i: usize) -> f64 {
        match i % 3 {
            0 => self.core.rho.value,
            1 => self.rho2.value,
            _ => self.rho3.value,
        }
    }

    fn pay(&mut self, i: usize, cost: LogNum) {
        match i % 3 {
            0 => self.core.rho -= cost,
            1 => self.rho2 -= cost,
            _ => self.rho3 -= cost,
        }
    }

    fn buying_condition(&self, i: usize) -> bool {
        use T3Strat::*;
        if let Some(fixed) = self.fixed {
            return fixed[i];
        }
        let cm = self.core.cur_mult;
        let c = |j: usize| self.cost(j);
        let k = &self.consts;
        let coast_ok = |j: usize| self.vars[j].should_buy;
        match self.strat {
            T3Play2 => {
                let behind = self.core.last_pub_rho - self.core.max_rho > 1.;
                match i {
                    B1 => behind && c(B1) + k.l10_8 < c(C31),
                    B2 => {
                        (if cm < 1.2 {
                            c(B2) + k.l10_5 < c(C32)
                        } else {
                            c(B2) + k.l10_8 < c(C12)
                        }) || cm > 2.4
                    }
                    B3 => cm >= 2.4 || c(B3) + k.l10_8 < c(C23),
                    C12 => cm >= 1.2 || c(C12) + 2. < c(C32),
                    C22 => {
                        if cm < 1.2 {
                            c(C22) + k.l10_2_5 < c(C32)
                        } else {
                            c(C22) + k.l10_8 < c(C12)
                        }
                    }
                    C23 => true,
                    C31 => behind,
                    C32 => cm < 1.2 || (cm < 2.4 && c(C32) + k.l10_8 < c(C12)),
                    C33 => cm < 1.2 && c(C33) + k.l10_10 < c(C23),
                    _ => false,
                }
            }
            T3Play => match i {
                B1 => cm < 2. && c(B1) + k.l10_8 < c(C31),
                B2 => {
                    cm >= 2. || (c(B2) + k.l10_4 < c(C12).min(c(C32)) && c(B2) + k.l10_2 < c(C22))
                }
                B3 => c(B3) + k.l10_8 < c(C23) && c(B3) + k.l10_2 < c(C33),
                C12 | C23 | C32 => true,
                C22 => cm >= 2. || c(C22) + k.l10_2 < c(C12).min(c(C32)),
                C31 => cm < 2.,
                C33 => c(C33) + k.l10_4 < c(C23),
                _ => false,
            },
            T3Snax | T3SnaxCoast | T3SnaxdC12 => match i {
                B1 | C31 | C32 | C33 => cm < 1.,
                B2 | B3 if self.strat == T3SnaxCoast => coast_ok(i),
                B2 | B3 | C22 | C23 => true,
                C12 if self.strat == T3SnaxdC12 => cm >= 1. || c(C12) + 2. < c(C32),
                C12 => true,
                _ => false,
            },
            T3Snax2 => match i {
                B1 => cm < 1. && c(B1) + 1. < self.core.rho.value,
                B2 => c(B2) + k.l10_3 < self.rho2.value,
                B3 => c(B3) + k.l10_5 < self.rho3.value,
                C12 => cm >= 1. || c(C12) + 2. < self.core.rho.value,
                C22 => cm < 1. || c(C22) + k.l10_8 < self.rho2.value,
                C23 => true,
                C31 | C32 => cm < 1.,
                C33 => cm < 1. && c(C33) + 1. < self.rho3.value,
                _ => false,
            },
            T3P2C23d | T3P2C23C33d => match i {
                B2 => c(B2) + k.l10_3 < min3(c(C12), c(C22), c(C32)),
                B3 => c(B3) + k.l10_9 < c(C23),
                C12 | C22 | C23 | C32 => true,
                C33 => self.strat == T3P2C23C33d,
                _ => false,
            },
            T3P2C23C33Coast => match i {
                B2 | B3 => coast_ok(i),
                C12 | C22 | C23 | C32 | C33 => true,
                _ => false,
            },
            T3noC11C13C21C33d => match i {
                B1 => c(B1) + k.l10_8 < c(C31),
                B2 => c(B2) + k.l10_5 < min3(c(C12), c(C22), c(C32)),
                B3 => c(B3) + k.l10_8 < c(C23),
                C12 | C22 | C23 | C31 | C32 => true,
                _ => false,
            },
            T3noC13C33d => match i {
                B1 => c(B1) + k.l10_10 < min3(c(C11), c(C21), c(C31)),
                B2 => c(B2) + k.l10_4 < min3(c(C12), c(C22), c(C32)),
                B3 => c(B3) + k.l10_10 < c(C23),
                C13 | C33 => false,
                _ => true,
            },
            T3noC11C13C33d => match i {
                B1 => c(B1) + k.l10_10 < c(C21).min(c(C31)),
                B2 => c(B2) + k.l10_4 < min3(c(C12), c(C22), c(C32)),
                B3 => c(B3) + k.l10_10 < c(C23),
                C11 | C13 | C33 => false,
                _ => true,
            },
            T3noC13C32C33d => match i {
                B1 => c(B1) + k.l10_8 < min3(c(C11), c(C21), c(C31)),
                B2 => c(B2) + k.l10_5 < c(C12).min(c(C22)),
                B3 => c(B3) + k.l10_8 < c(C23),
                C13 | C32 | C33 => false,
                _ => true,
            },
            T3C11C12C21d => match i {
                B1 => c(B1) + k.l10_7 < c(C11).min(c(C21)),
                B2 => c(B2) + k.l10_7 < c(C12),
                C11 | C12 | C21 => true,
                _ => false,
            },
            // The rest only turn variables on or off (handled by `fixed`)
            _ => unreachable!("{:?} has fixed conditions", self.strat),
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        match i {
            B3 | C13 | C23 | C31 | C32 | C33 => self.core.milestones[0] > 0,
            _ => true,
        }
    }

    fn tick(&mut self) {
        let ms = &self.core.milestones;
        let v = |i: usize| self.vars[i].value.value;
        let vb1 = v(B1) * (1. + 0.05 * ms[1] as f64);
        let vb2 = v(B2) * (1. + 0.05 * ms[2] as f64);
        let vb3 = v(B3) * (1. + 0.05 * ms[3] as f64);
        let base = l10(self.core.dt) + self.core.tot_mult;
        let add3 = |a: f64, b: f64, c: f64| add_log(add_log(a, b), c);

        let rhodot = add3(v(C11) + vb1, v(C12) + vb2, v(C13) + vb3);
        let rho2dot = add3(v(C21) + vb1, v(C22) + vb2, v(C23) + vb3);
        let rho3dot = add3(v(C31) + vb1, v(C32) + vb2, v(C33) + vb3);
        let dims = ms[0] > 0;

        self.core.rho += LogNum {
            value: base + rhodot,
            sign: 1,
        };
        self.rho2 += LogNum {
            value: base + rho2dot,
            sign: 1,
        };
        if dims {
            self.rho3 += LogNum {
                value: base + rho3dot,
                sign: 1,
            };
        }
    }

    fn buy_variables(&mut self) {
        // T3 has no onAnyVariablePurchased step, so whether something was bought is not tracked
        for i in (0..self.vars.len()).rev() {
            while self.currency(i) > self.cost(i)
                && self.buying_condition(i)
                && self.variable_available(i)
            {
                self.core.record_purchase_in(&self.vars[i], SYMBOLS[i % 3]);
                let cost = self.vars[i].cost;
                self.pay(i, cost);
                self.vars[i].buy();
                self.on_variable_purchased(i);
            }
        }
    }

    fn on_variable_purchased(&mut self, id: usize) {
        if matches!(id, B2 | B3) && self.strat.is_coast() {
            let v = &mut self.vars[id];
            // For this strat, there is no sense to test levels above cap
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
            if self.core.last_pub_rho < 175. {
                self.core.update_milestones(&MILESTONE_PRIORITY);
            }
            self.buy_variables();
            for id in [B2, B3] {
                if self.vars[id].should_fork {
                    self.do_fork_variable(id);
                }
            }
            self.core.collect_pub_table();
        }
        self.core.trim_bought_vars();

        let mut extra = String::new();
        if self.strat.is_coast() {
            let bv = &self.core.bought_vars;
            extra += &self.vars[B2].prepare_extra_for_cap(get_last_level("b2", bv));
            extra += &self.vars[B3].prepare_extra_for_cap(get_last_level("b3", bv));
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
    fn purchases_record_the_currency_they_were_paid_with() {
        let res = simulate_t3(T3Strat::T3, &input(150., 90));
        assert!(!res.bought_vars.is_empty());
        for b in &res.bought_vars {
            let expected = match b.var_name.as_str() {
                "b1" | "c11" | "c21" | "c31" => "rho_1",
                "b2" | "c12" | "c22" | "c32" => "rho_2",
                _ => "rho_3",
            };
            assert_eq!(b.symbol, expected, "{}", b.var_name);
        }
    }

    #[test]
    fn rho3_and_its_variables_need_the_dimension_milestone() {
        // dimensions is the 5th milestone (after b1exp and b2exp), unlocked at e125; without it
        // rho_3 never grows and b3/c13/c23/c31/c32/c33 can't be bought
        let mut sim = T3Sim::new(T3Strat::T3, &input(5., 20));
        for _ in 0..2000 {
            sim.tick();
            sim.core.update_sim_status();
            sim.core.update_milestones(&MILESTONE_PRIORITY);
            sim.buy_variables();
        }
        assert_eq!(sim.core.milestones[0], 0);
        assert_eq!(sim.rho3, lognum::ZERO);
        assert!(sim.rho2.value > 0.);
        for i in [B3, C13, C23, C31, C32, C33] {
            assert!(!sim.variable_available(i));
            assert_eq!(sim.vars[i].level, 0, "{}", sim.vars[i].name);
        }

        let sim = T3Sim::new(T3Strat::T3, &input(120., 80));
        assert_eq!(sim.core.milestones, [0, 2, 2, 0]);
        assert!(!sim.variable_available(B3));
        let sim = T3Sim::new(T3Strat::T3, &input(130., 80));
        assert_eq!(sim.core.milestones, [1, 2, 2, 0]);
        assert!(sim.variable_available(B3));
    }

    #[test]
    fn coasting_strats_report_their_b2_and_b3_levels() {
        for strat in [T3Strat::T3SnaxCoast, T3Strat::T3P2C23C33Coast] {
            let res = simulate_t3(strat, &input(450., 190));
            let extra = res
                .strat
                .strip_prefix(strat.name())
                .unwrap_or_else(|| panic!("{}", res.strat));
            assert!(
                extra.starts_with(" b2: ") && extra.contains(" b3: "),
                "{extra}"
            );
            // coasting (with forks over the last levels) is never worse than not coasting
            let base = simulate_t3(strat.without_coast(), &input(450., 190));
            assert!(res.tau_h >= base.tau_h, "{strat:?}");
        }
        assert_eq!(T3Strat::T3Snax.without_coast(), T3Strat::T3Snax);
    }

    #[test]
    fn fixed_strats_turn_whole_variables_on_or_off() {
        assert_eq!(fixed_conditions(T3Strat::T3), Some([true; 12]));
        assert_eq!(fixed_conditions(T3Strat::T3Play), None);
        let res = simulate_t3(T3Strat::T3C11C12C21, &input(100., 70));
        // only b1, b2, c11, c12 and c21 are bought
        assert!(
            res.bought_vars
                .iter()
                .all(|b| ["b1", "b2", "c11", "c12", "c21"].contains(&b.var_name.as_str())),
        );
    }
}
