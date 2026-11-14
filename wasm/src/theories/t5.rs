/*!
 * Theory 5, ported from sim-3 `src/Theories/T1-T8/T5.ts`.
 *
 * Variable order: q1, q2, c1, c2, c3
 */

use std::cell::Cell;
use std::f64::consts::E;
use std::sync::OnceLock;

use crate::api::config::TheoryType;
use crate::theories::traditional::*;
use crate::utils::{
    cost::{Cost, ExponentialCost, FirstFreeCost},
    fastmath,
    lognum::LogNum,
    result::SimResult,
    value::Value,
    variable::Variable,
};

const Q1: usize = 0;
const Q2: usize = 1;
const C1: usize = 2;
const C2: usize = 3;
const C3: usize = 4;

/** Order in which milestones are assigned: [q1exp, c3term, c3exp] */
const MILESTONE_PRIORITY: [usize; 3] = [1, 0, 2];

const CONVERTER: Converter = Converter {
    r9_affected: true,
    tau_factor: 1.,
    mult_exponent: 0.159,
    mult_factor: 0.,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum T5Strat {
    T5,
    T5Idle,
    T5IdleCoast,
    T5Idle2Coast,
    T5AI2,
    T5AI2Coast,
    T5AI2PT,
    T5AI2PTCoast,
}

impl T5Strat {
    /** In data.json order */
    pub const ALL: [T5Strat; 8] = [
        T5Strat::T5,
        T5Strat::T5Idle,
        T5Strat::T5IdleCoast,
        T5Strat::T5Idle2Coast,
        T5Strat::T5AI2,
        T5Strat::T5AI2Coast,
        T5Strat::T5AI2PT,
        T5Strat::T5AI2PTCoast,
    ];

    pub fn name(self) -> &'static str {
        match self {
            T5Strat::T5 => "T5",
            T5Strat::T5Idle => "T5Idle",
            T5Strat::T5IdleCoast => "T5IdleCoast",
            T5Strat::T5Idle2Coast => "T5Idle2Coast",
            T5Strat::T5AI2 => "T5AI2",
            T5Strat::T5AI2Coast => "T5AI2Coast",
            T5Strat::T5AI2PT => "T5AI2PT",
            T5Strat::T5AI2PTCoast => "T5AI2PTCoast",
        }
    }

    pub fn from_name(name: &str) -> Option<T5Strat> {
        T5Strat::ALL.into_iter().find(|s| s.name() == name)
    }

    fn is_coast(self) -> bool {
        self.name().contains("Coast")
    }

    /** "PT" strats publish at the point given by the active pub table */
    fn uses_pub_table(self) -> bool {
        self.name().contains("PT")
    }

    /**
     * Strat of the first pass of coasting strats: "Coast" and "PT" removed, and T5Idle2Coast
     * becomes T5Idle (sim-3 renames it to T5IdleCoast first)
     */
    fn first_pass(self) -> T5Strat {
        match self {
            T5Strat::T5IdleCoast | T5Strat::T5Idle2Coast => T5Strat::T5Idle,
            T5Strat::T5AI2Coast | T5Strat::T5AI2PTCoast => T5Strat::T5AI2,
            s => s,
        }
    }
}

/** Simulates T5 with the given strat (sim-3 `t5` function) */
pub fn simulate_t5(strat: T5Strat, input: &TheoryInput) -> SimResult {
    if !strat.is_coast() {
        return T5Sim::new(strat, input).simulate();
    }
    let mut sim1 = T5Sim::new(strat.first_pass(), input);
    let res1 = sim1.simulate();

    let mut sim = T5Sim::new(strat, input);
    sim.vars[Q1].set_original_cap(get_last_level("q1", &res1.bought_vars));
    sim.vars[Q1].configure_cap(13);
    sim.vars[C2].set_original_cap(get_last_level("c2", &res1.bought_vars));
    sim.vars[C2].configure_cap(1);
    if strat == T5Strat::T5Idle2Coast {
        let last_c1 = match get_last_level("c1", &res1.bought_vars) {
            0 => sim1.vars[C1].level,
            level => level,
        };
        sim.vars[C1].set_original_cap(last_c1);
        sim.vars[C1].configure_cap(200);
    }
    sim.simulate()
}

/**
 * The active pub table (sim-3 `table_t5_0_01_overall_coded.json`, copied unchanged): for each
 * last pub rho in steps of 0.01 up to e2000, the rho to publish at.
 *
 * sim-3 decodes it into string keys; every value it can look up (rounded rho from 0 to 1999 in
 * steps of 0.01) is `data[k] / 100`, where k = rounded rho * 100 and `data` is the decoded list.
 * `bench/` checks that against sim-3's own decoder.
 */
