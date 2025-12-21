#[allow(dead_code)]
mod utils;
#[allow(dead_code)]
mod api;

use std::str::FromStr;

use utils::lognum::LogNum;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn test(input: &str) -> String {
    let _q = serde_json::from_str::<api::query::SimQuery>(input);

    let res = serde_json::to_string(
        &api::response::SimResponse::CHAIN(
            api::response::ChainSimResponse {
                results: vec![
                    utils::result::SimResult {
                    theory: "T1".to_owned(),
                    sigma: 20,
                    last_pub: LogNum::from_str("1e20").unwrap(),
                    pub_rho: LogNum::from_str("1e22").unwrap(),
                    delta_tau: LogNum::from_f64(100.),
                    pub_multi: 4.,
                    strat: "Test".to_owned(),
                    tau_h: 2.,
                    time: 15.*60.,
                    bought_vars: vec![utils::var_buy::VarBuy {
                        var_name: "c1".to_owned(),
                        level: 10,
                        cost: LogNum::from_str("1e18").unwrap(),
                        symbol: "ρ".to_owned(),
                        timestamp: 10.*60.
                    }]
                },
                utils::result::SimResult {
                    theory: "T1".to_owned(),
                    sigma: 20,
                    last_pub: LogNum::from_str("1e22").unwrap(),
                    pub_rho: LogNum::from_str("1e24").unwrap(),
                    delta_tau: LogNum::from_f64(100.),
                    pub_multi: 4.,
                    strat: "Test".to_owned(),
                    tau_h: 2.,
                    time: 15.*60.,
                    bought_vars: vec![utils::var_buy::VarBuy {
                        var_name: "c1".to_owned(),
                        level: 10,
                        cost: LogNum::from_str("1e18").unwrap(),
                        symbol: "ρ".to_owned(),
                        timestamp: 10.*60.
                    }]
                }],
                average_rate: 3.,
                delta_tau: LogNum::from_str("e4").unwrap(),
                total_time: 30.*60.
            }
        )
    );
    
    
    res.unwrap()
}
