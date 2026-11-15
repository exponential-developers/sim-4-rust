/*!
 * Theory MF (magnetic field), ported from sim-3 `src/Theories/CTs/MF.ts`.
 *
 * Variable order: c1, c2, a1, a2, delta, v1, v2, v3, v4
 *
 * v1 to v4 set the speed of the particle. They are only bought together when the particle is
 * reset ("reset bundles"), and the sim forks to try different reset points: how many resets
 * ahead it looks is the `mf_reset_depth` setting.
 */

use std::borrow::Cow;
use std::f64::consts::{E, PI};
use std::sync::OnceLock;

use crate::api::config::TheoryType;
use crate::theories::pubtable::{CodedTable, PubTable};
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

const C1: usize = 0;
const C2: usize = 1;
const A1: usize = 2;
const A2: usize = 3;
const DELTA: usize = 4;
const V1: usize = 5;
const V2: usize = 6;
const V3: usize = 7;
const V4: usize = 8;

const VARIABLE_COUNT: usize = 9;

/** Order in which milestones are assigned */
const MILESTONE_PRIORITY: [usize; 6] = [0, 1, 2, 3, 4, 5];

/** Rho at which each milestone is unlocked */
const MILESTONE_UNLOCKS: [f64; 9] = [20., 50., 175., 225., 275., 325., 425., 475., 525.];

/** How close to the last pub the sim looks at more resets, by reset depth (0 to 5) */
const DEPTH_CONVERT: [f64; 6] = [-99999., 8., 15., 25., 35., 45.];

/** The reset bundles tried: how many levels of v1 to v4 are bought at each reset */
const RESET_BUNDLES: [[i32; 4]; 3] = [[0, 1, 0, 0], [0, 1, 0, 1], [0, 2, 0, 0]];

/** Exponent of delta's cost per milestone level in the active strats */
const D_POWER: [f64; 3] = [3.09152, 3.00238, 2.91940];

// Physical constants
const MU0: f64 = 4e-7 * PI;
const Q0: f64 = 1.602e-19;
const I0: f64 = 1e-15;
const M0: f64 = 1e-3;

const CONVERTER: Converter = Converter {
    r9_affected: false,
    tau_factor: 1.,
    mult_exponent: 0.17,
    mult_factor: 0.,
};

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MfStrat {
        MF,
        MFRC,
        MFd,
        MFd2,
        MFd3,
        MFRCCoast,
        MFRCPTCoast,
        MFCoast,
        MFdCoast,
        MFdPTCoast,
        MFd2Coast,
        MFd2PTCoast,
        MFd3Coast,
        MFd3PTCoast,
        MFdRCCoast,
        MFd2RCCoast,
        MFd3RCCoast,
        MFVariantd1d1d2Coast,
        MFVariantd1d1d2PTCoast,
        MFVariantd1d1d3Coast,
        MFVariantd1d2d1Coast,
        MFVariantd1d2d1PTCoast,
        MFVariantd1d2d2Coast,
        MFVariantd1d2d2PTCoast,
        MFVariantd2d1d1Coast,
        MFVariantd2d1d2Coast,
        MFVariantd2d1d3Coast,
        MFVariantd2d2d1Coast,
        MFVariantd2d2d3Coast,
        MFVariantd2d2d3PTCoast,
        MFVariantd1d1d2RCCoast,
        MFVariantd1d1d3RCCoast,
        MFVariantd1d2d1RCCoast,
        MFVariantd1d2d2RCCoast,
        MFVariantd2d1d1RCCoast,
        MFVariantd2d1d2RCCoast,
        MFVariantd2d1d3RCCoast,
        MFVariantd2d2d1RCCoast,
        MFVariantd2d2d3RCCoast,
        MFPerfect,
        MFPerfectCoast,
    }
}

/** How c1 and a1 decide to buy: not at all conditionally (idle), or by the "d" or "d2" rule */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rule {
    Idle,
    D,
    D2,
}

/** How delta decides to buy */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeltaRule {
    Always,
    D1,
    D2,
    D3,
}

