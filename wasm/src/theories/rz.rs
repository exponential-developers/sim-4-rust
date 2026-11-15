/*!
 * Theory RZ (Riemann zeta), ported from sim-3 `src/Theories/CTs/RZ.ts` and its helper
 * `helpers/RZ.ts` (the zeta function).
 *
 * Variable order: c1, c2, b, w1, w2, w3. w1 to w3 are bought with delta, a second currency.
 *
 * The "BH" strats stop time at a zero of the zeta function (a "black hole"): the function at the
 * top simulates one publication for each zero in a list and keeps the best.
 */

use std::cell::RefCell;
use std::f64::consts::{E, PI};
use std::sync::OnceLock;

use crate::api::config::TheoryType;
use crate::theories::pubtable::js_to_fixed;
use crate::theories::traditional::*;
use crate::utils::{
    cost::{Cost, CostTrait, ExponentialCost, FirstFreeCost, StepwiseCost},
    fastmath,
    lognum::{self, LogNum},
    result::SimResult,
    value::Value,
    variable::Variable,
};

const C1: usize = 0;
const C2: usize = 1;
const B: usize = 2;
const W1: usize = 3;
const W2: usize = 4;
const W3: usize = 5;

/** Rho at which each milestone is unlocked */
const MILESTONE_UNLOCKS: [f64; 6] = [25., 50., 125., 250., 400., 600.];

const CONVERTER: Converter = Converter {
    r9_affected: false,
    tau_factor: 0.4,
    mult_exponent: 21.02 / 40.,
    mult_factor: std::f64::consts::LOG10_2,
};

/** How fast t moves per tick (sim-3 `resolution`) */
const RESOLUTION: f64 = 4.;
/** Exponent of c1 by the first milestone */
const C1_EXP: [f64; 4] = [1., 1.14, 1.21, 1.25];

const BLACK_HOLE_ERROR: &str =
    "Black Hole algorithm did not converge.\nThis is likely due to precision issues at large t.";

strat_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum RzStrat {
        RZ,
        RZd,
        RZBH,
        RZBHLong,
        RZdBHLong,
        RZdBH,
        RZSpiralswap,
        RZMS,
        RZdMS,
        RZdBHRewind,
    }
}

impl RzStrat {
    /** The strats with black holes */
    fn is_bh(self) -> bool {
        matches!(
            self,
            RzStrat::RZBH
                | RzStrat::RZdBH
                | RzStrat::RZBHLong
                | RzStrat::RZdBHLong
                | RzStrat::RZdBHRewind
        )
    }

    /** Active strats ("RZd...", and the spiralswap one) */
    fn is_active(self) -> bool {
        matches!(
            self,
            RzStrat::RZd
                | RzStrat::RZdBH
                | RzStrat::RZdBHLong
                | RzStrat::RZdBHRewind
                | RzStrat::RZSpiralswap
                | RzStrat::RZdMS
        )
    }

    fn name_starts_with_rzd(self) -> bool {
        self.name().starts_with("RZd")
    }

    fn is_ms(self) -> bool {
        self.name().contains("MS")
    }
}

// ---- The zeta function (sim-3 `helpers/RZ.ts`) ----

// The coefficients are copied as sim-3 writes them (more digits than a double holds)
#[allow(clippy::excessive_precision)]
mod tables {
    pub const ZETA_01_TABLE: [[f64; 2]; 41] = [
        [-1.4603545088095868, 0.0],
        [-1.4553643660270397, -0.097816768303847834],
        [-1.4405420816461549, -0.19415203999960912],
        [-1.4163212212231056, -0.28759676077859003],
        [-1.3833896356482762, -0.37687944704548237],
        [-1.342642133546631, -0.46091792561979039],
        [-1.2951228211781993, -0.53885377540755575],
        [-1.241963631033884, -0.61006813708679553],
        [-1.1843251208316332, -0.67417998953208147],
        [-1.1233443487784422, -0.73102985025790079],
        [-1.0600929156957051, -0.78065292187264657],
        [-0.99554650742447182, -0.82324597632456875],
        [-0.93056577332974644, -0.85913190352918178],
        [-0.86588730259376534, -0.88872508711130638],
        [-0.80212284363487529, -0.91249984322356881],
        [-0.73976469567777448, -0.93096325430469185],
        [-0.679195280748696, -0.94463296464946644],
        [-0.62069916587171792, -0.954019930248939],
        [-0.56447615104191817, -0.95961573448940385],
        [-0.5106543967354793, -0.96188386780424429],
        [-0.45930289034601818, -0.96125428450587913],
        [-0.41044282155026063, -0.95812055392531381],
        [-0.36405764581325084, -0.95283898111582577],
        [-0.32010176657189976, -0.94572915808929037],
        [-0.27850786866599236, -0.93707550120555738],
        [-0.23919299859739693, -0.92712942212746241],
        [-0.20206352115099815, -0.91611186212496687],
        [-0.167019095423191, -0.90421598956695581],
        [-0.13395581328989362, -0.89160991763812381],
        [-0.10276863503870383, -0.87843934448552852],
        [-0.073353244053944222, -0.86483005263542623],
        [-0.045607427657960491, -0.850890230359152],
        [-0.019432076150895955, -0.836712596410336],
        [0.0052681222316752355, -0.82237632273994077],
        [0.028584225755178324, -0.80794875873014349],
        [0.050602769823360656, -0.7934869662472297],
        [0.071405640533511838, -0.77903907825261309],
        [0.091070056261173163, -0.76464549549431216],
        [0.10966862939766708, -0.750339936434268],
        [0.12726948615366909, -0.73615035542727014],
        [0.14393642707718907, -0.722099743531673],
    ];

