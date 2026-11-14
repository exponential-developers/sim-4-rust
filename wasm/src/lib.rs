#[allow(dead_code)]
pub mod utils;
#[allow(dead_code)]
pub mod api;
#[allow(dead_code, unused_variables)]
mod sim;
pub mod theories;

use std::sync::OnceLock;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use api::{
    query::SimQuery,
    response::SimResponse,
    config::Config
};
use sim::simulate;

/** Holds the configuration on the sim (data.json) */
static CONFIG: OnceLock<Config> = OnceLock::new();
const DEFAULT_ERR: &str = "{\"response_type\": \"failure\", \"data\":\"API Error\"}";

#[derive(Serialize)]
#[serde(tag = "response_type", content = "data", rename_all = "lowercase")]
enum ApiResponse {
    Success(SimResponse),
    Failure(String)
}

/**
 * Sets the config (data.json) for the wasm module
 */
#[wasm_bindgen]
pub fn set_config(config: &str) -> String {
    let cfg: Config = serde_json::from_str(config).unwrap();
    // Only the first call counts (the page calls this once; tests may call it repeatedly)
    let _ = CONFIG.set(cfg);

    format!("Sim config loaded ({} theories)", CONFIG.get().map_or(0, |c| c.theories.len()))
}

fn create_error(msg: &str) -> String {
    serde_json::to_string(&ApiResponse::Failure(msg.to_owned()))
        .unwrap_or(DEFAULT_ERR.to_owned())
}

/**
 * Benchmark/test entry point: simulates one named strat of a ported theory (e.g. "T1", "T1Ratio")
 * with default settings (dt 1.5, ddt 1.0001). `rho` is the log10 of the last publication rho.
 * Returns a JSON summary of the result.
 */
#[wasm_bindgen]
pub fn sim_strat(theory: &str, strat: &str, sigma: u32, rho: f64) -> String {
    use theories::traditional::{SimParams, TheoryInput};
    let Ok(theory) = serde_json::from_value::<api::config::TheoryType>(serde_json::Value::from(theory)) else {
        return create_error(&format!("Unknown theory {theory}"));
    };
    let input = TheoryInput { sigma: sigma as i64, last_pub_rho: rho, rho_cap: f64::INFINITY, params: SimParams::default() };
    let res = match sim::run_strat(theory, strat, &input) {
        Ok(res) => res,
        Err(err) => return create_error(&err),
    };
    serde_json::json!({
        "strat": res.strat,
        "tauH": res.tau_h,
        "pubRho": res.pub_rho.value,
        "time": res.time,
        "boughtVars": res.bought_vars.len(),
    }).to_string()
}

/**
 * Main API function of the wasm module: runs a JSON sim query and returns a JSON response
 */
#[wasm_bindgen]
pub fn run_query(input: &str) -> String {
    run_query_impl(input, &|_| {})
}

/**
 * Same as `run_query`, calling `progress(message)` with status text such as
 * "Simulating 1e85/1e150" during long simulations (chain, steps, all).
 */
#[wasm_bindgen]
pub fn run_query_with_progress(input: &str, progress: &js_sys::Function) -> String {
    run_query_impl(input, &|msg: &str| {
        let _ = progress.call1(&JsValue::NULL, &JsValue::from_str(msg));
    })
}

fn run_query_impl(input: &str, progress: sim::Progress) -> String {
    let query = match serde_json::from_str::<SimQuery>(input) {
        Ok(query) => query,
        Err(err) => return create_error(&format!("Error parsing query: {err}"))
    };

    let res = match simulate(query, progress) {
        Ok(res) => res,
        Err(err) => return create_error(&err)
    };

    serde_json::to_string(&ApiResponse::Success(res))
        .unwrap_or(DEFAULT_ERR.to_owned())
}

/** Names of the theories ported to the Rust sim, as a JSON array (e.g. ["T1","T2"]) */
#[wasm_bindgen]
pub fn ported_theories() -> String {
    serde_json::to_string(&sim::PORTED_THEORIES).unwrap_or_else(|_| "[]".to_owned())
}