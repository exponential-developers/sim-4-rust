/*!
 * Contains structures for the different types of queries that can be sent to the wasm module
 */

use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;

use crate::api::config::TheoryType;
use crate::utils::{lognum::LogNum, settings::SimSettings};

/** A theory's own inputs (sim-3 `specificInputs`) by id, as text, e.g. {"depth": "2"} */
pub type TheoryInputs = BTreeMap<String, String>;

#[derive(Debug, Deserialize)]
pub struct SingleSimQuery {
    pub theory: TheoryType, // Maybe change to an enum later
    pub strat: String,
    pub sigma: i64, // Type could change
    pub rho: LogNum,
    /**
     * log10 tau to start from, when the sim starts from a tau value like sim-3's chains do (then
     * `rho` is `tau / tau_factor`). Not sent by the website: None means `rho * tau_factor`.
     */
    #[serde(default)]
    pub tau: Option<f64>,
    pub cap: Option<LogNum>,
    pub last_strat: Option<String>,
    /** The theory's own inputs (data.json `specificInputs`), as entered */
    #[serde(default)]
    pub theory_inputs: TheoryInputs,
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
    /** The theory's own inputs (data.json `specificInputs`), as entered */
    #[serde(default)]
    pub theory_inputs: TheoryInputs,
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
    /** The theory's own inputs (data.json `specificInputs`), as entered */
    #[serde(default)]
    pub theory_inputs: TheoryInputs,
    pub settings: SimSettings,
}

#[derive(Debug, Deserialize)]
pub struct SimAllQuery {
    pub values: Vec<LogNum>,
    pub sigma: i64, // Type could change
    pub very_active: bool,
    pub semi_idle: bool,
    /** Each theory's own inputs */
    #[serde(default)]
    pub theory_inputs: HashMap<TheoryType, TheoryInputs>,
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
    /** The theory's own inputs (data.json `specificInputs`), as entered */
    #[serde(default)]
    pub theory_inputs: TheoryInputs,
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
    /** The theory's own inputs (data.json `specificInputs`), as entered */
    #[serde(default)]
    pub theory_inputs: TheoryInputs,
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
    /** The theory's own inputs (data.json `specificInputs`), as entered */
    #[serde(default)]
    pub theory_inputs: TheoryInputs,
    pub settings: SimSettings,
}

/** Every strat of a theory, sorted by tau/h */
#[derive(Debug, Deserialize)]
pub struct ComparisonSimQuery {
    pub theory: TheoryType,
    pub sigma: i64,
    pub rho: LogNum,
    /** The theory's own inputs (data.json `specificInputs`), as entered */
    #[serde(default)]
    pub theory_inputs: TheoryInputs,
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
    /** The theory's own inputs (data.json `specificInputs`), as entered */
    #[serde(default)]
    pub theory_inputs: TheoryInputs,
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