    /** Coefficients and exponents of sim-3 `C(0, z)` */
    pub const C0_COEFFS: [(f64, f64); 22] = [
        (0.38268343236508977173, 0.),
        (0.43724046807752044936, 2.),
        (0.13237657548034352332, 4.),
        (-0.01360502604767418865, 6.),
        (-0.01356762197010358089, 8.),
        (-0.00162372532314446528, 10.),
        (0.00029705353733379691, 12.),
        (0.00007943300879521470, 14.),
        (0.00000046556124614505, 16.),
        (-0.00000143272516309551, 18.),
        (-0.00000010354847112313, 20.),
        (0.00000001235792708386, 22.),
        (0.00000000178810838580, 24.),
        (-0.00000000003391414390, 26.),
        (-0.00000000001632663390, 28.),
        (-0.00000000000037851093, 30.),
        (0.00000000000009327423, 32.),
        (0.00000000000000522184, 34.),
        (-0.00000000000000033507, 36.),
        (-0.00000000000000003412, 38.),
        (0.00000000000000000058, 40.),
        (0.00000000000000000015, 42.),
    ];

    /** Coefficients and exponents of sim-3 `C(1, z)` */
    pub const C1_COEFFS: [(f64, f64); 23] = [
        (-0.02682510262837534703, 1.),
        (0.01378477342635185305, 3.),
        (0.03849125048223508223, 5.),
        (0.00987106629906207647, 7.),
        (-0.00331075976085840433, 9.),
        (-0.00146478085779541508, 11.),
        (-0.00001320794062487696, 13.),
        (0.00005922748701847141, 15.),
        (0.00000598024258537345, 17.),
        (-0.00000096413224561698, 19.),
        (-0.00000018334733722714, 21.),
        (0.00000000446708756272, 23.),
        (0.00000000270963508218, 25.),
        (0.00000000007785288654, 27.),
        (-0.00000000002343762601, 29.),
        (-0.00000000000158301728, 31.),
        (0.00000000000012119942, 33.),
        (0.00000000000001458378, 35.),
        (-0.00000000000000028786, 37.),
        (-0.00000000000000008663, 39.),
        (-0.00000000000000000084, 41.),
        (0.00000000000000000036, 43.),
        (0.00000000000000000001, 45.),
    ];

    /** Coefficients and exponents of sim-3 `C(2, z)` */
    pub const C2_COEFFS: [(f64, f64); 24] = [
        (0.00518854283029316849, 0.),
        (0.00030946583880634746, 2.),
        (-0.01133594107822937338, 4.),
        (0.00223304574195814477, 6.),
        (0.00519663740886233021, 8.),
        (0.00034399144076208337, 10.),
        (-0.00059106484274705828, 12.),
        (-0.00010229972547935857, 14.),
        (0.00002088839221699276, 16.),
        (0.00000592766549309654, 18.),
        (-0.00000016423838362436, 20.),
        (-0.00000015161199700941, 22.),
        (-0.00000000590780369821, 24.),
        (0.00000000209115148595, 26.),
        (0.00000000017815649583, 28.),
        (-0.00000000001616407246, 30.),
        (-0.00000000000238069625, 32.),
        (0.00000000000005398265, 34.),
        (0.00000000000001975014, 36.),
        (0.00000000000000023333, 38.),
        (-0.00000000000000011188, 40.),
        (-0.00000000000000000416, 42.),
        (0.00000000000000000044, 44.),
        (0.00000000000000000003, 46.),
    ];

    /** Coefficients and exponents of sim-3 `C(3, z)` */
    pub const C3_COEFFS: [(f64, f64); 24] = [
        (-0.00133971609071945690, 1.),
        (0.00374421513637939370, 3.),
        (-0.00133031789193214681, 5.),
        (-0.00226546607654717871, 7.),
        (0.00095484999985067304, 9.),
        (0.00060100384589636039, 11.),
        (-0.00010128858286776622, 13.),
        (-0.00006865733449299826, 15.),
        (0.00000059853667915386, 17.),
        (0.00000333165985123995, 19.),
        (0.00000021919289102435, 21.),
        (-0.00000007890884245681, 23.),
        (-0.00000000941468508130, 25.),
        (0.00000000095701162109, 27.),
        (0.00000000018763137453, 29.),
        (-0.00000000000443783768, 31.),
        (-0.00000000000224267385, 33.),
        (-0.00000000000003627687, 35.),
        (0.00000000000001763981, 37.),
        (0.00000000000000079608, 39.),
        (-0.00000000000000009420, 41.),
        (-0.00000000000000000713, 43.),
        (0.00000000000000000033, 45.),
        (0.00000000000000000004, 47.),
    ];

    /** Coefficients and exponents of sim-3 `C(4, z)` */
    pub const C4_COEFFS: [(f64, f64); 25] = [
        (0.00046483389361763382, 0.),
        (-0.00100566073653404708, 2.),
        (0.00024044856573725793, 4.),
        (0.00102830861497023219, 6.),
        (-0.00076578610717556442, 8.),
        (-0.00020365286803084818, 10.),
        (0.00023212290491068728, 12.),
        (0.00003260214424386520, 14.),
        (-0.00002557906251794953, 16.),
        (-0.00000410746443891574, 18.),
        (0.00000117811136403713, 20.),
        (0.00000024456561422485, 22.),
        (-0.00000002391582476734, 24.),
        (-0.00000000750521420704, 26.),
        (0.00000000013312279416, 28.),
        (0.00000000013440626754, 30.),
        (0.00000000000351377004, 32.),
        (-0.00000000000151915445, 34.),
        (-0.00000000000008915418, 36.),
        (0.00000000000001119589, 38.),
        (0.00000000000000105160, 40.),
        (-0.00000000000000005179, 42.),
        (-0.00000000000000000807, 44.),
        (0.00000000000000000011, 46.),
        (0.00000000000000000004, 48.),
    ];
}
use tables::*;

type Complex = [f64; 3];

/** Values the zeta function remembers by tick (sim-3 `lookups`); the time of a tick is the same in every sim */
struct Lookups {
    zeta: Vec<Option<Complex>>,
    deriv: Vec<Option<Complex>>,
    prev_dt: f64,
    prev_ddt: f64,
}

thread_local! {
    static LOOKUPS: RefCell<Lookups> = const {
        RefCell::new(Lookups {
            zeta: Vec::new(),
            deriv: Vec::new(),
            prev_dt: 1.5,
            prev_ddt: 1.0001,
        })
    };
    /** log(j) and sqrt(j), for j = 0, 1, 2, ... */
    static LOG_SQRT: RefCell<Vec<(f64, f64)>> = const { RefCell::new(Vec::new()) };
}

