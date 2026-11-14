/*!
 * Contains structures for the different types of queries that can be sent to the wasm module
 */

use serde::Deserialize;

use crate::api::config::TheoryType;
use crate::utils::{lognum::LogNum, settings::SimSettings};

#[derive(Debug, Deserialize)]
pub struct SingleSimQuery {
    pub theory: TheoryType, // Maybe change to an enum later
    pub strat: String,
    pub sigma: i64, // Type could change
    pub rho: LogNum,
    pub cap: Option<LogNum>,
    pub last_strat: Option<String>,
    pub settings: SimSettings,
}

#[derive(Debug, Deserialize)]
pub struct ChainSimQuery {
    pub theory: TheoryType, // Maybe change to an enum later
    pub strat: String,
    pub sigma: i64, // Type could change
    pub rho: LogNum,
    pub cap: LogNum,
    pub hard_cap: bool,
    pub settings: SimSettings,
}

#[derive(Debug, Deserialize)]
pub struct StepSimQuery {
    pub theory: TheoryType, // Maybe change to an enum later
    pub strat: String,
    pub sigma: i64, // Type could change
    pub rho: LogNum,
    pub cap: LogNum,
    pub step: LogNum,
    pub settings: SimSettings,
}

#[derive(Debug, Deserialize)]
pub struct SimAllQuery {
    pub values: Vec<LogNum>,
    pub sigma: i64, // Type could change
    pub very_active: bool,
    pub semi_idle: bool,
    pub settings: SimSettings,
}

/** Chain of a fixed number of publications */
#[derive(Debug, Deserialize)]
pub struct AmountSimQuery {
    pub theory: TheoryType,
    pub strat: String,
    pub sigma: i64,
    pub rho: LogNum,
    pub amount: u32,
    pub settings: SimSettings,
}

/** Chain until the total time (in seconds) is reached */
#[derive(Debug, Deserialize)]
pub struct TimeSimQuery {
    pub theory: TheoryType,
    pub strat: String,
    pub sigma: i64,
    pub rho: LogNum,
    pub time: f64,
    pub settings: SimSettings,
}

/** For each step from rho to cap, the best number of pubs to chain towards cap */
#[derive(Debug, Deserialize)]
pub struct StepChainSimQuery {
    pub theory: TheoryType,
    pub strat: String,
    pub sigma: i64,
    pub rho: LogNum,
    pub cap: LogNum,
    pub step: LogNum,
    pub hard_cap: bool,
    pub settings: SimSettings,
}

/** Every strat of a theory, sorted by tau/h */
#[derive(Debug, Deserialize)]
pub struct ComparisonSimQuery {
    pub theory: TheoryType,
    pub sigma: i64,
    pub rho: LogNum,
    pub settings: SimSettings,
}

/** Table of the fastest way to reach cap from each step between rho and cap */
#[derive(Debug, Deserialize)]
pub struct PubTableSimQuery {
    pub theory: TheoryType,
    pub strat: String,
    pub sigma: i64,
    pub rho: LogNum,
    pub cap: LogNum,
    pub step: f64,
    pub settings: SimSettings,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "lowercase")]
pub enum SimQuery {
    Single(SingleSimQuery),
    Chain(ChainSimQuery),
    Step(StepSimQuery),
    All(SimAllQuery),
    #[serde(rename = "step_chain")]
    StepChain(StepChainSimQuery),
    Amount(AmountSimQuery),
    Time(TimeSimQuery),
    Comparison(ComparisonSimQuery),
    #[serde(rename = "pub_table")]
    PubTable(PubTableSimQuery),
}
