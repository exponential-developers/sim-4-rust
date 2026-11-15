/*!
 * Theory FI, ported from sim-3 `src/Theories/CTs/FI.ts`.
 *
 * Variable order: tdot, q1, q2, K, m, n
 */

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
    variable::Variable,
};

const TDOT: usize = 0;
const Q1: usize = 1;
const Q2: usize = 2;
const K: usize = 3;
const M: usize = 4;
const N: usize = 5;

/** Rho at which each milestone is unlocked */
const MILESTONE_UNLOCKS: [f64; 12] = [
    10., 20., 30., 70., 210., 300., 425., 530., 700., 800., 950., 1150.,
];
const FX_POINTS: [f64; 3] = [100., 450., 1050.];
const LAMBDA_POINTS: [f64; 2] = [350., 750.];

const Q_PRIORITY: [usize; 7] = [0, 1, 5, 6, 2, 3, 4];
const RHO_PRIORITY: [usize; 7] = [0, 1, 5, 6, 3, 4, 2];

const CONVERTER: Converter = Converter {
    r9_affected: false,
    tau_factor: 0.4,
    mult_exponent: 0.1625,
    mult_factor: 0.,
};

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FiStrat {
        FI,
        FIPTCoast,
        FICoast,
        FId,
        FIdCoast,
        FIPermaSwap,
        FIPTPermaSwapCoast,
        FIPermaSwapCoast,
        FIdPermaSwap,
        FIdPermaSwapCoast,
        FIMS,
        FIMSCoast,
        FIPTMSCoast,
        FIMSd,
        FIMSdCoast,
        FIMSPermaSwap,
        FIMSPermaSwapCoast,
        FIMSdPermaSwap,
        FIMSdPermaSwapCoast,
    }
}

impl FiStrat {
    fn is_coast(self) -> bool {
        self.name().contains("Coast")
    }

    fn uses_pub_table(self) -> bool {
        self.name().contains("PT")
    }

    /** Active ("d") strats: they only buy q1 and n when they are cheap enough */
    fn is_active(self) -> bool {
        let name = self.name();
        name.starts_with("FId") || name.starts_with("FIMSd")
    }

    fn is_perma_swap(self) -> bool {
        self.name().contains("PermaSwap")
    }

    /** Strats that build q before spending the milestone points */
    fn is_ms(self) -> bool {
        self.name().contains("MS")
    }

    fn without_pt(self) -> FiStrat {
        FiStrat::from_name(&self.name().replacen("PT", "", 1)).unwrap_or(self)
    }

    fn without_coast_and_pt(self) -> FiStrat {
        FiStrat::from_name(&self.name().replacen("Coast", "", 1).replacen("PT", "", 1))
            .unwrap_or(self)
    }
}

/** Publication table (sim-3 loads the passive table for the active strats as well) */
fn pub_table() -> &'static PubTable {
    static TABLE: OnceLock<PubTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        PubTable::decode_format1(&CodedTable::parse(include_str!(
            "data/table_fi_0_1_passive_coast_coded.json"
        )))
    })
}

/** Simulates FI with the given strat (sim-3 `fi` function) */
pub fn simulate_fi(strat: FiStrat, input: &TheoryInput) -> SimResult {
    let rho = input.last_pub_rho;
    let mut pt_continuity_res = None;
    if strat.uses_pub_table() && rho < 1499. {
        let next_rho = pub_table().target(rho, 10.);
        if next_rho == 1500. {
            let res = simulate_fi(strat.without_pt(), input);
            if res.pub_rho.value >= 1500. {
                pt_continuity_res = Some(res);
            }
        }
    }

    let res = if strat.is_coast() {
        let res1 = FiSim::new(strat.without_coast_and_pt(), input).simulate();
        let mut sim = FiSim::new(strat, input);
        sim.vars[Q1].set_original_cap(get_last_level("q1", &res1.bought_vars));
        // Maximum seen in the wild is 4, we test 5 and 6 to be safe.
        sim.vars[Q1].configure_cap(6);
        sim.simulate()
    } else {
        FiSim::new(strat, input).simulate()
    };
    best_result(res, pt_continuity_res)
}

