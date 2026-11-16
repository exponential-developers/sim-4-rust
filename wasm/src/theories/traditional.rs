/*!
 * Shared state and logic for "traditional" theories (log(tau) proportional to log(rho)).
 *
 * Port of sim-3's `theory.ts` + `traditionalTheory.ts`. Values are kept as log10 f64s,
 * exactly like sim-3, so results can be compared one-to-one.
 */

use crate::api::config::TheoryType;
use crate::utils::{
    fastmath,
    lognum::{self, LogNum},
    result::SimResult,
    var_buy::VarBuy,
    variable::Variable,
};

/** Settings needed by a simulation (subset of the sim settings) */
#[derive(Debug, Clone, Copy)]
pub struct SimParams {
    pub dt: f64,
    pub ddt: f64,
    pub bought_vars_delta: f64,
    /** Show the a2 and a3 levels in EF's strat column */
    pub show_a23: bool,
    /** MF: how many resets ahead the sim looks (0 to 5, MF's "depth" theory input) */
    pub mf_reset_depth: i32,
    /** NLI: the player's lifetime α as log10 (NLI's "lifetime_alpha" theory input), if given */
    pub nli_lifetime_alpha: Option<f64>,
}

impl Default for SimParams {
    /** sim-3 defaults: dt 1.5, ddt 1.0001, bought vars delta e5 */
    fn default() -> Self {
        SimParams {
            dt: 1.5,
            ddt: 1.0001,
            bought_vars_delta: 5.,
            show_a23: false,
            mf_reset_depth: 0,
            nli_lifetime_alpha: None,
        }
    }
}

/** Inputs of a single theory simulation */
#[derive(Debug, Clone)]
pub struct TheoryInput {
    pub sigma: i64,
    /** log10 of the rho of the last publication */
    pub last_pub_rho: f64,
    /**
     * log10 tau of the last publication when the sim starts from a tau value (as sim-3's chains
     * do); `tau / tau_factor` is then not exactly `last_pub_rho * tau_factor` again. None: the tau
     * is `last_pub_rho * tau_factor`.
     */
    pub last_pub_tau: Option<f64>,
    /** log10 rho cap (f64::INFINITY for none) */
    pub rho_cap: f64,
    pub params: SimParams,
}

/** Progress converter for traditional theories (sim-3 `traditionalConverter`) */
#[derive(Debug, Clone, Copy)]
pub struct Converter {
    pub r9_affected: bool,
    pub tau_factor: f64,
    pub mult_exponent: f64,
    pub mult_factor: f64,
}

impl Converter {
    fn r9(&self, sigma: i64) -> f64 {
        if self.r9_affected {
            r9_multiplier(sigma)
        } else {
            0.
        }
    }
    pub fn rho_to_tau(&self, rho: f64) -> f64 {
        rho * self.tau_factor
    }
    pub fn rho_to_mult(&self, rho: f64, sigma: i64) -> f64 {
        (self.rho_to_tau(rho) * self.mult_exponent + self.mult_factor + self.r9(sigma)).max(0.)
    }
}

/** R9 multiplier as a log10 value */
pub fn r9_multiplier(sigma: i64) -> f64 {
    let exp = if sigma < 65 {
        0
    } else if sigma < 75 {
        1
    } else if sigma < 85 {
        2
    } else {
        3
    };
    l10(fastmath::pow(sigma as f64 / 20., exp as f64))
}

/** log10 (JavaScript's `Math.log10` in the browser, so results match sim-3 exactly) */
#[inline]
pub fn l10(x: f64) -> f64 {
    fastmath::log10(x)
}

/** Adds two log10 values (sim-3 `add2`) */
#[inline]
pub fn add_log(a: f64, b: f64) -> f64 {
    let (max, min) = if a > b { (a, b) } else { (b, a) };
    if max != f64::NEG_INFINITY {
        max + fastmath::log10_1p_exp10(min - max)
    } else {
        max
    }
}