/** The buying rules of a strat, which are combined from a few building blocks */
#[derive(Debug, Clone, Copy)]
struct Rules {
    c1: Rule,
    /** RC strats stop buying c1 when the next reset is close: the factor of the goal cost (log10) */
    c1_reset_factor: Option<f64>,
    a1: Rule,
    a1_reset: bool,
    delta: DeltaRule,
}

impl MfStrat {
    fn is_coast(self) -> bool {
        self.name().contains("Coast")
    }

    fn is_perfect(self) -> bool {
        self.name().contains("Perfect")
    }

    /** The publication table: the passive one for MFRCPT, the active one for the other PT strats */
    fn passive_pub_table(self) -> bool {
        self.name().contains("MFRCPT")
    }

    /** Derives the rules from the strat name (sim-3 spells each combination out) */
    fn rules(self) -> Rules {
        let name = self.name();
        let rc = name.contains("RC");
        let l10_2 = l10(2.);
        let (c1, c1_reset, a1, delta) = if let Some(rest) = name.strip_prefix("MFVariantd") {
            // MFVariantd<c1><a1><delta>...: e.g. "1d1d2RCCoast" (c1 rule 1, a1 rule 1, delta rule 2)
            let digit = |i: usize| rest.as_bytes()[i] - b'0';
            let (x, y, z) = (digit(0), digit(2), digit(4));
            let rule = |n: u8| if n == 1 { Rule::D } else { Rule::D2 };
            let c1_reset = match (x, y) {
                (2, _) => l10(4. / 3.),
                (_, 1) => l10(5.),
                _ => l10_2,
            };
            let delta = match z {
                1 => DeltaRule::D1,
                2 => DeltaRule::D2,
                _ => DeltaRule::D3,
            };
            (rule(x), c1_reset, rule(y), delta)
        } else {
            let rest = &name[2..];
            if rest.starts_with("d3") {
                (Rule::D, l10(5.), Rule::D2, DeltaRule::D3)
            } else if rest.starts_with("d2") {
                (Rule::D2, l10(4. / 3.), Rule::D2, DeltaRule::D2)
            } else if rest.starts_with('d') {
                (Rule::D, l10_2, Rule::D, DeltaRule::D1)
            } else {
                // MF, MFCoast, MFRC...: buys everything
                (Rule::Idle, l10_2, Rule::Idle, DeltaRule::Always)
            }
        };
        Rules {
            c1,
            c1_reset_factor: rc.then_some(c1_reset),
            a1,
            a1_reset: rc,
            delta,
        }
    }
}

/** Publication tables */
fn pub_table(passive: bool) -> &'static PubTable {
    static PASSIVE: OnceLock<PubTable> = OnceLock::new();
    static ACTIVE: OnceLock<PubTable> = OnceLock::new();
    if passive {
        PASSIVE.get_or_init(|| {
            PubTable::decode_format1(&CodedTable::parse(include_str!(
                "data/table_mf_0_05_mfrccoast_coded.json"
            )))
        })
    } else {
        ACTIVE.get_or_init(|| {
            PubTable::decode_format1(&CodedTable::parse(include_str!(
                "data/table_mf_0_05_mf_overall_coded.json"
            )))
        })
    }
}

/** Simulates MF with the given strat (sim-3 `mf` function): the best of the reset bundles */
pub fn simulate_mf(strat: MfStrat, input: &TheoryInput) -> SimResult {
    let mut best = default_result();
    for bundle in RESET_BUNDLES {
        if input.last_pub_rho <= 100. && bundle[3] > 0 {
            continue;
        }
        let mut sim = MfSim::new(strat, input, bundle);
        if strat.is_coast() && sim.mf_reset_depth > 0 {
            // First find how many levels of c1 the strat buys without looking ahead
            let mut temp = MfSim::new(strat, input, bundle);
            temp.mf_reset_depth = 0;
            let temp_res = temp.simulate();
            if let Some((_, rest)) = temp_res.strat.split_once("c1: ") {
                let level = rest.split(' ').next().unwrap_or("");
                sim.last_c1 = level.parse::<i64>().map_or(f64::NAN, |v| v as f64);
            }
        }
        let res = sim.simulate();
        best = best_result(best, Some(res));
    }
    best
}

