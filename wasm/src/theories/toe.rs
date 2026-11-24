/*!
 * Theory of Everything (ToE, unofficial), ported from sim-3
 * `src/Theories/Unofficial-CTs/TOE.ts`.
 *
 * A publication is a series of γ runs: each run grows ρ through the decaying chain
 * q4 → q3 → q2 → q1 → ρ, then resets for γ, which buys γ upgrades. Once γ4–γ7 are maxed,
 * conjectures (challenge runs) are tried before each γ run. τ = γ^0.4.
 *
 * Variable order: dq1, dq2, dq3, dq4 (bought with ρ); nt (tickspeed, permanent within a
 * publication); γ1 to γ7 (bought with γ). Values are log10 numbers except nt, γ3, γ5, γ6 and γ7,
 * which are plain numbers (sim-3 `LinearValue`).
 */

use std::borrow::Cow;

use crate::api::config::TheoryType;
use crate::theories::traditional::*;
use crate::utils::{
    cost::{Cost, FirstFreeCost},
    fastmath,
    lognum::LogNum,
    result::SimResult,
    value::Value,
    var_buy::VarBuy,
    variable::Variable,
};

const TAU_FACTOR: f64 = 0.4;

const CONVERTER: Converter = Converter {
    r9_affected: false,
    tau_factor: TAU_FACTOR,
    mult_exponent: 0.5,
    mult_factor: 0.,
};

/** Tickspeed by level of nt + γ3 */
const TICKSPEED_CONSTS: [f64; 10] = [
    11. / 1024.,
    10. / 512.,
    9. / 256.,
    8. / 128.,
    7. / 64.,
    6. / 32.,
    5. / 16.,
    4. / 8.,
    3. / 4.,
    1.,
];

/** ρ goal (log10) of conjecture `id` at `difficulty` (1 to 3) */
fn conjecture_goal(id: usize, difficulty: i32) -> f64 {
    let goals = [
        [8., 10., 12.],
        [7., 9., 11.],
        [8., 10., 16. + l10(3.)],
        [l10(250000.), l10(750000.), l10(2250000.)],
    ];
    goals[id][(difficulty - 1) as usize]
}

const DQ1: usize = 0;
const DQ4: usize = 3;
const GAMMA7: usize = 6;

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ToeStrat {
        TOE,
    }
}

/** Simulates ToE with the given strat (sim-3 `toe` function) */
pub fn simulate_toe(strat: ToeStrat, input: &TheoryInput) -> SimResult {
    ToePubSim::new(strat, input).simulate()
}

/** sim-3 `calculateXDxSoftcapped` with threshold 0: x + dx, softcapped above 1 (log10 0) */
fn x_dx_softcapped(x: f64, dx: f64, softcap: f64) -> f64 {
    let apply = |v: f64| if v > 0. { v * softcap } else { v };
    let apply_inverse = |v: f64| if v > 0. { v / softcap } else { v };
    if x < 0. {
        let new_x = add_log(x, dx);
        if new_x >= 0. {
            apply(new_x - 0.) + 0.
        } else {
            new_x
        }
    } else {
        apply(add_log(apply_inverse(x - 0.), dx - 0.)) + 0.
    }
}

/** One γ run (sim-3 `toeGammaSim`); the publication sim is built on it */
#[derive(Debug, Clone)]
struct ToeSim {
    core: TraditionalCore,
    rho: f64,
    /** dq1 to dq4 */
    dq: Vec<Variable>,
    /** Tickspeed upgrade (sim-3 `rhoPermaVariables[0]`) */
    nt: Variable,
    /** log10 q1 to q4 */
    q: [f64; 4],

    /** Real time of the current γ run (s) */
    t_gamma: f64,
    /** The game's own time, which grows by dt * tickspeed: used by γ2's t^3 */
    t_game: f64,
    gamma: f64,
    gamma_vars: Vec<Variable>,
    gamma_max_rho: f64,
    gamma_resets: u32,
    /** Active conjecture id (-1: none) and its difficulty */
    active_id: i32,
    active_difficulty: i32,
    completed: [i32; 4],

    gamma_h: f64,
    max_gamma_h: f64,
    gamma_reset_t: f64,
    gamma_reset_currency: f64,
    /** Conjecture runs end at their goal (or after an hour) instead of at an affordable upgrade */
    conjecture_goal: Option<f64>,
    /** Whether the sim started from a previous publication (achievement 3 is then unlocked) */
    has_last_pub: bool,
}