/** log10 of n! (sim-3 `getFactorial`, which adds up the logs from the bottom) */
fn log_factorials() -> [f64; 7] {
    let mut table = [0.; 7];
    for n in 1..7 {
        table[n] = l10(n as f64) + table[n - 1];
    }
    table
}

/** Constants the sim computes once */
#[derive(Debug, Clone)]
struct FiConsts {
    factorial: [f64; 7],
    l10_pi: f64,
    l10_2: f64,
    l10_12: f64,
    l10_30: f64,
    l10_6: f64,
    l10_20: f64,
    l10_ln_10: f64,
    /** `-norm_int(log10(pi)) - 1 / (e + 1.519)`, by the fx milestone */
    approx_base: [f64; 4],
    /** log10(2), log10(3) and log10(4): the base of K's approximation, by the lambda milestone */
    l10_base: [f64; 3],
}

#[derive(Debug, Clone)]
struct FiSim {
    core: TraditionalCore,
    strat: FiStrat,
    coast: bool,
    vars: Vec<Variable>,
    q: f64,
    r: f64,
    tval: f64,
    max_fx: i32,
    max_lambda: i32,
    /** State of the "MS" strats building q: 0 not building, 1 start, 2 building */
    ms_state: i32,
    ms_q: f64,
    consts: FiConsts,
}

impl FiConsts {
    fn new() -> Self {
        let mut consts = FiConsts {
            factorial: log_factorials(),
            l10_pi: l10(PI),
            l10_2: l10(2.),
            l10_12: l10(12.),
            l10_30: l10(30.),
            l10_6: l10(6.),
            l10_20: l10(20.),
            l10_ln_10: l10(fastmath::ln(10.)),
            approx_base: [0.; 4],
            l10_base: [l10(2.), l10(3.), l10(4.)],
        };
        for fx in 0..4 {
            consts.approx_base[fx] = -consts.norm_int(fx as i32, consts.l10_pi) - 1. / (E + 1.519);
        }
        consts
    }

    fn norm_int(&self, fx: i32, limit: f64) -> f64 {
        let f = &self.factorial;
        match fx {
            // approxCos
            0 => {
                let positives = add_log(limit, limit * 5. - f[5]);
                let negatives = limit * 3. - f[3];
                sub_log(positives, negatives)
            }
            // approxSin
            1 => {
                let positives = add_log(limit * 2. - f[2], limit * 6. - f[6]);
                let negatives = limit * 4. - f[4];
                sub_log(positives, negatives)
            }
            // approxL10
            2 => {
                let positives = add_log(
                    add_log(limit * 2. - self.l10_2, limit * 4. - self.l10_12),
                    limit * 6. - self.l10_30,
                );
                let negatives = add_log(limit * 3. - self.l10_6, limit * 5. - self.l10_20);
                sub_log(positives, negatives) - self.l10_ln_10
            }
            // approxEX
            3 => {
                let mut sum = add_log(limit * 6. - f[6], limit * 5. - f[5]);
                sum = add_log(sum, limit * 4. - f[4]);
                sum = add_log(sum, limit * 3. - f[3]);
                sum = add_log(sum, limit * 2. - f[2]);
                add_log(sum, limit)
            }
            _ => unreachable!("sim-3 throws \"Error in norm_int().\""),
        }
    }
}

impl FiSim {
    fn new(strat: FiStrat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::FI, false, input, CONVERTER, 8.);
        core.milestone_unlocks = MILESTONE_UNLOCKS.to_vec();

