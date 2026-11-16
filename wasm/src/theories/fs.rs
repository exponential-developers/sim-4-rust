/*!
 * Theory FS (Fibonacci Sequence, unofficial), ported from sim-3
 * `src/Theories/Unofficial-CTs/FS.ts`.
 *
 * Variable order: c1, c2, n, m (bought with rho), c3, f1, f2 (bought with F), l1, l2, c4
 * (bought with L)
 */

use std::cell::RefCell;
use std::sync::OnceLock;

use crate::api::config::TheoryType;
use crate::theories::pubtable::{CodedTable, PubTable, js_round, js_to_fixed};
use crate::theories::traditional::*;
use crate::utils::{
    cost::{Cost, CostTrait, FirstFreeCost},
    fastmath,
    lognum::{self, LogNum},
    result::SimResult,
    value::Value,
    variable::Variable,
};

const C1: usize = 0;
const C2: usize = 1;
const N: usize = 2;
const M: usize = 3;
const C3: usize = 4;
const F1: usize = 5;
const F2: usize = 6;
const L1: usize = 7;
const L2: usize = 8;
const C4: usize = 9;

/** Currency of each variable */
const SYMBOLS: [&str; 10] = ["rho", "rho", "rho", "rho", "F", "F", "F", "L", "L", "L"];

const MILESTONE_PRIORITY: [usize; 6] = [0, 1, 2, 3, 4, 5];
const MILESTONE_UNLOCKS: [f64; 10] = [8., 13., 21., 34., 55., 89., 144., 233., 377., 610.];

fn phi() -> f64 {
    (1. + 5f64.sqrt()) / 2.
}

fn converter() -> Converter {
    Converter {
        r9_affected: false,
        tau_factor: 1. / phi(),
        mult_exponent: 1. / 5f64.sqrt(),
        mult_factor: 0.,
    }
}

/** log10 of phi and of sqrt(5) */
fn log_consts() -> (f64, f64) {
    static CONSTS: OnceLock<(f64, f64)> = OnceLock::new();
    *CONSTS.get_or_init(|| (l10(phi()), l10(5f64.sqrt())))
}

/** Fibonacci and Lucas numbers 0 to 49 as log10 (sim-3 `fibLogCache` and `lucasLogCache`) */
fn sequences() -> &'static ([f64; 50], [f64; 50]) {
    static SEQ: OnceLock<([f64; 50], [f64; 50])> = OnceLock::new();
    SEQ.get_or_init(|| {
        let mut fib = [0.; 50];
        let mut lucas = [0.; 50];
        fib[0] = f64::NEG_INFINITY;
        lucas[0] = l10(2.);
        for i in 2..50 {
            fib[i] = add_log(fib[i - 1], fib[i - 2]);
            lucas[i] = add_log(lucas[i - 1], lucas[i - 2]);
        }
        (fib, lucas)
    })
}

/** sim-3 `getFibLog`: log10 of the n-th Fibonacci number */
fn fib_log(n: f64) -> f64 {
    let index = n.floor();
    if index <= 0. {
        return f64::NEG_INFINITY;
    }
    if index >= 50. {
        // (phi^n - (1 - phi)^n) / sqrt(5), the second term being negligible
        let (phi_log, sqrt5_log) = log_consts();
        return n * phi_log - sqrt5_log;
    }
    sequences().0[index as usize]
}

/** sim-3 `getLucasLog`: log10 of the n-th Lucas number */
fn lucas_log(n: f64) -> f64 {
    let index = n.floor();
    if index < 0. {
        return f64::NEG_INFINITY;
    }
    if index >= 50. {
        // phi^n + (1 - phi)^n, the second term being negligible
        return n * log_consts().0;
    }
    sequences().1[index as usize]
}

thread_local! {
    /** Tribonacci numbers as log10, extended as needed (sim-3 `tribLogCache`) */
    static TRIB_LOG: RefCell<Vec<f64>> =
        RefCell::new(vec![f64::NEG_INFINITY, f64::NEG_INFINITY, 0.]);
}