/** Difference of two log10 values, |10^a - 10^b| as a log10 value (sim-3 `subtract`) */
#[inline]
pub fn sub_log(a: f64, b: f64) -> f64 {
    let (max, min) = if a > b { (a, b) } else { (b, a) };
    if max != f64::NEG_INFINITY {
        max + fastmath::log10_1m_exp10(min - max)
    } else {
        max
    }
}

/**
 * log10 of a cost parameter, computed like sim-3's `parseValue(String(x))`: JavaScript writes
 * numbers >= 1e21 as "3e+25", which sim-3 turns into 25 + log10(3).
 */
pub fn parse_value(x: f64) -> f64 {
    if x >= 1e21 {
        let s = format!("{x:e}");
        let (mantissa, exp) = s.split_once('e').expect("exponent format");
        exp.parse::<f64>().unwrap() + l10(mantissa.parse::<f64>().unwrap().max(1.))
    } else {
        l10(x)
    }
}

/** Signed log10 number (sim-3 `SignedLogNumber`) */
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignedLog {
    pub neg: bool,
    pub value: f64,
}

impl SignedLog {
    pub fn pos(value: f64) -> Self {
        SignedLog { neg: false, value }
    }
}

/** sim-3 `adds`: sum of signed log numbers */
pub fn adds(values: &[SignedLog]) -> SignedLog {
    let Some((&first, rest)) = values.split_first() else {
        return SignedLog::pos(f64::NEG_INFINITY);
    };
    let mut result = first;
    for &adder in rest {
        if result.neg == adder.neg {
            result.value = add_log(result.value, adder.value);
        } else {
            result.neg = if adder.neg {
                adder.value > result.value
            } else {
                result.value > adder.value
            };
            result.value = sub_log(result.value, adder.value);
        }
    }
    result
}

/** sim-3 `muls`: product of signed log numbers */
pub fn muls(values: &[SignedLog]) -> SignedLog {
    let Some((&first, rest)) = values.split_first() else {
        return SignedLog::pos(0.);
    };
    let mut result = first;
    for adder in rest {
        result.value += adder.value;
        if adder.neg {
            result.neg = !result.neg;
        }
    }
    result
}

/** sim-3 `mulm1`: multiplies by -1 */
pub fn mulm1(v: SignedLog) -> SignedLog {
    SignedLog {
        neg: !v.neg,
        value: v.value,
    }
}

/** JavaScript `Math.max` semantics: NaN if any argument is NaN */
#[inline]
pub fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/** sim-3's `LinearValue(power, offset)`: `offset + power * level` as a plain number */
#[derive(Debug, Clone, Copy)]
pub struct PlainLinearValue {
    pub power: f64,
    pub offset: f64,
}

impl crate::utils::value::ValueTrait for PlainLinearValue {
    fn compute_from_zero(&self, level: i32) -> LogNum {
        LogNum {
            value: self.offset + self.power * level as f64,
            sign: 1,
        }
    }

    fn compute_next(&self, _old_value: LogNum, current_level: i32) -> LogNum {
        LogNum {
            value: self.offset + self.power * (current_level + 1) as f64,
            sign: 1,
        }
    }
}

/** Number of unlock points at or below `target` (sim-3 `binaryInsertionSearch`) */
pub fn binary_insertion_search(unlocks: &[f64], target: f64) -> usize {
    if target < unlocks[0] {
        return 0;
    }
    let (mut l, mut r) = (0, unlocks.len() - 1);
    while l < r {
        let m = (l + r).div_ceil(2);
        if unlocks[m] <= target {
            l = m;
        } else {
            r = m - 1;
        }
    }
    l + 1
}

/** JavaScript `Math.min` semantics: NaN if any argument is NaN */
#[inline]
pub fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/**
 * Exponential cost from a base cost and a cost increase, parsed like sim-3's
 * `ExponentialCost(base, costInc)` (`parseValue(String(x))`, which reads 1e25 as 25 + log10(1)).
 */