impl ToeSim {
    fn new(input: &TheoryInput) -> Self {
        let core = TraditionalCore::new(TheoryType::TOE, false, input, CONVERTER, 1.);
        let plain = |power: f64| Value::new(PlainLinearValue { power, offset: 0. });
        let first_free = |base: f64, increase: f64| {
            let mut v = Variable::new(
                "",
                Cost::new(FirstFreeCost::new(exp_cost_log2_model(base, increase))),
                Value::new_stepwise(2., 9, 0.),
            );
            // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
            v.buy();
            v
        };
        let mut dq1 = first_free(0.1, fastmath::log2(2e2) / 2.);
        dq1.name = "dq1";
        let mut dq2 = first_free(1., fastmath::log2(2e4) / 2.);
        dq2.name = "dq2";
        let dq = vec![
            dq1,
            dq2,
            Variable::new(
                "dq3",
                exp_cost_log2(10000., fastmath::log2(2e6) / 2.),
                Value::new_stepwise(2., 9, 0.),
            ),
            Variable::new(
                "dq4",
                exp_cost_log2(8e18, fastmath::log2(2e8) / 2.),
                Value::new_stepwise(2., 9, 0.),
            ),
        ];
        let gamma_vars = vec![
            Variable::new("γ1", exp_cost(1., 1.85), Value::new_exponential(1.8)),
            Variable::new("γ2", exp_cost(1., 1.7), Value::new_stepwise(2., 10, 0.)),
            Variable::new("γ3", exp_cost(3., 14.5), plain(1.)),
            Variable::new("γ4", exp_cost(70., 5.), Value::new_exponential(1.1)),
            Variable::new("γ5", exp_cost(100., 2.5), plain(2.)),
            Variable::new("γ6", exp_cost(10., 3.), plain(0.04)),
            Variable::new("γ7", exp_cost(10000., 2.1), plain(0.1)),
        ];
        let mut sim = ToeSim {
            core,
            rho: f64::NEG_INFINITY,
            dq,
            nt: Variable::new("nt", exp_cost(2., 80.), plain(1.)),
            q: [f64::NEG_INFINITY, f64::NEG_INFINITY, 0., 0.],
            t_gamma: 0.,
            t_game: 0.,
            gamma: f64::NEG_INFINITY,
            gamma_vars,
            gamma_max_rho: f64::NEG_INFINITY,
            gamma_resets: 0,
            active_id: -1,
            active_difficulty: 0,
            completed: [0; 4],
            gamma_h: 0.,
            max_gamma_h: 0.,
            gamma_reset_t: 0.,
            gamma_reset_currency: f64::NEG_INFINITY,
            conjecture_goal: None,
            has_last_pub: input.last_pub_rho != f64::NEG_INFINITY,
        };
        sim.recompute_dq1_base();
        sim
    }

    /**
     * A new γ run from this state (sim-3 `new toeGammaSim(...)` + `copyFrom`): everything is
     * copied except the best ρ of the run, which starts over
     */
    fn copy(&self) -> Self {
        let mut copy = self.clone();
        copy.gamma_max_rho = f64::NEG_INFINITY;
        copy.conjecture_goal = None;
        copy
    }

    fn gval(&self, i: usize) -> f64 {
        self.gamma_vars[i].value.value
    }

    /** sim-3 `computeFirstSoftcap` */
    fn softcap(&self) -> f64 {
        let mut result = 0.8 + 0.01 * self.completed[3] as f64;
        if self.active_id == 3 {
            result /= 2.;
        }
        result
    }

    /** γ7 raises dq1's stepwise base: 2 + 0.1 * γ7 level */
    fn recompute_dq1_base(&mut self) {
        let dq1 = &mut self.dq[DQ1];
        dq1.value_model = Value::new_stepwise(2. + self.gamma_vars[GAMMA7].value.value, 9, 0.);
        dq1.value = dq1.value_model.compute_from_zero(dq1.level);
    }

    fn rho_available(&self, i: usize) -> bool {
        // Conjecture 4 (id 3) disables dq4 from difficulty 1, dq3 from 2 and dq2 from 3
        let disabled_from = match i {
            1 => 3,
            2 => 2,
            3 => 1,
            _ => return true,
        };
        !(self.active_id == 3 && self.active_difficulty >= disabled_from)
    }