fn interpolate(t: f64) -> f64 {
    let v1 = t * t;
    let v2 = 1. - (1. - t) * (1. - t);
    v1 * (1. - t) + v2 * t
}

fn zeta_small(t: f64) -> Complex {
    let full_index = t * (ZETA_01_TABLE.len() - 1) as f64;
    let index = full_index.floor();
    let offset = full_index - index;
    let index = index as usize;
    let re = ZETA_01_TABLE[index][0] * (1. - offset) + ZETA_01_TABLE[index + 1][0] * offset;
    let im = ZETA_01_TABLE[index][1] * (1. - offset) + ZETA_01_TABLE[index + 1][1] * offset;
    // Minus sign for the last element because of theta polarity
    [re, im, -(re * re + im * im).sqrt()]
}

fn even(n: f64) -> f64 {
    if n % 2. == 0. { 1. } else { -1. }
}

fn theta(t: f64) -> f64 {
    t / 2. * fastmath::ln(t / 2. / PI) - t / 2. - PI / 8. + 1. / 48. / t + 7. / 5760. / t / t / t
}

/** sim-3 `C(n, z)` */
fn c_poly(n: i32, z: f64) -> f64 {
    let terms: &[(f64, f64)] = match n {
        0 => &C0_COEFFS,
        1 => &C1_COEFFS,
        2 => &C2_COEFFS,
        3 => &C3_COEFFS,
        _ => &C4_COEFFS,
    };
    let mut sum = 0.;
    for (i, &(coef, exponent)) in terms.iter().enumerate() {
        let term = coef * fastmath::pow(z, exponent);
        sum = if i == 0 { term } else { sum + term };
    }
    sum
}

fn riemann_siegel_zeta(t: f64, n: i32) -> Complex {
    let mut z = 0.;
    let mut r = 0.;
    let full_n = (t / (2. * PI)).sqrt();
    let big_n = full_n.floor();
    let p = full_n - big_n;
    let th = theta(t);

    let count = big_n as usize;
    LOG_SQRT.with(|cache| {
        let mut cache = cache.borrow_mut();
        while cache.len() <= count {
            let j = cache.len() as f64;
            cache.push((fastmath::ln(j), j.sqrt()));
        }
        for &(log_j, sqrt_j) in &cache[1..=count] {
            z += fastmath::cos(th - t * log_j) / sqrt_j;
        }
    });
    z *= 2.;

    let tpot = 2. * PI / t;
    for k in 0..=n {
        r += c_poly(k, 2. * p - 1.) * fastmath::pow(tpot, k as f64 * 0.5);
    }
    r *= even(big_n - 1.) * fastmath::pow(tpot, 0.25);

    z += r;
    [z * fastmath::cos(th), -z * fastmath::sin(th), z]
}

/** sim-3 `zeta`: the zeta function at `big_t`; sims on the regular time grid remember it by tick */
fn zeta(big_t: f64, ticks: u64, off_grid: bool, derivative: bool) -> Complex {
    if !off_grid {
        let cached = LOOKUPS.with(|l| {
            let l = l.borrow();
            let cache = if derivative { &l.deriv } else { &l.zeta };
            cache.get(ticks as usize).copied().flatten()
        });
        if let Some(z) = cached {
            return z;
        }
    }
    let t = big_t.abs();
    let mut z: Complex;
    if t >= 1. {
        z = riemann_siegel_zeta(t, 2);
    } else if t < 0.1 {
        z = zeta_small(t);
    } else {
        let offset = interpolate((t - 0.1) * 10. / 9.);
        let a = zeta_small(t);
        let b = riemann_siegel_zeta(t, 1);
        z = [
            a[0] * (1. - offset) + b[0] * offset,
            a[1] * (1. - offset) + b[1] * offset,
            0.,
        ];
        z[2] = -(z[0] * z[0] + z[1] * z[1]).sqrt();
    }
    if big_t < 0. {
        z[1] = -z[1];
    }

    if !off_grid {
        LOOKUPS.with(|l| {
            let mut l = l.borrow_mut();
            let cache = if derivative {
                &mut l.deriv
            } else {
                &mut l.zeta
            };
            if cache.len() <= ticks as usize {
                cache.resize(ticks as usize + 1, None);
            }
            cache[ticks as usize] = Some(z);
        });
    }
    z
}

// ---- The theory ----

/** Cost of b (sim-3 `VariableBcost`): log10 costs 15, 45, 360, 810, 1050 and 1200 */
#[derive(Debug, Clone, Copy)]
struct BCost;

impl CostTrait for BCost {
    fn get_cost_to(&self, level: i32) -> LogNum {
        const COSTS: [f64; 6] = [15., 45., 360., 810., 1050., 1200.];
        if level <= 0 {
            return lognum::ZERO;
        }
        // there is no cost after the last level (sim-3: undefined)
        LogNum {
            value: COSTS.get(level as usize - 1).copied().unwrap_or(f64::NAN),
            sign: 1,
        }
    }

    fn get_total_cost_to(&self, level: i32) -> LogNum {
        (1..=level).fold(lognum::ZERO, |sum, l| sum + self.get_cost_to(l))
    }
}

/** The data of zeros to try (sim-3 `RZgoodzeros.json`) */
#[derive(serde::Deserialize)]
struct Boundary {
    #[serde(rename = "toRho")]
    to_rho: f64,
    from: f64,
    to: f64,
}

#[derive(serde::Deserialize)]
struct GoodZeros {
    #[serde(rename = "genericZeros")]
    generic_zeros: Vec<f64>,
    #[serde(rename = "longZeros")]
    long_zeros: Vec<f64>,
    #[serde(rename = "rzSpecificZeros")]
    rz_specific_zeros: Vec<f64>,
    #[serde(rename = "rzdSpecificZeros")]
    rzd_specific_zeros: Vec<f64>,
    #[serde(rename = "rzRewind")]
    rz_rewind: Vec<f64>,
    #[serde(rename = "rzRewindBoundaries")]
    rz_rewind_boundaries: Vec<Boundary>,
    #[serde(rename = "rzIdleBHBoundaries")]
    rz_idle_bh_boundaries: Vec<Boundary>,
    #[serde(rename = "rzdIdleBoundaries")]
    rzd_idle_boundaries: Vec<Boundary>,
}