pub fn exp_cost_model(base: f64, increase: f64) -> crate::utils::cost::ExponentialCost {
    let log = |x: f64| LogNum {
        value: parse_value(x),
        sign: 1,
    };
    crate::utils::cost::ExponentialCost::new(log(base), log(increase))
}

/**
 * sim-3 `ExponentialCost(base, increase, true)`: the cost increase is a power of 2
 * (`l10(2) * 10 ** parseValue(String(increase))`)
 */
pub fn exp_cost_log2_model(base: f64, increase: f64) -> crate::utils::cost::ExponentialCost {
    let increase_log = l10(2.) * fastmath::exp10(parse_value(increase));
    crate::utils::cost::ExponentialCost::new(
        LogNum {
            value: parse_value(base),
            sign: 1,
        },
        LogNum {
            value: increase_log,
            sign: 1,
        },
    )
}

/** `exp_cost_log2_model` as a cost */
pub fn exp_cost_log2(base: f64, increase: f64) -> crate::utils::cost::Cost {
    crate::utils::cost::Cost::Exponential(exp_cost_log2_model(base, increase))
}

/** `exp_cost_model` as a cost */
pub fn exp_cost(base: f64, increase: f64) -> crate::utils::cost::Cost {
    crate::utils::cost::Cost::Exponential(exp_cost_model(base, increase))
}

/** sim-3 `logToExp`: formats a log10 value as "x.xxey" */
pub fn log_to_exp(num: f64, dec: i32) -> String {
    let whole = num.floor();
    let frac = num - whole;
    let p = fastmath::pow(10., dec as f64);
    let frac1 = (fastmath::exp10(frac) * p).round() / p;
    if frac1 >= 10. {
        format!("{}e{}", frac1 / 10., whole + 1.)
    } else {
        format!("{}e{}", frac1, whole)
    }
}

/** Returns the last bought level of `name` in a purchase list, 0 if none (sim-3 `getLastLevel`) */
pub fn get_last_level(name: &str, bought: &[VarBuy]) -> i32 {
    crate::utils::get_last_purchase(name, bought).unwrap_or(0)
}

/** Cost (log10) of the last purchase of a variable, 0 if there is none (sim-3 `getLastLevelCost`) */
pub fn get_last_level_cost(name: &str, bought: &[VarBuy]) -> f64 {
    bought
        .iter()
        .rev()
        .find(|b| b.var_name == name)
        .map_or(0., |b| b.cost.value)
}

/**
 * Records, during simulations, the earliest time each rho step above `last_pub` is reached
 * (sim-3 `StepPubTableCollector`). Used by the pub table mode.
 */
#[derive(Debug)]
pub struct PubTableCollector {
    last_pub: f64,
    step: f64,
    cap: f64,
    pub timings: Vec<f64>,
}

impl PubTableCollector {
    pub fn new(last_pub: f64, step: f64, cap: f64) -> Self {
        // last_pub can be reached immediately
        PubTableCollector {
            last_pub,
            step,
            cap,
            timings: vec![0.],
        }
    }

    fn collect(&mut self, max_rho: f64, pub_unlock_rho: f64, t: f64) {
        // Not interested if we are over cap, or if pubs are not available yet (1e-7: precision)
        if max_rho > self.cap + self.step
            || max_rho + 0.0000001 < pub_unlock_rho
            || max_rho < self.last_pub
        {
            return;
        }
        let delta_num = ((max_rho - self.last_pub) / self.step).floor() as usize;
        while self.timings.len() <= delta_num {
            self.timings.push(t);
        }
        if t < self.timings[delta_num] {
            self.timings[delta_num] = t;
        }
    }
}

pub type SharedCollector = std::rc::Rc<std::cell::RefCell<PubTableCollector>>;

thread_local! {
    /**
     * Collector that new sims attach to (sim-3 `collectorCache.currentCollector`): every sim
     * created while it is set, forks included, records into it.
     */
    static CURRENT_COLLECTOR: std::cell::RefCell<Option<SharedCollector>> = const { std::cell::RefCell::new(None) };
}