/** sim-3 `getTribonacciLog` */
fn tribonacci_log(n: f64) -> f64 {
    let index = n.floor();
    if index < 0. {
        return f64::NEG_INFINITY;
    }
    let index = index as usize;
    TRIB_LOG.with(|cache| {
        let mut cache = cache.borrow_mut();
        for i in cache.len()..=index {
            let next = add_log(add_log(cache[i - 1], cache[i - 2]), cache[i - 3]);
            cache.push(next);
        }
        cache[index]
    })
}

/** sim-3 `getTribonacciIndex` */
fn tribonacci_index(t: f64) -> f64 {
    let raw = fastmath::pow(js_max(1., t), 0.2);
    if !raw.is_finite() || raw < 2. {
        2.
    } else {
        raw.floor()
    }
}

/** Index of the last purchase of a variable, 0 if there is none (sim-3 `getLastIndex`) */
fn last_index(name: &str, bought: &[crate::utils::var_buy::VarBuy]) -> usize {
    bought.iter().rposition(|b| b.var_name == name).unwrap_or(0)
}

/** Costs following a sequence (sim-3 `SequenceCost`) */
#[derive(Debug, Clone, Copy)]
enum SequenceCost {
    /** Cost of level L is the (L + 1)-th Fibonacci number */
    FibonacciNext,
    /** Cost of level L is the L-th Lucas number */
    Lucas,
}

impl SequenceCost {
    /** sim-3 `getCost(level)`: cost of buying level `level + 1` */
    fn cost_at(self, level: i32) -> LogNum {
        let value = match self {
            SequenceCost::FibonacciNext => fib_log(level as f64 + 1.),
            SequenceCost::Lucas => lucas_log(level as f64),
        };
        LogNum { value, sign: 1 }
    }
}

impl CostTrait for SequenceCost {
    fn get_cost_to(&self, level: i32) -> LogNum {
        self.cost_at(level - 1)
    }

    fn get_total_cost_to(&self, level: i32) -> LogNum {
        (0..level).fold(lognum::ZERO, |sum, l| sum + self.cost_at(l))
    }
}

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FsStrat {
        FS,
        FSCoast,
        FSd,
        FSdCoast,
        FSdPTCoast,
        FSStopNM,
        FSStopNMCoast,
        FSPTStopNMCoast,
        FSPTAStopNMCoast,
        FSdStopNM,
        FSdStopNMCoast,
        FSdPTStopNMCoast,
    }
}

impl FsStrat {
    fn is_coast(self) -> bool {
        self.name().contains("Coast")
    }

    fn is_d(self) -> bool {
        self.name().starts_with("FSd")
    }

    fn is_stop_nm(self) -> bool {
        self.name().contains("StopNM")
    }

    fn uses_pub_table(self) -> bool {
        self.name().contains("PT")
    }

    /** sim-3 picks the active table for "PTA" strats and the d strats */
    fn active_pub_table(self) -> bool {
        self.name().contains("PTA") || self.is_d()
    }

    /** Strat without "PTA" or "PT" (the first passes of the pub table strats) */
    fn without_pub_table(self) -> FsStrat {
        let name = self.name().replacen("PTA", "", 1).replacen("PT", "", 1);
        FsStrat::from_name(&name).unwrap_or(self)
    }
}

fn pub_table(active: bool) -> &'static PubTable {
    static ACTIVE: OnceLock<PubTable> = OnceLock::new();
    static PASSIVE: OnceLock<PubTable> = OnceLock::new();
    // sim-3 prepares these tables with the suffix "04" instead of "00"
    if active {
        ACTIVE.get_or_init(|| {
            PubTable::decode_format1_with_suffix(
                &CodedTable::parse(include_str!("data/table_fs_0_02_active_coast_coded.json")),
                "04",
            )
        })
    } else {
        PASSIVE.get_or_init(|| {
            PubTable::decode_format1_with_suffix(
                &CodedTable::parse(include_str!("data/table_fs_0_02_passive_coast_coded.json")),
                "04",
            )
        })
    }
}

/** Last levels of n bought before the last c3, and of m before the last c4 */
fn last_n_m(res: &SimResult) -> (i32, i32) {
    let bv = &res.bought_vars;
    (
        get_last_level("n", &bv[..last_index("c3", bv)]),
        get_last_level("m", &bv[..last_index("c4", bv)]),
    )
}