        let mut q1 = Variable::new(
            "q1",
            Cost::new(FirstFreeCost::new(exp_cost_model(5., 14.6))),
            Value::new_stepwise(50., 23, 0.),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        q1.buy();
        let mut n = Variable::new("n", exp_cost(1e69, 11.), Value::new_stepwise(3., 11, 0.));
        // sim-3 buys the first level of n for free
        n.buy();

        let vars = vec![
            Variable::new("tdot", exp_cost(1e25, 1e50), Value::new_exponential(10.)),
            q1,
            Variable::new("q2", exp_cost(1e7, 5e3), Value::new_exponential(2.)),
            Variable::new("K", exp_cost(1e2, 10.), Value::new_exponential(10.)),
            Variable::new("m", exp_cost(1e4, 4.44), Value::new_exponential(1.5)),
            n,
        ];

        let lpr = core.last_pub_rho;
        if strat.uses_pub_table() && lpr < 1499. {
            core.do_sim_end_conditions = false;
            core.pub_target = Some(pub_table().target(lpr, 10.));
        }

        let mut sim = FiSim {
            core,
            strat,
            coast: strat.is_coast(),
            vars,
            q: 0.,
            r: 0.,
            tval: 0.,
            max_fx: 0,
            max_lambda: 0,
            ms_state: 0,
            ms_q: 0.,
            consts: FiConsts::new(),
        };
        sim.update_milestones(0.);
        sim
    }

    /** Sets what a milestone-based variable costs and resets it to level 0 (sim-3 `reset`) */
    fn reset_variable(&mut self, id: usize, base: f64, increase: f64) {
        let v = &mut self.vars[id];
        v.cost_model = exp_cost(base, increase);
        v.level = 0;
        v.compute_from_zero();
    }

    /** sim-3 `getMilestonePriority`: also sets the maximum level of each milestone */
    fn milestone_priority(&mut self, max_rho: f64) -> &'static [usize] {
        let rho = max_rho.max(self.core.last_pub_rho);
        let total_points = binary_insertion_search(&MILESTONE_UNLOCKS, rho) as i32;

        let mut available_fx = binary_insertion_search(&FX_POINTS, rho) as i32;
        let available_lambda = binary_insertion_search(&LAMBDA_POINTS, rho) as i32;
        let use_fx_level3 = if self.strat.is_perma_swap() {
            max_rho >= 1076.
        } else {
            rho >= 1150.
        };
        if !use_fx_level3 {
            available_fx = available_fx.min(2);
        }
        self.core.milestones_max = vec![1, 1, 3, 1, 1, available_fx, available_lambda];

