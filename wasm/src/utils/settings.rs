use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SimAllStrats {
    All,
    Active,
    Idle
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CompletedCTs {
    In,
    End,
    No
}

#[derive(Debug, Clone, Deserialize)]
pub struct SimSettings {
    pub dt: f64,
    pub ddt: f64,
    pub mf_reset_depth: i32,
    pub bought_vars_delta: i32,
    pub sim_all_strats: SimAllStrats,
    pub completed_cts: CompletedCTs,
    pub show_a23: bool,
    pub show_unofficials: bool
}