/*!
 * Theory 4, ported from sim-3 `src/Theories/T1-T8/T4.ts`.
 *
 * Variable order: c1, c2, c3, c4, c5, c6, q1, q2
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

const C1: usize = 0;
const C2: usize = 1;
const C3: usize = 2;
const C4: usize = 3;
const C5: usize = 4;
const C6: usize = 5;
const Q1: usize = 6;
const Q2: usize = 7;

const CONVERTER: Converter = Converter {
    r9_affected: true,
    tau_factor: 1.,
    mult_exponent: 0.165,
    // -log10(4)
    mult_factor: -0.6020599913279624,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum T4Strat {
    T4C3dCoast,
    T4C3d,
    T4C3Coast,
    T4C3,
    T4C3dC12rcv,
    T4C356dC12rcv,
    T4C356dC12rcvCoast,
    T4C456dC12rcvMS,
    T4C456dC12rcvMSCoast,
    T4C123d,
    T4C123,
    T4C12d,
    T4C12,
    T4C56,
    T4C56Coast,
    T4C4,
    T4C4d,
    T4C5,
    T4,
}

impl T4Strat {
    /** In data.json order */
    pub const ALL: [T4Strat; 19] = [
        T4Strat::T4C3dCoast,
        T4Strat::T4C3d,
        T4Strat::T4C3Coast,
        T4Strat::T4C3,
        T4Strat::T4C3dC12rcv,
        T4Strat::T4C356dC12rcv,
        T4Strat::T4C356dC12rcvCoast,
        T4Strat::T4C456dC12rcvMS,
        T4Strat::T4C456dC12rcvMSCoast,
        T4Strat::T4C123d,
        T4Strat::T4C123,
        T4Strat::T4C12d,
        T4Strat::T4C12,
        T4Strat::T4C56,
        T4Strat::T4C56Coast,
        T4Strat::T4C4,
        T4Strat::T4C4d,
        T4Strat::T4C5,
        T4Strat::T4,
    ];

    pub fn name(self) -> &'static str {
        use T4Strat::*;
        match self {
            T4C3dCoast => "T4C3dCoast",
            T4C3d => "T4C3d",
            T4C3Coast => "T4C3Coast",
            T4C3 => "T4C3",
            T4C3dC12rcv => "T4C3dC12rcv",
            T4C356dC12rcv => "T4C356dC12rcv",
            T4C356dC12rcvCoast => "T4C356dC12rcvCoast",
            T4C456dC12rcvMS => "T4C456dC12rcvMS",
            T4C456dC12rcvMSCoast => "T4C456dC12rcvMSCoast",
            T4C123d => "T4C123d",
            T4C123 => "T4C123",
            T4C12d => "T4C12d",
            T4C12 => "T4C12",
            T4C56 => "T4C56",
            T4C56Coast => "T4C56Coast",
            T4C4 => "T4C4",
            T4C4d => "T4C4d",
            T4C5 => "T4C5",
            T4 => "T4",
        }
    }

    pub fn from_name(name: &str) -> Option<T4Strat> {
        T4Strat::ALL.into_iter().find(|s| s.name() == name)
    }

    fn is_coast(self) -> bool {
        self.name().contains("Coast")
    }

    /** Strat with "Coast" removed (used for the first pass of coasting strats) */
    fn without_coast(self) -> T4Strat {
        use T4Strat::*;
        match self {
            T4C3dCoast => T4C3d,
            T4C3Coast => T4C3,
            T4C356dC12rcvCoast => T4C356dC12rcv,
            T4C456dC12rcvMSCoast => T4C456dC12rcvMS,
            T4C56Coast => T4C56,
            s => s,
        }
    }

    /** Maximum milestone levels: [terms, c1exp, multQdot] */
    fn milestones_max(self) -> Vec<i32> {
        match self {
            T4Strat::T4C4 | T4Strat::T4C4d => vec![1, 0, 3],
            T4Strat::T4C5 => vec![2, 0, 3],
            _ => vec![3, 1, 3],
        }
    }

    /** Variables reported after the strat name ("q1: 12 ...") by coasting strats */
    fn reported_levels(self) -> &'static [usize] {
        match self {
            T4Strat::T4C56Coast => &[Q1, Q2, C5, C6],
            T4Strat::T4C356dC12rcvCoast => &[Q1, Q2, C3, C5, C6],
            T4Strat::T4C456dC12rcvMSCoast => &[Q1, Q2, C4, C5, C6],
            T4Strat::T4C3dCoast | T4Strat::T4C3Coast => &[Q1, Q2, C3],
            _ => &[],
        }
    }
}