/** Sets (or clears) the collector that sims created from now on record into */
pub fn set_current_collector(collector: Option<SharedCollector>) {
    CURRENT_COLLECTOR.with(|c| *c.borrow_mut() = collector);
}

/** Result with tau/h = 0, used where sim-3 uses `defaultResult()` */
pub fn default_result() -> SimResult {
    SimResult {
        tau_h: 0.,
        time: 0.,
        ..SimResult::default()
    }
}

/** Returns the result with the highest tau/h, the first one on ties (sim-3 `getBestResult`) */
pub fn best_result(a: SimResult, b: Option<SimResult>) -> SimResult {
    let b = b.unwrap_or_else(default_result);
    if a.tau_h >= b.tau_h { a } else { b }
}

/** State shared by every traditional theory */
#[derive(Debug, Clone)]
pub struct TraditionalCore {
    pub theory: TheoryType,
    pub is_main_theory: bool,
    pub sigma: i64,
    pub params: SimParams,
    pub converter: Converter,

    pub pub_unlock_rho: f64,
    pub last_pub_rho: f64,
    /** Tau of the last publication */
    pub start_tau: f64,
    pub rho_cap: f64,
    pub tot_mult: f64,
    pub cur_mult: f64,

    pub rho: LogNum,
    pub max_rho: f64,

    pub t: f64,
    pub dt: f64,
    pub ddt: f64,
    pub ticks: u64,

    pub tau_h: f64,
    pub max_tau_h: f64,
    pub pub_t: f64,
    pub pub_rho: f64,

    pub bought_vars: Vec<VarBuy>,

    pub milestones: Vec<i32>,
    pub milestones_max: Vec<i32>,
    pub milestone_unlock_steps: f64,
    /** Rho at which each milestone is unlocked; used when `milestone_unlock_steps` is not set */
    pub milestone_unlocks: Vec<f64>,

    /** Extra forced pub condition: the pub rho must be at least this (sim-3 TC) */
    pub forced_pub_min_rho: Option<f64>,

    /** Extra pub condition: the pub multiplier reached this value (needs `track_cur_mult`) */
    pub pub_end_cur_mult: Option<f64>,

    /** Extra sim end condition: the pub multiplier reached this value (needs `track_cur_mult`) */
    pub sim_end_cur_mult: Option<f64>,

    /** Whether the "t > 2 * pubT" sim end condition is checked */
    pub do_sim_end_conditions: bool,
    /** The sim ends once t exceeds this many times the time of the best publication point */
    pub sim_end_t_factor: f64,
    /** Theory-specific publication point: the pub ends when max rho reaches it */
    pub pub_target: Option<f64>,

    /** Best result found by forks */
    pub best_fork_res: Option<SimResult>,

    /**
     * Whether `cur_mult` (publication multiplier so far) is updated every tick.
     * It costs one pow per tick, so theories that never read it turn it off.
     */
    pub track_cur_mult: bool,

    /** Pub table collector this sim records into, if any (pub table mode) */
    pub collector: Option<SharedCollector>,
}

impl TraditionalCore {
    pub fn new(
        theory: TheoryType,
        is_main_theory: bool,
        input: &TheoryInput,
        converter: Converter,
        pub_unlock_rho: f64,
    ) -> Self {
        let mut core = TraditionalCore {
            theory,
            is_main_theory,
            sigma: input.sigma,
            params: input.params,
            converter,
            pub_unlock_rho,
            last_pub_rho: input.last_pub_rho,
            start_tau: input
                .last_pub_tau
                .unwrap_or_else(|| converter.rho_to_tau(input.last_pub_rho)),
            rho_cap: input.rho_cap,
            tot_mult: 0.,
            cur_mult: 0.,
            rho: lognum::ZERO,
            max_rho: 0.,
            t: 0.,
            dt: input.params.dt,
            ddt: input.params.ddt,
            ticks: 0,
            tau_h: 0.,
            max_tau_h: 0.,
            pub_t: 0.,
            pub_rho: 0.,
            bought_vars: Vec::new(),
            milestones: Vec::new(),
            milestones_max: Vec::new(),
            milestone_unlock_steps: -1.,
            milestone_unlocks: Vec::new(),
            sim_end_cur_mult: None,
            forced_pub_min_rho: None,
            pub_end_cur_mult: None,
            do_sim_end_conditions: true,
            sim_end_t_factor: 2.,
            pub_target: None,
            best_fork_res: None,
            track_cur_mult: true,
            collector: CURRENT_COLLECTOR.with(|c| c.borrow().clone()),
        };
        core.tot_mult = core.tot_mult_from_rho(core.last_pub_rho);
        core
    }