const PUB_TABLE_JSON: &str = include_str!("data/table_t5_0_01_overall_coded.json");

#[derive(serde::Deserialize)]
struct CodedTable {
    /** base64 of zlib-compressed big-endian numbers */
    t: String,
    /** bytes per number */
    sz: usize,
    /** added to every delta */
    d: i64,
    /** initial value */
    i: i64,
}

fn base64_decode(s: &str) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for &c in s.as_bytes().iter().take_while(|&&c| c != b'=') {
        let v = ALPHABET
            .iter()
            .position(|&a| a == c)
            .expect("base64 character") as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

/** Decoded pub table: `data[k]` for k = 0, 1, ... (sim-3 `ptDecodeFormat1`) */
fn pub_table() -> &'static [i64] {
    static TABLE: OnceLock<Vec<i64>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let coded: CodedTable = serde_json::from_str(PUB_TABLE_JSON).expect("T5 pub table");
        let bytes = miniz_oxide::inflate::decompress_to_vec_zlib(&base64_decode(&coded.t))
            .expect("T5 pub table data");
        let mut prev = coded.i;
        bytes
            .chunks(coded.sz)
            .map(|chunk| {
                let n = chunk.iter().fold(0i64, |acc, &b| (acc << 8) | b as i64);
                prev += n + coded.d;
                prev
            })
            .collect()
    })
}

/** JavaScript `Math.round`: rounds halves up */
fn js_round(x: f64) -> f64 {
    let floor = x.floor();
    if x - floor >= 0.5 { floor + 1. } else { floor }
}

/** Publication rho given by the pub table for a last pub rho <= 1999, NaN if it has none */
fn pub_table_target(last_pub_rho: f64) -> f64 {
    // sim-3 looks up (Math.round(rho * 100) / 100).toFixed(4)
    let k = js_round(last_pub_rho * 100.);
    let table = pub_table();
    if k >= 0. && (k as usize) < table.len() {
        table[k as usize] as f64 / 100.
    } else {
        f64::NAN
    }
}

/** log10 constants, computed once per sim (sim-3 module constants and buying conditions) */
#[derive(Debug, Clone)]
struct T5Consts {
    l10_2_3: f64,
    l10_2: f64,
    l10_1_5: f64,
    l10_e: f64,
    /** log10(3 + n) for n = q1 level % 10 */
    l10_3_plus: [f64; 10],
}

impl T5Consts {
    fn new() -> Self {
        T5Consts {
            l10_2_3: l10(2. / 3.),
            l10_2: l10(2.),
            l10_1_5: l10(1.5),
            l10_e: l10(E),
            l10_3_plus: std::array::from_fn(|n| l10(3. + n as f64)),
        }
    }
}

/** JavaScript `Math.min` semantics: NaN if any argument is NaN */
fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

#[derive(Debug, Clone)]
struct T5Sim {
    core: TraditionalCore,
    strat: T5Strat,
    vars: Vec<Variable>,
    q: f64,
    c2worth: bool,
    c2_counter: i32,
    nc3: f64,
    consts: T5Consts,
    /**
     * Last q growth parameter (`gamma` in `calculate_q`) and the bits of the values it was
     * computed from: it only changes when c1, c2 or c3 is bought, so this saves a pow per call
     */
    gamma_cache: Cell<([u64; 3], f64)>,
}

impl T5Sim {
    fn new(strat: T5Strat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::T5, true, input, CONVERTER, 7.);
        core.milestone_unlock_steps = 25.;
        core.milestones_max = vec![3, 1, 2];

