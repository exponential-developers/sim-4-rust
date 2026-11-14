/*!
 * Theory 2, ported from sim-3 `src/Theories/T1-T8/T2.ts`.
 *
 * Variable order: q1, q2, q3, q4, r1, r2, r3, r4
 */

use crate::api::config::TheoryType;
use crate::theories::traditional::*;
use crate::utils::{
    cost::{Cost, ExponentialCost, FirstFreeCost},
    lognum::LogNum,
    result::SimResult,
    value::Value,
    variable::Variable,
};

const CONVERTER: Converter = Converter {
    r9_affected: true,
    tau_factor: 1.,
    mult_exponent: 0.198,
    // -log10(100)
    mult_factor: -2.,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum T2Strat {
    T2,
    T2MC,
    T2MCAlt,
    T2MCAlt2,
    T2MCAlt3,
    T2Haxolotl,
    T2MS,
    T2QS,
    T2ROI,
    T2ROIMS,
}

impl T2Strat {
    pub const ALL: [T2Strat; 10] = [
        T2Strat::T2,
        T2Strat::T2MC,
        T2Strat::T2MCAlt,
        T2Strat::T2MCAlt2,
        T2Strat::T2MCAlt3,
        T2Strat::T2Haxolotl,
        T2Strat::T2MS,
        T2Strat::T2QS,
        T2Strat::T2ROI,
        T2Strat::T2ROIMS,
    ];

    pub fn name(self) -> &'static str {
        match self {
            T2Strat::T2 => "T2",
            T2Strat::T2MC => "T2MC",
            T2Strat::T2MCAlt => "T2MCAlt",
            T2Strat::T2MCAlt2 => "T2MCAlt2",
            T2Strat::T2MCAlt3 => "T2MCAlt3",
            T2Strat::T2Haxolotl => "T2Haxolotl",
            T2Strat::T2MS => "T2MS",
            T2Strat::T2QS => "T2QS",
            T2Strat::T2ROI => "T2ROI",
            T2Strat::T2ROIMS => "T2ROIMS",
        }
    }

    pub fn from_name(name: &str) -> Option<T2Strat> {
        T2Strat::ALL.into_iter().find(|s| s.name() == name)
    }

    fn is_roi(self) -> bool {
        matches!(self, T2Strat::T2ROI | T2Strat::T2ROIMS)
    }
}

/** Simulates T2 with the given strat (sim-3 `t2` function) */
pub fn simulate_t2(strat: T2Strat, input: &TheoryInput) -> SimResult {
    match strat {
        T2Strat::T2Haxolotl => {
            let target = T2Sim::new(T2Strat::T2MC, input).simulate().pub_rho.value;
            const STOP_POINTS: [[f64; 4]; 3] = [
                [3700., 2650., 1700., 750.],
                [4650., 2900., 2250., 1150.],
                [3500., 2700., 2050., 550.],
            ];
            const C_TO_TRY: [f64; 7] = [1.0, 1.05, 1.1, 1.15, 1.2, 1.25, 1.3];
            const C1_TO_TRY: [f64; 11] =
                [1.0, 1.05, 1.1, 1.15, 1.2, 1.25, 1.3, 1.35, 1.4, 1.45, 1.5];
            let mut best = default_result();
            for c1 in C1_TO_TRY {
                for c2 in C_TO_TRY {
                    for c3 in C_TO_TRY {
                        for stops in STOP_POINTS {
                            let mut sim = T2Sim::new(strat, input);
                            sim.haxolotl = [c1, c2, c3];
                            sim.stops = stops;
                            sim.target_rho = target;
                            let res = sim.simulate();
                            // getBestResult(newRes, bestRes): the newer result wins ties
                            if res.tau_h >= best.tau_h {
                                best = res;
                            }
                        }
                    }
                }
            }
            best
        }
        T2Strat::T2MCAlt2 | T2Strat::T2MCAlt3 => {
            let target = T2Sim::new(T2Strat::T2MC, input).simulate().pub_rho.value;
            let mut sim = T2Sim::new(strat, input);
            if strat == T2Strat::T2MCAlt3 {
                sim.stops = [3700., 2650., 1700., 750.];
            }
            sim.target_rho = target;
            sim.simulate()
        }
        T2Strat::T2ROI | T2Strat::T2ROIMS => {
            let time = T2Sim::new(strat, input).simulate().time;
            let mut sim = T2Sim::new(strat, input);
            sim.time_to_pub = time;
            sim.simulate()
        }
        _ => T2Sim::new(strat, input).simulate(),
    }
}

