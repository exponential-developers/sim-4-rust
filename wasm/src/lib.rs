#[allow(dead_code)]
mod utils;
#[allow(dead_code)]
mod api;

use std::str::FromStr;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use utils::lognum::LogNum;
use api::response::SimResponse;
use utils::result::SimResult;

const DEFAULT_ERR: &'static str = "{\"response_type\": \"failure\", \"data\":\"API Error\"}";

#[derive(Serialize)]
#[serde(tag = "response_type", content = "data", rename_all = "lowercase")]
#[allow(dead_code)]
enum ApiResponse {
    SUCCESS(SimResponse),
    FAILURE(String)
}

#[wasm_bindgen]
#[allow(unused_variables)]
pub fn test(input: &str) -> String {

    "Test".to_owned()
}

#[wasm_bindgen]
pub fn main(input: &str) -> String {
    let res = api::response::SimResponse::ALL(
        api::response::SimAllResponse {
            sigma: 20,
            strat_type: utils::settings::SimAllStrats::ALL,
            completed_cts: utils::settings::CompletedCTs::IN,
            results: vec![
                utils::result::SimAllResult {
                    theory: "T1".to_owned(),
                    ratio: 1.2,
                    last_pub: LogNum::from_str("1.5e25").unwrap(),
                    active: SimResult {
                        theory: "T1".to_owned(),
                        sigma: 20,
                        last_pub: LogNum::from_str("1.5e25").unwrap(),
                        pub_rho: LogNum::from_str("1e27").unwrap(),
                        delta_tau: LogNum::from(1e27 / 1.5e25),
                        pub_multi: 3.,
                        strat: "T1SolarXLII".to_owned(),
                        tau_h: 2.5,
                        time: 1000.,
                        bought_vars: Vec::new()
                    },
                    idle: SimResult {
                        theory: "T1".to_owned(),
                        sigma: 20,
                        last_pub: LogNum::from_str("1.5e25").unwrap(),
                        pub_rho: LogNum::from_str("1e27").unwrap(),
                        delta_tau: LogNum::from(1e27 / 1.5e25),
                        pub_multi: 3.,
                        strat: "T1C34".to_owned(),
                        tau_h: 1.75,
                        time: 1200.,
                        bought_vars: Vec::new()
                    },
                },
                utils::result::SimAllResult {
                    theory: "T2".to_owned(),
                    ratio: 1.05,
                    last_pub: LogNum::from_str("1e400").unwrap(),
                    active: SimResult {
                        theory: "T2".to_owned(),
                        sigma: 20,
                        last_pub: LogNum::from_str("1e400").unwrap(),
                        pub_rho: LogNum::from_str("1e420").unwrap(),
                        delta_tau: LogNum::from(1e20),
                        pub_multi: 700.,
                        strat: "T2".to_owned(),
                        tau_h: 0.8,
                        time: 150000.,
                        bought_vars: Vec::new()
                    },
                    idle: SimResult {
                        theory: "T2".to_owned(),
                        sigma: 20,
                        last_pub: LogNum::from_str("1e400").unwrap(),
                        pub_rho: LogNum::from_str("1e420").unwrap(),
                        delta_tau: LogNum::from(1e20),
                        pub_multi: 2.5,
                        strat: "T2Idle".to_owned(),
                        tau_h: 0.6,
                        time: 155000.,
                        bought_vars: Vec::new()
                    },
                }
            ]
        }
    );

    serde_json::to_string(&ApiResponse::SUCCESS(res))
        .unwrap_or(DEFAULT_ERR.to_owned())

    //serde_json::to_string(&ApiResponse::FAILURE("API not implemented".to_owned() + "\ninput was " + input))
    //    .unwrap_or(DEFAULT_ERR.to_owned())
}