/** sim-3 `getBestResult(first, second)` where the first may be missing */
fn best_of(first: Option<SimResult>, second: SimResult) -> SimResult {
    best_result(first.unwrap_or_else(default_result), Some(second))
}

/** log10 constants, computed once per sim */
#[derive(Debug, Clone)]
struct MfConsts {
    l10_q0_m0_mu0: f64,
    l10_2: f64,
    l10_9_9: f64,
    l10_20: f64,
    l10_1_2: f64,
    l10_1_012: f64,
    l10_0_6: f64,
    l10_0_75: f64,
    l10_10: f64,
    /** log10(8 + n) for n = c1 level % 7 */
    l10_8_plus: [f64; 7],
    l10_d_power: [f64; 3],
    // for the constant c
    l10_4e13: f64,
    l10_omega: f64,
    l10_3e19: f64,
    l10_1e5: f64,
    l10_8_67e23: f64,
}

#[derive(Debug, Clone)]
struct MfSim {
    core: TraditionalCore,
    strat: MfStrat,
    rules: Rules,
    coast: bool,
    vars: Vec<Variable>,
    /** Levels of c1 the strat buys (infinity: no limit) */
    last_c1: f64,
    fork_on_c1: bool,
    c: f64,
    x: f64,
    i: f64,
    vx: f64,
    vz: f64,
    vtot: f64,
    resets: i32,
    stop_reset: bool,
    reset_bundle: [i32; 4],
    goal_bundle: [i32; 4],
    goal_bundle_cost: f64,
    mf_reset_depth: i32,
    best_res: Option<SimResult>,
    prev_milestone_count: i32,
    // Values that only change with milestones or variable levels (sim-3 `precomp_*`)
    precomp_omegaexp: f64,
    precomp_xexp: f64,
    precomp_vexp: f64,
    precomp_a1exp: f64,
    precomp_vterm: f64,
    precomp_va1: f64,
    precomp_va2: f64,
    consts: MfConsts,
}

