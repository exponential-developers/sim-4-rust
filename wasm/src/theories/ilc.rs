/*!
 * Theory ILC (unofficial), ported from sim-3 `src/Theories/Unofficial-CTs/ILC.ts`.
 *
 * Variable order: c1, c2, e1, e2, e3, e4
 *
 * The number of points N on the attractor grows with epsilon (the e variables); rho grows with
 * a power of N.
 */

use std::f64::consts::{E, LOG10_E};
use std::sync::OnceLock;

use crate::api::config::TheoryType;
use crate::theories::pubtable::{CodedTable, PubTable};
use crate::theories::traditional::*;
use crate::utils::{
    cost::{Cost, FirstFreeCost},
    lognum::LogNum,
    result::SimResult,
    value::Value,
    variable::Variable,
};

const C1: usize = 0;
const C2: usize = 1;
const E1: usize = 2;
const E2: usize = 3;
const E3: usize = 4;
const E4: usize = 5;

/** Last variable whose purchases can be coasted */
const LAST_COAST_VAR: usize = E4;

const MILESTONE_PRIORITY: [usize; 4] = [0, 1, 2, 3];
const MILESTONE_UNLOCKS: [f64; 10] = [25., 50., 75., 100., 120., 140., 160., 180., 200., 220.];

/** -ln(q) and ln(C) of the attractor for each level of the third milestone (sim-3's literals) */
#[allow(clippy::excessive_precision, clippy::unreadable_literal)]
const LOG_ATTRACTOR_POINTS_CONSTANTS: [[f64; 2]; 4] = [
    [0.55958025121547164703, -1.3567399465875839466],
    [0.553346, -1.40365],
    [0.54660087299449209265, -1.4589578628112783156],
    [0.539266, -1.52466],
];

/** Coasting caps of c1, c2, e1, e2, e3, e4 */
const COAST_CAPS: [i32; 6] = [6, 1, 5, 1, 1, 1];
const VAR_NAMES: [&str; 6] = ["c1", "c2", "e1", "e2", "e3", "e4"];

/** The converter (its multiplier factor is computed with log10, as in sim-3) */
fn converter() -> Converter {
    Converter {
        r9_affected: false,
        tau_factor: 2.,
        mult_exponent: 0.39,
        mult_factor: -7. * l10(E),
    }
}

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum IlcStrat {
        ILC,
        ILCd,
        ILCCoast,
        ILCdCoast,
        ILCPTCoast,
        ILCdPTCoast,
    }
}

impl IlcStrat {
    fn is_coast(self) -> bool {
        !matches!(self, IlcStrat::ILC | IlcStrat::ILCd)
    }

    fn is_d(self) -> bool {
        matches!(
            self,
            IlcStrat::ILCd | IlcStrat::ILCdCoast | IlcStrat::ILCdPTCoast
        )
    }

    fn uses_pub_table(self) -> bool {
        matches!(self, IlcStrat::ILCPTCoast | IlcStrat::ILCdPTCoast)
    }

    /** Strat without "Coast" and "PT" (the first pass of the coasting strats) */
    fn first_pass(self) -> IlcStrat {
        if self.is_d() {
            IlcStrat::ILCd
        } else {
            IlcStrat::ILC
        }
    }
}

fn pub_table() -> &'static PubTable {
    static TABLE: OnceLock<PubTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        PubTable::decode_format1(&CodedTable::parse(include_str!(
            "data/table_ilc_0_01_ilccoast_coded.json"
        )))
    })
}

