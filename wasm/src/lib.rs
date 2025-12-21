#[allow(dead_code)]
mod utils;
#[allow(dead_code)]
mod api;

use std::str::FromStr;

use utils::lognum::LogNum;

use wasm_bindgen::prelude::*;

use crate::utils::result::SimResult;

#[wasm_bindgen]
pub fn test(input: &str) -> String {
    let _q = serde_json::from_str::<api::query::SimQuery>(input);

    let res = serde_json::to_string(
        &api::response::SimResponse::ALL(
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
        )
    );
    
    
    res.unwrap()
}
