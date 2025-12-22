use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use num_enum::TryFromPrimitive;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TryFromPrimitive)]
#[repr(usize)]
//#[serde(rename_all = "lowercase")]
pub enum TheoryType {
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    T7,
    T8,
    WSP,
    SL,
    EF,
    CSR2,
    FI,
    FP,
    RZ,
    MF,
    BaP,
    TC,
    BT
}

#[derive(Debug, Deserialize)]
pub struct ConfigTheories {
    pub tau_factor: f64
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub theories: HashMap<TheoryType, ConfigTheories>,
    pub strat_categories: Vec<String>
}