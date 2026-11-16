/*!
 * Theory NLI (unofficial, beta in sim-3), ported from sim-3 `src/Theories/Unofficial-CTs/NLI.ts`.
 *
 * NLI has two currencies: α, grown in a first phase, then ρ. α buys permanent upgrades and decides
 * the milestones; tau comes from h (the b0 level) and ρ. NLI has no ρ to publish from, so sim-4
 * gives it a tau factor of 1: its "rho" values are tau values.
 *
 * The sim (sim-3 `MainNLISim`):
 * - simulates α alone up to a target, recording a state each time something that matters changes;
 * - for each group of states with the same permanent upgrades and milestone count, simulates ρ on
 *   top of the best of those states;
 * - from late enough, simulates both currencies in parallel instead.
 *
 * Variable order: q1, a0, a1, a2, a3, b0, b1, b2
 */

use std::collections::BTreeMap;

use crate::api::config::TheoryType;
use crate::theories::traditional::{
    TheoryInput, add_log, best_result, binary_insertion_search, default_result, js_max, l10,
    log_to_exp, sub_log,
};
use crate::utils::{
    cost::{Cost, FirstFreeCost},
    fastmath,
    lognum::{self, LogNum},
    result::SimResult,
    value::{Value, ValueTrait},
    var_buy::VarBuy,
    variable::Variable,
};

const Q1: usize = 0;
const B0: usize = 5;

const RHO_CONVERSION: f64 = 0.4;
const H_CONVERSION: f64 = 0.4;
const MULT_EXPONENT: f64 = 0.2;

const PUB_UNLOCK: f64 = 5.;
const RHO_UNLOCK: f64 = 16.;
const MS_MENU_UNLOCK: f64 = 10.;
const K_PERMA_UNLOCKS: [f64; 2] = [50., 140.];
const H_PERMA_UNLOCKS: [f64; 1] = [90.];
const MS_INCREASE_PERMA_UNLOCKS: [f64; 6] = [60., 100., 160., 500., 650., 770.];

/** Milestone caps for each level of the third permanent upgrade */
const MILESTONE_CAPS: [[i32; 7]; 7] = [
    [1, 1, 1, 1, 0, 0, 0],
    [2, 2, 2, 2, 1, 0, 0],
    [3, 3, 3, 3, 2, 1, 0],
    [4, 4, 4, 4, 3, 2, 1],
    [4, 4, 4, 4, 4, 3, 2],
    [4, 4, 4, 4, 4, 4, 3],
    [4, 4, 4, 4, 4, 4, 4],
];

/** Tau (plus the h reduction) at which each milestone is unlocked */
const MILESTONE_COSTS: [f64; 29] = [
    60., 80., 90., // group 1
    180., 200., 210., 220., // group 2
    300., 320., 330., 340., 350., // group 3
    800., 850., 900., 925., 950., 1000., // group 4
    1425., 1525., 1625., // group 5
    1850., 1950., // group 6
    2280., // group 7
    2290., 2300., 2310., 2320., // useless milestones
    2375., // final
];

/** Bases of a0, a1, a2, a3, b0, b1, b2 for each level of their milestone */
const BASES: [[f64; 5]; 7] = [
    [1.39, 1.4, 1.41, 1.42, 1.43],
    [1.435, 1.455, 1.475, 1.495, 1.515],
    [1.43, 1.455, 1.48, 1.505, 1.53],
    [1.38, 1.405, 1.43, 1.455, 1.48],
    [1.55, 1.57, 1.59, 1.61, 1.63],
    [1.7, 1.72, 1.74, 1.76, 1.78],
    [1.605, 1.615, 1.63, 1.655, 1.68],
];

/** Milestone that sets the base of variable i + 1 */
const VAR_ID_TO_MS_ID: [usize; 7] = [0, 2, 4, 6, 1, 3, 5];

fn phi() -> f64 {
    (1. + 5f64.sqrt()) / 2.
}

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum NliStrat {
        NLI,
        NLInoA2RB2H,
        NLInoB2H,
        NLInoA2A3RB2,
        NLInoA2A3B2,
        NLInoA3B2,
        NLInoA3,
        NLInoA1HA2H,
    }
}

impl NliStrat {
    /** Which variables the strat buys during the α phase */
    fn alpha_conditions(self) -> [bool; 8] {
        const T: bool = true;
        const F: bool = false;
        match self {
            NliStrat::NLI => [T, F, T, T, T, T, T, T],
            NliStrat::NLInoA2RB2H | NliStrat::NLInoB2H => [T, F, T, T, T, T, T, F],
            NliStrat::NLInoA2A3RB2 => [T, F, T, F, T, T, T, F],
            NliStrat::NLInoA2A3B2 => [T, F, T, F, F, T, T, F],
            NliStrat::NLInoA3B2 => [T, F, T, T, F, T, T, F],
            NliStrat::NLInoA3 => [T, F, T, T, F, T, T, T],
            NliStrat::NLInoA1HA2H => [T, F, F, F, T, T, T, T],
        }
    }