    /** Total multiplier (log10) for a given rho */
    pub fn tot_mult_from_rho(&self, rho: f64) -> f64 {
        if rho < self.pub_unlock_rho {
            if self.is_main_theory {
                r9_multiplier(self.sigma)
            } else {
                0.
            }
        } else {
            self.converter.rho_to_mult(rho, self.sigma)
        }
    }

    #[inline]
    pub fn forced_pub_conditions(&self) -> bool {
        self.pub_rho >= self.pub_unlock_rho
            && self.forced_pub_min_rho.is_none_or(|m| self.pub_rho >= m)
    }

    #[inline]
    pub fn pub_conditions(&self) -> bool {
        self.max_rho >= self.rho_cap
            || self.pub_target.is_some_and(|p| self.max_rho >= p)
            || self.pub_end_cur_mult.is_some_and(|m| self.cur_mult > m)
    }

    #[inline]
    pub fn sim_end_conditions(&self) -> bool {
        self.t > self.pub_t * self.sim_end_t_factor
            || self.sim_end_cur_mult.is_some_and(|m| self.cur_mult > m)
    }

    /** True when the simulation loop should stop */
    #[inline]
    pub fn end_simulation(&self) -> bool {
        self.forced_pub_conditions()
            && (self.pub_conditions() || (self.do_sim_end_conditions && self.sim_end_conditions()))
    }

    #[inline]
    pub fn update_t(&mut self) {
        self.t += self.dt / 1.5;
        self.dt *= self.ddt;
    }

    /** Updates max rho, time, tau/h and the publication point */
    pub fn update_sim_status(&mut self) {
        self.update_max_rho();
        self.update_t();
        self.update_sim_status_after_t();
    }

    /** First part of `update_sim_status` */
    pub fn update_max_rho(&mut self) {
        if self.rho.value > self.max_rho {
            self.max_rho = self.rho.value;
        }
    }

    /** Last part of `update_sim_status`, for theories with their own time step (RZ) */
    pub fn update_sim_status_after_t(&mut self) {
        self.tau_h = (self.converter.rho_to_tau(self.max_rho) - self.start_tau) / (self.t / 3600.);
        if self.max_tau_h < self.tau_h || !self.forced_pub_conditions() || self.pub_conditions() {
            self.max_tau_h = self.tau_h;
            self.pub_t = self.t;
            self.pub_rho = self.max_rho;
        }

        if self.track_cur_mult {
            self.cur_mult = fastmath::exp10(self.tot_mult_from_rho(self.max_rho) - self.tot_mult);
        }
        self.ticks += 1;
    }

    /** Distributes milestones for the given rho following `priority` */
    pub fn set_milestones_at(&mut self, rho: f64, priority: &[usize]) {
        let mut count = if self.milestone_unlock_steps > 0. {
            (rho / self.milestone_unlock_steps).floor() as i64
        } else {
            binary_insertion_search(&self.milestone_unlocks, rho) as i64
        };
        self.milestones.clear();
        self.milestones.resize(self.milestones_max.len(), 0);
        for &p in priority {
            while self.milestones[p] < self.milestones_max[p] && count > 0 {
                self.milestones[p] += 1;
                count -= 1;
            }
        }
    }

    /** sim-3 `updateMilestones` (only `milestoneUnlockSteps` is supported for now) */
    pub fn update_milestones(&mut self, priority: &[usize]) {
        let rho = self.max_rho.max(self.last_pub_rho);
        self.set_milestones_at(rho, priority);
    }

