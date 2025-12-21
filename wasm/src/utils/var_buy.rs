use crate::utils::lognum::LogNum;

#[allow(dead_code)]
pub struct VarBuy {
    pub var_name: String,
    pub level: i32,
    pub cost: LogNum,
    pub symbol: String,
    pub timestamp: f64
}