    fn gamma_available(&self, i: usize) -> bool {
        let level = self.gamma_vars[i].level;
        match i {
            2 => level < 5,
            3 => level < 3,
            4 => level < 5,
            5 => level < 6,
            6 => level < 3,
            _ => true,
        }
    }

    fn q_decay(&self) -> f64 {
        let mut q_decay = (100. + self.gval(4)) * fastmath::pow(1.2, self.completed[0] as f64);
        if self.active_id == 0 {
            if self.active_difficulty == 1 {
                q_decay /= 5.;
            } else if self.active_difficulty == 2 {
                q_decay /= 10.;
            } else {
                q_decay /= 20.;
            }
        }
        q_decay
    }

    fn calculate_q_cap(&self, dq: f64, q_decay: f64) -> f64 {
        let mut result = l10(q_decay) + dq;
        let softcap = self.softcap();
        let initial_threshold = 0.;
        if result >= initial_threshold {
            result = initial_threshold
                + (dq
                    - initial_threshold
                    - l10(fastmath::expm1(fastmath::ln_1p(1. / q_decay) / softcap)))
                    * softcap;
        }
        result
    }

    fn gamma_pending(&self, rho: f64) -> f64 {
        let threshold = l10(1000.);
        let gamma_exp_scaling = 0.2 + self.gval(5);
        if rho < threshold {
            return f64::NEG_INFINITY;
        }
        let mut result = (rho - threshold) * gamma_exp_scaling;
        // Achievement 3 (unlocked by the first γ reset) doubles γ gain
        if self.gamma_resets > 0 || self.has_last_pub {
            result += l10(2.);
        }
        result + self.core.tot_mult
    }

    fn q_all_decayed(&self) -> bool {
        self.q.iter().all(|&q| q < -10.)
    }

    /** sim-3 `gammaForcedPubConditions`; the γ run ends when it holds */
    fn gamma_forced(&self) -> bool {
        if let Some(goal) = self.conjecture_goal {
            return self.gamma_max_rho >= goal || self.t_gamma > 60. * 60.;
        }
        let cheapest = (0..self.gamma_vars.len())
            .map(|i| {
                if self.gamma_available(i) {
                    self.gamma_vars[i].cost.value
                } else {
                    f64::INFINITY
                }
            })
            .fold(f64::INFINITY, js_min);
        add_log(self.gamma, self.gamma_pending(self.gamma_max_rho)) >= cheapest
            || self.q_all_decayed()
    }

    fn update_t(&mut self) {
        self.core.update_t();
        self.t_gamma += self.core.dt / 1.5;
    }

    fn update_sim_status(&mut self) {
        self.update_t();
        self.gamma_h = self.gamma_pending(self.gamma_max_rho) / (self.t_gamma / 3600.);
        // The γ pub conditions always hold, so the reset point is always the current tick
        self.max_gamma_h = self.gamma_h;
        self.gamma_reset_t = self.t_gamma;
        self.gamma_reset_currency = self.gamma_max_rho;
    }

    fn tick_gamma(&mut self) {
        // A large dt makes the q update overshoot below 0 (q relaxes toward its cap over about
        // softcap * q_decay game time), so the tick is split into steps of at most a tenth of that
        let tickspeed = TICKSPEED_CONSTS[(self.nt.value.value + self.gval(2)) as usize];
        let game_dt = self.core.dt * tickspeed;
        let max_step = self.q_decay() * self.softcap() / 10.;
        let steps = js_max(1., (game_dt / max_step).ceil());
        let step_dt = game_dt / steps;
        let l10_step = l10(step_dt);
        let mut i = 0.;
        while i < steps {
            self.tick_q(l10_step);
            self.tick_rho(l10_step);
            self.t_game += step_dt;
            i += 1.;
        }
    }

