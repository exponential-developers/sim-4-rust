/**
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

pub use imp::{log10, log2, pow};

/** 10^x */
#[inline]
pub fn exp10(x: f64) -> f64 {
    pow(10., x)
}
