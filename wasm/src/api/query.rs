use serde::Deserialize;

use crate::utils::{
    lognum::LogNum,
    settings::SimSettings
};

#[derive(Debug, Deserialize)]
pub struct SingleSimQuery {
    theory: String, // Maybe change to an enum later
    strat: String,
    sigma: i64, // Type could change
    rho: LogNum,
    cap: Option<LogNum>,
    last_strat: Option<String>,
    settings: SimSettings
}

pub struct ChainSimQuery {
    theory: String, // Maybe change to an enum later
    strat: String,
    sigma: i64, // Type could change
    rho: LogNum,
    cap: LogNum,
    hard_cap: bool,
    settings: SimSettings
}

pub struct StepSimQuery {
    theory: String, // Maybe change to an enum later
    strat: String,
    sigma: i64, // Type could change
    rho: LogNum,
    cap: LogNum,
    step: LogNum,
    settings: SimSettings
}

pub struct SimAllQuery {
    values: Vec<LogNum>,
    sigma: i64, // Type could change
    very_active: bool,
    semi_idle: bool,
    settings: SimSettings
}