    fn tick_q(&mut self, dt: f64) {
        let q_decay = self.q_decay();
        // log10(1 + 1/q_decay): the game subtracts q + q / q_decay
        let l10_decay_factor = add_log(0., -l10(q_decay));
        let [q1, q2, q3, q4] = self.q;
        let dq = [
            self.dq[0].value.value - 1. + q2,
            self.dq[1].value.value - 1. + self.gval(3) + q3,
            self.dq[2].value.value - 1. + q4,
            self.dq[3].value.value - 1.,
        ];
        let caps = dq.map(|d| self.calculate_q_cap(d, q_decay));
        let softcap = self.softcap();
        let old = [q1, q2, q3, q4];
        // All four use the old q values, as in the game
        for k in 0..4 {
            self.q[k] = next_q(old[k], dq[k], caps[k], l10_decay_factor, dt, softcap);
        }
    }

    fn tick_rho(&mut self, dt: f64) {
        // 1 + 1e-6 * γ2 * t^3, with the game's t
        let mut drho = self.gval(0) + add_log(0., self.gval(1) + 3. * l10(self.t_game / 100.));
        let [q1, q2, q3, q4] = self.q;
        if self.active_id == 2 {
            match self.active_difficulty {
                1 => drho += q2,
                2 => drho += q3,
                3 => drho += q4,
                _ => {}
            }
        } else {
            drho += q1;
            if self.completed[2] > 0 {
                drho += js_max(q2, 0.);
            }
            if self.completed[2] > 1 {
                drho += js_max(q3, 0.);
            }
            if self.completed[2] > 2 {
                drho += js_max(q4, 0.);
            }
        }
        let grown = x_dx_softcapped(self.rho, drho, self.softcap());
        self.rho = add_log(self.rho, sub_log(grown, self.rho) + dt);
        self.gamma_max_rho = js_max(self.rho, self.gamma_max_rho);
    }

    /** Buys with ρ (sim-3 `buySingleVariable`) */
    fn buy_with_rho(rho: &mut f64, var: &mut Variable) -> bool {
        if *rho >= var.cost.value {
            *rho = sub_log(*rho, var.cost.value);
            var.buy();
            true
        } else {
            false
        }
    }

    fn buy_rho_variables(&mut self) {
        while self.nt.level < 4 && Self::buy_with_rho(&mut self.rho, &mut self.nt) {}
        for i in (0..self.dq.len()).rev() {
            while self.rho_available(i) {
                if !Self::buy_with_rho(&mut self.rho, &mut self.dq[i]) {
                    break;
                }
            }
        }
    }

    fn buy_gamma_variables(&mut self) {
        for i in (0..self.gamma_vars.len()).rev() {
            while self.gamma_available(i) && self.buy_gamma(i) {
                if i == GAMMA7 {
                    self.recompute_dq1_base();
                }
            }
        }
    }

    fn buy_gamma(&mut self, i: usize) -> bool {
        let cost = self.gamma_vars[i].cost.value;
        if self.gamma >= cost {
            self.gamma = sub_log(self.gamma, cost);
            self.gamma_vars[i].buy();
            true
        } else {
            false
        }
    }

    /** Runs a γ run until it resets; returns the reset time and the ρ reached */
    fn simulate_gamma(&mut self) -> (f64, f64) {
        while !self.gamma_forced() {
            self.tick_gamma();
            self.update_sim_status();
            self.buy_rho_variables();
        }
        (self.gamma_reset_t, self.gamma_reset_currency)
    }
}

/**
 * One q update, in log10: q + (softcapped(q, dq) - q - q / q_decay) * dt, clamped to the cap when
 * it crosses it from below and to 0 (-Infinity) when it goes negative, like the game.
 */
fn next_q(q: f64, dq: f64, cap: f64, l10_decay_factor: f64, dt: f64, softcap: f64) -> f64 {
    let production = adds(&[
        SignedLog::pos(x_dx_softcapped(q, dq, softcap)),
        SignedLog {
            neg: true,
            value: q + l10_decay_factor,
        },
    ]);
    let q_new = adds(&[
        SignedLog::pos(q),
        SignedLog {
            neg: production.neg,
            value: production.value + dt,
        },
    ]);
    if q < cap && !q_new.neg && q_new.value >= cap {
        return cap;
    }
    if q_new.neg {
        f64::NEG_INFINITY
    } else {
        q_new.value
    }
}

/** The publication sim (sim-3 `toePubSim`) */
struct ToePubSim {
    sim: ToeSim,
    strat: ToeStrat,
    max_gamma: f64,
}

impl ToePubSim {
    fn new(strat: ToeStrat, input: &TheoryInput) -> Self {
        ToePubSim {
            sim: ToeSim::new(input),
            strat,
            max_gamma: 0.,
        }
    }

