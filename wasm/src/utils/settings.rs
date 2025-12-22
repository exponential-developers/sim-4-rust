use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    dt: f64,
    ddt: f64,
    mf_reset_depth: i32,
    bought_vars_delta: i32,
    sim_all_strats: SimAllStrats,
    completed_cts: CompletedCTs,
    show_a23: bool,
    show_unofficials: bool
}