    /** Which variables the strat buys during the ρ phase */
    fn rho_conditions(self) -> [bool; 8] {
        const T: bool = true;
        const F: bool = false;
        let base = [T, T, T, T, T, F, T, T];
        let no_a23_b2 = [T, T, T, F, F, F, T, F];
        match self {
            NliStrat::NLI | NliStrat::NLInoB2H | NliStrat::NLInoA1HA2H => base,
            NliStrat::NLInoA2RB2H => [T, T, T, F, T, F, T, T],
            NliStrat::NLInoA2A3RB2 | NliStrat::NLInoA2A3B2 => no_a23_b2,
            NliStrat::NLInoA3B2 => [T, T, T, T, F, F, T, F],
            NliStrat::NLInoA3 => [T, T, T, T, F, F, T, T],
        }
    }
}

/** sim-3 `ExponentialValueM1`: power^level minus (almost) 1 */
#[derive(Debug, Clone, Copy)]
struct ExponentialValueM1 {
    power: f64,
}

impl ValueTrait for ExponentialValueM1 {
    fn compute_from_zero(&self, level: i32) -> LogNum {
        LogNum {
            value: sub_log(l10(self.power) * level as f64, -0.00001),
            sign: 1,
        }
    }

    fn compute_next(&self, _old_value: LogNum, current_level: i32) -> LogNum {
        self.compute_from_zero(current_level + 1)
    }
}

/**
 * sim-3 `parseExponentialValue`: "123.4", "e123.4" or "1.5e123" as a log10 value.
 * Err for any other format.
 */
pub fn parse_exponential_value(s: &str) -> Result<f64, String> {
    let is_decimal = |s: &str| {
        let (int, frac) = s.split_once('.').unwrap_or((s, "0"));
        !int.is_empty()
            && int.bytes().all(|b| b.is_ascii_digit())
            && !frac.is_empty()
            && frac.bytes().all(|b| b.is_ascii_digit())
    };
    let invalid =
        || format!("Invalid value {s}. Value must be in formats <number>, <exxxx> or <xexxxx>.");
    let plain = s.strip_prefix('e').unwrap_or(s);
    if is_decimal(plain) {
        return plain.parse::<f64>().map_err(|_| invalid());
    }
    if let Some((mantissa, exp)) = s.split_once('e')
        && is_decimal(mantissa)
        && !exp.is_empty()
        && exp.bytes().all(|b| b.is_ascii_digit())
    {
        // sim-3 `parseLog10String`
        let mantissa: f64 = mantissa.parse().map_err(|_| invalid())?;
        let exp: f64 = exp.parse().map_err(|_| invalid())?;
        return Ok(exp + l10(js_max(1., mantissa)));
    }
    Err(invalid())
}

fn milestone_reduction(h: f64) -> f64 {
    add_log(h * 4., 0.)
}

/** sim-3 `alphaToH`: the h a player with this lifetime α has */
fn alpha_to_h(tau: f64, alpha: f64) -> f64 {
    let b0_levels = js_max(1., 1. + ((alpha - l10(200.)) / l10(2.54)).floor());
    let perma3_levels = binary_insertion_search(&MS_INCREASE_PERMA_UNLOCKS, alpha);

    if perma3_levels == 0 {
        let h_no_ms = l10(BASES[4][0]) * b0_levels;
        let ms1_price = MILESTONE_COSTS[0] - milestone_reduction(h_no_ms);
        if tau > ms1_price {
            l10(BASES[4][1]) * b0_levels
        } else {
            h_no_ms
        }
    } else {
        l10(BASES[4][4.min(1 + perma3_levels)]) * b0_levels
    }
}

fn h_to_milestone_count(tau: f64, h: f64) -> usize {
    if tau < MS_MENU_UNLOCK {
        return 0;
    }
    binary_insertion_search(&MILESTONE_COSTS, tau + milestone_reduction(h))
}

fn alpha_to_milestone_count(tau: f64, alpha: f64) -> usize {
    h_to_milestone_count(tau, alpha_to_h(tau, alpha))
}

fn alpha_to_permas(alpha: f64) -> [usize; 3] {
    [
        binary_insertion_search(&K_PERMA_UNLOCKS, alpha),
        binary_insertion_search(&H_PERMA_UNLOCKS, alpha),
        binary_insertion_search(&MS_INCREASE_PERMA_UNLOCKS, alpha),
    ]
}

