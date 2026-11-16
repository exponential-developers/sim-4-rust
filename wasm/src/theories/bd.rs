/*!
 * Theory BD (Binomial Distribution, unofficial), ported from sim-3
 * `src/Theories/Unofficial-CTs/BD.ts`.
 *
 * Variable order: a1, a2, b1, b2, c1, c2, n
 *
 * k walks along row n of Pascal's triangle; the row term rewards the binomial coefficients
 * (and their moments) up to the current position.
 */

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
const C1: usize = 4;
const C2: usize = 5;
const N: usize = 6;

const TAU_FACTOR: f64 = 0.4;

/** Milestone order, and the order SingleMS uses before its swap */
const PRIORITY: [usize; 9] = [1, 2, 0, 3, 4, 5, 7, 8, 6];
const SINGLE_MS_PRIORITY: [usize; 4] = [1, 2, 0, 3];

/** Tau at which each milestone is unlocked (converted to rho by dividing by the tau factor) */
const MILESTONE_UNLOCKS_TAU: [f64; 17] = [
    4., 10., 25., 45., 65., 130., 190., 250., 310., 340., 370., 400., 425., 455., 485., 535., 585.,
];

const Q_ETA_SCALE: f64 = 10.;

const CONVERTER: Converter = Converter {
    r9_affected: false,
    tau_factor: TAU_FACTOR,
    mult_exponent: 0.375,
    mult_factor: 0.,
};

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum BdStrat {
        BD,
        BDCoast,
        BD2,
        BD2Coast,
        BDManageKSingleMS,
        BDManageKSingleMSCoast,
    }
}

impl BdStrat {
    fn is_coast(self) -> bool {
        matches!(
            self,
            BdStrat::BDCoast | BdStrat::BD2Coast | BdStrat::BDManageKSingleMSCoast
        )
    }

    fn is_single_ms(self) -> bool {
        matches!(
            self,
            BdStrat::BDManageKSingleMS | BdStrat::BDManageKSingleMSCoast
        )
    }

    fn without_coast(self) -> BdStrat {
        match self {
            BdStrat::BDCoast => BdStrat::BD,
            BdStrat::BD2Coast => BdStrat::BD2,
            BdStrat::BDManageKSingleMSCoast => BdStrat::BDManageKSingleMS,
            s => s,
        }
    }
}

/** Buying rules shared by a strat and its Coast version */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rules {
    Plain,
    Bd2,
    ManageK,
}

fn rules(strat: BdStrat) -> Rules {
    match strat.without_coast() {
        BdStrat::BD2 => Rules::Bd2,
        BdStrat::BDManageKSingleMS => Rules::ManageK,
        _ => Rules::Plain,
    }
}

/** Simulates BD with the given strat (sim-3 `bd` function) */
pub fn simulate_bd(strat: BdStrat, input: &TheoryInput) -> SimResult {
    if !strat.is_coast() {
        return BdSim::new(strat, input).simulate();
    }
    let res1 = BdSim::new(strat.without_coast(), input).simulate();
    let mut sim = BdSim::new(strat, input);
    for (id, name) in [(A1, "a1"), (B1, "b1"), (C1, "c1")] {
        sim.vars[id].set_original_cap(get_last_level(name, &res1.bought_vars));
        sim.vars[id].configure_cap(2);
    }
    sim.simulate()
}

/** Sums of row `row` of Pascal's triangle up to `position` (sim-3 `getRowTerms`), as log10 */
struct RowTerms {
    coefficient: f64,
    sum: f64,
    second: f64,
    fourth: f64,
}

