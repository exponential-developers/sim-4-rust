/*!
 * Theory TC (unofficial), ported from sim-3 `src/Theories/Unofficial-CTs/TC.ts`.
 *
 * Variable order: c1, r1, r2, c2, dTexp, p1, p2, c1exp, r1exp, r2exp, c1base
 *
 * The theory simulates a heated aluminium element controlled by a PID controller; how far the
 * temperature is from a target decides how fast p grows.
 */

use std::f64::consts::{E, LOG10_E};

use crate::api::config::TheoryType;
use crate::theories::pubtable::js_round;
use crate::theories::traditional::*;
use crate::utils::{
    cost::{Cost, ExponentialCost},
    fastmath,
    lognum::LogNum,
    result::SimResult,
    value::Value,
    variable::Variable,
};

const C1: usize = 0;
const R1: usize = 1;
const R2: usize = 2;
const C2: usize = 3;
const DTEXP: usize = 4;
const P1: usize = 5;
const P2: usize = 6;
const C1EXP: usize = 7;
const R1EXP: usize = 8;
const R2EXP: usize = 9;
const C1BASE: usize = 10;

const VARIABLE_COUNT: usize = 11;

/** Order in which milestones are assigned */
const MILESTONE_PRIORITY: [usize; 6] = [0, 1, 2, 3, 4, 5];

/** Rho at which each milestone is unlocked */
const MILESTONE_UNLOCKS: [f64; 8] = [10., 50., 100., 400., 420., 440., 1000., 1200.];

const CONVERTER: Converter = Converter {
    r9_affected: false,
    tau_factor: 0.45,
    mult_exponent: 0.2 / 0.45,
    mult_factor: -std::f64::consts::LOG10_2,
};

/** Controller settings (all strats use the same): kp, ki, kd and the set point */
const KP: f64 = 5.;
const KI: f64 = 30.;
const KD: f64 = 11.;
const SET_POINT: f64 = 189.;
/** Automation settings: the temperature is reset to the amplitude every `FREQUENCY` seconds */
const AMPLITUDE: f64 = 200.;
const FREQUENCY: f64 = 1.;

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TcStrat {
        TC,
        TCd,
        TCCoast,
        TCdCoast,
    }
}

impl TcStrat {
    fn is_coast(self) -> bool {
        self.name().contains("Coast")
    }

    fn is_active(self) -> bool {
        matches!(self, TcStrat::TCd | TcStrat::TCdCoast)
    }
}

/**
 * sim-3 `lfsr16BitScrambled`: a 16-bit LFSR clocked 16 times, as a number between 0 and 1.
 * Gives the "random" target temperature of a publication.
 */
fn lfsr16_bit_scrambled(seed: f64) -> f64 {
    // Map input to 16-bit space and ensure non-zero
    let mut state = (seed % 65536.) as i64;
    if state == 0 {
        state = 1;
    }
    // Clock 16 times to completely roll over the 16-bit register
    for _ in 0..16 {
        if state & 1 != 0 {
            // Shift right and XOR with 16-bit polynomial 0xB400
            state = (state >> 1) ^ 0xB400;
        } else {
            state >>= 1;
        }
    }
    state as f64 / 65535.
}

/** Simulates TC with the given strat (sim-3 `tc` function) */
pub fn simulate_tc(strat: TcStrat, input: &TheoryInput) -> SimResult {
    if !strat.is_coast() {
        return TcSim::new(strat, input).simulate();
    }
    let first_pass = TcStrat::from_name(&strat.name().replacen("Coast", "", 1)).unwrap_or(strat);
    let res1 = TcSim::new(first_pass, input).simulate();
    let mut sim = TcSim::new(strat, input);
    sim.vars[R1].set_original_cap(get_last_level("r1", &res1.bought_vars));
    sim.vars[R1].configure_cap(20);
    sim.vars[P1].set_original_cap(get_last_level("p1", &res1.bought_vars));
    sim.vars[P1].configure_cap(1);
    sim.simulate()
}