impl MfSim {
    fn new(strat: MfStrat, input: &TheoryInput, reset_bundle: [i32; 4]) -> Self {
        let mut core = TraditionalCore::new(TheoryType::MF, false, input, CONVERTER, 8.);
        core.milestone_unlocks = MILESTONE_UNLOCKS.to_vec();
        core.milestones_max = vec![1, 1, 2, 2, 2, 1];
        // MF never reads the publication multiplier during the sim
        core.track_cur_mult = false;

        let exp = |value: f64| Value::new_exponential(value);
        let mut c1 = Variable::new(
            "c1",
            Cost::new(FirstFreeCost::new(exp_cost_model(10., 2.))),
            Value::new_stepwise(2., 7, 0.),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        c1.buy();
        let vars = vec![
            c1,
            Variable::new("c2", exp_cost(1e3, 50.), exp(2.)),
            Variable::new("a1", exp_cost(1e3, 25.), Value::new_stepwise(2., 5, 3.)),
            Variable::new("a2", exp_cost(1e4, 100.), exp(1.25)),
            Variable::new("δ", exp_cost(1e50, 300.), exp(1.1)),
            Variable::new("v1", exp_cost(80., 80.), Value::new_stepwise(2., 10, 1.)),
            Variable::new("v2", exp_cost(1e4, fastmath::pow(10., 4.5)), exp(1.3)),
            Variable::new("v3", exp_cost(1e50, 70.), Value::new_stepwise(2., 10, 0.)),
            Variable::new("v4", exp_cost(1e52, 1e6), exp(1.5)),
        ];

        let consts = MfConsts {
            l10_q0_m0_mu0: l10((Q0 / M0) * MU0),
            l10_2: l10(2.),
            l10_9_9: l10(9.9),
            l10_20: l10(20.),
            l10_1_2: l10(1.2),
            l10_1_012: l10(1.012),
            l10_0_6: l10(0.6),
            l10_0_75: l10(0.75),
            l10_10: l10(10.),
            l10_8_plus: std::array::from_fn(|n| l10(8. + n as f64)),
            l10_d_power: D_POWER.map(l10),
            l10_4e13: l10(4e13),
            l10_omega: l10(M0 / (Q0 * MU0 * I0)) - l10(900.),
            l10_3e19: l10(3e19),
            l10_1e5: l10(1e5),
            l10_8_67e23: l10(8.67e23),
        };

        if strat.name().contains("PT") {
            let table = pub_table(strat.passive_pub_table());
            if let Some(next_rho) = table.lookup(core.last_pub_rho, 20.) {
                core.do_sim_end_conditions = false;
                core.pub_target = Some(next_rho);
            }
        }

        let precomp_va2 = fastmath::pow(10., vars[A2].value.value);
        let mut sim = MfSim {
            core,
            strat,
            rules: strat.rules(),
            coast: strat.is_coast(),
            vars,
            last_c1: f64::INFINITY,
            fork_on_c1: false,
            c: 0.,
            x: 0.,
            i: 0.,
            vx: 0.,
            vz: 0.,
            vtot: 0.,
            resets: 0,
            stop_reset: false,
            reset_bundle,
            goal_bundle: [0; 4],
            goal_bundle_cost: 0.,
            mf_reset_depth: input.params.mf_reset_depth,
            best_res: None,
            prev_milestone_count: -1,
            precomp_omegaexp: -1.,
            precomp_xexp: -1.,
            precomp_vexp: -1.,
            precomp_a1exp: -1.,
            precomp_vterm: -1.,
            precomp_va1: -1.,
            precomp_va2,
            consts,
        };
        sim.update_milestones_no_ms();
        // This will update precomp_vterm:
        sim.precompute_exps();
        sim.reset_particle();
        sim
    }

    /** sim-3 `copy()`: copyFrom keeps most of the state, but the constructor's milestone count */
    fn copy(&self) -> MfSim {
        let mut fork = self.clone();
        fork.best_res = None;
        fork.fork_on_c1 = false;
        fork.prev_milestone_count =
            binary_insertion_search(&MILESTONE_UNLOCKS, fork.core.last_pub_rho.max(0.)) as i32;
        fork.precompute_exps();
        fork
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    fn omegaexp(&self) -> f64 {
        4.1 + 0.15 * self.core.milestones[2] as f64
    }

    fn xexp(&self) -> f64 {
        3.2 + 0.1 * self.core.milestones[3] as f64
    }

    fn vexp(&self) -> f64 {
        1.3 + 0.31 * self.core.milestones[4] as f64
    }

    fn a1exp(&self) -> f64 {
        1. + 0.01 * self.core.milestones[5] as f64
    }

    fn precompute_exps(&mut self) {
        self.precomp_a1exp = self.a1exp();
        self.precomp_vexp = self.vexp();
        self.precomp_xexp = self.xexp();
        self.precomp_omegaexp = self.omegaexp();
        self.compute_vterm();
        // We may have just updated a1exp, we need to update va1
        self.compute_va1();
    }

    fn compute_vterm(&mut self) {
        self.precomp_vterm = if self.core.milestones[0] != 0 {
            l10(self.vtot) * self.precomp_vexp
        } else {
            0.
        };
    }

    fn compute_va1(&mut self) {
        self.precomp_va1 = fastmath::pow(10., self.val(A1) * self.precomp_a1exp);
    }

    /** sim-3 `updateMilestonesNoMS`: only recalculates when the number of milestones changes */
    fn update_milestones_no_ms(&mut self) {
        let rho = self.core.max_rho.max(self.core.last_pub_rho);
        let count = binary_insertion_search(&MILESTONE_UNLOCKS, rho) as i32;
        if count != self.prev_milestone_count {
            self.prev_milestone_count = count;
            self.core.set_milestones_at(rho, &MILESTONE_PRIORITY);
            self.precompute_exps();
            self.update_c();
        }
    }

    fn update_c(&mut self) {
        let k = &self.consts;
        let xterm = k.l10_4e13 * self.precomp_xexp;
        let omegaterm = k.l10_omega * self.precomp_omegaexp;
        let vterm = if self.core.milestones[0] != 0 {
            k.l10_3e19 * 1.3 + k.l10_1e5 * (self.precomp_vexp - 1.3)
        } else {
            0.
        };
        self.c = xterm + omegaterm + vterm + k.l10_8_67e23;
    }

    /** sim-3 `resetParticle`: the particle starts again, with the speed v1 to v4 give it */
    fn reset_particle(&mut self) {
        self.x = 0.;
        self.vx = fastmath::pow(10., self.val(V1) + self.val(V2) - 20.);
        self.vz = fastmath::pow(10., self.val(V3) + self.val(V4) - 18.);
        self.vtot = (self.vx * self.vx + self.vz * self.vz).sqrt();
        self.compute_vterm();
        self.resets += 1;
        if self.resets > 1 {
            let level = |i: usize| self.vars[i].level;
            let name = format!(
                "Reset at V={},{},{},{}",
                level(V1),
                level(V2),
                level(V3),
                level(V4)
            );
            self.core.bought_vars.push(VarBuy {
                var_name: Cow::Owned(name),
                level: self.resets - 1,
                cost: LogNum {
                    value: self.core.max_rho,
                    sign: 1,
                },
                symbol: "rho",
                timestamp: self.core.t,
            });
        }
        self.goal_bundle = self.get_goal_bundle(self.reset_bundle);
        self.goal_bundle_cost = self.calc_bundle_cost(self.goal_bundle);
    }

    /** sim-3 `getCostForLevels`: the cost of buying levels `from` to `to - 1` of a variable */
    fn cost_for_levels(&self, id: usize, from: i32, to: i32) -> f64 {
        let model = &self.vars[id].cost_model;
        let mut total = model.get_cost(from).value;
        for level in (from + 1)..to {
            total = add_log(total, model.get_cost(level).value);
        }
        total
    }

    /** Cost of the next level of variable `id`, `extra` levels ahead (sim-3 `getCostForLevel`) */
    fn cost_at(&self, id: usize, extra: i32) -> f64 {
        self.vars[id]
            .cost_model
            .get_cost(self.vars[id].level + extra)
            .value
    }

    /** sim-3 `calcBundleCost` */
    fn calc_bundle_cost(&self, bundle: [i32; 4]) -> f64 {
        let mut cost = 0.;
        for (i, &levels) in bundle.iter().enumerate() {
            if levels == 0 {
                continue;
            }
            let id = V1 + i;
            cost = add_log(
                cost,
                self.cost_for_levels(id, self.vars[id].level, self.vars[id].level + levels),
            );
        }
        cost
    }

    /** sim-3 `getGoalBundle`: how many levels of v1 to v4 the next reset buys */
    fn get_goal_bundle(&self, bundle: [i32; 4]) -> [i32; 4] {
        let mut goal = bundle;
        if self.core.max_rho <= 65. {
            goal[2] = 0;
            goal[3] = 0;
        }

        let mut bundle_cost = self.calc_bundle_cost(goal);
        while self.cost_at(V2, goal[1]) < bundle_cost + 0.01 {
            goal[1] += 1;
        }
        bundle_cost = self.calc_bundle_cost(goal);
        while self.cost_at(V4, goal[3]) < bundle_cost + 0.01 {
            goal[3] += 1;
        }
        bundle_cost = self.calc_bundle_cost(goal);
        while self.cost_at(V1, goal[0]) < bundle_cost + 0.01 {
            goal[0] += 1;
        }
        bundle_cost = self.calc_bundle_cost(goal);
        while self.cost_at(V3, goal[2]) < bundle_cost + 0.01 {
            goal[2] += 1;
        }
        goal
    }

    fn c1_rule_ok(&self) -> bool {
        let c = |j: usize| self.cost(j);
        let k = &self.consts;
        if let Some(factor) = self.rules.c1_reset_factor
            && !(c(C1) < self.goal_bundle_cost - factor || self.stop_reset)
        {
            return false;
        }
        if (self.vars[C1].level as f64) >= self.last_c1 {
            return false;
        }
        match self.rules.c1 {
            Rule::Idle => true,
            Rule::D => c(C1) + k.l10_9_9 <= c(C2).min(c(A2)).min(c(DELTA)),
            Rule::D2 => {
                let delta = if self.core.milestones[1] > 0 {
                    c(DELTA) + k.l10_d_power[self.core.milestones[2] as usize]
                } else {
                    f64::INFINITY
                };
                c(C1) + k.l10_8_plus[(self.vars[C1].level % 7) as usize]
                    <= (c(C2) + k.l10_2).min(c(A2)).min(delta)
            }
        }
    }

    fn a1_rule_ok(&self) -> bool {
        let c = |j: usize| self.cost(j);
        let k = &self.consts;
        if self.rules.a1_reset && !(c(A1) < self.goal_bundle_cost - k.l10_10 || self.stop_reset) {
            return false;
        }
        match self.rules.a1 {
            Rule::Idle => true,
            Rule::D => self.i / (I0 * self.precomp_va2) < 0.5 || c(A1) + 1. < self.core.max_rho,
            Rule::D2 => {
                l10(self.i) + k.l10_1_2 < self.val(A2) - 15.
                    || (c(A1) + k.l10_20 < self.core.max_rho
                        && l10(self.i) + k.l10_1_012 < self.val(A2) - 15.)
            }
        }
    }

    fn delta_rule_ok(&self) -> bool {
        let c = |j: usize| self.cost(j);
        let k = &self.consts;
        match self.rules.delta {
            DeltaRule::Always => true,
            DeltaRule::D1 => c(DELTA) < c(C2).min(c(A2)),
            DeltaRule::D2 => {
                c(DELTA) + k.l10_d_power[self.core.milestones[2] as usize]
                    < (c(C2) + k.l10_2).min(c(A2))
            }
            DeltaRule::D3 => c(DELTA) < (c(A2) + k.l10_0_6).min(c(C2) + k.l10_0_75),
        }
    }

    fn buying_condition(&self, i: usize) -> bool {
        match i {
            C1 => self.c1_rule_ok(),
            A1 => self.a1_rule_ok(),
            DELTA => self.delta_rule_ok(),
            _ => true,
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        let ms = &self.core.milestones;
        match i {
            DELTA => ms[1] > 0,
            V3 | V4 => ms[0] > 0,
            _ => true,
        }
    }

    fn tick(&mut self) {
        // Deal with i
        let icap = self.precomp_va2 * I0; // max reachable i value
        let dt = self.core.dt;
        if self.i < icap {
            // if max i is not reached, we add a value to it:
            let mut scale =
                1. - fastmath::pow(E, -dt * self.precomp_va1 / (400. * self.precomp_va2));
            if scale < 1e-13 {
                scale = dt * self.precomp_va1 / (400. * self.precomp_va2);
            }
            self.i += scale * (icap - self.i);
            self.i = js_min(self.i, icap);
        }

        self.x += dt * self.vx;
        let xterm = l10(self.x) * self.precomp_xexp;
        let omegaterm =
            (self.consts.l10_q0_m0_mu0 + l10(self.i) + self.val(DELTA)) * self.precomp_omegaexp;

        let rhodot = self.core.tot_mult
            + self.c
            + self.val(C1)
            + self.val(C2)
            + xterm
            + omegaterm
            + self.precomp_vterm;
        self.core.rho += LogNum {
            value: rhodot + l10(dt),
            sign: 1,
        };
    }

    /** Buys c1 to delta, and nothing else (sim-3 `buyNormalVariables`) */
    fn buy_normal_variables(&mut self) {
        for i in (0..=DELTA).rev() {
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

    /** Buys v1 to v4 as far as possible (sim-3 `buyVVariables`) */
    fn buy_v_variables(&mut self) {
        for i in (V1..=V4).rev() {
            while self.core.rho.value > self.cost(i) && self.variable_available(i) {
                self.core.record_purchase(&self.vars[i]);
                let cost = self.vars[i].cost;
                self.core.rho -= cost;
                self.vars[i].buy();
            }
        }
    }

    /** sim-3 `getVariableWeights` (MFPerfect) */
    fn variable_weights(&self) -> [f64; VARIABLE_COUNT] {
        let c1 = &self.vars[C1];
        let c1_new = c1.value_model.compute_next(c1.value, c1.level + 1).value;
        let c1_instant = fastmath::pow(10., c1_new - c1.value.value);
        let c2_instant = 2.;
        let a2_instant = 1.25 * self.precomp_omegaexp;
        let delta_instant = fastmath::pow(1.1, self.precomp_omegaexp);

        let weight_c1 = js_max(
            js_max(1. - 1. / c2_instant, 1. - 1. / a2_instant),
            1. - 1. / delta_instant,
        ) / (1. - 1. / c1_instant);
        let weight_delta =
            js_max(1. - 1. / c2_instant, 1. - 1. / a2_instant) / (1. - 1. / delta_instant);
        let k = &self.consts;
        let a1_ok = l10(self.i) + k.l10_1_2 < self.val(A2) - 15.
            || (self.cost(A1) + k.l10_20 < self.core.max_rho
                && l10(self.i) + k.l10_1_012 < self.val(A2) - 15.);
        [
            if (self.vars[C1].level as f64) < self.last_c1 {
                l10(weight_c1)
            } else {
                f64::INFINITY
            },
            0.,
            if a1_ok { 0. } else { f64::INFINITY },
            0.,
            l10(weight_delta),
            f64::INFINITY,
            f64::INFINITY,
            f64::INFINITY,
            f64::INFINITY,
        ]
    }

    /** sim-3 `buyVariablesWeight`: always buys the variable with the lowest cost + weight */
    fn buy_variables_weight(&mut self) {
        loop {
            let weights = self.variable_weights();
            let mut min_cost = f64::MAX;
            let mut min_index = None;
            for i in (0..VARIABLE_COUNT).rev() {
                let weighted = self.cost(i) + weights[i];
                if weighted < min_cost && self.variable_available(i) {
                    min_cost = weighted;
                    min_index = Some(i);
                }
            }
            match min_index {
                Some(i) if self.cost(i) < self.core.rho.value => {
                    let cost = self.vars[i].cost;
                    self.core.rho -= cost;
                    self.core.record_purchase(&self.vars[i]);
                    self.vars[i].buy();
                    self.on_variable_purchased(i);
                }
                _ => break,
            }
        }
    }

    fn buy_variables(&mut self) {
        if self.strat.is_perfect() {
            self.buy_variables_weight();
        } else {
            self.buy_normal_variables();
        }
    }

    fn on_variable_purchased(&mut self, id: usize) {
        if id == A1 {
            // Update va1 after an a1 level purchase
            self.compute_va1();
        }
        if id == A2 {
            self.precomp_va2 = fastmath::pow(10., self.val(A2));
        }
        if self.mf_reset_depth == 0
            && self.coast
            && id == C1
            && self.last_c1 == f64::INFINITY
            && self.core.max_rho > self.core.last_pub_rho + 6.
        {
            self.fork_on_c1 = true;
        }
    }

    /** sim-3 `doForkC1`: also tries with a limit on the levels of c1 */
    fn do_fork_c1(&mut self) {
        let mut fork = self.copy();
        fork.last_c1 = self.vars[C1].level as f64;
        fork.fork_on_c1 = false;
        let res = fork.simulate();
        self.best_res = Some(best_of(self.best_res.take(), res));
        self.fork_on_c1 = false;
    }

    /** sim-3 `testFinalReset`: also tries never resetting again */
    fn test_final_reset(&mut self) {
        let mut fork = self.copy();
        fork.stop_reset = true;
        let res = fork.simulate();
        self.best_res = Some(best_of(self.best_res.take(), res));
    }

    /** sim-3 `checkForReset`: resets the particle, and forks to try other reset points */
    fn check_for_reset(&mut self) {
        let depth = DEPTH_CONVERT
            .get(self.mf_reset_depth as usize)
            .copied()
            .unwrap_or(f64::NAN);
        self.buy_v_variables();
        self.reset_particle();
        if self.core.max_rho >= self.core.last_pub_rho - 10. {
            self.test_final_reset();
        }
        if depth > 0. && self.core.last_pub_rho - self.core.max_rho <= depth {
            // extra v1 test, then extra v2 test
            for extra in [0, 1] {
                if self.core.last_pub_rho - self.core.max_rho <= depth {
                    let mut fork = self.copy();
                    let mut bundle = fork.goal_bundle;
                    bundle[extra] += 1;
                    fork.goal_bundle = fork.get_goal_bundle(bundle);
                    fork.goal_bundle_cost = fork.calc_bundle_cost(fork.goal_bundle);
                    let res = fork.simulate();
                    self.best_res = Some(best_of(self.best_res.take(), res));
                }
            }
        }
    }

    fn simulate(&mut self) -> SimResult {
        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            self.update_milestones_no_ms();
            self.buy_variables();
            // These checks are here for optimization:
            if !self.stop_reset && self.core.rho.value >= self.goal_bundle_cost + 0.0001 {
                self.check_for_reset();
            }
            if self.fork_on_c1 {
                self.do_fork_c1();
            }
            self.core.collect_pub_table();
        }
        self.core.trim_bought_vars();

        let mut extra = format!(" Depth: {}", self.mf_reset_depth);
        if self.last_c1 != f64::INFINITY {
            extra += &format!(" c1: {}", self.last_c1);
        }
        let result = self.core.create_result(self.strat.name(), &extra);
        best_result(result, self.best_res.take())
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
    fn deeper_reset_search_never_loses_and_reports_its_depth() {
        let mut deep = input(150., 90);
        deep.params.mf_reset_depth = 2;
        let shallow = simulate_mf(MfStrat::MFdCoast, &input(150., 90));
        let res = simulate_mf(MfStrat::MFdCoast, &deep);
        assert!(res.strat.contains(" Depth: 2"), "{}", res.strat);
        assert!(shallow.strat.contains(" Depth: 0"), "{}", shallow.strat);
        // resets are recorded as purchases of "Reset at V=..."
        assert!(
            res.bought_vars
                .iter()
                .any(|b| b.var_name.starts_with("Reset at V=")),
            "{:?}",
            res.bought_vars.len()
        );
    }

    #[test]
    fn rules_are_derived_from_the_strat_name() {
        let r = MfStrat::MFVariantd2d1d3RCCoast.rules();
        assert_eq!((r.c1, r.a1, r.delta), (Rule::D2, Rule::D, DeltaRule::D3));
        assert!(r.a1_reset && r.c1_reset_factor.is_some());
        let r = MfStrat::MFd3PTCoast.rules();
        assert_eq!((r.c1, r.a1, r.delta), (Rule::D, Rule::D2, DeltaRule::D3));
        assert!(r.c1_reset_factor.is_none());
        let r = MfStrat::MFRCCoast.rules();
        assert_eq!(
            (r.c1, r.a1, r.delta),
            (Rule::Idle, Rule::Idle, DeltaRule::Always)
        );
        assert!(MfStrat::MFPerfectCoast.is_perfect());
    }

    #[test]
    fn perfect_strats_buy_by_weight() {
        let res = simulate_mf(MfStrat::MFPerfect, &input(100., 70));
        assert!(res.tau_h > 0., "{res:?}");
        let sim = MfSim::new(MfStrat::MFPerfect, &input(100., 70), [0, 1, 0, 0]);
        let weights = sim.variable_weights();
        assert!(weights[V1..].iter().all(|w| *w == f64::INFINITY));
    }
}
