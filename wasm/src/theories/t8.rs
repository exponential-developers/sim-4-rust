/*!
 * Theory 8, ported from sim-3 `src/Theories/T1-T8/T8.ts`.
 *
 * Variable order: c1, c2, c3, c4, c5. rho grows with the speed of a point moving on a chaotic
 * attractor (Lorenz, Chen-Lee or Rossler, chosen by the first milestone).
 */

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

const C1: usize = 0;
const C2: usize = 1;
const C3: usize = 2;
const C4: usize = 3;
const C5: usize = 4;

const CONVERTER: Converter = Converter {
    r9_affected: true,
    tau_factor: 1.,
    mult_exponent: 0.15,
    mult_factor: 0.,
};

/** Bounds of each attractor: [center, min, max] for x, y and z */
const BOUNDS: [[[f64; 3]; 3]; 3] = [
    [[0., -20., 20.], [0., -27., 27.], [24.5, 1., 48.]],
    [[0.5, -23., 24.], [1., -25., 27.], [20.5, 1., 40.]],
    [[1., -20., 22.], [-1.5, -21., 18.], [8., 0., 37.]],
];
/** Starting point of each attractor */
const DEFAULT_STATES: [[f64; 3]; 3] = [[-6., -8., 26.], [-10.6, -4.4, 28.6], [-6., 15., 0.]];
/** Integration step of each attractor */
const DTS: [f64; 3] = [0.02, 0.002, 0.00014];

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum T8Strat {
        T8,
        T8Coast,
        T8noC3,
        T8noC3Coast,
        T8noC5,
        T8noC5Coast,
        T8noC35,
        T8noC35Coast,
        T8Snax,
        T8noC3d,
        T8noC3dCoast,
        T8noC3dSingleMS0,
        T8noC3dSingleMS2,
        T8noC3dSingleMS0Coast,
        T8noC3dSingleMS2Coast,
        T8noC5d,
        T8noC5dCoast,
        T8noC35d,
        T8noC35dCoast,
        T8PlayCoast,
        T8PlaySolarswap,
        T8PlaySolarswapCoast,
    }
}

impl T8Strat {
    fn is_coast(self) -> bool {
        self.name().contains("Coast")
    }

    fn is_single_ms(self) -> bool {
        self.name().contains("T8noC3dSingleMS")
    }

    /** 0 or 2 for the SingleMS strats (the number after "SingleMS") */
    fn single_ms_variant(self) -> usize {
        let name = self.name();
        let digit = name[name.find("SingleMS").unwrap_or(0) + 8..]
            .chars()
            .next();
        digit.and_then(|c| c.to_digit(10)).unwrap_or(0) as usize
    }

    /** Strat of the first pass of coasting strats (T8PlayCoast is its own first pass) */
    fn first_pass(self) -> T8Strat {
        if self == T8Strat::T8PlayCoast {
            self
        } else {
            T8Strat::from_name(&self.name().replace("Coast", "")).unwrap_or(self)
        }
    }

    /** Order in which milestones are assigned (sim-3 `getMilestonePriority`) */
    fn milestone_priority(
        self,
        last_pub_rho: f64,
        max_rho: f64,
        single_ms_point: f64,
    ) -> &'static [usize] {
        use T8Strat::*;
        let count = (last_pub_rho.max(max_rho) / 20.).floor().min(11.);
        match self {
            T8noC3 | T8noC3Coast | T8noC3dCoast | T8noC3d => {
                if max_rho.max(last_pub_rho) < 50. {
                    &[0]
                } else {
                    &[3, 0, 2]
                }
            }
            T8noC3dSingleMS0Coast | T8noC3dSingleMS0 => {
                if max_rho < single_ms_point {
                    &[0, 3, 2]
                } else {
                    &[3, 0, 2]
                }
            }
            T8noC3dSingleMS2Coast | T8noC3dSingleMS2 => {
                if max_rho < single_ms_point {
                    &[2, 0, 3]
                } else {
                    &[3, 0, 2]
                }
            }
            T8noC5 | T8noC5Coast | T8noC5dCoast | T8noC5d => &[0, 2, 1],
            T8noC35 | T8noC35Coast | T8noC35dCoast | T8noC35d => &[2, 0],
            T8PlayCoast => {
                if count < 4. {
                    &[0, 3]
                } else {
                    &[2, 0, 3, 1]
                }
            }
            T8PlaySolarswap | T8PlaySolarswapCoast => {
                if count < 4. {
                    &[0, 3]
                } else {
                    &[0, 2, 3, 1]
                }
            }
            T8 | T8Coast | T8Snax => {
                if count < 3. {
                    &[0]
                } else if count == 3. {
                    &[3]
                } else {
                    &[2, 0, 3, 1]
                }
            }
        }
    }
}

