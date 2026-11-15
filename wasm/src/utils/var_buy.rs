use std::borrow::Cow;

use crate::utils::lognum::LogNum;
use serde::Serialize;

/**
 * Holds a record of a variable purchase.
 *
 * Names are borrowed static strings (copying them is free): forks copy the whole purchase list,
 * which with heap strings meant two allocations per purchase per fork. Only MF's "Reset at V=..."
 * records have a name that is built when they are made.
 */
#[derive(Debug, Clone, Serialize)]
pub struct VarBuy {
    pub var_name: Cow<'static, str>,
    pub level: i32,
    pub cost: LogNum,
    pub symbol: &'static str,
    pub timestamp: f64,
}
