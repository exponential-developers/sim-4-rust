use serde::Serialize;

use crate::utils::{
    lognum::LogNum,
    result::{SimResult, SimAllResult},
    settings
};

#[derive(Debug, Serialize)]
pub struct SingleSimResponse {
    pub result: SimResult
}

#[derive(Debug, Serialize)]
pub struct ChainSimResponse {
    results: Vec<SimResult>,
    delta_tau: LogNum,
    average_rate: f64,
    total_time: f64
}

#[derive(Debug, Serialize)]
pub struct StepSimResponse {
    results: Vec<SimResult>
}

#[derive(Debug, Serialize)]
pub struct SimAllResponse {
    sigma: i64,
    strat_type: settings::SimAllStrats,
    completed_cts: settings::CompletedCTs,
    results: Vec<SimAllResult>
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "lowercase")]
pub enum SimResponse {
    SINGLE(SingleSimResponse),
    CHAIN(ChainSimResponse),
    STEP(StepSimResponse),
    ALL(SimAllResponse)
}