/** The two phases of a publication */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Alpha,
    Rho,
}

/** What a simulation needs from the query */
#[derive(Debug, Clone, Copy)]
struct NliInput {
    sigma: i64,
    base_tau: f64,
    /** Lifetime α given by the player (0 when not given: sim-3 can't derive it from tau yet) */
    base_alpha: f64,
    dt: f64,
    ddt: f64,
    bought_vars_delta: f64,
}

/** One of the two currencies' simulations (sim-3 `NLIAlphaSim` and `NLIRhoSim`) */
#[derive(Debug, Clone)]
struct NliSim {
    phase: Phase,
    strat: NliStrat,
    conditions: [bool; 8],
    base_tau: f64,
    base_alpha: f64,
    tot_mult: f64,
    q: f64,
    permas: [usize; 3],
    perma_count: usize,
    ms_count: usize,
    milestones: [i32; 8],
    milestones_max: [i32; 8],
    vars: Vec<Variable>,
    /** α or ρ */
    currency: LogNum,
    /** Highest α or ρ so far */
    max_currency: f64,
    maxh: f64,
    /** α phase: something changed since the last recorded state */
    state_dirty: bool,
    t: f64,
    dt: f64,
    ddt: f64,
    bought_vars: Vec<VarBuy>,
    // ρ phase publication point
    pub_tau: f64,
    pub_rho: f64,
    pub_t: f64,
    max_tau_h: f64,
    /** log10(j / (i + j)) for the tau integral */
    l10_ratios: [[f64; 4]; 4],
}

impl NliSim {
    fn new(phase: Phase, strat: NliStrat, input: &NliInput, maxh: f64) -> Self {
        let exp = |base: f64| Value::new_exponential(base);
        let m1 = |power: f64| Value::new(ExponentialValueM1 { power });
        let first_free = |base: f64, increase: f64| {
            Cost::new(FirstFreeCost::new(
                crate::theories::traditional::exp_cost_model(base, increase),
            ))
        };
        let cost = crate::theories::traditional::exp_cost;
        let mut vars = match phase {
            Phase::Alpha => vec![
                Variable::new("q1", first_free(5000., 31.2), m1(2.)),
                // unused
                Variable::new("a0", cost(1e6, 1e6), exp(BASES[0][0])),
                Variable::new("a1", cost(80., 4.9), exp(BASES[1][0])),
                Variable::new("a2", cost(1e5, 10.15), exp(BASES[2][0])),
                Variable::new("a3", cost(1e8, 40.), exp(BASES[3][0])),
                Variable::new("b0", first_free(200., 2.54), m1(BASES[4][0])),
                Variable::new("b1", first_free(3000., 3.65), m1(BASES[5][0])),
                Variable::new("b2", first_free(1e7, 4.1), m1(BASES[6][0])),
            ],
            Phase::Rho => vec![
                Variable::new("q1", first_free(5000., 31.2), m1(2.)),
                Variable::new("a0", cost(50., 1.891), exp(BASES[0][0])),
                Variable::new("a1", cost(1e4, 2.362), exp(BASES[1][0])),
                Variable::new("a2", cost(1e4, 2.855), exp(BASES[2][0])),
                Variable::new("a3", cost(1e8, 3.31), exp(BASES[3][0])),
                // unused
                Variable::new("b0", first_free(1e6, 1e6), m1(BASES[4][0])),
                Variable::new("b1", first_free(1e5, 11.25), m1(BASES[5][0])),
                Variable::new("b2", first_free(1e10, 25.55), m1(BASES[6][0])),
            ],
        };
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        for i in [0, 5, 6, 7] {
            vars[i].buy();
        }

        let permas = alpha_to_permas(input.base_alpha);
        let mut sim = NliSim {
            phase,
            strat,
            conditions: match phase {
                Phase::Alpha => strat.alpha_conditions(),
                Phase::Rho => strat.rho_conditions(),
            },
            base_tau: input.base_tau,
            base_alpha: input.base_alpha,
            // sim-3 `getTotMult`: tau * multExponent + multFactor
            tot_mult: input.base_tau * MULT_EXPONENT + 0.,
            q: f64::NEG_INFINITY,
            permas,
            perma_count: permas.iter().sum(),
            ms_count: alpha_to_milestone_count(input.base_tau, input.base_alpha),
            milestones: [0; 8],
            milestones_max: [0; 8],
            vars,
            currency: lognum::ZERO,
            max_currency: f64::NEG_INFINITY,
            maxh,
            state_dirty: true,
            t: 0.,
            dt: input.dt,
            ddt: input.ddt,
            bought_vars: Vec::new(),
            pub_tau: 0.,
            pub_rho: 0.,
            pub_t: 0.,
            max_tau_h: 0.,
            l10_ratios: std::array::from_fn(|i| {
                std::array::from_fn(|j| l10(j as f64 / (i + j) as f64))
            }),
        };
        sim.update_milestones();
        sim
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    fn symbol(&self) -> &'static str {
        match self.phase {
            Phase::Alpha => "α",
            Phase::Rho => "ρ",
        }
    }

