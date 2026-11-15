/*!
 * Theory BT (unofficial), ported from sim-3 `src/Theories/Unofficial-CTs/BT.ts`.
 *
 * Variable order: tai, rao, tay
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

const TAI: usize = 0;
const RAO: usize = 1;
const TAY: usize = 2;

/** Order in which milestones are assigned */
const MILESTONE_PRIORITY: [usize; 4] = [1, 0, 2, 3];

/** Rho at which each milestone is unlocked */
const MILESTONE_UNLOCKS: [f64; 13] = [
    20., 40., 60., 100., 150., 250., 750., 850., 950., 1050., 1150., 1250., 1450.,
];

const TAU_FACTOR: f64 = 0.4;

const CONVERTER: Converter = Converter {
    r9_affected: false,
    tau_factor: TAU_FACTOR,
    mult_exponent: 1.25,
    mult_factor: 0.,
};

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum BtStrat {
        BT,
        BTd,
    }
}

/** Simulates BT with the given strat (sim-3 `bt` function) */
pub fn simulate_bt(strat: BtStrat, input: &TheoryInput) -> SimResult {
    BtSim::new(strat, input).simulate()
}

struct BtSim {
    core: TraditionalCore,
    strat: BtStrat,
    vars: Vec<Variable>,
    /** sim-3 `parseLog10String("9e599")` and `("1.05e1500")` */
    log_9e599: f64,
    log_1_05e1500: f64,
}

impl BtSim {
    fn new(strat: BtStrat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::BT, false, input, CONVERTER, 7.);
        core.milestone_unlocks = MILESTONE_UNLOCKS.to_vec();
        core.milestones_max = vec![3, 3, 6, 1];
        // BT never reads the publication multiplier during the sim
        core.track_cur_mult = false;

        let mut tai = Variable::new(
            "tai",
            Cost::new(FirstFreeCost::new(exp_cost_model(15., 2.))),
            Value::new_stepwise(2., 10, 0.),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        tai.buy();
        let vars = vec![
            tai,
            Variable::new("rao", exp_cost(5., 10.), Value::new_exponential(2.)),
            Variable::new("tay", exp_cost_log2(1e10, 10.), Value::new_exponential(10.)),
        ];

        let mut sim = BtSim {
            core,
            strat,
            vars,
            log_9e599: 599. + l10(9.),
            log_1_05e1500: 1500. + l10(1.05),
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

    fn buying_condition(&self, i: usize) -> bool {
        if self.strat == BtStrat::BT || i != TAI {
            return true;
        }
        let lvl = (self.vars[TAI].level % 10) as f64;
        let base = if self.core.last_pub_rho < 275. {
            12.
        } else {
            10.
        };
        self.cost(TAI) + l10(base + lvl) < self.cost(RAO)
    }

    fn variable_available(&self, i: usize) -> bool {
        i != TAY || self.core.milestones[2] > 0
    }

    fn tick(&mut self) {
        let ms = &self.core.milestones;
        let (ms0, ms1, ms2, ms3) = (ms[0], ms[1], ms[2], ms[3]);
        let tay_exponent = (((ms2 + 1) * (ms2 + 2)) as f64 * 0.5 - 1.) * 0.0003;
        let vtai = self.val(TAI) * (1. + 0.08 * ms0 as f64);
        let vrao = self.val(RAO) * (1. + 0.077 * ms1 as f64);
        let vtay = self.val(TAY) * if ms3 == 0 { tay_exponent } else { 0.015 };
        let rhodot = self.core.tot_mult + vtai + vrao + vtay;

        self.core.rho += LogNum {
            value: rhodot + l10(self.core.dt),
            sign: 1,
        };
        if ms3 == 1 && self.core.max_rho.max(self.core.last_pub_rho) * TAU_FACTOR < self.log_9e599 {
            self.core.rho = LogNum {
                value: self.log_1_05e1500,
                sign: 1,
            };
        }
    }

    fn buy_variables(&mut self) {
        // BT has no onAnyVariablePurchased step, so whether something was bought is not tracked
        for i in (0..self.vars.len()).rev() {
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
        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            self.core.update_milestones(&MILESTONE_PRIORITY);
            self.buy_variables();
            self.core.collect_pub_table();
        }
        self.core.trim_bought_vars();
        self.core.create_result(self.strat.name(), "")
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
    fn bt_publishes_and_the_active_strat_buys_tai_carefully() {
        let res = simulate_bt(BtStrat::BT, &input(100., 60));
        assert!(res.tau_h > 0. && res.strat == "BT", "{res:?}");
        let sim = BtSim::new(BtStrat::BTd, &input(100., 60));
        assert!(sim.buying_condition(RAO) && sim.variable_available(TAI));
        assert!(!sim.variable_available(TAY));
    }

    #[test]
    fn the_last_milestone_sets_rho_to_a_huge_value() {
        // milestone 3 (the "1e1500" jump) unlocks at e1450 or more
        let mut sim = BtSim::new(BtStrat::BT, &input(1470., 60));
        assert_eq!(sim.core.milestones[3], 1);
        sim.tick();
        // tau (rho * 0.4) is below e599.95, so rho is set to 1.05e1500
        assert_eq!(sim.core.rho.value, 1500. + l10(1.05));
        // from e1500 on (tau above e599.95) there is no jump
        let mut sim = BtSim::new(BtStrat::BT, &input(1500., 60));
        sim.tick();
        assert!(sim.core.rho.value < 1600.);
    }
}
