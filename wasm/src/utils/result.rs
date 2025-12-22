use serde::Serialize;

use crate::utils::{
    lognum::{self, LogNum},
    var_buy::VarBuy
};
use crate::api::config::TheoryType;

#[derive(Debug, Clone, Serialize)]
pub struct SimResult {
    pub theory: TheoryType,
    pub sigma: i64, // Type could change
    pub last_pub: LogNum,
    pub pub_rho: LogNum,
    pub delta_tau: LogNum,
    pub pub_multi: f64,
    pub strat: String,
    pub tau_h: f64,
    pub time: f64,
    pub bought_vars: Vec<VarBuy>
}

impl Default for SimResult {
    fn default() -> Self {
        SimResult {
            theory: TheoryType::T1,
            sigma: 0,
            last_pub: lognum::ONE,
            pub_rho: lognum::ONE,
            delta_tau: lognum::ONE,
            pub_multi: 1.,
            strat: "Result undefined".to_owned(),
            tau_h: 0.,
            time: 1.,
            bought_vars: Vec::new()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SimAllResult {
    pub theory: TheoryType,
    pub ratio: f64,
    pub last_pub: LogNum,
    pub active: SimResult,
    pub idle: SimResult
}