        let q1mn_points = total_points - (2 + available_fx + available_lambda);
        let q1m23 = self.vars[Q1].level % 23;
        let qf = if q1m23 < 5 {
            4.
        } else if q1m23 < 10 {
            3.
        } else if q1m23 < 20 {
            2.5
        } else {
            2.
        };
        if self.strat.is_ms() && q1mn_points > 0 && q1mn_points < 5 && self.ms_state > 0 {
            if self.ms_state == 1 {
                // start q build
                self.ms_state = 2;
                self.ms_q = self.q;
                return &Q_PRIORITY;
            } else if self.ms_state == 2 && self.ms_q + l10(qf) < self.q {
                // end q build
                self.ms_state = 0;
                return &RHO_PRIORITY;
            }
            // continue q build
            return &Q_PRIORITY;
        }
        &RHO_PRIORITY
    }

    /** sim-3 `updateMilestones`: also changes the costs of q2 and K when fx or lambda go up */
    fn update_milestones(&mut self, max_rho: f64) {
        let priority = self.milestone_priority(max_rho);
        self.core.update_milestones(priority);

        let fx = self.core.milestones[5];
        let lambda = self.core.milestones[6];
        if fx > self.max_fx {
            match fx {
                1 => {
                    self.reset_variable(Q2, 1e7, 3e3);
                    self.q = 0.;
                }
                2 => {
                    self.reset_variable(Q2, 1e-10, 2.27e3);
                    self.q = 0.;
                }
                3 => {
                    self.reset_variable(Q2, 1e95, 1.08e3);
                    self.q = 0.;
                    self.ms_q = 0.;
                }
                _ => {}
            }
            self.max_fx = fx;
        }
        if lambda > self.max_lambda {
            match lambda {
                1 => self.reset_variable(K, 1e-5, 37.),
                2 => self.reset_variable(K, 1e-10, 95.),
                _ => {}
            }
            self.max_lambda = lambda;
        }
    }

    /** sim-3's fork: a new sim (milestones from the last pub rho alone) with this one's state */
    fn fork_without_milestones(&self) -> FiSim {
        let mut fork = self.clone();
        // the constructor starts with no milestone state
        let saved = (fork.ms_state, fork.ms_q);
        fork.ms_state = 0;
        let priority = fork.milestone_priority(0.);
        fork.core
            .set_milestones_at(fork.core.last_pub_rho.max(0.), priority);
        (fork.ms_state, fork.ms_q) = saved;
        fork
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    fn buying_condition(&self, i: usize) -> bool {
        match i {
            Q1 => {
                let ok = !self.coast || self.vars[Q1].should_buy;
                ok && (!self.strat.is_active()
                    || self.cost(Q1) + l10((self.vars[Q1].level % 23 + 1) as f64)
                        < self.cost(Q2).min(self.cost(K)))
            }
            N if self.strat.is_active() => {
                self.cost(N) + l10((self.vars[N].level % 11 + 1) as f64)
                    < self.cost(Q2).min(self.cost(K)).min(self.cost(M))
            }
            _ => true,
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        let ms = &self.core.milestones;
        match i {
            TDOT => self.vars[TDOT].level < 4,
            K => ms[1] > 0,
            M => ms[3] > 0,
            N => ms[4] > 0,
            _ => true,
        }
    }

    fn tick(&mut self) {
        let ms = &self.core.milestones;
        let (ms0, ms2, ms3, ms4, ms5, ms6) = (ms[0], ms[2], ms[3], ms[4], ms[5], ms[6]);
        let c = &self.consts;
        let vq1 = self.val(Q1) * (1. + 0.01 * ms2 as f64);
        let vden = c.approx_base[ms5 as usize] + self.val(K) * c.l10_base[ms6 as usize];
        let l10dt = l10(self.core.dt);

        self.tval += ((self.val(TDOT) + 1.) / 5.) * self.core.dt;
        self.q = add_log(self.q, vq1 + self.val(Q2) + l10dt);
        self.r = add_log(self.r, vden + l10dt);

        let integral = (if ms0 != 0 {
            c.norm_int(ms5, self.q - if ms5 < 3 { c.l10_pi } else { 0. })
        } else {
            self.q - c.l10_pi
        }) * (1. / PI);
        let vm = if ms3 != 0 { self.val(M) } else { 0. };
        let vn = if ms4 != 0 { self.val(N) } else { 0. };

        let rhodot = l10(self.tval) + integral + self.r + vm + vn;
        self.core.rho += LogNum {
            value: self.core.tot_mult + rhodot + l10dt,
            sign: 1,
        };
    }

    fn buy_variables(&mut self) {
        // FI has no onAnyVariablePurchased step, so whether something was bought is not tracked
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
        if id == Q2 && self.ms_state == 0 {
            self.ms_state = 1;
        }
        if id == Q1 && self.coast {
            let v = &mut self.vars[id];
            if v.should_buy && v.coasting_cap_reached() {
                v.should_fork = true;
            }
        }
    }

    /** Simulates a fork where variable `id` is no longer bought (sim-3 `doForkVariable`) */
    fn do_fork_variable(&mut self, id: usize) {
        self.vars[id].should_fork = false;
        let mut fork = self.fork_without_milestones();
        fork.vars[id].stop_buying();
        let res = fork.simulate();
        self.core.best_fork_res = Some(best_result(res, self.core.best_fork_res.take()));
    }

    fn simulate(&mut self) -> SimResult {
        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            self.update_milestones(self.core.max_rho);
            self.buy_variables();
            if self.vars[Q1].should_fork {
                self.do_fork_variable(Q1);
            }
            self.core.collect_pub_table();
        }
        self.core.trim_bought_vars();

        let mut extra = String::new();
        if self.coast {
            extra +=
                &self.vars[Q1].prepare_extra_for_cap(get_last_level("q1", &self.core.bought_vars));
        }
        let res = self.core.create_result(self.strat.name(), &extra);
        best_result(res, self.core.best_fork_res.take())
    }
}