    fn milestone_priority(&self) -> &'static [usize] {
        match (self.phase, self.permas[2]) {
            (Phase::Alpha, 0) => &[2, 3, 1],
            (Phase::Alpha, 1) => &[4, 3, 2, 1],
            (Phase::Alpha, 2) => &[4, 3, 2, 1, 5],
            (Phase::Alpha, 3) => &[3, 2, 1, 4, 6, 5],
            (Phase::Alpha, 4) => &[4, 3, 2, 1, 6, 5],
            (Phase::Alpha, 5) => &[5, 4, 3, 2, 1, 6],
            (Phase::Alpha, 6) => &[6, 5, 4, 3, 1, 2],
            (Phase::Rho, 0) => &[0, 3, 2],
            (Phase::Rho, 1) => &[3, 4, 0, 2],
            (Phase::Rho, 2) => &[0, 3, 2, 5, 4],
            (Phase::Rho, 3) => &[0, 2, 3, 5, 4, 6],
            (Phase::Rho, 4 | 5) => &[0, 2, 3, 4, 5, 6],
            (Phase::Rho, 6) => &[0, 6, 5, 4, 3, 2],
            _ => &[],
        }
    }

    fn update_bases(&mut self) {
        for (i, &ms) in VAR_ID_TO_MS_ID.iter().enumerate() {
            let power = BASES[i][self.milestones[ms] as usize];
            let v = &mut self.vars[i + 1];
            v.value_model = if i + 1 >= B0 {
                Value::new(ExponentialValueM1 { power })
            } else {
                Value::new_exponential(power)
            };
            v.value = v.value_model.compute_from_zero(v.level);
        }
    }

    fn update_milestones(&mut self) {
        let priority = self.milestone_priority();
        let mut count = self.ms_count;
        let caps = MILESTONE_CAPS[self.permas[2]];
        self.milestones_max[..7].copy_from_slice(&caps);
        self.milestones_max[7] = 1;
        self.milestones = [0; 8];
        for &p in priority {
            while self.milestones[p] < self.milestones_max[p] && count > 0 {
                self.milestones[p] += 1;
                count -= 1;
            }
        }
        self.update_bases();
    }

    fn rsp_integral(&self, poly1: &[f64], poly2: &[f64], bound: f64) -> f64 {
        let mut res = f64::NEG_INFINITY;
        for (i, p1) in poly1.iter().enumerate() {
            for (j, p2) in poly2.iter().enumerate().skip(1) {
                res = add_log(
                    res,
                    self.l10_ratios[i][j] + p1 + p2 + bound * (i + j) as f64,
                );
            }
        }
        res
    }

    fn tick(&mut self) {
        let one_plus_phi = 1. + phi();
        self.q = add_log(self.q * one_plus_phi, self.val(Q1) + l10(self.dt)) * (1. / one_plus_phi);

        let gated = |on: bool, v: f64| if on { v } else { f64::NEG_INFINITY };
        let (k0, h0) = match self.phase {
            Phase::Alpha => (0., self.val(5)),
            Phase::Rho => (self.val(1), 0.),
        };
        let k = [
            k0,
            self.val(2),
            gated(self.permas[0] > 0, self.val(3)),
            gated(self.permas[0] > 1, self.val(4)),
        ];
        let h = [h0, self.val(6), gated(self.permas[1] > 0, self.val(7))];
        let integral = match self.phase {
            Phase::Alpha => self.rsp_integral(&h, &k, self.q),
            Phase::Rho => self.rsp_integral(&k, &h, self.q),
        };
        self.currency += LogNum {
            value: integral + self.tot_mult + l10(self.dt),
            sign: 1,
        };
    }

    fn update_t(&mut self) {
        self.t += self.dt / 1.5;
        self.dt *= self.ddt;
    }

    fn tau_power(&self) -> f64 {
        match self.phase {
            Phase::Alpha => self.maxh * H_CONVERSION,
            Phase::Rho => self.maxh * H_CONVERSION + self.max_currency * RHO_CONVERSION,
        }
    }

    fn update_sim_status(&mut self) {
        if self.currency.value > self.max_currency {
            self.max_currency = self.currency.value;
        }
        self.update_t();
        match self.phase {
            Phase::Alpha => {
                // sim-3 `updateMaxH`
                let ms = (self.ms_count as i32).min(MILESTONE_CAPS[self.permas[2]][1]);
                self.maxh = l10(BASES[4][ms as usize]) * self.vars[B0].level as f64;
                // sim-3 `updatePermas`
                if self.max_currency < self.base_alpha {
                    return;
                }
                self.permas = alpha_to_permas(self.max_currency);
                let count: usize = self.permas.iter().sum();
                if count > self.perma_count {
                    self.perma_count = count;
                    self.state_dirty = true;
                    self.update_milestones();
                }
            }
            Phase::Rho => {
                // sim-3 `updateMilestoneCount`
                let count =
                    h_to_milestone_count(js_max(self.base_tau, self.tau_power()), self.maxh);
                if count > self.ms_count {
                    self.ms_count = count;
                    self.update_milestones();
                }
            }
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        match i {
            3 => self.permas[0] > 0,
            4 => self.permas[0] > 1,
            7 => self.permas[1] > 0,
            _ => true,
        }
    }

    fn buy_variables(&mut self) {
        let symbol = self.symbol();
        for i in (0..self.vars.len()).rev() {
            while self.currency.value > self.vars[i].cost.value
                && self.conditions[i]
                && self.variable_available(i)
            {
                // NLI records every purchase (they are filtered when the result is made)
                let v = &self.vars[i];
                self.bought_vars.push(VarBuy {
                    var_name: v.name.into(),
                    level: v.level + 1,
                    cost: v.cost,
                    symbol,
                    timestamp: self.t,
                });
                let cost = self.vars[i].cost;
                self.currency -= cost;
                self.vars[i].buy();
                if i == B0 && self.phase == Phase::Alpha {
                    self.state_dirty = true;
                }
            }
        }
    }

    /** sim-3 `endSimulation` for the ρ phase: published past the last tau, and t > 2 pub t */
    fn rho_phase_done(&self) -> bool {
        self.pub_tau > self.base_tau && self.t > self.pub_t * 2.
    }

    /** sim-3 `deriveState` */
    fn state(&self) -> AlphaState {
        AlphaState {
            b0_level: self.vars[B0].level,
            max_alpha: self.max_currency,
            t: self.t,
            tau_power: self.tau_power(),
            maxh: self.maxh,
            perma_points: self.perma_count,
            ms_points: self.ms_count,
        }
    }

    /** sim-3 `getTotMult` for a tau, relative to this publication's multiplier */
    fn pub_multi(&self, tau: f64) -> f64 {
        fastmath::pow(10., tau * MULT_EXPONENT + 0. - self.tot_mult)
    }
}