/**
 * Rho at which the SingleMS strats switch milestones: c4 and c5 costs between e45 and e86.
 * [0] is for SingleMS0 (c4 and c5 levels), [1] for SingleMS2 (c4 only).
 */
fn single_ms_points() -> &'static [Vec<f64>; 2] {
    static POINTS: OnceLock<[Vec<f64>; 2]> = OnceLock::new();
    POINTS.get_or_init(|| {
        let l10_100 = l10(100.);
        let l10_2 = l10(2.);
        let c4 = |lvl: usize| l10_100 + (l10_2 * (1.15 * fastmath::log2(5.)) * (lvl as f64 + 53.));
        let c5 = |lvl: usize| l10_100 + (l10_2 * (1.15 * fastmath::log2(7.)) * (lvl as f64 + 44.));
        let mut both: Vec<f64> = (0..=(104 - 53))
            .map(c4)
            .chain((0..=(86 - 44)).map(c5))
            .collect();
        both.sort_by(|a, b| a.total_cmp(b));
        [both, (0..=(104 - 53)).map(c4).collect()]
    })
}

/** Simulates T8 with the given strat (sim-3 `t8` function) */
pub fn simulate_t8(strat: T8Strat, input: &TheoryInput) -> SimResult {
    if !strat.is_single_ms() {
        return get_result(strat, input, 0.);
    }
    let variant = strat.single_ms_variant();
    let rho = input.last_pub_rho;
    let mut result = default_result();
    // Only swap points within e5 of recovery (except M2 before e57)
    for &point in &single_ms_points()[variant / 2] {
        let in_range = point > rho - 5. && (point <= rho || (rho <= 57. && variant == 2));
        if !in_range {
            continue;
        }
        result = best_result(result, Some(get_result(strat, input, point)));
        if result.pub_rho.value < point {
            break;
        }
    }
    result
}

fn get_result(strat: T8Strat, input: &TheoryInput, single_ms_point: f64) -> SimResult {
    if !strat.is_coast() {
        return T8Sim::new(strat, input, single_ms_point).simulate();
    }
    let res1 = T8Sim::new(strat.first_pass(), input, single_ms_point).simulate();
    let mut sim = T8Sim::new(strat, input, single_ms_point);
    sim.vars[C1].set_original_cap(get_last_level("c1", &res1.bought_vars));
    sim.vars[C1].configure_cap(13);
    sim.vars[C3].set_original_cap(get_last_level("c3", &res1.bought_vars));
    sim.vars[C3].configure_cap(4);
    sim.vars[C5].set_original_cap(get_last_level("c5", &res1.bought_vars));
    sim.vars[C5].configure_cap(1);
    sim.simulate()
}

/** log10 constants used by the buying conditions, computed once per sim */
#[derive(Debug, Clone)]
struct T8Consts {
    l10_2_5: f64,
    l10_4: f64,
    /** log10(5 + 0.5 * n) for n = c1 level % 10 */
    l10_5_plus: [f64; 10],
}