#[derive(Debug, Clone)]
struct TcSim {
    core: TraditionalCore,
    strat: TcStrat,
    coast: bool,
    vars: Vec<Variable>,
    r: f64,
    p: f64,
    achievement_multi: f64,
    /** Controller state */
    error: [f64; 2],
    timer: f64,
    integral: f64,
    /** Temperature of the element, and the temperature p likes */
    temperature: f64,
    target_temperature: f64,
    l10_achievement_multi: f64,
    l10_10: f64,
}

impl TcSim {
    fn new(strat: TcStrat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::TC, false, input, CONVERTER, 8.);
        core.milestone_unlocks = MILESTONE_UNLOCKS.to_vec();
        core.milestones_max = vec![1, 1, 1, 2, 1, 2];
        // The pub can't end below the last pub, and the sim ends at a large pub multiplier
        core.forced_pub_min_rho = Some(core.last_pub_rho);
        core.sim_end_cur_mult = Some(15.);
        core.cur_mult = 0.;

        let seed = js_round(core.start_tau);
        let rng = lfsr16_bit_scrambled(seed);
        let target_temperature = js_round(rng * (120. - 60.) + 60.);

        let lpr = core.last_pub_rho;
        let achievement_multi = if lpr >= 750. {
            30.
        } else if lpr >= 600. {
            10.
        } else {
            1.
        };

        let log = |value: f64| LogNum { value, sign: 1 };
        let linear = |power: f64, offset: f64| Value::new(PlainLinearValue { power, offset });
        let vars = vec![
            Variable::new("c1", exp_cost(1e5, 18.), Value::new_exponential(2.75)),
            Variable::new("r1", exp_cost(10., 1.585), Value::new_stepwise(2., 10, 0.)),
            Variable::new("r2", exp_cost(1000., 8.), Value::new_exponential(2.)),
            Variable::new(
                "c2",
                // "1e420" is too large for a number
                Cost::Exponential(ExponentialCost::new(
                    log(420.),
                    log(parse_value(fastmath::pow(10., 4.5))),
                )),
                Value::new_exponential(E),
            ),
            Variable::new("dTexp", exp_cost(1e15, 1000.), linear(1., 0.)),
            Variable::new(
                "p1",
                Cost::Exponential(ExponentialCost::new(log(800.), log(parse_value(1e4)))),
                Value::new_stepwise(2., 10, 1.),
            ),
            Variable::new(
                "p2",
                Cost::Exponential(ExponentialCost::new(log(830.), log(parse_value(1e10)))),
                Value::new_exponential(2.2),
            ),
            Variable::new("c1exp", exp_cost(1e30, 1e30), linear(0.05, 1.)),
            Variable::new("r1exp", exp_cost(1e40, 1e40), linear(0.05, 1.)),
            Variable::new("r2exp", exp_cost(1e150, 1e175), linear(0.03, 1.)),
            Variable::new("c1base", exp_cost(1e200, 1e175), linear(0.125, 2.75)),
        ];