/** Simulates T4 with the given strat (sim-3 `t4` function) */
pub fn simulate_t4(strat: T4Strat, input: &TheoryInput) -> SimResult {
    use T4Strat::*;
    if !strat.is_coast() {
        return T4Sim::new(strat, input).simulate();
    }
    let base = strat.without_coast();
    let res1 = T4Sim::new(base, input).simulate();
    let last = |name: &str| get_last_level(name, &res1.bought_vars);

    let mut sim = T4Sim::new(strat, input);
    let mut cap = |id: usize, name: &str, delta: i32| {
        sim.vars[id].set_original_cap(last(name));
        sim.vars[id].configure_cap(delta);
    };
    if matches!(base, T4C56 | T4C356dC12rcv | T4C456dC12rcvMS) {
        cap(C5, "c5", 1);
        cap(C6, "c6", 1);
    }
    if matches!(base, T4C3d | T4C3 | T4C356dC12rcv) {
        cap(C3, "c3", 3);
    }
    if base == T4C456dC12rcvMS {
        cap(C4, "c4", 1);
    }
    cap(Q1, "q1", if base == T4C3d { 3 } else { 4 });
    cap(Q2, "q2", 1);
    best_result(sim.simulate(), Some(res1))
}

/** log10 constants used by the buying conditions, computed once per sim */
#[derive(Debug, Clone)]
struct T4Consts {
    l10_2: f64,
    l10_3: f64,
    l10_1_5: f64,
    /** log10(10 + n) for n = level % 10 */
    l10_10_plus: [f64; 10],
    /** log10((1 - 1/2) / (1 - (10 + n) / (11 + n))) for n = level % 10, written like sim-3 */
    ratio: [f64; 10],
    /** log10((1 - 1/2) / (1 - 1/10)) */
    ratio_c5: f64,
}

impl T4Consts {
    fn new() -> Self {
        T4Consts {
            l10_2: l10(2.),
            l10_3: l10(3.),
            l10_1_5: l10(1.5),
            l10_10_plus: std::array::from_fn(|n| l10(10. + n as f64)),
            ratio: std::array::from_fn(|n| {
                let n = n as f64;
                l10((1. - 1. / 2.) / (1. - (10. + n) / (11. + n)))
            }),
            ratio_c5: l10((1. - 1. / 2.) / (1. - 1. / 10.)),
        }
    }
}

#[derive(Debug, Clone)]
struct T4Sim {
    core: TraditionalCore,
    strat: T4Strat,
    vars: Vec<Variable>,
    q: f64,
    consts: T4Consts,
}

impl T4Sim {
    fn new(strat: T4Strat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::T4, true, input, CONVERTER, 9.);
        core.milestone_unlock_steps = 25.;
        core.milestones_max = strat.milestones_max();

        let stepwise = || Value::new_stepwise(2., 10, 0.);
        let mut c1 = Variable::new(
            "c1",
            Cost::new(FirstFreeCost::new(ExponentialCost::new(5., 1.305))),
            stepwise(),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        c1.buy();

        let vars = vec![
            c1,
            Variable::new(
                "c2",
                Cost::new_exponential(20., 3.75),
                Value::new_exponential(2.),
            ),
            Variable::new(
                "c3",
                Cost::new_exponential(2000., 2.468),
                Value::new_exponential(2.),
            ),
            Variable::new(
                "c4",
                Cost::new_exponential(1e4, 4.85),
                Value::new_exponential(3.),
            ),
            Variable::new(
                "c5",
                Cost::new_exponential(1e8, 12.5),
                Value::new_exponential(5.),
            ),
            Variable::new(
                "c6",
                Cost::new_exponential(1e10, 58.),
                Value::new_exponential(10.),
            ),
            Variable::new("q1", Cost::new_exponential(1e3, 100.), stepwise()),
            Variable::new(
                "q2",
                Cost::new_exponential(1e4, 1000.),
                Value::new_exponential(2.),
            ),
        ];

        let mut sim = T4Sim {
            core,
            strat,
            vars,
            q: 0.,
            consts: T4Consts::new(),
        };
        let priority = sim.milestone_priority(0., 0.);
        sim.core.update_milestones(priority);
        sim
    }