struct Zeros {
    data: GoodZeros,
    rz: Vec<f64>,
    rzd: Vec<f64>,
}

/** sim-3 `mergeSortedLists` */
fn merge_sorted_lists(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut merged = Vec::with_capacity(a.len() + b.len());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] <= b[j] {
            merged.push(a[i]);
            i += 1;
        } else {
            merged.push(b[j]);
            j += 1;
        }
    }
    merged.extend_from_slice(&a[i..]);
    merged.extend_from_slice(&b[j..]);
    merged
}

fn zeros() -> &'static Zeros {
    static ZEROS: OnceLock<Zeros> = OnceLock::new();
    ZEROS.get_or_init(|| {
        let data: GoodZeros =
            serde_json::from_str(include_str!("data/RZgoodzeros.json")).expect("RZ zeros");
        let rz = merge_sorted_lists(&data.generic_zeros, &data.rz_specific_zeros);
        let rzd = merge_sorted_lists(&data.generic_zeros, &data.rzd_specific_zeros);
        Zeros { data, rz, rzd }
    })
}

/** sim-3 `getBestResult(first, second)` where the first may be missing */
fn best_of(first: Option<SimResult>, second: SimResult) -> SimResult {
    best_result(first.unwrap_or_else(default_result), Some(second))
}

/** The first boundary that applies up to this rho */
fn boundary_for(boundaries: &[Boundary], rho: f64) -> Option<&Boundary> {
    boundaries.iter().find(|b| rho <= b.to_rho)
}

/** A boundary on the zeros to try: from and to */
#[derive(Clone, Copy)]
struct ZeroRange {
    from: f64,
    to: f64,
}

impl ZeroRange {
    fn contains(range: Option<ZeroRange>, zero: f64) -> bool {
        range.is_none_or(|r| !(zero < r.from || zero > r.to))
    }
}

/** Simulates RZ with the given strat (sim-3 `rz` function) */
pub fn simulate_rz(strat: RzStrat, input: &TheoryInput) -> SimResult {
    let rho = input.last_pub_rho;
    let name = strat.name();
    let zeros = zeros();
    if strat.is_bh() && !name.contains("Rewind") && rho >= 600. {
        return normal_black_hole(strat, input, zeros);
    }
    if name.contains("BHRewind") && rho >= 600. {
        return rewind_black_hole(strat, input, zeros);
    }

    let mut internal_sim = RzSim::new(strat, input);
    let ret = internal_sim.simulate();
    let c1_level = internal_sim.vars[C1].level as f64;
    let mut internal_sim2 = RzSim::new(strat, input);
    internal_sim2.normal_pub_rho = internal_sim.core.pub_rho;
    internal_sim2.max_c1_level = c1_level - 14.;
    let ret2 = internal_sim2.simulate();
    let mut best = best_result(ret, Some(ret2));

    let mut internal_sim3 = RzSim::new(strat, input);
    internal_sim3.normal_pub_rho = internal_sim.core.pub_rho;
    internal_sim3.max_c1_level = c1_level - 15.;
    let ret3 = internal_sim3.simulate();
    best = best_result(best, Some(ret3));
    best
}

/** The "BH" strats: one publication for each zero that could be a good place to stop time */
fn normal_black_hole(strat: RzStrat, input: &TheoryInput, zeros: &Zeros) -> SimResult {
    let rho = input.last_pub_rho;
    let long = strat.name().contains("Long");
    let rzd = strat.name_starts_with_rzd();
    let zero_list: &[f64] = if long {
        &zeros.data.long_zeros
    } else if rzd {
        &zeros.rzd
    } else {
        &zeros.rz
    };

    let mut sim = RzSim::new(strat, input);
    sim.bh_at_recovery = true;
    let mut best: Option<SimResult> = Some(sim.simulate());

    let boundaries = if rzd {
        &zeros.data.rzd_idle_boundaries
    } else {
        &zeros.data.rz_idle_bh_boundaries
    };
    let mut range = boundary_for(boundaries, rho).map(|b| ZeroRange {
        from: b.from,
        to: b.to,
    });
    if long {
        range = Some(ZeroRange {
            from: 3000.,
            to: 999999999.,
        });
        best = None;
    }
    for &zero in zero_list {
        if !ZeroRange::contains(range, zero) {
            continue;
        }
        let mut internal_sim = RzSim::new(strat, input);
        internal_sim.target_zero = zero;
        let res = internal_sim.simulate();
        best = Some(best_of(best, res));
        let c1_level = internal_sim.vars[C1].level as f64;

        if !rzd {
            // Actual bounds are 14 to 18, saves 23m, but performance is shit.
            for j in 14..=15 {
                let mut internal_sim3 = RzSim::new(strat, input);
                internal_sim3.target_zero = zero;
                internal_sim3.normal_pub_rho = internal_sim.core.pub_rho;
                internal_sim3.max_c1_level = c1_level - j as f64;
                let res3 = internal_sim3.simulate();
                best = Some(best_of(best, res3));
            }
        } else {
            let mut internal_sim2 = RzSim::new(strat, input);
            internal_sim2.target_zero = zero;
            internal_sim2.normal_pub_rho = internal_sim.core.pub_rho;
            let res2 = internal_sim2.simulate();
            best = Some(best_of(best, res2));
        }
    }
    // sim-3 throws "result somehow not set?" without any result
    best.unwrap_or_else(default_result)
}