/** log10 constants, computed once per sim */
#[derive(Debug, Clone)]
struct T2Consts {
    l10_2: f64,
    l10_4: f64,
    l10_6: f64,
    l10_15: f64,
    l10_24: f64,
}

#[derive(Debug, Clone)]
struct T2Sim {
    core: TraditionalCore,
    strat: T2Strat,
    vars: Vec<Variable>,
    /** q1..q4 and r1..r4 (log10 values) */
    q: [f64; 4],
    r: [f64; 4],
    /** Haxolotl cost factors c1, c2, c3 */
    haxolotl: [f64; 3],
    haxolotl_l10: [f64; 3],
    /** -1 when unset */
    target_rho: f64,
    /** stop1, stop2, stop3, stop4 */
    stops: [f64; 4],
    time_to_pub: f64,
    consts: T2Consts,
}

impl T2Sim {
    fn new(strat: T2Strat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::T2, true, input, CONVERTER, 15.);
        core.milestone_unlock_steps = 25.;
        core.milestones_max = vec![2, 2, 3, 3];

        // Costs parsed like sim-3's ExponentialCost (see parse_value)
        let exp_cost = |base: f64, inc: f64| {
            ExponentialCost::new(
                LogNum {
                    value: parse_value(base),
                    sign: 1,
                },
                LogNum {
                    value: parse_value(inc),
                    sign: 1,
                },
            )
        };
        let stepwise = || Value::new_stepwise(2., 10, 0.);

        let mut q1 = Variable::new(
            "q1",
            Cost::new(FirstFreeCost::new(exp_cost(10., 2.))),
            stepwise(),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        q1.buy();

        let vars = vec![
            q1,
            Variable::new("q2", Cost::Exponential(exp_cost(5000., 2.)), stepwise()),
            Variable::new("q3", Cost::Exponential(exp_cost(3e25, 3.)), stepwise()),
            Variable::new("q4", Cost::Exponential(exp_cost(8e50, 4.)), stepwise()),
            Variable::new("r1", Cost::Exponential(exp_cost(2e6, 2.)), stepwise()),
            Variable::new("r2", Cost::Exponential(exp_cost(3e9, 2.)), stepwise()),
            Variable::new("r3", Cost::Exponential(exp_cost(4e25, 3.)), stepwise()),
            Variable::new("r4", Cost::Exponential(exp_cost(5e50, 4.)), stepwise()),
        ];

        let mut sim = T2Sim {
            core,
            strat,
            vars,
            q: [f64::NEG_INFINITY, 0., 0., 0.],
            r: [0., 0., 0., 0.],
            haxolotl: [1., 1., 1.],
            haxolotl_l10: [0., 0., 0.],
            target_rho: -1.,
            stops: [3500., 2700., 2050., 550.],
            time_to_pub: f64::INFINITY,
            consts: T2Consts {
                l10_2: l10(2.),
                l10_4: l10(4.),
                l10_6: l10(6.),
                l10_15: l10(15.),
                l10_24: l10(24.),
            },
        };
        let priority = sim.milestone_priority();
        sim.core.update_milestones(&priority);
        sim
    }