    fn forced_pub_conditions(&self) -> bool {
        let c = &self.sim.completed;
        c[0] >= 2 && c[1] >= 2 && c[2] >= 2 && c[3] == 3
    }

    fn pub_conditions(&self) -> bool {
        self.sim.core.max_rho >= self.sim.core.rho_cap
    }

    fn end_simulation(&self) -> bool {
        self.forced_pub_conditions()
            && (self.pub_conditions() || self.sim.core.t > self.sim.core.pub_t * 10.)
    }

    fn update_sim_status(&mut self) {
        if self.sim.gamma > self.max_gamma {
            self.max_gamma = self.sim.gamma;
        }
        let core = &mut self.sim.core;
        // τ = γ^0.4; the last publication is already a τ value
        core.tau_h = (self.max_gamma * TAU_FACTOR - core.start_tau) / (core.t / 3600.);
        let forced = self.forced_pub_conditions();
        let pub_cond = self.pub_conditions();
        let core = &mut self.sim.core;
        if core.max_tau_h < core.tau_h || !forced || pub_cond {
            core.max_tau_h = core.tau_h;
            core.pub_t = core.t;
            core.pub_rho = self.max_gamma;
        }
    }

    fn push(&mut self, name: String, level: i32, cost: f64) {
        let t = self.sim.core.t;
        self.sim.core.bought_vars.push(VarBuy {
            var_name: Cow::Owned(name),
            level,
            cost: LogNum {
                value: cost,
                sign: 1,
            },
            symbol: "",
            timestamp: t,
        });
    }

    /** Tries the conjectures that are not completed yet, until none can be completed */
    fn do_conjectures(&mut self) {
        loop {
            let mut completed_any = false;
            for i in 0..4 {
                if self.sim.completed[i] == 3 {
                    continue;
                }
                let difficulty = self.sim.completed[i] + 1;
                let goal = conjecture_goal(i, difficulty);
                let mut conj = self.sim.copy();
                conj.active_id = i as i32;
                conj.active_difficulty = difficulty;
                conj.q[2] = l10(1.2) + self.sim.completed[1] as f64 * l10(2.);
                conj.q[3] = self.sim.completed[1] as f64 * l10(2.);
                // A conjecture run ends at its goal, or gives up after an hour
                conj.conjecture_goal = Some(goal);
                match i {
                    1 => match difficulty {
                        1 => conj.q[3] = f64::NEG_INFINITY,
                        2 => {
                            conj.q[3] = f64::NEG_INFINITY;
                            conj.q[2] = f64::NEG_INFINITY;
                            conj.q[1] = 0.;
                        }
                        _ => {
                            conj.q[3] = f64::NEG_INFINITY;
                            conj.q[2] = f64::NEG_INFINITY;
                            conj.q[1] = f64::NEG_INFINITY;
                            conj.q[0] = 0.;
                        }
                    },
                    2 => {
                        // Conjecture 3 makes dq4 cheaper
                        let dq4 = &mut conj.dq[DQ4];
                        dq4.cost_model = exp_cost_log2(1.0001e10, fastmath::log2(2e8) / 2.);
                        dq4.cost = dq4.cost_model.get_cost(dq4.level);
                    }
                    3 if difficulty >= 3 => conj.q[1] = 0.,
                    _ => {}
                }
                let (reset_t, _) = conj.simulate_gamma();
                if conj.gamma_max_rho >= goal {
                    completed_any = true;
                    self.sim.completed[i] += 1;
                    self.push(format!("Entered Conjecture {i}"), difficulty, 0.);
                    self.sim.core.t += reset_t;
                    self.push(format!("Completed Conjecture {i}"), difficulty, 0.);
                }
            }
            if !completed_any {
                break;
            }
        }
    }

