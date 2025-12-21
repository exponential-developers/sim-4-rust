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
    pub results: Vec<SimResult>,
    pub delta_tau: LogNum,
    pub average_rate: f64,
    pub total_time: f64
}

#[derive(Debug, Serialize)]
pub struct StepSimResponse {
    pub results: Vec<SimResult>
}

#[derive(Debug, Serialize)]
pub struct SimAllResponse {
    pub sigma: i64,
    pub strat_type: settings::SimAllStrats,
    pub completed_cts: settings::CompletedCTs,
    pub results: Vec<SimAllResult>
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "lowercase")]
pub enum SimResponse {
    Single(SingleSimResponse),
    Chain(ChainSimResponse),
    Step(StepSimResponse),
    All(SimAllResponse)
}