#[derive(Debug, Clone)]
struct T8Sim {
    core: TraditionalCore,
    strat: T8Strat,
    coast: bool,
    solarswap: bool,
    vars: Vec<Variable>,
    x: f64,
    y: f64,
    z: f64,
    dx: f64,
    dy: f64,
    dz: f64,
    ms_timer: i32,
    single_ms_point: f64,
    consts: T8Consts,
}

impl T8Sim {
    fn new(strat: T8Strat, input: &TheoryInput, single_ms_point: f64) -> Self {
        let mut core = TraditionalCore::new(TheoryType::T8, true, input, CONVERTER, 8.);
        core.milestone_unlock_steps = 20.;
        core.milestones_max = vec![2, 3, 3, 3];
        // Only T8Snax reads the publication multiplier
        core.track_cur_mult = strat == T8Strat::T8Snax;

        // Costs are built from log10 values the same way sim-3's ExponentialCost parses them
        let log_cost = |v: f64| LogNum { value: v, sign: 1 };
        let log2_inc = |inc: f64| log_cost(l10(2.) * fastmath::exp10(parse_value(inc)));
        let coefficient = log_cost(parse_value(1e2));
        let mut c1 = Variable::new(
            "c1",
            Cost::new(FirstFreeCost::new(ExponentialCost::new(10., 1.5172))),
            Value::new_stepwise(2., 10, 0.),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        c1.buy();

        let vars = vec![
            c1,
            Variable::new(
                "c2",
                Cost::new_exponential(20., 64.),
                Value::new_exponential(2.),
            ),
            Variable::new(
                "c3",
                Cost::Exponential(ExponentialCost::new(
                    coefficient,
                    log2_inc(1.15 * fastmath::log2(3.)),
                )),
                Value::new_exponential(3.),
            ),
            Variable::new(
                "c4",
                Cost::Exponential(ExponentialCost::new(
                    coefficient,
                    log2_inc(1.15 * fastmath::log2(5.)),
                )),
                Value::new_exponential(5.),
            ),
            Variable::new(
                "c5",
                Cost::Exponential(ExponentialCost::new(
                    coefficient,
                    log2_inc(1.15 * fastmath::log2(7.)),
                )),
                Value::new_exponential(7.),
            ),
        ];

        let mut sim = T8Sim {
            core,
            strat,
            coast: strat.is_coast(),
            solarswap: strat.name().contains("Solarswap"),
            vars,
            x: DEFAULT_STATES[0][0],
            y: DEFAULT_STATES[0][1],
            z: DEFAULT_STATES[0][2],
            dx: 0.,
            dy: 0.,
            dz: 0.,
            ms_timer: 0,
            single_ms_point,
            consts: T8Consts {
                l10_2_5: l10(2.5),
                l10_4: l10(4.),
                l10_5_plus: std::array::from_fn(|n| l10(5. + 0.5 * n as f64)),
            },
        };
        sim.update_milestones(0.);
        sim
    }

    /** sim-3 `updateMilestones`: also restarts the attractor when it changes */
    fn update_milestones(&mut self, max_rho: f64) {
        let previous_attractor = self.core.milestones.first().copied();
        let priority =
            self.strat
                .milestone_priority(self.core.last_pub_rho, max_rho, self.single_ms_point);
        self.core.update_milestones(priority);
        if previous_attractor != Some(self.core.milestones[0]) {
            self.reset_attractor();
        }
    }

    fn reset_attractor(&mut self) {
        let [x, y, z] = DEFAULT_STATES[self.core.milestones[0] as usize];
        self.x = x;
        self.y = y;
        self.z = z;
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    /** Derivatives of the attractor at (ix, iy, iz) (sim-3 `dn`) */
    fn dn(&mut self, ix: f64, iy: f64, iz: f64) {
        match self.core.milestones[0] {
            0 => {
                self.dx = 10. * (iy - ix);
                self.dy = ix * (28. - iz) - iy;
                self.dz = ix * iy - (8. * iz) / 3.;
            }
            1 => {
                self.dx = 10. * (40. * (iy - ix));
                self.dy = 10. * (-12. * ix - ix * iz + 28. * iy);
                self.dz = 10. * (ix * iy - 3. * iz);
            }
            2 => {
                self.dx = 500. * (-iy - iz);
                self.dy = 500. * (ix + 0.1 * iy);
                self.dz = 500. * (0.1 + iz * (ix - 14.));
            }
            _ => {}
        }
    }

    fn buying_condition(&self, i: usize) -> bool {
        use T8Strat::*;
        let c = |j: usize| self.cost(j);
        let k = &self.consts;
        let cm = self.core.cur_mult;
        let ok = |j: usize| self.vars[j].should_buy;
        let m24 = c(C2).min(c(C4));
        let c1_d = || c(C1) + 1. < m24;
        match self.strat {
            T8 | T8Coast => match i {
                C1 | C3 | C5 => ok(i),
                _ => true,
            },
            T8noC3 | T8noC3Coast => match i {
                C1 | C5 => ok(i),
                C3 => false,
                _ => true,
            },
            T8noC5 | T8noC5Coast => match i {
                C1 | C3 => ok(i),
                C5 => false,
                _ => true,
            },
            T8noC35 | T8noC35Coast => match i {
                C1 => ok(C1),
                C3 | C5 => false,
                _ => true,
            },
            T8Snax => match i {
                C1 => cm < 1.6,
                C3 | C5 => cm < 2.3,
                _ => true,
            },
            T8noC3d | T8noC3dSingleMS0 | T8noC3dSingleMS2 => match i {
                C1 => c1_d(),
                C3 => false,
                _ => true,
            },
            T8noC3dCoast | T8noC3dSingleMS0Coast | T8noC3dSingleMS2Coast => match i {
                C1 => ok(C1) && c1_d(),
                C3 => false,
                C5 => ok(C5),
                _ => true,
            },
            T8noC5d => match i {
                C1 => c1_d(),
                C5 => false,
                _ => true,
            },
            // c3 follows c5's coasting flag (as in sim-3)
            T8noC5dCoast => match i {
                C1 => ok(C1) && c1_d(),
                C3 => ok(C5),
                C5 => false,
                _ => true,
            },
            // sim-3 uses the non-coasting conditions for this strat
            T8noC35d | T8noC35dCoast => match i {
                C1 => c1_d(),
                C3 | C5 => false,
                _ => true,
            },
            T8PlayCoast => match i {
                C1 => ok(C1) && c(C1) + k.l10_5_plus[(self.vars[C1].level % 10) as usize] < m24,
                C3 => ok(C3) && c(C3) + k.l10_2_5 < m24,
                C5 => ok(C5) && c(C5) + k.l10_4 < m24,
                _ => true,
            },
            T8PlaySolarswap => match i {
                C1 => c(C1) + k.l10_5_plus[(self.vars[C1].level % 10) as usize] < m24,
                C3 | C5 => c(i) + k.l10_2_5 < m24,
                _ => true,
            },
            T8PlaySolarswapCoast => match i {
                C1 => ok(C1) && c(C1) + k.l10_5_plus[(self.vars[C1].level % 10) as usize] < m24,
                C3 | C5 => ok(i) && c(i) + k.l10_2_5 < m24,
                _ => true,
            },
        }
    }

    fn tick(&mut self) {
        let m0 = self.core.milestones[0] as usize;
        let dt = DTS[m0];
        let (x, y, z) = (self.x, self.y, self.z);
        self.dn(x, y, z);

        let midpointx = self.x + self.dx * 0.5 * dt;
        let midpointy = self.y + self.dy * 0.5 * dt;
        let midpointz = self.z + self.dz * 0.5 * dt;

        self.dn(midpointx, midpointy, midpointz);

        self.x += self.dx * dt;
        self.y += self.dy * dt;
        self.z += self.dz * dt;

        let (x, y, z) = (self.x, self.y, self.z);
        self.dn(x, y, z);

        let bounds = |axis: usize| {
            let b = BOUNDS[m0][axis];
            ((b[1] - b[0]) * 5. + b[0], (b[2] - b[0]) * 5. + b[0])
        };
        let (xlower, xupper) = bounds(0);
        let (ylower, yupper) = bounds(1);
        let (zlower, zupper) = bounds(2);

        if self.x < xlower
            || self.x > xupper
            || self.y < ylower
            || self.y > yupper
            || self.z < zlower
            || self.z > zupper
        {
            self.reset_attractor();
        }

        let (x, y, z) = (self.x, self.y, self.z);
        self.dn(x, y, z);

        self.ms_timer += 1;
        if self.ms_timer == 335 && self.solarswap {
            self.reset_attractor();
            self.ms_timer = 0;
        }

        let ms = &self.core.milestones;
        let vc3 = self.val(C3) * (1. + 0.05 * ms[1] as f64);
        let vc4 = self.val(C4) * (1. + 0.05 * ms[2] as f64);
        let vc5 = self.val(C5) * (1. + 0.05 * ms[3] as f64);

        let dx2_term = vc3 + l10(self.dx * self.dx);
        let dy2_term = vc4 + l10(self.dy * self.dy);
        let dz2_term = vc5 + l10(self.dz * self.dz);

        let rhodot = l10(self.core.dt)
            + self.core.tot_mult
            + self.val(C1)
            + self.val(C2)
            + add_log(add_log(dx2_term, dy2_term), dz2_term) / 2.
            - 2.;
        self.core.rho += LogNum {
            value: rhodot,
            sign: 1,
        };
    }

    fn buy_variables(&mut self) {
        // T8 has no onAnyVariablePurchased step, so whether something was bought is not tracked
        for i in (0..self.vars.len()).rev() {
            while self.core.rho.value > self.cost(i) && self.buying_condition(i) {
                self.core.record_purchase(&self.vars[i]);
                let cost = self.vars[i].cost;
                self.core.rho -= cost;
                self.vars[i].buy();
                self.on_variable_purchased(i);
            }
        }
    }

    fn on_variable_purchased(&mut self, id: usize) {
        if matches!(id, C1 | C3 | C5) && self.coast {
            let v = &mut self.vars[id];
            if v.should_buy && v.coasting_cap_reached() {
                if v.level as f64 == v.original_cap {
                    // Specific condition for t8 solarswap coast, the strat goes crazy without this
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
        // sim-3 builds the fork with its constructor, which computes milestones from the last pub
        // rho only (max rho and the SingleMS point are still 0 then); copyFrom does not copy
        // milestones, but does copy the attractor state.
        let priority = fork
            .strat
            .milestone_priority(fork.core.last_pub_rho, 0., 0.);
        fork.core
            .set_milestones_at(fork.core.last_pub_rho.max(0.), priority);
        fork.vars[id].stop_buying();
        let res = fork.simulate();
        self.core.best_fork_res = Some(best_result(res, self.core.best_fork_res.take()));
    }

    fn simulate(&mut self) -> SimResult {
        while !self.core.end_simulation() {
            self.tick();
            self.core.update_sim_status();
            if self.core.last_pub_rho < 220. {
                self.update_milestones(self.core.max_rho);
            }
            self.buy_variables();
            for id in [C1, C3, C5] {
                if self.vars[id].should_fork {
                    self.do_fork_variable(id);
                }
            }
            self.core.collect_pub_table();
        }

        // sim-3 reads the levels before removing the purchases made after the publication point
        let name = self.strat.name();
        let bv = &self.core.bought_vars;
        let mut extra = String::new();
        if name.contains("SingleMS") {
            extra += &format!(" {}", log_to_exp(self.single_ms_point, 2));
        }
        if self.coast {
            extra += &self.vars[C1].prepare_extra_for_cap(get_last_level("c1", bv));
            if !name.contains("noC3") && !name.contains("noC35") {
                extra += &self.vars[C3].prepare_extra_for_cap(get_last_level("c3", bv));
            }
            if !name.contains("noC5") && !name.contains("noC35") {
                extra += &self.vars[C5].prepare_extra_for_cap(get_last_level("c5", bv));
            }
        }
        self.core.trim_bought_vars();
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
    fn single_ms_points_cover_the_c4_and_c5_costs() {
        let points = single_ms_points();
        // c4 levels 53..=104, and c5 levels 44..=86 for SingleMS0
        assert_eq!(points[0].len(), 52 + 43);
        assert_eq!(points[1].len(), 52);
        assert!(points[0].windows(2).all(|w| w[0] <= w[1]));
        assert!(points[1].windows(2).all(|w| w[0] < w[1]));
        // about e44.6 to e85.6 (sim-3 calls it e45 to e86)
        assert!(points[0][0] > 44. && *points[0].last().unwrap() < 86.);
        assert_eq!(T8Strat::T8noC3dSingleMS0Coast.single_ms_variant(), 0);
        assert_eq!(T8Strat::T8noC3dSingleMS2.single_ms_variant(), 2);
    }

    #[test]
    fn single_ms_strats_try_the_swap_points_near_recovery() {
        let res = simulate_t8(T8Strat::T8noC3dSingleMS2, &input(52., 60));
        assert!(res.strat.starts_with("T8noC3dSingleMS2 "), "{}", res.strat);
        // no swap point within e5 of the last pub: nothing to simulate
        let res = simulate_t8(T8Strat::T8noC3dSingleMS0, &input(200., 60));
        assert_eq!(res.tau_h, 0.);
    }

    #[test]
    fn attractor_restarts_when_the_milestone_changes() {
        let mut sim = T8Sim::new(T8Strat::T8, &input(5., 30), 0.);
        assert_eq!(sim.core.milestones[0], 0);
        for _ in 0..50 {
            sim.tick();
        }
        assert_ne!(
            (sim.x, sim.y, sim.z),
            (
                DEFAULT_STATES[0][0],
                DEFAULT_STATES[0][1],
                DEFAULT_STATES[0][2]
            )
        );
        // a new publication point with the Chen-Lee attractor
        let sim = T8Sim::new(T8Strat::T8noC5, &input(60., 30), 0.);
        // three milestones: attractor (max 2), then c3 term (index 2)
        assert_eq!(sim.core.milestones, [2, 0, 1, 0]);
        assert_eq!(
            (sim.x, sim.y, sim.z),
            (
                DEFAULT_STATES[2][0],
                DEFAULT_STATES[2][1],
                DEFAULT_STATES[2][2]
            )
        );
    }

    #[test]
    fn coasting_strats_report_levels_like_sim3() {
        let res = simulate_t8(T8Strat::T8noC35Coast, &input(100., 80));
        assert!(
            res.strat.contains(" c1: ")
                && !res.strat.contains(" c3: ")
                && !res.strat.contains(" c5: "),
            "{}",
            res.strat
        );
        let res = simulate_t8(T8Strat::T8Coast, &input(100., 80));
        assert!(
            res.strat.contains(" c1: ")
                && res.strat.contains(" c3: ")
                && res.strat.contains(" c5: "),
            "{}",
            res.strat
        );
        assert_eq!(T8Strat::T8PlayCoast.first_pass(), T8Strat::T8PlayCoast);
        assert_eq!(
            T8Strat::T8noC3dSingleMS0Coast.first_pass(),
            T8Strat::T8noC3dSingleMS0
        );
    }

    #[test]
    fn snax_is_the_only_strat_reading_the_publication_multiplier() {
        assert!(
            T8Sim::new(T8Strat::T8Snax, &input(100., 80), 0.)
                .core
                .track_cur_mult
        );
        assert!(
            !T8Sim::new(T8Strat::T8, &input(100., 80), 0.)
                .core
                .track_cur_mult
        );
    }
}