/** RZdBHRewind: also rewinds time to buy more w1 */
fn rewind_black_hole(strat: RzStrat, input: &TheoryInput, zeros: &Zeros) -> SimResult {
    let rho = input.last_pub_rho;
    let mut sim = RzSim::new(strat, input);
    sim.bh_at_recovery = true;
    let mut best = sim.simulate();
    let mut best2: Option<SimResult> = None;
    let range = boundary_for(&zeros.data.rz_idle_bh_boundaries, rho).map(|b| ZeroRange {
        from: b.from,
        to: b.to,
    });

    // Get best normal zero
    for &zero in &zeros.rzd {
        if !ZeroRange::contains(range, zero) {
            continue;
        }
        let mut internal_sim = RzSim::new(strat, input);
        internal_sim.target_zero = zero;
        let res = internal_sim.simulate();
        best = best_result(best, Some(res));

        let mut internal_sim2 = RzSim::new(strat, input);
        internal_sim2.target_zero = zero;
        internal_sim2.normal_pub_rho = internal_sim.core.pub_rho;
        let res2 = internal_sim2.simulate();
        best = best_result(best, Some(res2));
    }

    let mut max_w1 = f64::INFINITY;
    for buy in best.bought_vars.iter().rev() {
        if buy.var_name == "w1" {
            max_w1 = buy.level as f64;
            break;
        }
    }
    if max_w1 % 6. != 0. {
        max_w1 += 6. - (max_w1 % 6.);
    }

    let rewind_range = boundary_for(&zeros.data.rz_rewind_boundaries, rho).map(|b| ZeroRange {
        from: b.from,
        to: b.to,
    });
    // 18 rarely better so disabled due to performance
    for extra_w1 in [6., 12.] {
        for &rewind_point in &zeros.data.rz_rewind {
            if !ZeroRange::contains(rewind_range, rewind_point) {
                continue;
            }
            let mut internal_sim = RzSim::new(strat, input);
            internal_sim.target_zero = rewind_point;
            internal_sim.max_w1 = max_w1 + extra_w1;
            let res = internal_sim.simulate();
            best2 = Some(best_of(best2, res));

            let mut internal_sim2 = RzSim::new(strat, input);
            internal_sim2.target_zero = rewind_point;
            internal_sim2.max_w1 = max_w1 + extra_w1;
            internal_sim2.normal_pub_rho = internal_sim.core.pub_rho;
            let res2 = internal_sim2.simulate();
            best2 = Some(best_of(best2, res2));
        }
    }
    // sim-3 throws "result somehow not set?" without any result
    best2.unwrap_or_else(default_result)
}

/** log10 constants and other values the sim uses over and over */
#[derive(Debug, Clone)]
struct RzConsts {
    l10_2: f64,
    /** log10(10 + n) for n = c1 level % 8 */
    l10_10_plus: [f64; 8],
    /** log10(4 + 0.5 * n + 0.0001) for n = w1 level % 8 */
    l10_w1_weight: [f64; 8],
}

#[derive(Debug, Clone)]
struct RzSim {
    core: TraditionalCore,
    strat: RzStrat,
    vars: Vec<Variable>,
    /** The second currency, delta (log10) */
    delta: f64,
    t_var: f64,
    best_res: Option<SimResult>,
    // Zeta parameters
    z_term: f64,
    off_grid: bool,
    // Pub parameters
    normal_pub_rho: f64,
    max_c1_level: f64,
    fork_on_w1: bool,
    last_w1: f64,
    // BH parameters
    target_zero: f64,
    blackhole: bool,
    bh_searching_rewind: bool,
    bh_found_zero: bool,
    bh_at_recovery: bool,
    bhz_term: f64,
    bhd_term: f64,
    // RZdBHRewind control parameters
    max_w1: f64,
    bh_rewind_status: i32,
    bh_rewind_t: f64,
    bh_rewind_norm: f64,
    bh_rewind_deriv: f64,
    bh_process_counter: i32,
    /** Set when the black hole search fails (sim-3 throws; the sim then returns an empty result) */
    bh_error: bool,
    consts: RzConsts,
}

impl RzSim {
    fn new(strat: RzStrat, input: &TheoryInput) -> Self {
        let mut core = TraditionalCore::new(TheoryType::RZ, false, input, CONVERTER, 9.);
        core.milestone_unlocks = MILESTONE_UNLOCKS.to_vec();
        core.milestones_max = vec![3, 1, 1, 1];
        // The pub also ends at a large pub multiplier
        core.pub_end_cur_mult = Some(1000.);

        let log = |value: f64| LogNum { value, sign: 1 };
        let mut c1 = Variable::new(
            "c1",
            Cost::new(FirstFreeCost::new(exp_cost_model(
                225.,
                fastmath::pow(2., 0.699),
            ))),
            Value::new_stepwise(2., 8, 0.),
        );
        // FirstFreeCost: level 1 is free (sim-3 buys it in Variable.init)
        c1.buy();
        let vars = vec![
            c1,
            Variable::new(
                "c2",
                exp_cost(1500., fastmath::pow(2., 0.699 * 4.)),
                Value::new_exponential(2.),
            ),
            Variable::new(
                "b",
                Cost::new(BCost),
                Value::new(PlainLinearValue {
                    power: 0.5,
                    offset: 0.,
                }),
            ),
            Variable::new(
                "w1",
                Cost::new(StepwiseCost::new(
                    exp_cost_model(12000., fastmath::pow(100., 1. / 3.)),
                    6,
                )),
                Value::new_stepwise(2., 8, 1.),
            ),
            Variable::new("w2", exp_cost(1e5, 10.), Value::new_exponential(2.)),
            Variable::new(
                "w3",
                // "3.16227766017e600" is too large for a number
                Cost::Exponential(ExponentialCost::new(
                    log(600. + l10(3.16227766017)),
                    log(30.),
                )),
                Value::new_exponential(2.),
            ),
        ];

        let mut sim = RzSim {
            core,
            strat,
            vars,
            delta: f64::NEG_INFINITY,
            t_var: 0.,
            best_res: None,
            z_term: 0.,
            off_grid: false,
            normal_pub_rho: -1.,
            max_c1_level: -1.,
            fork_on_w1: false,
            last_w1: f64::INFINITY,
            target_zero: 999999999.,
            blackhole: false,
            bh_searching_rewind: true,
            bh_found_zero: false,
            bh_at_recovery: false,
            bhz_term: 0.,
            bhd_term: 0.,
            max_w1: f64::INFINITY,
            bh_rewind_status: 0,
            bh_rewind_t: 0.,
            bh_rewind_norm: 0.,
            bh_rewind_deriv: 0.,
            bh_process_counter: 0,
            bh_error: false,
            consts: RzConsts {
                l10_2: l10(2.),
                l10_10_plus: std::array::from_fn(|n| l10(10. + n as f64)),
                l10_w1_weight: std::array::from_fn(|n| l10(4. + 0.5 * n as f64 + 0.0001)),
            },
        };
        sim.update_milestones(0.);
        sim
    }