        let mut sim = TcSim {
            core,
            strat,
            coast: strat.is_coast(),
            vars,
            r: 0.,
            p: 0.,
            achievement_multi,
            error: [0., 0.],
            timer: 0.,
            integral: 0.,
            temperature: 30.,
            target_temperature,
            l10_achievement_multi: l10(achievement_multi),
            l10_10: l10(10.),
        };
        // Permanent upgrades already affordable at the start of the publication
        for i in C1EXP..VARIABLE_COUNT {
            while sim.cost(i) <= lpr && sim.variable_available(i) {
                sim.vars[i].buy();
            }
        }
        sim.recompute_c1_base();
        sim.core.update_milestones(&MILESTONE_PRIORITY);
        sim
    }

    /** sim-3's fork: a new sim (milestones from the last pub rho alone) with this one's state */
    fn copy(&self) -> TcSim {
        let mut fork = self.clone();
        // copyFrom doesn't copy the temperature or the pub multiplier
        fork.temperature = 30.;
        fork.core.cur_mult = 0.;
        fork.core
            .set_milestones_at(fork.core.last_pub_rho.max(0.), &MILESTONE_PRIORITY);
        fork.recompute_c1_base();
        fork
    }

    /** c1's growth per level is the value of c1base */
    fn recompute_c1_base(&mut self) {
        let power = self.val(C1BASE);
        let v = &mut self.vars[C1];
        v.value_model = Value::new_exponential(power);
        v.value = v.value_model.compute_from_zero(v.level);
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    fn buying_condition(&self, i: usize) -> bool {
        let r1_cheap = || self.cost(R1) + self.l10_10 < self.cost(R2);
        match i {
            R1 => {
                let ok = !self.coast || self.vars[R1].should_buy;
                ok && (!self.strat.is_active() || r1_cheap())
            }
            P1 => !self.coast || self.vars[P1].should_buy,
            _ => true,
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        match i {
            C2 => self.core.milestones[4] >= 0 && self.vars[C2].level <= 75,
            DTEXP => self.vars[DTEXP].level <= 100,
            P1 | P2 => self.achievement_multi == 30.,
            C1EXP | R1EXP => self.vars[i].level < 3,
            R2EXP | C1BASE => self.vars[i].level < 2,
            _ => true,
        }
    }

    fn tick(&mut self) {
        // System update
        const Q: f64 = 20.; // max heat duty in W
        const H: f64 = 5.; // thermal passive convection coefficient for Al (W/m^2 k)
        const CP: f64 = 0.89; // heat capacity for Al (J/g/k)
        const AREA: f64 = 0.024; // area of element (m^2)
        const MASS: f64 = 10.; // grams
        const TC: f64 = 30.;
        const SYSTEM_DT: f64 = 0.1;

        let ms = &self.core.milestones;
        let (ms1, ms2, ms3, ms4) = (ms[1], ms[2], ms[3], ms[4]);
        let vki = if ms1 > 0 { KI } else { 0. };
        let vkd = if ms2 > 0 { KD } else { 0. };
        self.timer += SYSTEM_DT;

        if self.timer > FREQUENCY {
            self.temperature = AMPLITUDE;
            self.timer = 0.;
            self.integral = 0.;
        }

        self.error[1] = self.error[0];
        self.error[0] = SET_POINT - self.temperature;
        self.integral += self.error[0];
        let derivative = (self.error[0] - self.error[1]) / SYSTEM_DT;
        // Anti-windup scheme
        self.integral = self.integral.clamp(-100., 100.);
        // range 0-512
        let output = js_round(js_max(
            0.,
            js_min(
                KP * self.error[0] + vki * self.integral + vkd * derivative,
                512.,
            ),
        ));

        // Heating simulation
        let supplied_heat = Q * output / 512.;
        let d_t = (1. / MASS / CP * (supplied_heat - (self.temperature - TC) * H * AREA)).abs();
        let exponential_term = (supplied_heat - H * AREA * (self.temperature - TC))
            * fastmath::pow(E, -SYSTEM_DT / MASS / CP);
        self.temperature = TC + (supplied_heat - exponential_term) / (H * AREA);

        let l10dt = l10(self.core.dt);

        // P update
        if self.achievement_multi == 30. {
            let d_p = self.val(P1)
                + self.val(P2)
                + LOG10_E
                    * (-0.2
                        * fastmath::pow(0.08, ms4 as f64)
                        * (self.temperature - self.target_temperature).abs());
            self.p = add_log(self.p, d_p + l10dt);
        }

        // R update
        let r1_exp = self.val(R1EXP);
        let r2_exp = self.val(R2EXP);
        let d_r =
            self.val(R1) * r1_exp + self.val(R2) * r2_exp - l10(1. + l10(1. + self.error[0].abs()));
        self.r = add_log(self.r, d_r + l10dt);

        // rho update
        let c1_exp = self.val(C1EXP);
        let vc1 = self.val(C1) * c1_exp;
        let vc2 = if ms4 > 0 { self.val(C2) } else { 0. };
        let mr_exp = ms3;
        self.core.rho += LogNum {
            value: self.p
                + self.r * (1. + mr_exp as f64 * 0.001)
                + (vc1 + vc2 + l10(d_t) * (2. + self.val(DTEXP))) / 2.
                + l10dt
                + self.core.tot_mult
                + self.l10_achievement_multi,
            sign: 1,
        };
    }

    fn buy_variables(&mut self) {
        // TC has no onAnyVariablePurchased step, so whether something was bought is not tracked
        for i in (0..VARIABLE_COUNT).rev() {
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
        if id == C1BASE {
            self.recompute_c1_base();
        }
        if matches!(id, R1 | P1) && self.coast {
            let v = &mut self.vars[id];
            if v.should_buy && v.coasting_cap_reached() {
                v.should_fork = true;
            }
        }
    }

    /** Simulates a fork where variable `id` is no longer bought (sim-3 `doForkVariable`) */
    fn do_fork_variable(&mut self, id: usize) {
        self.vars[id].should_fork = false;
        let mut fork = self.copy();
        fork.vars[id].stop_buying();
        let res = fork.simulate();
        self.core.best_fork_res = Some(best_result(res, self.core.best_fork_res.take()));
    }

    fn simulate(&mut self) -> SimResult {
        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            self.core.update_milestones(&MILESTONE_PRIORITY);
            self.buy_variables();
            for id in [R1, P1] {
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
            extra = self.vars[R1].prepare_extra_for_cap(get_last_level("r1", bv));
            let last_p1 = get_last_level("p1", bv);
            if last_p1 > 0 {
                extra += &self.vars[P1].prepare_extra_for_cap(last_p1);
            }
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
            last_pub_tau: None,
            last_pub_rho: rho,
            rho_cap: f64::INFINITY,
            params: SimParams::default(),
        }
    }

    #[test]
    fn lfsr_gives_numbers_between_0_and_1() {
        for seed in [0., 1., 123., 65535., 70000.] {
            let v = lfsr16_bit_scrambled(seed);
            assert!((0. ..=1.).contains(&v), "{seed}: {v}");
        }
        // zero is mapped to 1, and the seed wraps at 65536
        assert_eq!(lfsr16_bit_scrambled(0.), lfsr16_bit_scrambled(1.));
        assert_eq!(lfsr16_bit_scrambled(65537.), lfsr16_bit_scrambled(1.));
        // the target temperature is between 60 and 120
        let sim = TcSim::new(TcStrat::TC, &input(100., 60));
        assert!((60. ..=120.).contains(&sim.target_temperature));
    }

    #[test]
    fn achievements_multiply_p_from_e750() {
        assert_eq!(
            TcSim::new(TcStrat::TC, &input(100., 60)).achievement_multi,
            1.
        );
        assert_eq!(
            TcSim::new(TcStrat::TC, &input(650., 60)).achievement_multi,
            10.
        );
        let sim = TcSim::new(TcStrat::TC, &input(800., 60));
        assert_eq!(sim.achievement_multi, 30.);
        assert!(sim.variable_available(P1) && sim.variable_available(P2));
        assert!(!TcSim::new(TcStrat::TC, &input(100., 60)).variable_available(P1));
    }

    #[test]
    fn permanent_upgrades_that_are_affordable_are_bought_at_the_start() {
        let sim = TcSim::new(TcStrat::TC, &input(250., 60));
        assert!(sim.vars[C1EXP].level > 0 && sim.vars[R1EXP].level > 0);
        assert_eq!(sim.vars[C1BASE].level, 1);
        // c1's growth follows c1base
        assert_eq!(sim.val(C1BASE), 2.75 + 0.125);
    }

    #[test]
    fn coasting_strats_report_r1_and_p1() {
        let res = simulate_tc(TcStrat::TCdCoast, &input(100., 60));
        assert!(
            res.tau_h > 0. && res.strat.starts_with("TCdCoast r1: "),
            "{}",
            res.strat
        );
        let res = simulate_tc(TcStrat::TCCoast, &input(800., 100));
        assert!(res.strat.contains(" r1: "), "{}", res.strat);
    }
}