    pub fn record_purchase(&mut self, variable: &Variable) {
        self.record_purchase_in(variable, "rho");
    }

    /** Records a purchase paid with the currency named `symbol` (e.g. "rho_2") */
    pub fn record_purchase_in(&mut self, variable: &Variable, symbol: &'static str) {
        if self.max_rho + self.params.bought_vars_delta > self.last_pub_rho {
            self.bought_vars.push(VarBuy {
                var_name: variable.name.into(),
                level: variable.level + 1,
                cost: variable.cost,
                symbol,
                timestamp: self.t,
            });
        }
    }

    /** Records progress for the pub table mode; call at the end of each tick (sim-3 `collectData`) */
    #[inline]
    pub fn collect_pub_table(&self) {
        if let Some(c) = &self.collector {
            c.borrow_mut()
                .collect(self.max_rho, self.pub_unlock_rho, self.t);
        }
    }

    /** Removes purchases made after the publication point */
    pub fn trim_bought_vars(&mut self) {
        while self
            .bought_vars
            .last()
            .is_some_and(|b| b.timestamp > self.pub_t)
        {
            self.bought_vars.pop();
        }
    }

    pub fn create_result(&self, strat: &str, strat_extra: &str) -> SimResult {
        let start_tau = self.start_tau;
        let pub_tau = self.converter.rho_to_tau(self.pub_rho);
        SimResult {
            theory: self.theory,
            sigma: self.sigma,
            last_pub: LogNum {
                value: self.last_pub_rho,
                sign: 1,
            },
            pub_rho: LogNum {
                value: self.pub_rho,
                sign: 1,
            },
            delta_tau: LogNum {
                value: pub_tau - start_tau,
                sign: 1,
            },
            pub_multi: fastmath::exp10(self.tot_mult_from_rho(self.pub_rho) - self.tot_mult),
            strat: format!("{strat}{strat_extra}"),
            tau_h: self.max_tau_h,
            time: self.pub_t,
            bought_vars: self.bought_vars.clone(),
            theory_inputs: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NEG_INF: f64 = f64::NEG_INFINITY;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12 || a == b
    }

    #[test]
    fn add_log_adds_in_log_space() {
        assert!(close(add_log(2., 2.), 2. + 2f64.log10())); // 100 + 100 = 200
        assert!(close(add_log(3., 2.), 110f64.log10() + 1.)); // 1000 + 100 = 1100
        assert!(close(add_log(2., 3.), add_log(3., 2.)));
        assert_eq!(add_log(NEG_INF, 3.), 3.); // 0 + 1000
        assert_eq!(add_log(NEG_INF, NEG_INF), NEG_INF);
    }

    #[test]
    fn sub_log_is_the_absolute_difference() {
        assert!(close(sub_log(3., 2.), 900f64.log10()));
        assert!(close(sub_log(2., 3.), 900f64.log10())); // like sim-3: |a - b|
        assert_eq!(sub_log(5., 5.), NEG_INF);
        assert_eq!(sub_log(4., NEG_INF), 4.);
        assert_eq!(sub_log(NEG_INF, NEG_INF), NEG_INF);
    }

    #[test]
    fn parse_value_matches_sim3_parse_value() {
        assert!(close(parse_value(5000.), 5000f64.log10()));
        assert!(close(parse_value(2.), 2f64.log10()));
        // JavaScript writes numbers >= 1e21 as "3e+25"; sim-3 reads that as 25 + log10(3)
        assert_eq!(parse_value(3e25), 25. + l10(3.));
        assert_eq!(parse_value(8e50), 50. + l10(8.));
        assert_eq!(parse_value(1e21), 21.);
        assert_eq!(parse_value(0.), NEG_INF);
    }

    #[test]
    fn log_to_exp_formats_like_sim3() {
        assert_eq!(log_to_exp(2.5, 2), "3.16e2");
        assert_eq!(log_to_exp(0., 2), "1e0");
        assert_eq!(log_to_exp(150., 0), "1e150");
        // rounding up to 10 carries into the exponent
        assert_eq!(log_to_exp(2.9999999, 2), "1e3");
        assert_eq!(log_to_exp(77. + 3f64.log10(), 2), "3e77");
    }

    #[test]
    fn js_max_propagates_nan() {
        assert_eq!(js_max(1., 2.), 2.);
        assert_eq!(js_max(NEG_INF, -1.), -1.);
        assert!(js_max(f64::NAN, 1.).is_nan());
        assert!(js_max(1., f64::NAN).is_nan());
    }

    #[test]
    fn signed_log_arithmetic() {
        let pos = SignedLog::pos;
        let neg = |v: f64| SignedLog {
            neg: true,
            value: v,
        };

        // 100 - 10 = 90
        let r = adds(&[pos(2.), neg(1.)]);
        assert!(!r.neg && close(r.value, 90f64.log10()));
        // 10 - 100 = -90
        let r = adds(&[pos(1.), neg(2.)]);
        assert!(r.neg && close(r.value, 90f64.log10()));
        // -10 - 100 = -110
        let r = adds(&[neg(1.), neg(2.)]);
        assert!(r.neg && close(r.value, 110f64.log10()));
        assert_eq!(adds(&[]), pos(NEG_INF));
        assert_eq!(adds(&[neg(3.)]), neg(3.));

        // 100 * -1000 = -100000
        assert_eq!(muls(&[pos(2.), neg(3.)]), neg(5.));
        assert_eq!(muls(&[neg(2.), neg(3.)]), pos(5.));
        assert_eq!(muls(&[]), pos(0.));

        assert_eq!(mulm1(pos(4.)), neg(4.));
        assert_eq!(mulm1(neg(4.)), pos(4.));
    }

    #[test]
    fn best_result_prefers_higher_tau_h_and_first_on_ties() {
        let res = |tau_h: f64, strat: &str| SimResult {
            tau_h,
            strat: strat.to_owned(),
            ..default_result()
        };
        assert_eq!(best_result(res(2., "a"), Some(res(1., "b"))).strat, "a");
        assert_eq!(best_result(res(1., "a"), Some(res(2., "b"))).strat, "b");
        assert_eq!(best_result(res(1., "a"), Some(res(1., "b"))).strat, "a");
        assert_eq!(best_result(res(1., "a"), None).strat, "a");
        // like sim-3's defaultResult (tau/h 0), which beats negative rates
        assert_eq!(best_result(res(-1., "a"), None).tau_h, 0.);
    }

    #[test]
    fn r9_multiplier_by_students() {
        assert_eq!(r9_multiplier(20), 0.);
        assert_eq!(r9_multiplier(64), 0.);
        assert!(close(r9_multiplier(65), (65f64 / 20.).log10()));
        assert!(close(r9_multiplier(80), 2. * 4f64.log10()));
        assert!(close(r9_multiplier(100), 3. * 5f64.log10()));
    }

    #[test]
    fn pub_table_collector_keeps_earliest_time_per_step() {
        let mut c = PubTableCollector::new(100., 1., 110.);
        assert_eq!(c.timings, vec![0.]);
        c.collect(99., 10., 5.); // below the starting rho: ignored
        assert_eq!(c.timings, vec![0.]);
        c.collect(102.3, 10., 10.); // step 2 reached at t=10, steps in between filled too
        assert_eq!(c.timings, vec![0., 10., 10.]);
        c.collect(101.2, 10., 4.); // step 1 reached earlier
        assert_eq!(c.timings, vec![0., 4., 10.]);
        c.collect(102.9, 10., 20.); // later than the recorded time: no change
        assert_eq!(c.timings, vec![0., 4., 10.]);
        c.collect(200., 10., 1.); // above cap + step: ignored
        c.collect(105., 106., 1.); // before pubs unlock: ignored
        assert_eq!(c.timings, vec![0., 4., 10.]);
    }
}