/** Simulates ILC with the given strat (sim-3 `ilc` function) */
pub fn simulate_ilc(strat: IlcStrat, input: &TheoryInput) -> SimResult {
    if !strat.is_coast() {
        return IlcSim::new(strat, input).simulate();
    }
    let mut sim1 = IlcSim::new(strat.first_pass(), input);
    let res1 = sim1.simulate();
    let mut sim2 = IlcSim::new(strat, input);
    for (i, name) in VAR_NAMES.iter().enumerate() {
        let last = get_last_level(name, &res1.bought_vars);
        if last != 0 {
            let last_cost = get_last_level_cost(name, &res1.bought_vars);
            sim2.vars[i].set_original_cap(last);
            if last_cost < input.last_pub_rho - 2. {
                sim2.vars[i].configure_cap(0);
            } else {
                sim2.vars[i].configure_cap(COAST_CAPS[i]);
            }
        } else {
            // So deep that no purchase was recorded
            sim2.vars[i].set_original_cap(sim1.vars[i].level);
            sim2.vars[i].configure_cap(0);
        }
    }
    sim2.simulate()
}

#[derive(Debug, Clone)]
struct IlcSim {
    core: TraditionalCore,
    strat: IlcStrat,
    coast: bool,
    d: bool,
    vars: Vec<Variable>,
    rhodot: f64,
    prev_milestone_count: i64,
    /** log10(3 + n) for n = c1 level % 10 */
    l10_3_plus: [f64; 10],
}

impl IlcSim {
    fn new(strat: IlcStrat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::ILC, false, input, converter(), 6.);
        core.milestone_unlocks = MILESTONE_UNLOCKS.to_vec();
        core.milestones_max = vec![2, 2, 3, 3];
        // ILC never reads the publication multiplier during the sim
        core.track_cur_mult = false;

