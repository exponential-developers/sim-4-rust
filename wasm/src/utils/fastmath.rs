/*!
 * log10 / pow used in the simulation hot loop.
 *
 * WebAssembly has no built-in log10 or pow, so Rust's versions are compiled into the module
 * as (slow) software code. In the browser we call the JavaScript engine's `Math.log10`,
 * `Math.log2` and `Math.pow` instead, which are native and also give exactly the same results
 * as sim-3. Natively we use the standard library.
 *
 * Use these for every log computed by a theory, constants included (e.g. log10(5) in a buying
 * condition): Rust may compute a constant differently in the last digit than JavaScript does.
 */

#[cfg(target_arch = "wasm32")]
mod imp {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = Math, js_name = log10)]
        pub fn log10(x: f64) -> f64;
        #[wasm_bindgen(js_namespace = Math, js_name = log2)]
        pub fn log2(x: f64) -> f64;
        #[wasm_bindgen(js_namespace = Math, js_name = pow)]
        pub fn pow(x: f64, y: f64) -> f64;
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    #[inline]
    pub fn log10(x: f64) -> f64 {
        x.log10()
    }
    #[inline]
    pub fn log2(x: f64) -> f64 {
        x.log2()
    }
    #[inline]
    pub fn pow(x: f64, y: f64) -> f64 {
        x.powf(y)
    }
}

pub use imp::{log2, log10, pow};

/** 10^x */
#[inline]
pub fn exp10(x: f64) -> f64 {
    pow(10., x)
}

/**
 * Below this exponent difference, adding or subtracting the smaller of two log10 values changes
 * nothing: for d < -17, 10^d < 1e-17 is under half an ulp of 1 (1.1e-16 above, 5.5e-17 below),
 * so `1 ± 10^d` rounds to exactly 1 and `log10(1)` is exactly 0. Callers can then return
 * `max + 0.` without calling pow and log10 (two calls into JavaScript in the browser) and still
 * get bit-identical results.
 */
pub const NEGLIGIBLE_EXP_DIFF: f64 = -17.;

/** log10(1 + 10^d) for d = min - max <= 0, skipping the calls when the result is exactly 0 */
#[inline]
pub fn log10_1p_exp10(d: f64) -> f64 {
    if d < NEGLIGIBLE_EXP_DIFF {
        0.
    } else {
        log10(1. + exp10(d))
    }
}

/**
 * log10(1 + x) for x >= 0, skipping the call when the result is exactly 0: below 1e-17, x is
 * under half an ulp of 1, so `1 + x` rounds to exactly 1 (see `NEGLIGIBLE_EXP_DIFF`).
 */
#[inline]
pub fn log10_1p(x: f64) -> f64 {
    if x < 1e-17 { 0. } else { log10(1. + x) }
}

/** log10(1 - 10^d) for d = min - max <= 0, skipping the calls when the result is exactly 0 */
#[inline]
pub fn log10_1m_exp10(d: f64) -> f64 {
    if d < NEGLIGIBLE_EXP_DIFF {
        0.
    } else {
        log10(1. - exp10(d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skipping_negligible_differences_gives_the_same_bits() {
        // the full formula gives exactly 0 wherever the shortcut is taken
        let mut d = NEGLIGIBLE_EXP_DIFF - 1e-9;
        while d > -400. {
            assert_eq!(log10(1. + exp10(d)).to_bits(), 0f64.to_bits(), "{d}");
            assert_eq!(log10(1. - exp10(d)).to_bits(), 0f64.to_bits(), "{d}");
            assert_eq!(log10_1p_exp10(d).to_bits(), 0f64.to_bits());
            d -= 0.0137;
        }
        assert_eq!(log10_1p_exp10(f64::NEG_INFINITY), 0.);
        assert_eq!(log10_1m_exp10(f64::NEG_INFINITY), 0.);
        // log10(1 + x) for x below half an ulp of 1 is exactly 0
        for x in [0., 1e-300, 1e-18, 9.99e-18] {
            assert_eq!(log10(1. + x).to_bits(), 0f64.to_bits());
            assert_eq!(log10_1p(x).to_bits(), 0f64.to_bits());
        }
        for x in [1e-17, 1e-10, 0.5, 9.] {
            assert_eq!(log10_1p(x), log10(1. + x));
        }
        // above the threshold the full formula is used
        for d in [-16.9, -3., -0.5, 0.] {
            assert_eq!(log10_1p_exp10(d), log10(1. + exp10(d)));
            assert_eq!(log10_1m_exp10(d).to_bits(), log10(1. - exp10(d)).to_bits());
        }
    }
}