/**
 * Reruns `strat` with n and m capped at their last levels until those levels no longer change.
 * Returns the last result and the levels.
 */
fn settle_n_m(strat: FsStrat, input: &TheoryInput, first: SimResult) -> (SimResult, i32, i32) {
    let (mut new_n, mut new_m) = last_n_m(&first);
    let mut res;
    loop {
        let (last_n, last_m) = (new_n, new_m);
        let mut sim = FsSim::new(strat, input);
        sim.vars[N].set_original_cap(last_n);
        sim.vars[M].set_original_cap(last_m);
        res = sim.simulate();
        (new_n, new_m) = last_n_m(&res);
        if last_m == new_m && last_n == new_n {
            return (res, last_n, last_m);
        }
    }
}

/** Simulates FS with the given strat (sim-3 `fs` function) */
pub fn simulate_fs(strat: FsStrat, input: &TheoryInput) -> SimResult {
    if strat.is_coast() {
        let first_pass = strat.without_pub_table();
        let mut initial_res = FsSim::new(first_pass, input).simulate();
        let (mut last_n, mut last_m) = last_n_m(&initial_res);
        if strat.is_stop_nm() {
            (initial_res, last_n, last_m) = settle_n_m(first_pass, input, initial_res);
        }
        let last_c1 = get_last_level("c1", &initial_res.bought_vars);

        let mut sim = FsSim::new(strat, input);
        sim.vars[C1].set_original_cap(last_c1);
        sim.vars[C1].configure_cap(8);
        sim.vars[N].set_original_cap(last_n);
        sim.vars[M].set_original_cap(last_m);
        sim.simulate()
    } else if strat.is_stop_nm() {
        let first = FsSim::new(strat, input).simulate();
        settle_n_m(strat, input, first).0
    } else {
        FsSim::new(strat, input).simulate()
    }
}

#[derive(Debug, Clone)]
struct FsSim {
    core: TraditionalCore,
    strat: FsStrat,
    coast: bool,
    d: bool,
    stop_nm: bool,
    vars: Vec<Variable>,
    f: LogNum,
    l: LogNum,
    rhodot: f64,
    fdot: f64,
    ldot: f64,
    t_var: f64,
    /** Base of c2 (it grows with the fifth milestone) */
    c2_power: f64,
    /**
     * Whether the F and L variables are cut off from their currencies. In sim-3 a fork's
     * variables keep the currencies its constructor made, and copyFrom then replaces F and L with
     * new objects, so a fork (and its forks) can never buy c3, f1, f2, l1, l2 or c4.
     */
    f_l_cut_off: bool,
    prev_milestone_count: i64,
    consts: FsConsts,
}

/** log10 constants used by the buying conditions, computed once per sim */
#[derive(Debug, Clone)]
struct FsConsts {
    l10_2: f64,
    l10_4: f64,
    l10_5: f64,
    /** log10(1 + n / 2) for n = c1 level % 13 */
    l10_c1: [f64; 13],
}

impl FsSim {
    fn new(strat: FsStrat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::FS, false, input, converter(), 5.);
        core.milestone_unlocks = MILESTONE_UNLOCKS.to_vec();
        core.milestones_max = vec![1, 1, 2, 2, 3, 1];
        // FS never reads the publication multiplier during the sim
        core.track_cur_mult = false;

