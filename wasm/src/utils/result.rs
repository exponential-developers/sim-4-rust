use serde::Serialize;

use crate::utils::{
    lognum::LogNum,
    var_buy::VarBuy
};

#[derive(Debug, Clone, Serialize)]
pub struct SimResult {
    pub theory: String, // Maybe change to enum later
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

#[derive(Debug, Clone, Serialize)]
pub struct SimAllResult {
    pub theory: String,
    pub ratio: f64,
    pub last_pub: LogNum,
    pub active: SimResult,
    pub idle: SimResult
}