/** sim-3 `NLIAlphaState` */
#[derive(Debug, Clone, Copy)]
struct AlphaState {
    b0_level: i32,
    max_alpha: f64,
    t: f64,
    tau_power: f64,
    maxh: f64,
    perma_points: usize,
    ms_points: usize,
}

fn lifetime_alpha_output(value: String) -> Option<BTreeMap<String, String>> {
    Some(BTreeMap::from([("lifetime_alpha".to_owned(), value)]))
}

/** The α phase on its own (sim-3 `NLIAlphaSimulator`) */
struct AlphaSimulator {
    input: NliInput,
    sim: NliSim,
    states: Vec<AlphaState>,
}

impl AlphaSimulator {
    fn new(strat: NliStrat, input: &NliInput) -> Self {
        AlphaSimulator {
            input: *input,
            sim: NliSim::new(Phase::Alpha, strat, input, f64::NEG_INFINITY),
            states: Vec::new(),
        }
    }

    fn simulate_to(&mut self, alpha: f64) {
        let real_max_alpha = js_max(alpha, PUB_UNLOCK + 1.);
        while self.sim.max_currency < real_max_alpha {
            self.sim.tick();
            self.sim.update_sim_status();
            self.sim.buy_variables();
            if self.sim.state_dirty && self.sim.max_currency > PUB_UNLOCK {
                self.states.push(self.sim.state());
                self.sim.state_dirty = false;
            }
        }
    }

    fn state_tau_h(&self, state: &AlphaState) -> f64 {
        (state.tau_power - self.sim.base_tau) / (state.t / 3600.)
    }

    /** sim-3 `getBestAloneState`: `reduce((s1, s2) => tauH1 >= tauH2 ? s1 : s2)` */
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    fn best_alone_state(&self) -> AlphaState {
        let mut best = self.states[0];
        for s in &self.states[1..] {
            if !(self.state_tau_h(&best) >= self.state_tau_h(s)) {
                best = *s;
            }
        }
        best
    }

