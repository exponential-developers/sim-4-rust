use crate::utils::lognum::LogNum;
use serde::Serialize;

/**
 * Holds a record of a variable purchase.
 *
 * Names are static strings so the type is `Copy`: forks copy the whole purchase list, which with
 * heap strings meant two allocations per purchase per fork.
 */
#[derive(Debug, Clone, Copy, Serialize)]
pub struct VarBuy {
    pub var_name: &'static str,
    pub level: i32,
    pub cost: LogNum,
    pub symbol: &'static str,
    pub timestamp: f64,
}
