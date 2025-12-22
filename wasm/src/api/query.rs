use serde::Deserialize;

use crate::api::config::TheoryType;
use crate::utils::{
    lognum::LogNum,
    settings::SimSettings
};

#[derive(Debug, Deserialize)]
pub struct SingleSimQuery {
    pub theory: TheoryType, // Maybe change to an enum later
    pub strat: String,
    pub sigma: i64, // Type could change
    pub rho: LogNum,
    pub cap: Option<LogNum>,
    pub last_strat: Option<String>,
    pub settings: SimSettings
}

#[derive(Debug, Deserialize)]
pub struct ChainSimQuery {
    pub theory: TheoryType, // Maybe change to an enum later
    pub strat: String,
    pub sigma: i64, // Type could change
    pub rho: LogNum,
    pub cap: LogNum,
    pub hard_cap: bool,
    pub settings: SimSettings
}

#[derive(Debug, Deserialize)]
pub struct StepSimQuery {
    pub theory: TheoryType, // Maybe change to an enum later
    pub strat: String,
    pub sigma: i64, // Type could change
    pub rho: LogNum,
    pub cap: LogNum,
    pub step: LogNum,
    pub settings: SimSettings
}

#[derive(Debug, Deserialize)]
pub struct SimAllQuery {
    pub values: Vec<LogNum>,
    pub sigma: i64, // Type could change
    pub very_active: bool,
    pub semi_idle: bool,
    pub settings: SimSettings
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "lowercase")]
pub enum SimQuery {
    Single(SingleSimQuery),
    Chain(ChainSimQuery),
    Step(StepSimQuery),
    All(SimAllQuery)
}