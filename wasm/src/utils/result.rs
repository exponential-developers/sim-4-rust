use std::collections::BTreeMap;

use serde::{Deserialize, Serialize, Serializer};

use crate::api::config::TheoryType;
use crate::utils::{
    lognum::{self, LogNum},
    var_buy::VarBuy,
};

/**
 * What is needed to simulate a result again: one named strat from one starting point. Responses
 * leave out the variable purchases (a Steps run can have millions); the page gets a result's
 * purchases by simulating it again with this (`sim::replay_bought_vars`), which gives exactly the
 * same purchases.
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replay {
    pub theory: TheoryType,
    pub strat: String,
    pub sigma: i64,
    /** log10 rho (and tau when the sim started from a tau) of the last publication */
    pub rho: f64,
    #[serde(default)]
    pub tau: Option<f64>,
    /** log10 rho cap, if any */
    #[serde(default)]
    pub cap: Option<f64>,
    #[serde(default)]
    pub theory_inputs: BTreeMap<String, String>,
    pub dt: f64,
    pub ddt: f64,
    pub bought_vars_delta: f64,
    pub show_a23: bool,
}

/**
 * Holds a result of a single simulation. When sent to the page (`Serialize`), the variable
 * purchases are left out: `bought_vars` is empty and `bought_vars_count` says how many there are.
 */
#[derive(Debug, Clone)]
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
    pub bought_vars: Vec<VarBuy>,
    /**
     * Theory inputs to use for the next publication (NLI's lifetime α), which chains pass on
     * (sim-3 `theorySpecificInputs` of a result)
     */
    pub theory_inputs: Option<BTreeMap<String, String>>,
    /** How to simulate this result again, for its purchases (None for combined results) */
    pub replay: Option<Box<Replay>>,
}

impl Serialize for SimResult {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Sent<'a> {
            theory: TheoryType,
            sigma: i64,
            last_pub: LogNum,
            pub_rho: LogNum,
            delta_tau: LogNum,
            pub_multi: f64,
            strat: &'a str,
            tau_h: f64,
            time: f64,
            bought_vars: [VarBuy; 0],
            bought_vars_count: usize,
            #[serde(skip_serializing_if = "Option::is_none")]
            theory_inputs: Option<&'a BTreeMap<String, String>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            replay: Option<&'a Replay>,
        }
        Sent {
            theory: self.theory,
            sigma: self.sigma,
            last_pub: self.last_pub,
            pub_rho: self.pub_rho,
            delta_tau: self.delta_tau,
            pub_multi: self.pub_multi,
            strat: &self.strat,
            tau_h: self.tau_h,
            time: self.time,
            bought_vars: [],
            bought_vars_count: self.bought_vars.len(),
            theory_inputs: self.theory_inputs.as_ref(),
            replay: self.replay.as_deref(),
        }
        .serialize(serializer)
    }
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
            tau_h: 1.,
            time: 1.,
            bought_vars: Vec::new(),
            theory_inputs: None,
            replay: None,
        }
    }
}

/** Holds a result for a SimAll simulation for one theory */
#[derive(Debug, Clone, Serialize)]
pub struct SimAllResult {
    pub theory: TheoryType,
    pub ratio: f64,
    pub last_pub: LogNum,
    pub active: SimResult,
    pub idle: SimResult,
}