    /** sim-3 `getMilestonePriority`, for the given max rho and time */
    fn milestone_priority(&self, max_rho: f64, t: f64) -> &'static [usize] {
        use T4Strat::*;
        match self.strat {
            T4C3d | T4C3Coast | T4C3dCoast | T4C3 => &[2],
            T4C3dC12rcv | T4C123 | T4C123d => &[1, 2],
            T4C356dC12rcv | T4C356dC12rcvCoast => &[1, 2, 0],
            T4C456dC12rcvMS | T4C456dC12rcvMSCoast => {
                if max_rho < self.core.last_pub_rho {
                    &[1, 2, 0]
                } else if t % 100. < 50. {
                    &[2, 0, 1]
                } else {
                    &[0, 2, 1]
                }
            }
            T4C12 | T4C12d => &[1],
            T4C56 | T4C56Coast | T4C4 | T4C4d | T4C5 => &[0, 2],
            T4 => &[0, 2, 1],
        }
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    /** Cost + log10 ratio for the level of the variable (the `l10((1 - 1/2) / ...)` conditions) */
    fn cost_ratio(&self, i: usize) -> f64 {
        self.cost(i) + self.consts.ratio[(self.vars[i].level % 10) as usize]
    }

    fn buying_condition(&self, i: usize) -> bool {
        use T4Strat::*;
        let c = |j: usize| self.cost(j);
        let k = &self.consts;
        let cm = self.core.cur_mult;
        let recovering = self.core.max_rho < self.core.last_pub_rho;
        let coast_ok = |j: usize| !self.strat.is_coast() || self.vars[j].should_buy;
        match self.strat {
            T4C3d | T4C3dCoast => match i {
                C3 => coast_ok(C3),
                Q1 => {
                    let lvl = (self.vars[Q1].level % 10) as usize;
                    coast_ok(Q1) && c(Q1) + k.l10_10_plus[lvl] <= c(Q2).min(c(C3))
                }
                Q2 => {
                    let l = if self.strat == T4C3d {
                        k.l10_3
                    } else {
                        k.l10_1_5
                    };
                    coast_ok(Q2) && (cm < 1. || c(Q2) + l <= c(C3))
                }
                _ => false,
            },
            T4C3 | T4C3Coast => matches!(i, C3 | Q1 | Q2) && coast_ok(i),
            T4C3dC12rcv => match i {
                C1 => self.cost_ratio(C1) < c(C2).min(c(C3)) && recovering,
                C2 => recovering,
                C3 | Q2 => true,
                Q1 => c(Q1) + 1. < c(Q2),
                _ => false,
            },
            T4C356dC12rcv | T4C356dC12rcvCoast => match i {
                C1 => recovering && self.cost_ratio(C1) < c(C2).min(c(C3)),
                C2 => recovering,
                C4 => false,
                // Yes, c5 uses c6 cost instead of its own (as in sim-3)
                C5 => coast_ok(C5) && (recovering || c(C6) + k.ratio_c5 < c(C3)),
                Q1 => coast_ok(Q1) && self.cost_ratio(Q1) < c(Q2),
                _ => coast_ok(i),
            },
            T4C456dC12rcvMS | T4C456dC12rcvMSCoast => match i {
                C1 => c(C1) + 1. < c(C2).min(c(C4)).min(c(C5)) && recovering,
                C2 => recovering,
                C3 => false,
                Q1 => coast_ok(Q1) && c(Q1) + 1. < c(Q2),
                _ => coast_ok(i),
            },
            T4C123d => match i {
                C1 => self.cost_ratio(C1) < c(C2).min(c(C3)),
                C2 | C3 | Q2 => true,
                Q1 => c(Q1) + 1. < c(Q2),
                _ => false,
            },
            T4C123 => matches!(i, C1 | C2 | C3 | Q1 | Q2),
            T4C12d => match i {
                C1 => self.cost_ratio(C1) < c(C2),
                C2 => true,
                _ => false,
            },
            T4C12 => matches!(i, C1 | C2),
            T4C56 | T4C56Coast => matches!(i, C5 | C6 | Q1 | Q2) && coast_ok(i),
            T4C4 => matches!(i, C4 | Q1 | Q2),
            T4C4d => match i {
                C4 => true,
                Q1 => self.cost_ratio(Q1) <= c(Q2).min(c(C4)),
                Q2 => cm < 1. || c(Q2) + k.l10_1_5 <= c(C4),
                _ => false,
            },
            T4C5 => matches!(i, C5 | Q1 | Q2),
            T4 => true,
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        let terms = self.core.milestones[0];
        match i {
            C4 => terms > 0,
            C5 => terms > 1,
            C6 => terms > 2,
            _ => true,
        }
    }

    fn tick(&mut self) {
        let ms = &self.core.milestones;
        let v = |i: usize| self.vars[i].value.value;
        let l10dt = l10(self.core.dt);

        let p = add_log(self.q, 0.) * 2.;
        self.q = sub_log(
            add_log(
                p,
                self.consts.l10_2 * (1 + ms[2]) as f64 + v(Q1) + v(Q2) + l10dt,
            ) / 2.,
            0.,
        );
        let q = self.q;

        let vc1 = v(C1) * (1. + 0.15 * ms[1] as f64);
        let mut sum = vc1 + v(C2);
        sum = add_log(sum, v(C3) + q);
        if ms[0] >= 1 {
            sum = add_log(sum, v(C4) + q * 2.);
        }
        if ms[0] >= 2 {
            sum = add_log(sum, v(C5) + q * 3.);
        }
        if ms[0] >= 3 {
            sum = add_log(sum, v(C6) + q * 4.);
        }

        let rhodot = self.core.tot_mult + sum;
        self.core.rho += LogNum {
            value: rhodot + l10dt,
            sign: 1,
        };
    }

    fn buy_variables(&mut self) {
        // T4 has no onAnyVariablePurchased step, so whether something was bought is not tracked
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
        if matches!(id, C3 | Q1 | Q2) && self.strat.is_coast() {
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
        // last pub rho only (max rho and t are still 0 then); copyFrom does not copy milestones.
        let priority = fork.milestone_priority(0., 0.);
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
            if self.core.last_pub_rho < 176. {
                let priority = self.milestone_priority(self.core.max_rho, self.core.t);
                self.core.update_milestones(priority);
            }
            self.buy_variables();
            for id in [Q1, Q2, C3] {
                if self.vars[id].should_fork {
                    self.do_fork_variable(id);
                }
            }
            self.core.collect_pub_table();
        }
        self.core.trim_bought_vars();

        let mut extra = String::new();
        for &id in self.strat.reported_levels() {
            let v = &self.vars[id];
            extra += &v.prepare_extra_for_cap(get_last_level(v.name, &self.core.bought_vars));
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
    fn ms_strats_move_milestones_during_the_publication() {
        let sim = T4Sim::new(T4Strat::T4C456dC12rcvMS, &input(100., 70));
        // recovering (below the last pub): c1exp first
        assert_eq!(sim.milestone_priority(50., 0.), [1, 2, 0]);
        // then alternating every 50 seconds between multQdot and terms first
        assert_eq!(sim.milestone_priority(120., 20.), [2, 0, 1]);
        assert_eq!(sim.milestone_priority(120., 170.), [0, 2, 1]);
        // 4 milestones at e100, given in the recovering order: c1exp (max 1), then multQdot (max 3)
        assert_eq!(sim.core.milestones, [0, 1, 3]);
    }

    #[test]
    fn strats_without_c1exp_have_their_own_milestone_limits() {
        assert_eq!(T4Strat::T4C4.milestones_max(), [1, 0, 3]);
        assert_eq!(T4Strat::T4C5.milestones_max(), [2, 0, 3]);
        assert_eq!(T4Strat::T4.milestones_max(), [3, 1, 3]);
        let sim = T4Sim::new(T4Strat::T4C5, &input(200., 100));
        assert_eq!(sim.core.milestones, [2, 0, 3]);
        assert!(sim.variable_available(C5) && !sim.variable_available(C6));
    }

    #[test]
    fn coasting_strats_report_levels_and_never_lose_to_their_base_strat() {
        for (strat, rho, names) in [
            (T4Strat::T4C3Coast, 260., &["q1", "q2", "c3"][..]),
            (T4Strat::T4C56Coast, 120., &["q1", "q2", "c5", "c6"][..]),
            (
                T4Strat::T4C456dC12rcvMSCoast,
                150.,
                &["q1", "q2", "c4", "c5", "c6"][..],
            ),
        ] {
            let res = simulate_t4(strat, &input(rho, 120));
            let base = simulate_t4(strat.without_coast(), &input(rho, 120));
            // sim-3 keeps the first pass's result when coasting does worse
            assert!(res.tau_h >= base.tau_h, "{strat:?}");
            if res.strat != base.strat {
                for name in names {
                    assert!(res.strat.contains(&format!(" {name}: ")), "{}", res.strat);
                }
            }
        }
    }
}
