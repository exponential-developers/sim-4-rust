use crate::utils::{
    lognum::LogNum,
    var_buy::VarBuy
};

#[derive(Debug, Clone)]
pub struct SimResult {
    theory: String, // Maybe change to enum later
    sigma: i64, // Type could change
    last_pub: LogNum,
    pub_rho: LogNum,
    delta_tau: LogNum,
    pub_multi: f64,
    strat: String,
    tau_h: f64,
    time: f64,
    bought_vars: Vec<VarBuy>
}

#[derive(Debug, Clone)]
pub struct SimAllResult {
    theory: String,
    ratio: f64,
    last_pub: LogNum,
    active: SimResult,
    idle: SimResult
}