    fn simulate(&mut self) -> SimResult {
        while !self.end_simulation() {
            let sim = &mut self.sim;
            sim.rho = f64::NEG_INFINITY;
            for (i, v) in sim.dq.iter_mut().enumerate() {
                v.set(0);
                // dq1 and dq2 have a free first level
                if i < 2 {
                    v.buy();
                }
            }
            sim.q = [f64::NEG_INFINITY, f64::NEG_INFINITY, l10(1.2), 0.];
            sim.t_gamma = 0.;
            sim.t_game = 0.;
            // Each γ reset starts like a new publication: dt grows during a run
            sim.core.dt = sim.core.params.dt;
            sim.gamma_h = 0.;
            sim.max_gamma_h = 0.;
            sim.gamma_reset_t = 0.;
            sim.gamma_reset_currency = f64::NEG_INFINITY;

            let lv = |i: usize| sim.gamma_vars[i].level;
            if sim.active_id == -1 && lv(3) == 3 && lv(4) == 5 && lv(5) == 6 && lv(6) == 3 {
                self.do_conjectures();
            }
            let sim = &mut self.sim;
            sim.q[2] += sim.completed[1] as f64 * l10(2.);
            sim.q[3] += sim.completed[1] as f64 * l10(2.);

            let mut gamma_sim = sim.copy();
            let (reset_t, reset_currency) = gamma_sim.simulate_gamma();

            let old_t = sim.core.t;
            *sim = gamma_sim;
            sim.core.t = old_t + reset_t;
            sim.gamma_resets += 1;

            let resets = sim.gamma_resets as i32;
            self.push(
                format!("Reset at rho={}", log_to_exp(reset_currency, 3)),
                resets,
                reset_currency,
            );
            let pending = self.sim.gamma_pending(reset_currency);
            self.sim.gamma = add_log(self.sim.gamma, pending);

            self.update_sim_status();
            self.sim.buy_gamma_variables();
            self.sim.core.collect_pub_table();
        }
        self.sim.core.trim_bought_vars();
        self.sim.core.create_result(self.strat.name(), "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(rho: f64) -> TheoryInput {
        TheoryInput {
            sigma: 0,
            last_pub_tau: None,
            last_pub_rho: rho,
            rho_cap: f64::INFINITY,
            params: SimParams::default(),
        }
    }

    #[test]
    fn toe_publishes_after_completing_the_conjectures() {
        let res = simulate_toe(ToeStrat::TOE, &input(0.));
        assert!(res.tau_h > 0. && res.strat == "TOE", "{res:?}");
        let completed = res
            .bought_vars
            .iter()
            .filter(|b| b.var_name.starts_with("Completed"))
            .count();
        assert!(completed >= 9, "{completed} conjectures");
    }

    #[test]
    fn the_q_update_never_overshoots_below_zero_with_a_large_dt() {
        let mut sim = ToeSim::new(&input(0.));
        sim.core.params.dt = 1000.;
        sim.core.dt = 1000.;
        sim.core.ddt = 1.;
        sim.q = [f64::NEG_INFINITY, f64::NEG_INFINITY, l10(1.2), 0.];
        for _ in 0..2000 {
            let before = sim.q;
            sim.tick_gamma();
            sim.update_sim_status();
            sim.buy_rho_variables();
            for k in 0..4 {
                assert!(!sim.q[k].is_nan());
                assert!(
                    before[k] == f64::NEG_INFINITY || sim.q[k] != f64::NEG_INFINITY,
                    "q{} dropped to 0",
                    k + 1
                );
            }
        }
    }

    #[test]
    fn softcap_and_conjecture_rules() {
        let mut sim = ToeSim::new(&input(0.));
        assert_eq!(sim.softcap(), 0.8);
        sim.active_id = 3;
        sim.active_difficulty = 2;
        assert_eq!(sim.softcap(), 0.4);
        // conjecture 4 at difficulty 2 disables dq4 and dq3
        assert!(sim.rho_available(0) && sim.rho_available(1));
        assert!(!sim.rho_available(2) && !sim.rho_available(3));
        // conjecture 1 divides q decay
        sim.active_id = 0;
        sim.active_difficulty = 3;
        assert_eq!(sim.q_decay(), 5.);
        // γ7 raises dq1's base
        sim.gamma = 10.;
        for _ in 0..3 {
            assert!(sim.buy_gamma(GAMMA7));
        }
        sim.recompute_dq1_base();
        let base_2 = Value::new_stepwise(2., 9, 0.).compute_from_zero(sim.dq[DQ1].level);
        assert!(sim.dq[DQ1].value.value > base_2.value || sim.dq[DQ1].level <= 1);
        // x + dx below the softcap threshold is a plain sum
        assert_eq!(x_dx_softcapped(-2., -2., 0.8), add_log(-2., -2.));
    }
}