        let linear = || {
            Value::new(PlainLinearValue {
                power: 1.,
                offset: 0.,
            })
        };
        let mut c1 = Variable::new(
            "c1",
            Cost::new(FirstFreeCost::new(SequenceCost::FibonacciNext)),
            Value::new_stepwise(8., 13, 0.),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        c1.buy();
        let vars = vec![
            c1,
            Variable::new("c2", exp_cost(21., 21.), Value::new_exponential(1.5)),
            Variable::new("n", exp_cost(3., 3.), linear()),
            Variable::new("m", exp_cost(2., 5.), linear()),
            Variable::new("c3", exp_cost(987., 610.), Value::new_exponential(2.)),
            Variable::new(
                "f1",
                Cost::new(SequenceCost::FibonacciNext),
                Value::new_stepwise(21., 34, 1.),
            ),
            Variable::new("f2", exp_cost(2584., 14930352.), Value::new_exponential(5.)),
            Variable::new(
                "l1",
                Cost::new(SequenceCost::Lucas),
                Value::new_stepwise(29., 47, 1.),
            ),
            Variable::new("l2", exp_cost(1., 87403803.), Value::new_exponential(4.)),
            Variable::new("c4", exp_cost(1., 987.), Value::new_exponential(3.)),
        ];

        if strat.uses_pub_table() && core.last_pub_rho < 970. {
            let key = js_to_fixed(js_round(core.last_pub_rho * 50.) / 50., 3) + "4";
            let next_rho = pub_table(strat.active_pub_table())
                .get(&key)
                .unwrap_or(f64::NAN);
            core.do_sim_end_conditions = false;
            core.pub_target = Some(next_rho);
        }

        let mut sim = FsSim {
            core,
            strat,
            coast: strat.is_coast(),
            d: strat.is_d(),
            stop_nm: strat.is_stop_nm(),
            vars,
            f: lognum::ZERO,
            l: lognum::ZERO,
            rhodot: 0.,
            fdot: 0.,
            ldot: 0.,
            t_var: 1.,
            c2_power: 1.5,
            f_l_cut_off: false,
            prev_milestone_count: -1,
            consts: FsConsts {
                l10_2: l10(2.),
                l10_4: l10(4.),
                l10_5: l10(5.),
                l10_c1: std::array::from_fn(|n| l10(1. + n as f64 / 2.)),
            },
        };
        sim.update_milestones_no_ms();
        sim.update_rho_dot();
        sim.update_f_dot();
        sim.update_l_dot();
        sim
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    fn currency(&self, i: usize) -> f64 {
        if self.f_l_cut_off && i >= C3 {
            return f64::NEG_INFINITY;
        }
        match i {
            C3 | F1 | F2 => self.f.value,
            L1 | L2 | C4 => self.l.value,
            _ => self.core.rho.value,
        }
    }

    fn pay(&mut self, i: usize, cost: LogNum) {
        match i {
            C3 | F1 | F2 => self.f -= cost,
            L1 | L2 | C4 => self.l -= cost,
            _ => self.core.rho -= cost,
        }
    }

    /** sim-3 `updateMilestonesNoMS` (with FS's override): only when the count changes */
    fn update_milestones_no_ms(&mut self) {
        let rho = self.core.max_rho.max(self.core.last_pub_rho);
        let count = binary_insertion_search(&MILESTONE_UNLOCKS, rho) as i64;
        if count == self.prev_milestone_count {
            return;
        }
        self.prev_milestone_count = count;
        self.core.set_milestones_at(rho, &MILESTONE_PRIORITY);

        let m4 = self.core.milestones[4];
        let base = if m4 > 2 {
            phi()
        } else if m4 > 1 {
            1.6
        } else if m4 > 0 {
            11. / 7.
        } else {
            1.5
        };
        if self.c2_power != base {
            self.c2_power = base;
            let v = &mut self.vars[C2];
            v.value_model = Value::new_exponential(base);
            v.value = v.value_model.compute_from_zero(v.level);
        }

        self.update_rho_dot();
        self.update_f_dot();
        self.update_l_dot();
    }

    fn update_rho_dot(&mut self) {
        let ms = &self.core.milestones;
        let mut rho_term = self.val(C1) + self.val(C2);
        if ms[0] > 0 {
            rho_term += self.val(C3);
        }
        if ms[1] > 0 {
            rho_term += self.val(C4);
        }
        self.rhodot = self.core.tot_mult + rho_term;
    }

    fn update_f_dot(&mut self) {
        let ms = &self.core.milestones;
        let fib = fib_log(self.vars[N].level as f64);
        let mut multiplier = 0.;
        if ms[2] > 0 {
            multiplier += self.val(F1);
        }
        if ms[2] > 1 {
            multiplier += self.val(F2);
        }
        self.fdot = self.core.tot_mult + fib + multiplier;
    }

    fn update_l_dot(&mut self) {
        let ms = &self.core.milestones;
        let lucas = lucas_log(self.vars[M].level as f64);
        let mut multiplier = 0.;
        if ms[3] > 0 {
            multiplier += self.val(L1);
        }
        if ms[3] > 1 {
            multiplier += self.val(L2);
        }
        self.ldot = self.core.tot_mult + lucas + multiplier;
    }

    /** The "d" buying conditions (sim-3 `dStrat`) */
    fn d_condition(&self, i: usize) -> bool {
        let k = &self.consts;
        let c = |j: usize| self.cost(j);
        let ms = &self.core.milestones;
        match i {
            C1 => c(C1) + k.l10_c1[(self.vars[C1].level % 13) as usize] < c(C2),
            N => c(N) + k.l10_2 < c(C2) && ms[0] > 0,
            M => c(M) + k.l10_2 < c(C2),
            F1 => {
                let f2 = if ms[2] > 1 { c(F2) } else { f64::INFINITY };
                c(F1) + l10(((self.vars[F1].level % 34) + 1) as f64) < js_min(c(C3), f2)
            }
            F2 => c(F2) + k.l10_5 < c(C3),
            L1 => {
                let l2 = if ms[3] > 1 { c(L2) } else { f64::INFINITY };
                c(L1) + l10(((self.vars[L1].level % 47) + 1) as f64) < js_min(c(C4), l2)
            }
            L2 => c(L2) + k.l10_4 < c(C4),
            _ => true,
        }
    }

    fn buying_condition(&self, i: usize) -> bool {
        let d = !self.d || self.d_condition(i);
        match i {
            C1 => self.vars[C1].should_buy && d,
            N | M => d && (!self.stop_nm || self.vars[i].under_original_cap()),
            _ => d,
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        let ms = &self.core.milestones;
        match i {
            M | C4 => ms[1] > 0,
            C3 => ms[0] > 0,
            F1 => ms[2] > 0,
            F2 => ms[2] > 1,
            L1 => ms[3] > 0,
            L2 => ms[3] > 1,
            _ => true,
        }
    }

    fn tick(&mut self) {
        let l10dt = l10(self.core.dt);
        self.t_var += self.core.dt;

        let t_factor = -0.3 * l10(js_max(1., self.t_var));
        let mut rho_term = self.rhodot + t_factor;
        if self.core.milestones[5] > 0 {
            rho_term += tribonacci_log(tribonacci_index(self.t_var));
        }
        self.core.rho += LogNum {
            value: rho_term + l10dt,
            sign: 1,
        };

        self.f += LogNum {
            value: self.fdot + l10dt,
            sign: 1,
        };
        if self.core.milestones[1] > 0 {
            self.l += LogNum {
                value: self.ldot + l10dt,
                sign: 1,
            };
        }
    }

    fn buy_variables(&mut self) {
        // FS has no onAnyVariablePurchased step, so whether something was bought is not tracked
        for i in (0..self.vars.len()).rev() {
            while self.currency(i) > self.cost(i)
                && self.buying_condition(i)
                && self.variable_available(i)
            {
                self.core.record_purchase_in(&self.vars[i], SYMBOLS[i]);
                let cost = self.vars[i].cost;
                self.pay(i, cost);
                self.vars[i].buy();
                self.on_variable_purchased(i);
            }
        }
    }

    fn on_variable_purchased(&mut self, id: usize) {
        match id {
            C1 | C2 | C3 | C4 => self.update_rho_dot(),
            N | F1 | F2 => self.update_f_dot(),
            _ => self.update_l_dot(),
        }

        if id == C1 && self.coast {
            let v = &mut self.vars[C1];
            if v.should_buy && v.coasting_cap_reached() {
                if v.level as f64 > v.original_cap + 5. {
                    v.stop_buying();
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
        // pub rho; copyFrom copies neither the milestones nor their count. The variables (with
        // c2's base) and the rates are copied.
        let lpr = fork.core.last_pub_rho.max(0.);
        fork.core.set_milestones_at(lpr, &MILESTONE_PRIORITY);
        fork.prev_milestone_count = binary_insertion_search(&MILESTONE_UNLOCKS, lpr) as i64;
        fork.f_l_cut_off = true;
        fork.vars[id].stop_buying();
        let res = fork.simulate();
        self.core.best_fork_res = Some(best_result(res, self.core.best_fork_res.take()));
    }

    fn simulate(&mut self) -> SimResult {
        if self.core.last_pub_rho < 13. {
            self.core.sim_end_t_factor = 4.;
        }
        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            self.update_milestones_no_ms();
            self.buy_variables();
            if self.vars[C1].should_fork {
                self.do_fork_variable(C1);
            }
            self.core.collect_pub_table();
        }
        self.core.trim_bought_vars();

        let bv = &self.core.bought_vars;
        let mut extra = String::new();
        if self.coast {
            extra += &format!(" c1: {}", get_last_level("c1", bv));
        }
        if self.stop_nm {
            extra += &format!(" N: {}", get_last_level("n", bv));
            // m is capped at one milestone level, so sim-3 never shows "M: ..."
            if self.core.milestones[1] > 1 {
                extra += &format!("M: {}", get_last_level("m", bv));
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
    fn sequences_match_their_first_terms() {
        let fib: Vec<f64> = (0..10)
            .map(|n| fastmath::exp10(fib_log(n as f64)))
            .collect();
        let expected = [0., 1., 1., 2., 3., 5., 8., 13., 21., 34.];
        for (a, b) in fib.iter().zip(expected) {
            assert!((a - b).abs() < 1e-9, "{fib:?}");
        }
        let lucas: Vec<f64> = (0..6)
            .map(|n| fastmath::exp10(lucas_log(n as f64)))
            .collect();
        for (a, b) in lucas.iter().zip([2., 1., 3., 4., 7., 11.]) {
            assert!((a - b).abs() < 1e-9, "{lucas:?}");
        }
        let trib: Vec<f64> = (2..9)
            .map(|n| fastmath::exp10(tribonacci_log(n as f64)))
            .collect();
        for (a, b) in trib.iter().zip([1., 1., 2., 4., 7., 13., 24.]) {
            assert!((a - b).abs() < 1e-9, "{trib:?}");
        }
        // Past the cache, the closed forms take over smoothly
        assert!((fib_log(50.) - add_log(fib_log(49.), fib_log(48.))).abs() < 1e-9);
        assert_eq!(tribonacci_index(1.), 2.);
        assert_eq!(tribonacci_index(1e5), 10.);
    }

    #[test]
    fn sequence_costs_follow_the_sequences() {
        let c = Cost::new(SequenceCost::FibonacciNext);
        // Cost of level 5 is the 6th Fibonacci number
        assert!((fastmath::exp10(c.get_cost(5).value) - 8.).abs() < 1e-9);
        let total = c.get_total_cost_to(4).value;
        assert!((fastmath::exp10(total) - (1. + 1. + 2. + 3.)).abs() < 1e-9);
    }

    #[test]
    fn stop_nm_and_coast_strats_report_their_levels() {
        let res = simulate_fs(FsStrat::FSdStopNMCoast, &input(150., 120));
        assert!(res.tau_h > 0., "{res:?}");
        assert!(
            res.strat.contains(" c1: ") && res.strat.contains(" N: "),
            "{}",
            res.strat
        );
        assert!(get_last_level("F", &res.bought_vars) == 0);
        assert!(res.bought_vars.iter().any(|b| b.symbol == "F"));
    }

    #[test]
    fn pub_table_strats_use_the_shifted_table() {
        let rho = 300.;
        let key = js_to_fixed(js_round(rho * 50.) / 50., 3) + "4";
        let target = pub_table(false).get(&key).expect("table entry");
        // The "04" suffix shifts every target by 4e-4
        assert!(((target * 1e4).round() as i64) % 100 == 4, "{target}");
        let res = simulate_fs(FsStrat::FSPTStopNMCoast, &input(rho, 150));
        assert!(res.pub_rho.value >= target, "{res:?}");
    }
}