    /** Purchases of the α phase up to `state` */
    fn bought_vars_up_to(&self, state: &AlphaState) -> Vec<VarBuy> {
        let delta = self.input.bought_vars_delta;
        self.sim
            .bought_vars
            .iter()
            .filter(|b| {
                if b.cost.value < state.max_alpha - delta - 10. {
                    return false;
                }
                if b.var_name == "b0" && b.level <= state.b0_level {
                    return true;
                }
                b.cost.value < state.max_alpha
            })
            .cloned()
            .collect()
    }

    fn result_from_state(&self, state: &AlphaState) -> SimResult {
        let sim = &self.sim;
        SimResult {
            theory: TheoryType::NLI,
            sigma: self.input.sigma,
            last_pub: LogNum {
                value: sim.base_tau,
                sign: 1,
            },
            pub_rho: LogNum {
                value: state.tau_power,
                sign: 1,
            },
            delta_tau: LogNum {
                value: state.tau_power - sim.base_tau,
                sign: 1,
            },
            pub_multi: sim.pub_multi(state.tau_power),
            strat: format!(
                "{} α-only b0:{} α:{}",
                sim.strat.name(),
                state.b0_level,
                log_to_exp(state.max_alpha, 3)
            ),
            tau_h: self.state_tau_h(state),
            time: state.t,
            bought_vars: self.bought_vars_up_to(state),
            theory_inputs: lifetime_alpha_output(log_to_exp(state.max_alpha, 3)),
        }
    }

    /** States past ρ's unlock, grouped by permanent upgrade and milestone counts */
    fn cluster_states(&self) -> Result<Vec<Vec<AlphaState>>, String> {
        if self.states.is_empty() {
            return Ok(Vec::new());
        }
        let states: Vec<AlphaState> = self
            .states
            .iter()
            .filter(|s| s.max_alpha > RHO_UNLOCK)
            .copied()
            .collect();
        // sim-3 reads the first state's counts without checking there is one
        let first = states
            .first()
            .ok_or("NLI: no α state was recorded after ρ unlocks")?;
        let mut clusters: Vec<Vec<AlphaState>> = vec![Vec::new()];
        let (mut perma_count, mut ms_count) = (first.perma_points, first.ms_points);
        for state in states {
            if state.perma_points > perma_count || state.ms_points > ms_count {
                clusters.push(Vec::new());
                perma_count = state.perma_points;
                ms_count = state.ms_points;
            }
            clusters.last_mut().expect("never empty").push(state);
        }
        Ok(clusters)
    }
}

/** The ρ phase on top of a group of α states (sim-3 `NLIRhoSimulator`) */
fn simulate_rho_phase(
    strat: NliStrat,
    input: &NliInput,
    states: &[AlphaState],
    alpha_sim: &AlphaSimulator,
) -> Result<SimResult, String> {
    let first = states[0];
    let alpha = js_max(input.base_alpha, first.max_alpha + 0.01);
    // sim-3 passes the lifetime α to the ρ sim as text with 7 decimals
    let rho_input = NliInput {
        base_alpha: parse_exponential_value(&log_to_exp(alpha, 7))?,
        ..*input
    };
    let mut sim = NliSim::new(Phase::Rho, strat, &rho_input, first.maxh);
    sim.perma_count = first.perma_points;
    sim.permas = alpha_to_permas(alpha);
    sim.ms_count = first.ms_points;
    sim.update_milestones();
    let base_tau = sim.base_tau;

    let mut pub_state = first;
    while !sim.rho_phase_done() {
        sim.tick();
        sim.update_sim_status();
        sim.buy_variables();

        let rho_tau = sim.max_currency * RHO_CONVERSION;
        let mut best = first;
        let mut best_tau_h =
            (first.maxh * H_CONVERSION + rho_tau - base_tau) / (first.t + sim.t) * 3600.;
        for state in &states[1..] {
            let delta_tau = state.maxh * H_CONVERSION + rho_tau - base_tau;
            let tau_h = delta_tau / (state.t + sim.t) * 3600.;
            if tau_h > best_tau_h {
                best = *state;
                best_tau_h = tau_h;
            }
        }
        if best_tau_h > sim.max_tau_h {
            sim.pub_tau = best.maxh * H_CONVERSION + rho_tau;
            sim.pub_rho = sim.max_currency;
            sim.pub_t = sim.t;
            sim.max_tau_h = best_tau_h;
            pub_state = best;
        }
    }

    let delta_tau = sim.pub_tau - base_tau;
    let time = pub_state.t + sim.pub_t;
    let delta = input.bought_vars_delta;
    let mut bought_vars = alpha_sim.bought_vars_up_to(&pub_state);
    bought_vars.extend(
        sim.bought_vars
            .iter()
            .filter(|b| b.cost.value >= sim.pub_rho - delta - 10. && b.timestamp < sim.pub_t)
            .cloned(),
    );
    Ok(SimResult {
        theory: TheoryType::NLI,
        sigma: input.sigma,
        last_pub: LogNum {
            value: base_tau,
            sign: 1,
        },
        pub_rho: LogNum {
            value: sim.pub_tau,
            sign: 1,
        },
        delta_tau: LogNum {
            value: delta_tau,
            sign: 1,
        },
        pub_multi: sim.pub_multi(sim.pub_tau),
        strat: format!(
            "{} b0:{} α:{} {} ρ:{} {}",
            strat.name(),
            pub_state.b0_level,
            log_to_exp(pub_state.max_alpha, 3),
            crate::sim::convert_time(pub_state.t),
            log_to_exp(sim.pub_rho, 3),
            crate::sim::convert_time(sim.pub_t)
        ),
        tau_h: delta_tau / (time / 3600.),
        time,
        bought_vars,
        theory_inputs: lifetime_alpha_output(log_to_exp(pub_state.max_alpha, 7)),
    })
}