fn row_terms(row: f64, position: f64) -> RowTerms {
    let target = (row / 2.).floor();
    let pos = js_min(target, js_max(0., position.floor()));
    let mut coefficient = 0.;
    let (mut sum, mut second, mut fourth) =
        (f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    if pos == target {
        let mut j = 1.;
        while j <= (row / 2.).floor() {
            coefficient += l10(row - j + 1.) - l10(j);
            j += 1.;
        }
        sum = row * l10(2.);
        second = sum + l10(row) - l10(4.);
        fourth = sum + sub_log(l10(3.) + l10(row) * 2., l10(2.) + l10(row)) - l10(16.);
    } else {
        let mut j = 0.;
        while j <= pos {
            if j > 0. {
                coefficient += l10(row - j + 1.) - l10(j);
            }
            let copies = if 2. * j == row { 1. } else { 2. };
            let term = coefficient + l10(copies);
            let distance = (j - row / 2.).abs();

            sum = add_log(sum, term);
            second = add_log(second, term + l10(distance) * 2.);
            fourth = add_log(fourth, term + l10(distance) * 4.);
            j += 1.;
        }
    }
    RowTerms {
        coefficient,
        sum,
        second,
        fourth,
    }
}

#[derive(Debug, Clone)]
struct BdSim {
    core: TraditionalCore,
    strat: BdStrat,
    rules: Rules,
    coast: bool,
    single_ms: bool,
    vars: Vec<Variable>,
    q: f64,
    k: f64,
    cached_row_term: f64,
    row_term_dirty: bool,
    rhodot: f64,
    qdot: f64,
    kdot: f64,
    /** b1 levels bought in the last buying round, for the SingleMS swap (infinity when none) */
    koala_b1_purchased: f64,
    /** b1 level at which SingleMS swapped (0 while it hasn't) */
    koala_swap: i32,
    prev_milestone_count: i64,
    milestone_unlocks: [f64; 17],
    l10_k_time_divisor: f64,
}

impl BdSim {
    fn new(strat: BdStrat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::BD, false, input, CONVERTER, 8.);
        let milestone_unlocks = MILESTONE_UNLOCKS_TAU.map(|v| v / TAU_FACTOR);
        core.milestone_unlocks = milestone_unlocks.to_vec();
        core.milestones_max = vec![3, 1, 1, 1, 6, 1, 2, 1, 1];
        // BD never reads the publication multiplier during the sim
        core.track_cur_mult = false;

        let mut a1 = Variable::new(
            "a1",
            Cost::new(FirstFreeCost::new(exp_cost_log2_model(10., 0.82))),
            Value::new_stepwise(2., 10, 0.),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        a1.buy();

        let vars = vec![
            a1,
            Variable::new("a2", exp_cost_log2(1e3, 9.), Value::new_exponential(2.)),
            Variable::new(
                "b1",
                exp_cost_log2(1e4, 0.95),
                Value::new_stepwise(2., 10, 0.),
            ),
            Variable::new("b2", exp_cost_log2(1e15, 10.2), Value::new_exponential(2.)),
            Variable::new(
                "c1",
                exp_cost_log2(50., 1.015),
                Value::new_stepwise(2., 10, 1.),
            ),
            Variable::new("c2", exp_cost_log2(1e25, 10.15), Value::new_exponential(2.)),
            Variable::new(
                "n",
                exp_cost_log2(20., 2.573),
                Value::new(PlainLinearValue {
                    power: 1.,
                    offset: 1.,
                }),
            ),
        ];

        let mut sim = BdSim {
            core,
            strat,
            rules: rules(strat),
            coast: strat.is_coast(),
            single_ms: strat.is_single_ms(),
            vars,
            q: 0.,
            k: f64::NEG_INFINITY,
            cached_row_term: 0.,
            row_term_dirty: true,
            rhodot: f64::NEG_INFINITY,
            qdot: f64::NEG_INFINITY,
            kdot: f64::NEG_INFINITY,
            koala_b1_purchased: 0.,
            koala_swap: 0,
            prev_milestone_count: -1,
            milestone_unlocks,
            l10_k_time_divisor: l10(3.),
        };
        sim.update_milestones_no_ms();
        sim.update_cached_values();
        sim
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    /** floor(n / 2): the position k is walking towards */
    fn half_row(&self) -> f64 {
        (self.val(N) / 2.).floor()
    }

    fn milestone_count(&self) -> i64 {
        let rho = self.core.max_rho.max(self.core.last_pub_rho);
        binary_insertion_search(&self.milestone_unlocks, rho) as i64
    }

    /** sim-3 `getMilestonePriority`, which also decides when SingleMS swaps */
    fn milestone_priority(&mut self) -> &'static [usize] {
        if self.single_ms && self.koala_swap == 0 {
            if self.koala_b1_purchased > 20. {
                return &SINGLE_MS_PRIORITY;
            }
            self.koala_swap = self.vars[B1].level;
        }
        &PRIORITY
    }

    /** sim-3 `updateMilestonesNoMS` (with BD's override): only when the count changes */
    fn update_milestones_no_ms(&mut self) -> bool {
        let count = self.milestone_count();
        if count == self.prev_milestone_count {
            return false;
        }
        self.prev_milestone_count = count;
        let priority = self.milestone_priority();
        let rho = self.core.max_rho.max(self.core.last_pub_rho);
        self.core.set_milestones_at(rho, priority);
        self.update_cached_values();
        self.row_term_dirty = true;
        true
    }

    /** sim-3 `updateMilestones`: SingleMS redistributes every tick */
    fn update_milestones(&mut self) {
        if self.single_ms {
            let priority = self.milestone_priority();
            let rho = self.core.max_rho.max(self.core.last_pub_rho);
            self.core.set_milestones_at(rho, priority);
            self.update_cached_values();
            self.row_term_dirty = true;
            self.koala_b1_purchased = f64::INFINITY;
        } else {
            self.update_milestones_no_ms();
        }
    }

    fn update_row_term(&mut self) {
        let row = self.val(N);
        let target = self.half_row();
        let position = js_min(target, (fastmath::pow(10., self.k) + 0.0001).floor());
        let terms = row_terms(row, position);
        let ms = &self.core.milestones;
        let mut reward = if ms[5] > 0 {
            terms.sum
        } else {
            terms.coefficient
        };
        if ms[7] > 0 {
            reward = add_log(reward, l10(4.) + terms.second - l10(1. + row));
        }
        if ms[8] > 0 {
            reward = add_log(
                reward,
                l10(16.) + terms.fourth - l10(1. + row) * (2. - 0.1 * ms[6] as f64),
            );
        }
        if ms[3] > 0 {
            reward += l10(1. + row);
        }
        self.cached_row_term = reward;
        self.row_term_dirty = false;
    }

    /** sim-3 `updatedCachedValues` */
    fn update_cached_values(&mut self) {
        let ms = &self.core.milestones;
        let (va1, va2) = (self.val(A1), self.val(A2));
        let vb1 = self.val(B1);
        let vb2 = if ms[1] > 0 { self.val(B2) } else { 0. };
        let vc1 = self.val(C1);
        let vc2 = if ms[2] > 0 { self.val(C2) } else { 0. };

        let eta = 1. + 0.01 * ms[4] as f64;

        self.kdot = (vc1 + vc2) * eta
            - (add_log(add_log(0., vb1 + vb2), (vc1 + vc2) * eta)
                + self.l10_k_time_divisor
                + l10(2.) * ms[4] as f64);
        self.qdot = (vb1 + vb2 + vc1 + vc2) / 2.;
        self.rhodot = self.core.tot_mult + va1 * (1. + 0.05 * ms[0] as f64) + va2;
    }

    fn buying_condition(&self, i: usize) -> bool {
        let should_buy = |id: usize| self.vars[id].should_buy;
        let k_at_middle = || fastmath::pow(10., self.k) >= self.half_row();
        match i {
            A1 | C1 => should_buy(i),
            B1 => {
                should_buy(B1)
                    && match self.rules {
                        Rules::Plain => true,
                        Rules::Bd2 => self.vars[B1].level == 0 || k_at_middle(),
                        Rules::ManageK => {
                            self.core.milestones[4] > 0
                                || (fastmath::pow(10., self.k) >= self.half_row() - 0.001
                                    && self.val(B1) + self.val(B2)
                                        < self.val(C1) + self.val(C2) - 1.)
                        }
                    }
            }
            B2 => self.rules != Rules::Bd2 || k_at_middle(),
            _ => true,
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        match i {
            B2 => self.core.milestones[1] > 0,
            C2 => self.core.milestones[2] > 0,
            _ => true,
        }
    }

    fn tick(&mut self) {
        let l10dt = l10(self.core.dt);
        let target = l10(self.half_row());
        if self.k < target {
            let old_position = (fastmath::pow(10., self.k) + 0.0001).floor();
            self.k = add_log(self.k, self.kdot + l10dt);
            self.k = js_min(self.k, target);

            if (fastmath::pow(10., self.k) + 0.0001).floor() != old_position {
                self.row_term_dirty = true;
            }
        }
        if self.row_term_dirty {
            self.update_row_term();
        }

        let eta_term = add_log(0., self.k) * (Q_ETA_SCALE * 0.01 * self.core.milestones[4] as f64);
        self.q = add_log(self.q, self.qdot + self.cached_row_term + eta_term + l10dt);
        self.core.rho += LogNum {
            value: self.rhodot + self.q + l10dt,
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
            self.update_cached_values();
        }
    }

    fn on_variable_purchased(&mut self, id: usize) {
        if matches!(id, A1 | B1 | C1) && self.coast {
            let v = &mut self.vars[id];
            if v.should_buy && v.coasting_cap_reached() && !v.above_original_cap() {
                v.should_fork = true;
            }
        }
        if id == N {
            self.row_term_dirty = true;
        }
        if id == B1 && self.single_ms && self.koala_swap == 0 {
            if self.koala_b1_purchased == f64::INFINITY {
                self.koala_b1_purchased = 0.;
            }
            self.koala_b1_purchased += 1.;
        }
    }

    /** Simulates a fork where variable `id` is no longer bought (sim-3 `doForkVariable`) */
    fn do_fork_variable(&mut self, id: usize) {
        self.vars[id].should_fork = false;
        let mut fork = self.clone();
        // sim-3 builds the fork with its constructor, which computes the milestones from the last
        // pub rho with the full priority; copyFrom copies neither the milestones nor their count.
        let lpr = fork.core.last_pub_rho.max(0.);
        fork.core.set_milestones_at(lpr, &PRIORITY);
        fork.prev_milestone_count = binary_insertion_search(&fork.milestone_unlocks, lpr) as i64;
        fork.row_term_dirty = true;
        fork.update_cached_values();
        fork.vars[id].stop_buying();
        let res = fork.simulate();
        self.core.best_fork_res = Some(best_result(res, self.core.best_fork_res.take()));
    }

    fn simulate(&mut self) -> SimResult {
        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            self.update_milestones();
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
        if self.single_ms && self.koala_swap > 0 {
            extra += &format!(" {}", self.koala_swap);
        }
        if self.coast {
            let bv = &self.core.bought_vars;
            for (id, name) in [(A1, "a1"), (B1, "b1"), (C1, "c1")] {
                extra += &self.vars[id].prepare_extra_for_cap(get_last_level(name, bv));
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
    fn whole_row_terms_match_the_closed_forms() {
        // Row 6: coefficients 1 6 15 20 15 6 1, sum 64, middle coefficient 20
        let t = row_terms(6., 3.);
        assert!((fastmath::exp10(t.coefficient) - 20.).abs() < 1e-9);
        assert!((fastmath::exp10(t.sum) - 64.).abs() < 1e-9);
        // Partial row: positions 0 and 1 (and their mirror images) give 2 * (1 + 6) = 14
        let p = row_terms(6., 1.);
        assert!((fastmath::exp10(p.sum) - 14.).abs() < 1e-9);
        assert!((fastmath::exp10(p.coefficient) - 6.).abs() < 1e-9);
    }

    #[test]
    fn coast_strats_report_their_last_levels() {
        let res = simulate_bd(BdStrat::BDCoast, &input(100., 120));
        assert!(res.tau_h > 0., "{res:?}");
        assert!(
            res.strat.contains(" a1: ")
                && res.strat.contains(" b1: ")
                && res.strat.contains(" c1: "),
            "{}",
            res.strat
        );
    }

    #[test]
    fn single_ms_reports_the_swap_level() {
        let res = simulate_bd(BdStrat::BDManageKSingleMS, &input(100., 120));
        assert!(res.tau_h > 0., "{res:?}");
        let swap: i32 = res.strat["BDManageKSingleMS ".len()..].parse().unwrap();
        assert!(swap > 0, "{}", res.strat);
    }
}