        let mut c1 = Variable::new(
            "c1",
            Cost::new(FirstFreeCost::new(exp_cost_model(1., 2.37))),
            Value::new_stepwise(2., 10, 0.),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        c1.buy();
        let vars = vec![
            c1,
            Variable::new("c2", exp_cost(2., 2560.), Value::new_exponential(2.)),
            Variable::new("e1", exp_cost(10., 50.), Value::new_stepwise(2., 6, 1.)),
            Variable::new("e2", exp_cost(25., 150000.), Value::new_exponential(2.)),
            Variable::new("e3", exp_cost(1e10, 4e9), Value::new_exponential(3.)),
            Variable::new("e4", exp_cost(1e20, 1.9e14), Value::new_exponential(4.)),
        ];

        if strat.uses_pub_table()
            && let Some(next_rho) = pub_table().lookup(core.last_pub_rho, 100.)
        {
            core.do_sim_end_conditions = false;
            core.pub_target = Some(next_rho);
        }

        let mut sim = IlcSim {
            core,
            strat,
            coast: strat.is_coast(),
            d: strat.is_d(),
            vars,
            rhodot: 0.,
            prev_milestone_count: -1,
            l10_3_plus: std::array::from_fn(|n| l10(3. + n as f64)),
        };
        sim.update_milestones_no_ms();
        sim.update_rho_dot();
        sim
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    /** sim-3 `updateMilestonesNoMS`: only recalculates when the number of milestones changes */
    fn update_milestones_no_ms(&mut self) {
        let rho = self.core.max_rho.max(self.core.last_pub_rho);
        let count = binary_insertion_search(&MILESTONE_UNLOCKS, rho) as i64;
        if count != self.prev_milestone_count {
            self.prev_milestone_count = count;
            self.core.set_milestones_at(rho, &MILESTONE_PRIORITY);
            self.update_rho_dot();
        }
    }

    /** sim-3 `calculateN`: number of attractor points */
    fn calculate_n(index: usize, epsilon: f64) -> f64 {
        let [neg_ln_q, ln_c] = LOG_ATTRACTOR_POINTS_CONSTANTS[index];
        js_max(((ln_c + epsilon / LOG10_E) / neg_ln_q).ceil(), 0.)
    }

    fn update_rho_dot(&mut self) {
        let ms = &self.core.milestones;
        let mut epsilon = self.val(E1) + self.val(E2);
        if ms[0] > 0 {
            epsilon += self.val(E3);
        }
        if ms[0] > 1 {
            epsilon += self.val(E4);
        }
        let n_base = 1.1 + 0.01 * ms[1] as f64;
        let n = Self::calculate_n(ms[2] as usize, epsilon);

        self.rhodot = self.core.tot_mult
            + self.val(C1) * (1. + 0.02 * ms[3] as f64)
            + self.val(C2)
            + n * l10(n_base);
    }

    fn buying_condition(&self, i: usize) -> bool {
        let should_buy = self.vars[i].should_buy;
        if i == C1 && self.d {
            let level = (self.vars[C1].level % 10) as usize;
            return should_buy && self.cost(C1) + self.l10_3_plus[level] < self.cost(C2);
        }
        should_buy
    }

    fn variable_available(&self, i: usize) -> bool {
        match i {
            E3 => self.core.milestones[0] > 0,
            E4 => self.core.milestones[0] > 1,
            _ => true,
        }
    }

    fn tick(&mut self) {
        self.core.rho += LogNum {
            value: self.rhodot + l10(self.core.dt),
            sign: 1,
        };
    }

    fn buy_variables(&mut self) {
        let mut bought = false;
        for i in (0..self.vars.len()).rev() {
            while self.core.rho.value > self.cost(i)
                && self.buying_condition(i)
                && self.variable_available(i)
            {
                self.core.record_purchase(&self.vars[i]);
                let cost = self.vars[i].cost;
                self.core.rho -= cost;
                self.vars[i].buy();
                bought = true;
                self.on_variable_purchased(i);
            }
        }
        if bought {
            self.update_rho_dot();
        }
    }

    fn on_variable_purchased(&mut self, id: usize) {
        if id <= LAST_COAST_VAR && self.coast {
            let v = &mut self.vars[id];
            if v.should_buy && v.coasting_cap_reached() {
                if v.original_cap == v.level as f64 {
                    v.should_buy = false;
                } else {
                    v.should_fork = true;
                }
            }
        }
    }

    /** Simulates a fork where variable `id` is no longer bought (sim-3 `doForkVariable`) */
    fn do_fork_variable(&mut self, id: usize) {
        self.vars[id].should_fork = false;
        let mut fork = self.clone();
        // sim-3 builds the fork with its constructor, which computes milestones from the last
        // pub rho; copyFrom copies neither the milestones nor their count (but copies rhodot).
        let lpr = fork.core.last_pub_rho.max(0.);
        fork.core.set_milestones_at(lpr, &MILESTONE_PRIORITY);
        fork.prev_milestone_count = binary_insertion_search(&MILESTONE_UNLOCKS, lpr) as i64;
        fork.vars[id].stop_buying();
        let res = fork.simulate();
        self.core.best_fork_res = Some(best_result(res, self.core.best_fork_res.take()));
    }

    fn simulate(&mut self) -> SimResult {
        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            self.update_milestones_no_ms();
            self.buy_variables();
            for id in 0..=LAST_COAST_VAR {
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
            for (i, name) in VAR_NAMES.iter().enumerate() {
                extra += &self.vars[i].prepare_extra_for_cap(get_last_level(name, bv));
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
            last_pub_rho: rho,
            last_pub_tau: None,
            rho_cap: f64::INFINITY,
            params: SimParams::default(),
        }
    }

    #[test]
    fn attractor_points_never_go_negative() {
        assert_eq!(IlcSim::calculate_n(0, f64::NEG_INFINITY), 0.);
        assert!(IlcSim::calculate_n(3, 10.) > IlcSim::calculate_n(0, 10.));
    }

    #[test]
    fn pub_table_strats_publish_at_the_table_rho() {
        let rho = 100.;
        let target = pub_table().lookup(rho, 100.).expect("table entry");
        let res = simulate_ilc(IlcStrat::ILCPTCoast, &input(rho, 100));
        assert!(res.pub_rho.value >= target, "{res:?}");
        assert!(res.strat.contains(" c1: "), "{}", res.strat);
    }
}