/** Both phases at once, once every milestone is unlocked (sim-3 `NLIParallelSimulator`) */
fn simulate_parallel(strat: NliStrat, input: &NliInput) -> Result<SimResult, String> {
    let mut alpha = NliSim::new(Phase::Alpha, strat, input, f64::NEG_INFINITY);
    let mut rho = NliSim::new(Phase::Rho, strat, input, 0.);
    for sim in [&mut alpha, &mut rho] {
        sim.permas = [2, 1, 6];
        // sim-3 sets the milestones without updating the variable bases
        sim.milestones = [4, 4, 4, 4, 4, 4, 4, 1];
        sim.ms_count = 29;
    }

    let (mut pub_b0, mut pub_alpha) = (0, 0.);
    while !rho.rho_phase_done() {
        for sim in [&mut alpha, &mut rho] {
            sim.tick();
            sim.update_sim_status();
            sim.buy_variables();
        }
        let delta_tau =
            alpha.maxh * H_CONVERSION + rho.max_currency * RHO_CONVERSION - rho.base_tau;
        let tau_h = delta_tau / rho.t * 3600.;
        if tau_h > rho.max_tau_h {
            rho.pub_tau = alpha.maxh * H_CONVERSION + rho.max_currency * RHO_CONVERSION;
            rho.pub_rho = rho.max_currency;
            rho.pub_t = rho.t;
            rho.max_tau_h = tau_h;
            pub_b0 = alpha.vars[B0].level;
            pub_alpha = alpha.max_currency;
        }
    }

    let delta = input.bought_vars_delta;
    let recent = |bought: &[VarBuy], max: f64| -> Vec<VarBuy> {
        bought
            .iter()
            .filter(|b| b.cost.value >= max - delta - 10. && b.cost.value < max)
            .cloned()
            .collect()
    };
    let alpha_bought = recent(&alpha.bought_vars, pub_alpha);
    let rho_bought = recent(&rho.bought_vars, rho.pub_rho);
    // sim-3 reads the first purchase of each list without checking there is one
    let (Some(first_alpha), Some(first_rho)) = (alpha_bought.first(), rho_bought.first()) else {
        return Err("NLI: no purchase to show for the parallel publication".to_owned());
    };
    let threshold = js_max(first_alpha.timestamp, first_rho.timestamp) - 0.001;
    let mut bought_vars: Vec<VarBuy> = alpha_bought
        .iter()
        .chain(rho_bought.iter())
        .filter(|b| b.timestamp > threshold)
        .cloned()
        .collect();
    // Stable, like V8's sort
    bought_vars.sort_by(|a, b| {
        (a.timestamp - b.timestamp)
            .partial_cmp(&0.)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let delta_tau = rho.pub_tau - rho.base_tau;
    Ok(SimResult {
        theory: TheoryType::NLI,
        sigma: input.sigma,
        last_pub: LogNum {
            value: rho.base_tau,
            sign: 1,
        },
        pub_rho: LogNum {
            value: rho.pub_tau,
            sign: 1,
        },
        delta_tau: LogNum {
            value: delta_tau,
            sign: 1,
        },
        pub_multi: rho.pub_multi(rho.pub_tau),
        strat: format!(
            "{} parallel b0:{} α:{} ρ:{}",
            strat.name(),
            pub_b0,
            log_to_exp(pub_alpha, 3),
            log_to_exp(rho.pub_rho, 3)
        ),
        tau_h: delta_tau / (rho.pub_t / 3600.),
        time: rho.pub_t,
        bought_vars,
        theory_inputs: lifetime_alpha_output(log_to_exp(pub_alpha, 7)),
    })
}

/**
 * Simulates NLI with the given strat (sim-3 `MainNLISim`). The input's "rho" is tau (NLI's tau
 * factor is 1); the player's lifetime α (log10) comes from the theory inputs, if given.
 */
pub fn simulate_nli(strat: NliStrat, input: &TheoryInput) -> Result<SimResult, String> {
    let lifetime_alpha = input.params.nli_lifetime_alpha;
    let nli_input = NliInput {
        sigma: input.sigma,
        base_tau: input.last_pub_tau.unwrap_or(input.last_pub_rho),
        // sim-3 `deriveAlphaFromTau` is not written yet and returns 0
        base_alpha: lifetime_alpha.unwrap_or(0.),
        dt: input.params.dt,
        ddt: input.params.ddt,
        bought_vars_delta: input.params.bought_vars_delta,
    };
    if alpha_to_milestone_count(nli_input.base_tau, nli_input.base_alpha) >= MILESTONE_COSTS.len() {
        return simulate_parallel(strat, &nli_input);
    }

    let tau = nli_input.base_tau;
    let max_alpha_simulated = if tau < 20. {
        tau * 1.6 + 25.
    } else if tau < 120. {
        tau * 1.6 + 10.
    } else {
        tau * 1.55 + 5.
    };

    let mut alpha_sim = AlphaSimulator::new(strat, &nli_input);
    alpha_sim.simulate_to(max_alpha_simulated);
    let alpha_alone = alpha_sim.result_from_state(&alpha_sim.best_alone_state());

    let mut best_rho = default_result();
    for cluster in alpha_sim.cluster_states()? {
        let res = simulate_rho_phase(strat, &nli_input, &cluster, &alpha_sim)?;
        best_rho = best_result(best_rho, Some(res));
    }
    Ok(best_result(alpha_alone, Some(best_rho)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theories::traditional::SimParams;

    fn input(tau: f64, lifetime_alpha: Option<f64>) -> TheoryInput {
        TheoryInput {
            sigma: 0,
            last_pub_rho: tau,
            last_pub_tau: None,
            rho_cap: f64::INFINITY,
            params: SimParams {
                nli_lifetime_alpha: lifetime_alpha,
                ..SimParams::default()
            },
        }
    }

    #[test]
    fn parses_exponential_values_like_sim3() {
        assert_eq!(parse_exponential_value("150").unwrap(), 150.);
        assert_eq!(parse_exponential_value("e12.5").unwrap(), 12.5);
        assert!((parse_exponential_value("2e10").unwrap() - (10. + 2f64.log10())).abs() < 1e-12);
        // mantissas below 1 count as 1
        assert_eq!(parse_exponential_value("0.5e10").unwrap(), 10.);
        for bad in ["", "e", "1e", "1.e5", "-3", "1e-5", "abc"] {
            assert!(parse_exponential_value(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn exponential_value_m1_is_almost_power_minus_one() {
        let v = ExponentialValueM1 { power: 2. };
        let at = |level| fastmath::exp10(v.compute_from_zero(level).value);
        assert!((at(3) - 7.).abs() < 1e-3);
        assert!(at(0) < 1e-3);
    }

    #[test]
    fn milestones_follow_lifetime_alpha() {
        assert_eq!(alpha_to_permas(0.), [0, 0, 0]);
        assert_eq!(alpha_to_permas(150.), [2, 1, 2]);
        assert_eq!(alpha_to_milestone_count(5., 0.), 0);
        assert!(alpha_to_milestone_count(400., 300.) > alpha_to_milestone_count(100., 300.));
    }

    #[test]
    fn early_publication_runs_both_phases() {
        let res = simulate_nli(NliStrat::NLI, &input(30., None)).unwrap();
        assert!(res.tau_h > 0. && res.pub_rho.value > 30., "{res:?}");
        assert!(res.strat.starts_with("NLI "), "{}", res.strat);
        let alpha = &res.theory_inputs.as_ref().unwrap()["lifetime_alpha"];
        assert!(parse_exponential_value(alpha).is_ok(), "{alpha}");
    }

    #[test]
    fn late_publication_runs_in_parallel() {
        let res = simulate_nli(NliStrat::NLI, &input(2400., Some(1200.))).unwrap();
        assert!(res.strat.contains(" parallel "), "{}", res.strat);
        assert!(res.tau_h > 0.);
        assert!(
            res.bought_vars
                .windows(2)
                .all(|w| w[0].timestamp <= w[1].timestamp)
        );
    }
}
