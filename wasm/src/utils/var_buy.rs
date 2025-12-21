use serde::Serialize;
use crate::utils::lognum::LogNum;

#[derive(Debug, Clone, Serialize)]
pub struct VarBuy {
    pub var_name: String,
    pub level: i32,
    pub cost: LogNum,
    pub symbol: String,
    pub timestamp: f64
}