        let mut q1 = Variable::new(
            "q1",
            Cost::new(FirstFreeCost::new(ExponentialCost::new(10., 1.61328))),
            Value::new_stepwise(2., 10, 0.),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        q1.buy();

        let vars = vec![
            q1,
            Variable::new(
                "q2",
                Cost::new_exponential(15., 64.),
                Value::new_exponential(2.),
            ),
            Variable::new(
                "c1",
                Cost::new_exponential(1e6, 1.18099),
                Value::new_stepwise(2., 10, 1.),
            ),
            Variable::new(
                "c2",
                Cost::new_exponential(75., 4.53725),
                Value::new_exponential(2.),
            ),
            Variable::new(
                "c3",
                Cost::new_exponential(1e3, 8.85507e7),
                Value::new_exponential(2.),
            ),
        ];

        let mut sim = T5Sim {
            core,
            strat,
            vars,
            q: 0.,
            c2worth: true,
            c2_counter: 0,
            nc3: 0.,
            consts: T5Consts::new(),
            gamma_cache: Cell::new(([f64::NAN.to_bits(); 3], f64::NAN)),
        };
        // T5's buying conditions never read the publication multiplier
        sim.core.track_cur_mult = false;
        sim.core.update_milestones(&MILESTONE_PRIORITY);
        if strat.uses_pub_table() && sim.core.last_pub_rho <= 1999. {
            sim.core.do_sim_end_conditions = false;
            sim.core.pub_target = Some(pub_table_target(sim.core.last_pub_rho));
        }
        sim
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    /** c3 value with its exponent milestone */
    fn vc3_full(&self) -> f64 {
        self.val(C3) * (1. + 0.05 * self.core.milestones[2] as f64)
    }

    /** Solves q using the differential equation result (sim-3 `calculateQ`) */
    fn calculate_q(&self, ic1: f64, ic2: f64, ic3: f64) -> f64 {
        let l10_e = self.consts.l10_e;
        let qcap = ic2 + ic3;
        // q growth speed characteristic parameter
        let key = [ic1.to_bits(), ic2.to_bits(), ic3.to_bits()];
        let gamma = match self.gamma_cache.get() {
            (cached, gamma) if cached == key => gamma,
            _ => {
                let gamma = fastmath::exp10(ic1 + ic3 - ic2);
                self.gamma_cache.set((key, gamma));
                gamma
            }
        };
        // initial condition
        let adjust = self.q - sub_log(qcap, self.q);
        let sigma = fastmath::exp10(adjust + gamma * self.core.dt * l10_e);
        let newq = if sigma < 1e-30 {
            // Approximation when q << qcap
            qcap + adjust + gamma * self.core.dt * l10_e
        } else {
            qcap - fastmath::log10_1p(1. / sigma)
        };
        js_min(newq, qcap)
    }

    fn buying_condition(&self, i: usize) -> bool {
        use T5Strat::*;
        let coast_ok = |j: usize| !self.strat.is_coast() || self.vars[j].should_buy;
        match self.strat {
            T5 => true,
            T5Idle | T5IdleCoast | T5Idle2Coast => match i {
                Q1 => coast_ok(Q1),
                C1 if self.strat == T5Idle2Coast => coast_ok(C1),
                C1 => {
                    self.core.max_rho + (self.core.last_pub_rho - 200.) / 165.
                        < self.core.last_pub_rho
                }
                C2 => coast_ok(C2) && self.c2worth,
                _ => true,
            },
            T5AI2 | T5AI2Coast | T5AI2PT | T5AI2PTCoast => match i {
                Q1 => {
                    let lvl = (self.vars[Q1].level % 10) as usize;
                    let c3 = if self.core.milestones[2] > 0 {
                        self.cost(C3)
                    } else {
                        1000.
                    };
                    coast_ok(Q1)
                        && self.cost(Q1) + self.consts.l10_3_plus[lvl]
                            <= self.cost(Q2).min(self.cost(C2)).min(c3)
                }
                C1 => {
                    self.q + self.consts.l10_1_5 < self.val(C2) + self.vc3_full() || !self.c2worth
                }
                C2 => coast_ok(C2) && self.c2worth,
                _ => true,
            },
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        i != C3 || self.core.milestones[1] > 0
    }

    fn tick(&mut self) {
        let ms = &self.core.milestones;
        let vq1 = self.val(Q1) * (1. + 0.05 * ms[0] as f64);
        let vc3 = if ms[1] > 0 { self.vc3_full() } else { 0. };

        self.q = self.calculate_q(self.val(C1), self.val(C2), vc3);
        let rhodot = vq1 + self.val(Q2) + self.q;
        self.core.rho += LogNum {
            value: rhodot + self.core.tot_mult + l10(self.core.dt),
            sign: 1,
        };

        self.nc3 = vc3;
        let iq = self.calculate_q(self.val(C1), self.val(C2), vc3);
        self.c2worth = iq >= self.val(C2) + self.nc3 + self.consts.l10_2_3;
    }

    fn buy_variables(&mut self) {
        // T5 has no onAnyVariablePurchased step, so whether something was bought is not tracked
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
        if id == C2 {
            self.c2_counter += 1;
            let extra_c2 = self.consts.l10_2 * self.c2_counter as f64;
            let iq = self.calculate_q(self.val(C1), self.val(C2) + extra_c2, self.nc3);
            self.c2worth = iq >= self.val(C2) + extra_c2 + self.vc3_full() + self.consts.l10_2_3;
        }
        if matches!(id, Q1 | C1 | C2) && self.strat.is_coast() {
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
            if self.core.last_pub_rho < 150. {
                self.core.update_milestones(&MILESTONE_PRIORITY);
            }
            self.c2_counter = 0;
            self.buy_variables();
            for id in [Q1, C1, C2] {
                if self.vars[id].should_fork {
                    self.do_fork_variable(id);
                }
            }
            self.core.collect_pub_table();
        }
        self.core.trim_bought_vars();

        let name = self.strat.name();
        let bv = &self.core.bought_vars;
        let mut extra = String::new();
        if name.contains("T5Idle") && !name.contains("Idle2") {
            extra = format!(" {}", log_to_exp(self.cost(C1), 1));
        }
        if self.strat.is_coast() {
            extra += &self.vars[Q1].prepare_extra_for_cap(get_last_level("q1", bv));
            extra += &self.vars[C2].prepare_extra_for_cap(get_last_level("c2", bv));
        }
        if name.contains("Idle2") {
            extra += &self.vars[C1].prepare_extra_for_cap(get_last_level("c1", bv));
        }
        let res = self.core.create_result(name, &extra);
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
    fn pub_table_matches_sim3_decoder() {
        // values from sim-3's ptDecodeFormat1 + prepareTable (see bench/)
        let table = pub_table();
        assert_eq!(table.len(), 200001);
        // sum of the entries sim-3 can look up (rho 0 to 1999.00)
        assert_eq!(table[..=199900].iter().sum::<i64>(), 20085701774);
        for (rho, target) in [
            (0., 7.46),
            (0.01, 7.46),
            (123.45, 137.41),
            (150., 166.14),
            (300., 309.63),
            (1999., 2000.),
        ] {
            assert_eq!(pub_table_target(rho), target, "{rho}");
        }
        // sim-3 rounds the last pub rho to 0.01 with Math.round (halves up)
        assert_eq!(pub_table_target(149.996), 166.14);
        assert_eq!(js_round(2.5), 3.);
        assert_eq!(js_round(-2.5), -2.);
        assert!(pub_table_target(-1.).is_nan());
    }

    #[test]
    fn pub_table_strats_publish_at_the_table_point() {
        let res = simulate_t5(T5Strat::T5AI2PT, &input(150., 90));
        assert!(
            res.pub_rho.value >= 166.14 && res.pub_rho.value < 166.2,
            "{res:?}"
        );
        // above e1999 the table is not used
        let sim = T5Sim::new(T5Strat::T5AI2PT, &input(2000., 300));
        assert!(sim.core.pub_target.is_none() && sim.core.do_sim_end_conditions);
    }

    #[test]
    fn cached_q_growth_is_bit_identical_to_sim3_calculation() {
        // sim-3's calculateQ, written out without the cache and shortcuts
        let reference = |sim: &T5Sim, ic1: f64, ic2: f64, ic3: f64| {
            let qcap = ic2 + ic3;
            let gamma = 10f64.powf(ic1 + ic3 - ic2);
            let adjust = sim.q - sub_log(qcap, sim.q);
            let sigma = 10f64.powf(adjust + gamma * sim.core.dt * l10(E));
            let newq = if sigma < 1e-30 {
                qcap + adjust + gamma * sim.core.dt * l10(E)
            } else {
                qcap - (1. + 1. / sigma).log10()
            };
            js_min(newq, qcap)
        };
        let mut sim = T5Sim::new(T5Strat::T5AI2, &input(120., 85));
        for tick in 0..20000 {
            let (ic1, ic2) = (sim.val(C1), sim.val(C2));
            let ic3 = if sim.core.milestones[1] > 0 {
                sim.vc3_full()
            } else {
                0.
            };
            let q = sim.calculate_q(ic1, ic2, ic3);
            assert_eq!(
                q.to_bits(),
                reference(&sim, ic1, ic2, ic3).to_bits(),
                "tick {tick}"
            );
            sim.tick();
            sim.core.update_sim_status();
            sim.c2_counter = 0;
            sim.buy_variables();
        }
        assert!(sim.vars[C2].level > 0 && sim.vars[C1].level > 0);
    }

    #[test]
    fn idle_strats_report_their_c1_cost_and_coast_levels() {
        let res = simulate_t5(T5Strat::T5IdleCoast, &input(200., 110));
        assert!(res.strat.starts_with("T5IdleCoast ") && res.strat.contains(" q1: "));
        let res = simulate_t5(T5Strat::T5Idle2Coast, &input(200., 110));
        assert!(res.strat.contains(" c1: "), "{}", res.strat);
    }
}