    /** sim-3 `copy()`: copyFrom doesn't copy the milestones, the pub multiplier or the black hole counter */
    fn copy(&self) -> RzSim {
        let mut fork = self.clone();
        fork.bh_process_counter = 0;
        fork.core.cur_mult = 0.;
        // the constructor has no max rho, z term or last w1 yet
        let priority = fork.milestone_priority(0., 0., f64::INFINITY);
        fork.core
            .set_milestones_at(fork.core.last_pub_rho.max(0.), priority);
        fork
    }

    fn cost(&self, i: usize) -> f64 {
        self.vars[i].cost.value
    }

    fn val(&self, i: usize) -> f64 {
        self.vars[i].value.value
    }

    /** sim-3 `getMilestonePriority` */
    fn milestone_priority(&self, max_rho: f64, z_term: f64, last_w1: f64) -> &'static [usize] {
        const ORIGIN: [usize; 4] = [1, 0, 2, 3];
        const PERIPHERY: [usize; 4] = [1, 2, 0, 3];
        let stage =
            binary_insertion_search(&MILESTONE_UNLOCKS, self.core.last_pub_rho.max(max_rho));
        if self.strat == RzStrat::RZSpiralswap && (2..=4).contains(&stage) {
            return if z_term > 1. { &PERIPHERY } else { &ORIGIN };
        }
        if self.strat.is_ms() && (2..=4).contains(&stage) {
            return if last_w1 != f64::INFINITY {
                &ORIGIN
            } else {
                &PERIPHERY
            };
        }
        &PERIPHERY
    }

    fn update_milestones(&mut self, max_rho: f64) {
        let priority = self.milestone_priority(max_rho, self.z_term, self.last_w1);
        self.core.update_milestones(priority);
    }

    fn buying_condition(&self, i: usize) -> bool {
        let c = |j: usize| self.cost(j);
        let npr = self.normal_pub_rho;
        let level = |j: usize| self.vars[j].level;
        if self.strat.is_active() {
            match i {
                C1 => {
                    let l10_level = self.consts.l10_10_plus[(level(C1) % 8) as usize];
                    if npr != -1. && c(C2) > npr - self.consts.l10_2 {
                        c(C1) <= npr - l10_level
                    } else {
                        let precond = npr == -1. || c(C1) <= npr - l10_level;
                        let level_cond = (level(C1) as f64)
                            < (level(C2) * 4 + if self.core.milestones[0] != 0 { 2 } else { 1 })
                                as f64;
                        precond && level_cond
                    }
                }
                C2 => npr == -1. || c(C2) <= npr - self.consts.l10_2,
                B => self.t_var >= 16.,
                W1 => {
                    self.core.milestones[2] == 0 || {
                        let weight = self.consts.l10_w1_weight[(level(W1) % 8) as usize];
                        c(W1) + weight < c(W2).min(c(W3))
                    }
                }
                _ => true,
            }
        } else {
            match i {
                C1 => {
                    if npr == -1. {
                        return true;
                    }
                    if self.max_c1_level == -1. {
                        // `0.8 * level % 8` is (0.8 * level) % 8 in sim-3
                        return c(C1) <= npr - l10(7.3 + (0.8 * level(C1) as f64) % 8.);
                    }
                    (level(C1) as f64) < self.max_c1_level
                }
                C2 => npr == -1. || c(C2) <= npr - self.consts.l10_2,
                B => self.t_var >= 16.,
                _ => true,
            }
        }
    }

    fn variable_available(&self, i: usize) -> bool {
        let ms = &self.core.milestones;
        match i {
            B => self.vars[B].level < 6,
            W1 => ms[1] == 1,
            W2 | W3 => ms[2] == 1,
            _ => true,
        }
    }

    /** sim-3 `updateBHstatus` */
    fn update_bh_status(&mut self) {
        if self.max_w1 == f64::INFINITY {
            if !self.blackhole {
                // Black hole coasting
                if (!self.bh_at_recovery && self.t_var > self.target_zero)
                    || (self.bh_at_recovery && self.core.max_rho >= self.core.last_pub_rho)
                {
                    if !self.bh_at_recovery {
                        self.t_var = self.target_zero + 0.01;
                    }
                    self.blackhole = true;
                    self.off_grid = true;
                }
            }
        } else if (self.vars[W1].level as f64) < self.max_w1 {
            if self.bh_found_zero {
                if self.bh_rewind_status == 0 {
                    self.blackhole = false;
                    self.bh_searching_rewind = true;
                    self.bh_found_zero = false;
                    self.bh_rewind_status = 1;
                    self.off_grid = true;
                    self.core.dt = 0.15;
                } else {
                    self.bh_rewind_status = 2;
                    self.core.dt = self.bh_rewind_t;
                }
            } else if self.t_var > self.target_zero && !self.blackhole {
                self.t_var = self.target_zero;
                self.blackhole = true;
                self.off_grid = true;
                self.bh_searching_rewind = true;
                self.bh_found_zero = false;
            }
        } else {
            self.blackhole = true;
            self.bh_rewind_status = 3;
        }
    }

    /** sim-3 `bhProcess`: one Newton step towards the nearest zero */
    fn bh_process(&mut self, z_result: Option<Complex>, tmp_z: Option<Complex>) {
        self.off_grid = true;
        self.bh_process_counter += 1;
        if self.bh_process_counter > 500 {
            self.bh_error = true;
            return;
        }

        // This is a method to prevent the non-convergence of the algorithm
        let lock_prevention = if self.bh_process_counter < 100 {
            1.
        } else {
            fastmath::pow(E, -0.001 * (self.bh_process_counter - 100) as f64)
        };

        let ticks = self.core.ticks;
        let z_result = z_result.unwrap_or_else(|| zeta(self.t_var, ticks, self.off_grid, false));
        let tmp_z =
            tmp_z.unwrap_or_else(|| zeta(self.t_var + 1. / 100000., ticks, self.off_grid, true));

        let d_newt = (tmp_z[2] - z_result[2]) * 100000.;
        let bhdt = js_min(js_max(-0.5, -z_result[2] / d_newt), 0.375);

        if self.bh_searching_rewind && self.t_var > 14.5 && bhdt > 0. {
            let srdt = -js_min(0.125 / bhdt, 0.125);
            self.t_var += srdt * lock_prevention;
        } else {
            self.t_var += bhdt * lock_prevention;
            self.bh_searching_rewind = false;
            if bhdt.abs() < 1e-9 {
                self.bh_found_zero = true;
                let z = zeta(self.t_var, ticks, self.off_grid, false);
                let tmp_z = zeta(self.t_var + 1. / 100000., ticks, self.off_grid, true);
                let dr = tmp_z[0] - z[0];
                let di = tmp_z[1] - z[1];
                self.bhd_term = l10((dr * dr + di * di).sqrt() * 100000.);
                self.bhz_term = z[2].abs();
            }
        }
    }

    fn snap_zero(&mut self) {
        while !self.bh_found_zero && !self.bh_error {
            self.bh_process(None, None);
        }
    }

    /** sim-3 RZ's `updateT`: its own time step, which changes while rewinding */
    fn update_t(&mut self) {
        let core = &mut self.core;
        if self.max_w1 != f64::INFINITY
            && (self.vars[W1].level as f64) < self.max_w1
            && self.t_var > self.target_zero - 5.
            && self.bh_rewind_status < 2
        {
            self.off_grid = true;
            core.dt = 0.15;
            core.t += core.dt / 1.5;
            if self.bh_rewind_status == 1 {
                self.bh_rewind_t += core.dt;
            }
        } else if self.bh_rewind_status == 2 {
            core.t += core.dt / 1.5;
        } else {
            core.t += core.dt / 1.5;
            core.dt *= core.ddt;
        }
    }

    fn tick(&mut self) {
        if !self.blackhole {
            let t_dot = 1. / RESOLUTION;
            self.t_var += (self.core.dt * t_dot) / 1.5;
        }

        let dt = self.core.dt;
        let t_term = l10(self.t_var);
        let bonus = l10(dt) + self.core.tot_mult;
        let ms = &self.core.milestones;
        let (ms0, ms1, ms2) = (ms[0], ms[1], ms[2]);
        let w1_term = if ms1 != 0 { self.val(W1) } else { 0. };
        let w2_term = if ms2 != 0 { self.val(W2) } else { 0. };
        let w3_term = if ms2 != 0 { self.val(W3) } else { 0. };
        let c1_term = self.val(C1) * C1_EXP[ms0 as usize];
        let c2_term = self.val(C2);
        let b_term = self.val(B);
        let tot_mult = self.core.tot_mult;
        let ticks = self.core.ticks;

        if self.bh_rewind_status == 2 {
            self.delta = add_log(
                self.delta,
                l10(self.bh_rewind_deriv) + w1_term + w2_term + w3_term + tot_mult,
            );
            self.core.rho += LogNum {
                value: t_term + c1_term + c2_term + w1_term + tot_mult + l10(self.bh_rewind_norm),
                sign: 1,
            };
        } else if !self.bh_found_zero {
            let z = zeta(self.t_var, ticks, self.off_grid, false);
            if ms1 != 0 {
                let tmp_z = zeta(self.t_var + 1. / 100000., ticks, self.off_grid, true);
                let dr = tmp_z[0] - z[0];
                let di = tmp_z[1] - z[1];
                let deriv_term = l10((dr * dr + di * di).sqrt() * 100000.);
                self.delta = add_log(
                    self.delta,
                    deriv_term * b_term + w1_term + w2_term + w3_term + bonus,
                );
                if self.bh_rewind_status == 1 {
                    self.bh_rewind_deriv +=
                        dt * fastmath::pow((dr * dr + di * di).sqrt() * 100000., b_term);
                }

                if self.blackhole {
                    if self.max_w1 == f64::INFINITY || (self.vars[W1].level as f64) >= self.max_w1 {
                        self.snap_zero();
                    } else {
                        self.bh_process(Some(z), Some(tmp_z));
                    }
                    if self.bh_error {
                        return;
                    }
                }
            }
            self.z_term = z[2].abs();
            let two_pow_b = fastmath::pow(2., b_term);
            self.core.rho += LogNum {
                value: t_term + c1_term + c2_term + w1_term + bonus
                    - l10(self.z_term / two_pow_b + 0.01),
                sign: 1,
            };
            if self.bh_rewind_status == 1 {
                self.bh_rewind_norm += dt / (self.z_term / two_pow_b + 0.01);
            }
        } else {
            self.delta = add_log(
                self.delta,
                self.bhd_term * b_term + w1_term + w2_term + w3_term + bonus,
            );
            let two_pow_b = fastmath::pow(2., b_term);
            self.core.rho += LogNum {
                value: t_term + c1_term + c2_term + w1_term + bonus
                    - l10(self.bhz_term / two_pow_b + 0.01),
                sign: 1,
            };
        }
    }

    /** The value of the currency a variable is bought with */
    fn currency(&self, i: usize) -> f64 {
        if i >= W1 {
            self.delta
        } else {
            self.core.rho.value
        }
    }

    fn buy_variables(&mut self) {
        // RZ has no onAnyVariablePurchased step, so whether something was bought is not tracked
        for i in (0..self.vars.len()).rev() {
            while self.currency(i) > self.cost(i)
                && self.buying_condition(i)
                && self.variable_available(i)
            {
                let symbol = if i >= W1 { "delta" } else { "rho" };
                self.core.record_purchase_in(&self.vars[i], symbol);
                let cost = self.vars[i].cost;
                if i >= W1 {
                    self.delta = sub_log(self.delta, cost.value);
                } else {
                    self.core.rho -= cost;
                }
                self.vars[i].buy();
                self.on_variable_purchased(i);
            }
        }
    }

    fn on_variable_purchased(&mut self, id: usize) {
        if id == B && self.bh_rewind_status == 2 {
            // Reset RZdBHRewind status when buying b, to make sure the cache is recomputed with
            // the new b value
            self.bh_rewind_t = 0.;
            self.bh_rewind_norm = 0.;
            self.bh_rewind_deriv = 0.;
            self.blackhole = false;
            self.bh_searching_rewind = true;
            self.bh_found_zero = false;
            self.bh_rewind_status = 1;
        } else if id == W1 && self.strat.is_ms() {
            let rho = self.core.max_rho.max(self.core.last_pub_rho);
            if (50. ..400.).contains(&rho)
                && self.core.max_rho >= self.core.last_pub_rho - 7.
                && self.last_w1 == f64::INFINITY
                && (self.core.last_pub_rho <= 50. || self.core.max_rho < self.core.last_pub_rho)
            {
                self.fork_on_w1 = true;
            }
        }
    }

    /** sim-3 `doForkW1`: also tries with a limit on the levels of w1 */
    fn do_fork_w1(&mut self) {
        let mut fork = self.copy();
        fork.last_w1 = self.vars[W1].level as f64;
        fork.fork_on_w1 = false;
        let res = fork.simulate();
        self.best_res = Some(best_of(self.best_res.take(), res));
        self.last_w1 = f64::INFINITY;
        self.fork_on_w1 = false;
    }

    fn simulate(&mut self) -> SimResult {
        while !self.core.end_simulation() {
            // Prevent lookup table from retrieving values from wrong sim settings
            if self.core.ticks == 0 {
                LOOKUPS.with(|l| {
                    let mut l = l.borrow_mut();
                    if self.core.dt != l.prev_dt || self.core.ddt != l.prev_ddt {
                        l.prev_dt = self.core.dt;
                        l.prev_ddt = self.core.ddt;
                        l.zeta.clear();
                        l.deriv.clear();
                    }
                });
            }
            self.tick();
            if self.bh_error {
                let mut res = default_result();
                res.strat = BLACK_HOLE_ERROR.to_owned();
                return res;
            }
            self.core.update_max_rho();
            self.update_t();
            self.core.update_sim_status_after_t();
            if self.core.last_pub_rho < 600. {
                self.update_milestones(self.core.max_rho);
            }
            if self.core.milestones[3] > 0 && self.strat.is_bh() {
                self.update_bh_status();
            }
            self.buy_variables();
            if self.fork_on_w1 {
                self.do_fork_w1();
            }
            self.core.collect_pub_table();
        }

        let mut extra = String::new();
        if self.strat.name().contains("BH") {
            let t = if self.bh_at_recovery {
                self.t_var
            } else {
                self.target_zero
            };
            extra += &format!(" t={}", js_to_fixed(t, 2));
        }
        if self.last_w1 != f64::INFINITY {
            extra += &format!(" w1: {}", self.last_w1);
        }
        if self.normal_pub_rho != -1. {
            extra += &format!(" c1: {} c2: {}", self.vars[C1].level, self.vars[C2].level);
        }
        if self.max_w1 != f64::INFINITY {
            extra += &format!(" w1: {}", self.max_w1);
        }
        self.core.trim_bought_vars();
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
    fn zeta_matches_known_values() {
        // zeta(1/2) = -1.4603545088095868, and the first zero is at t = 14.134725
        let z0 = zeta(0., 0, true, false);
        assert_eq!(z0, [-1.4603545088095868, 0., -1.4603545088095868]);
        let zero = zeta(14.134725141734693, 0, true, false);
        // the Riemann-Siegel formula is accurate to about 1e-4 this low
        assert!(zero[2].abs() < 1e-3, "{zero:?}");
        // t < 0.1 uses the table, 0.1 to 1 blends both, negative t mirrors the imaginary part
        let small = zeta(0.05, 0, true, false);
        let blend = zeta(0.5, 0, true, false);
        let negative = zeta(-0.5, 0, true, false);
        assert!(small[2] < 0. && blend[2] < 0.);
        assert_eq!((negative[0], negative[1]), (blend[0], -blend[1]));
    }

    #[test]
    fn zeta_is_remembered_by_tick_on_the_regular_grid() {
        let first = zeta(20., 7, false, false);
        // sim-3 returns what it remembered for the tick, whatever time is asked
        assert_eq!(zeta(30., 7, false, false), first);
        assert_ne!(zeta(30., 7, true, false), first);
    }

    #[test]
    fn black_hole_search_gives_up_after_500_steps() {
        let mut sim = RzSim::new(RzStrat::RZBH, &input(100., 60.0 as i64));
        sim.bh_process_counter = 500;
        sim.bh_process(None, None);
        assert!(sim.bh_error);
        sim.bh_error = false;
        // a search close to a zero finds it
        sim.bh_process_counter = 0;
        sim.t_var = 14.13;
        sim.snap_zero();
        assert!(sim.bh_found_zero && (sim.t_var - 14.134725141734693).abs() < 1e-3);
    }

    #[test]
    fn bh_strats_report_their_zero_and_rewind_extends_w1() {
        let res = simulate_rz(RzStrat::RZBH, &input(650., 160));
        assert!(res.strat.starts_with("RZBH t="), "{}", res.strat);
        let res = simulate_rz(RzStrat::RZdBHRewind, &input(650., 160));
        assert!(
            res.tau_h >= 0. && res.strat.contains("RZdBHRewind"),
            "{}",
            res.strat
        );
        // merged lists stay sorted
        assert_eq!(
            merge_sorted_lists(&[1., 3.], &[2., 4., 5.]),
            [1., 2., 3., 4., 5.]
        );
    }

    #[test]
    fn ms_strats_fork_at_the_first_w1() {
        let res = simulate_rz(RzStrat::RZdMS, &input(60., 60));
        assert!(res.tau_h > 0., "{res:?}");
        let sim = RzSim::new(RzStrat::RZSpiralswap, &input(100., 60));
        assert_eq!(
            sim.milestone_priority(100., 2., f64::INFINITY),
            [1, 2, 0, 3]
        );
        assert_eq!(
            sim.milestone_priority(100., 0.5, f64::INFINITY),
            [1, 0, 2, 3]
        );
    }
}