    fn rho(&self) -> f64 {
        self.core.rho.value
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    fn milestone_priority(&self) -> [usize; 4] {
        match self.strat {
            T2Strat::T2MS | T2Strat::T2ROIMS => {
                let tm100 = self.core.t % 100.;
                if tm100 < 10. {
                    [2, 3, 0, 1]
                } else if tm100 < 50. {
                    [0, 1, 2, 3]
                } else if tm100 < 60. {
                    [2, 3, 0, 1]
                } else {
                    [1, 0, 2, 3]
                }
            }
            T2Strat::T2QS => {
                let lpr = self.core.last_pub_rho;
                let mut coast_multi = f64::INFINITY;
                if lpr > 0. {
                    coast_multi = 10.;
                }
                if lpr > 75. {
                    coast_multi = 200.;
                }
                if lpr > 150. {
                    coast_multi = 600.;
                }
                if lpr > 200. {
                    coast_multi = 100.;
                }
                if lpr > 225. {
                    coast_multi = 25.;
                }
                if self.core.cur_mult < coast_multi {
                    [0, 1, 2, 3]
                } else {
                    [2, 3, 0, 1]
                }
            }
            _ => [0, 1, 2, 3],
        }
    }

    /** Highest cost among q1..q(index+1) and r1..r(index+1) (sim-3 `getMax`) */
    fn max_cost(&self, index: usize) -> f64 {
        let mut max = -1f64;
        for j in 0..=index {
            max = js_max(max, js_max(self.cost(j), self.cost(4 + j)));
        }
        max
    }

    fn buying_condition(&self, i: usize) -> bool {
        const MC: [f64; 4] = [4650., 2900., 2250., 1150.];
        const MC_ALT: [f64; 4] = [3500., 2700., 2050., 550.];
        let cur_mult = self.core.cur_mult;
        let k = i % 4;
        match self.strat {
            T2Strat::T2 | T2Strat::T2MS | T2Strat::T2QS | T2Strat::T2ROI | T2Strat::T2ROIMS => true,
            T2Strat::T2MC => cur_mult < MC[k],
            T2Strat::T2MCAlt | T2Strat::T2MCAlt2 => cur_mult < MC_ALT[k],
            T2Strat::T2MCAlt3 => cur_mult < self.stops[k],
            T2Strat::T2Haxolotl => {
                if k == 0 {
                    cur_mult < self.stops[0]
                } else {
                    self.cost(i) + self.haxolotl_l10[k - 1] < self.max_cost(k - 1)
                        && cur_mult < self.stops[k]
                }
            }
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        let ms = &self.core.milestones;
        match i {
            2 => ms[0] > 0,
            3 => ms[0] > 1,
            6 => ms[1] > 0,
            7 => ms[1] > 1,
            _ => true,
        }
    }

    fn tick(&mut self) {
        let logdt = l10(self.core.dt);
        let ms = &self.core.milestones;
        let (m0, m1) = (ms[0], ms[1]);
        let v: [f64; 8] = std::array::from_fn(|i| self.val(i));

        self.q[0] = add_log(self.q[0], v[0] + self.q[1] + logdt);
        self.q[1] = add_log(self.q[1], v[1] + self.q[2] + logdt);
        if m0 > 0 {
            self.q[2] = add_log(self.q[2], v[2] + self.q[3] + logdt);
        }
        if m0 > 1 {
            self.q[3] = add_log(self.q[3], v[3] + logdt);
        }

        self.r[0] = add_log(self.r[0], v[4] + self.r[1] + logdt);
        self.r[1] = add_log(self.r[1], v[5] + self.r[2] + logdt);
        if m1 > 0 {
            self.r[2] = add_log(self.r[2], v[6] + self.r[3] + logdt);
        }
        if m1 > 1 {
            self.r[3] = add_log(self.r[3], v[7] + logdt);
        }

        let ms = &self.core.milestones;
        let rhodot = self.q[0] * (1. + 0.05 * ms[2] as f64)
            + self.r[0] * (1. + 0.05 * ms[3] as f64)
            + self.core.tot_mult
            + logdt;
        self.core.rho += LogNum {
            value: rhodot,
            sign: 1,
        };
    }

    /** Buys variable `i` if affordable (sim-3 `buyVariableROI`'s `buyVariable`) */
    fn buy_one(&mut self, i: usize) -> bool {
        if self.rho() > self.cost(i) {
            self.core.record_purchase(&self.vars[i]);
            let cost = self.vars[i].cost;
            self.core.rho -= cost;
            self.vars[i].buy();
            true
        } else {
            false
        }
    }

    fn buy_variables(&mut self) {
        if self.strat.is_roi() && self.time_to_pub != f64::INFINITY {
            if self.core.t < self.time_to_pub {
                self.buy_variables_roi();
            }
            return;
        }
        for i in (0..self.vars.len()).rev() {
            while self.rho() > self.cost(i)
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

    // ---- ROI strats: buy the variable with the best return on investment until timeToPub ----

    // Integrals of Q1(t) / R1(t) for 2, 3 and 4 active q (or r) terms (sim-3 `ft2Func`, `ft3Func`,
    // `ft4Func`). `d` holds dq1..dq4 (or dr1..dr4) and `q` holds q1..q4 (or r1..r4).

    fn ft2(&self, t: f64, d: &[f64], q: &[f64]) -> f64 {
        let (dq1, dq2, q1, q2) = (d[0], d[1], q[0], q[1]);
        add_log(dq1 + add_log(dq2 + t * 2. - self.consts.l10_2, q2 + t), q1)
    }

    fn ft3(&self, t: f64, d: &[f64], q: &[f64]) -> f64 {
        let (dq1, dq2, dq3, q1, q2, q3) = (d[0], d[1], d[2], q[0], q[1], q[2]);
        let c = &self.consts;
        add_log(
            dq1 + add_log(
                dq2 + add_log(dq3 + t * 3. - c.l10_6, q3 + t * 2. - c.l10_2),
                q2 + t,
            ),
            q1 + t,
        )
    }

    fn ft4(&self, t: f64, d: &[f64], q: &[f64]) -> f64 {
        let (dq1, dq2, dq3, dq4, q1, q2, q3, q4) = (d[0], d[1], d[2], d[3], q[0], q[1], q[2], q[3]);
        let c = &self.consts;
        add_log(
            dq1 + add_log(
                dq2 + add_log(
                    dq3 + add_log(dq4 + t * 4. - c.l10_24, q4 + t * 3. - c.l10_6),
                    q3 + t * 2. - c.l10_2,
                ),
                q2 + t,
            ),
            q1 + t,
        )
    }

    fn rho_dot_time_factor(
        &self,
        t: f64,
        exp: f64,
        milestone_count: i32,
        d: &[f64],
        q: &[f64; 4],
    ) -> f64 {
        let value = match milestone_count {
            0 => self.ft2(t, d, q),
            1 => self.ft3(t, d, q),
            2 => self.ft4(t, d, q),
            _ => f64::NEG_INFINITY,
        };
        value * exp
    }

    /** Simpson's rule on a segment (sim-3 `simp`) */
    fn simp(&self, x0: f64, x1: f64, f0: f64, fm: f64, f1: f64) -> SignedLog {
        let sum = add_log(add_log(f0, self.consts.l10_4 + fm), f1);
        muls(&[
            SignedLog::pos(sum - self.consts.l10_6),
            adds(&[SignedLog::pos(x1), mulm1(SignedLog::pos(x0))]),
        ])
    }

    #[allow(clippy::too_many_arguments)]
    fn recurse(
        &self,
        f: &dyn Fn(f64) -> f64,
        a: f64,
        b: f64,
        fa: f64,
        fm: f64,
        fb: f64,
        whole: SignedLog,
        precision: f64,
        depth: i32,
    ) -> SignedLog {
        let l10_2 = self.consts.l10_2;
        let m = add_log(a, b) - l10_2;
        let lm = add_log(a, m) - l10_2;
        let rm = add_log(m, b) - l10_2;
        let flm = f(lm);
        let frm = f(rm);

        let left = self.simp(a, m, fa, flm, fm);
        let right = self.simp(m, b, fm, frm, fb);
        let delta = adds(&[adds(&[left, right]), mulm1(whole)]);

        if depth <= 0 || delta.value <= l10(15. * precision) {
            return adds(&[adds(&[
                left,
                right,
                SignedLog::pos(delta.value - self.consts.l10_15),
            ])]);
        }

        adds(&[
            self.recurse(f, a, m, fa, flm, fm, left, precision / 2., depth - 1),
            self.recurse(f, m, b, fm, frm, fb, right, precision / 2., depth - 1),
        ])
    }

    /** Adaptive Simpson integration in log space (sim-3 `integrate`) */
    fn integrate(
        &self,
        f: &dyn Fn(f64) -> f64,
        mut a: f64,
        mut b: f64,
        precision: f64,
        max_depth: i32,
    ) -> SignedLog {
        let swap = a > b;
        if swap {
            std::mem::swap(&mut a, &mut b);
        }
        let m = add_log(a, b) - self.consts.l10_2;
        let (fa, fb, fm) = (f(a), f(b), f(m));
        let mut result = self.recurse(
            f,
            a,
            b,
            fa,
            fm,
            fb,
            self.simp(a, b, fa, fm, fb),
            precision,
            max_depth,
        );
        if swap {
            result = mulm1(result);
        }
        result
    }

    /** Integral of the extra rho gained by applying `increase` to the variables (sim-3 `calculateRoi`) */
    fn calculate_roi(&self, increase: &[f64; 8], t0: f64, t1: f64) -> f64 {
        let v: [f64; 8] = std::array::from_fn(|i| self.val(i));
        let d: [f64; 8] = std::array::from_fn(|i| add_log(increase[i], v[i]));
        let ms = &self.core.milestones;
        let q1exp = 1. + 0.05 * ms[2] as f64;
        let r1exp = 1. + 0.05 * ms[3] as f64;
        let (m0, m1) = (ms[0], ms[1]);

        let rho_integral = |t: f64| -> f64 {
            let new_q = self.rho_dot_time_factor(t, q1exp, m0, &d[0..4], &self.q);
            let new_r = self.rho_dot_time_factor(t, r1exp, m1, &d[4..8], &self.r);
            let old_q = self.rho_dot_time_factor(t, q1exp, m0, &v[0..4], &self.q);
            let old_r = self.rho_dot_time_factor(t, r1exp, m1, &v[4..8], &self.r);
            self.core.tot_mult + sub_log(new_q + new_r, old_q + old_r)
        };

        let value = self.integrate(&rho_integral, t0, t1, 1e-1, 1);
        if value.neg {
            f64::NEG_INFINITY
        } else {
            value.value
        }
    }

    fn buy_variables_roi(&mut self) {
        // skip if none of the variables are currently affordable
        if !(0..8).any(|i| self.variable_available(i) && self.cost(i) <= self.rho()) {
            return;
        }

        // buy level 0 upgrades just to be sure that all q and r are being generated
        for i in 0..8 {
            if self.variable_available(i) && self.vars[i].level == 0 {
                self.buy_one(i);
            }
        }

        let t0 = l10(self.core.t);
        let t1 = l10(self.time_to_pub);
        loop {
            let var_increase: [f64; 8] = std::array::from_fn(|i| {
                if !self.variable_available(i) {
                    return f64::NEG_INFINITY;
                }
                let next = self.vars[i]
                    .value_model
                    .compute_from_zero(self.vars[i].level + 1)
                    .value;
                sub_log(next, self.val(i))
            });

            let ms = &self.core.milestones;
            let (m0, m1) = (ms[0], ms[1]);
            let roi_for = |i: usize| {
                let mut inc = [f64::NEG_INFINITY; 8];
                inc[i] = var_increase[i];
                self.calculate_roi(&inc, t0, t1)
            };
            let roi2: [f64; 8] = [
                roi_for(0),
                roi_for(1),
                if m0 > 0 {
                    roi_for(2)
                } else {
                    f64::NEG_INFINITY
                },
                if m0 > 1 {
                    roi_for(3)
                } else {
                    f64::NEG_INFINITY
                },
                roi_for(4),
                roi_for(5),
                if m1 > 0 {
                    roi_for(6)
                } else {
                    f64::NEG_INFINITY
                },
                if m1 > 1 {
                    roi_for(7)
                } else {
                    f64::NEG_INFINITY
                },
            ];
            // rescale: divide by the variable cost and subtract 1
            let roi: [f64; 8] = std::array::from_fn(|i| {
                let v = adds(&[
                    muls(&[SignedLog::pos(roi2[i]), SignedLog::pos(-self.cost(i))]),
                    mulm1(SignedLog::pos(0.)),
                ]);
                if self.vars[i].should_buy && self.variable_available(i) && !v.neg {
                    v.value
                } else {
                    f64::NEG_INFINITY
                }
            });

            // switch (Math.max(...roi)) { case -Infinity; case roi[7]; ... case roi[0] }
            let max = roi.iter().copied().fold(f64::NEG_INFINITY, js_max);
            let mut bought = false;
            if max != f64::NEG_INFINITY
                && let Some(i) = (0..8).rev().find(|&i| roi[i] == max)
                && roi[i] != f64::NEG_INFINITY
            {
                bought = self.buy_one(i);
            }
            if !bought {
                break;
            }
        }
    }

    fn simulate(&mut self) -> SimResult {
        if self.target_rho != -1. {
            self.core.pub_target = Some(self.target_rho);
        }
        self.core.do_sim_end_conditions = self.target_rho == -1.;
        self.haxolotl_l10 = self.haxolotl.map(l10);

        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            if self.core.last_pub_rho < 250. {
                let priority = self.milestone_priority();
                self.core.update_milestones(&priority);
            }
            self.buy_variables();
            self.core.collect_pub_table();
        }
        self.core.trim_bought_vars();

        let mut extra = String::new();
        if matches!(self.strat, T2Strat::T2MCAlt3 | T2Strat::T2Haxolotl) {
            let [s1, s2, s3, s4] = self.stops;
            extra += &format!(" 4:{s4} 3:{s3} 2:{s2} 1:{s1}");
        }
        if self.strat == T2Strat::T2Haxolotl {
            let [c1, c2, c3] = self.haxolotl;
            extra += &format!(" c1:{c1} c2:{c2} c3: {c3}");
        }
        self.core.create_result(self.strat.name(), &extra)
    }
}
