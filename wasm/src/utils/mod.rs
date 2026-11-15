/*! utils */

pub mod complex_lognum;
pub mod fastmath;
pub mod lognum;

pub mod cost;
pub mod currency;
pub mod value;
pub mod variable;

mod lognumpoly;
pub mod result;
pub mod settings;
pub mod var_buy;

use var_buy::VarBuy;

/**
 * Returns the level reached by the most recent purchase of `var_name` in a purchase list
 * (purchases are in chronological order), or None if it was never bought.
 */
pub fn get_last_purchase(var_name: &str, var_buy_list: &[VarBuy]) -> Option<i32> {
    var_buy_list
        .iter()
        .rev()
        .find(|buy| buy.var_name == var_name)
        .map(|buy| buy.level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::lognum::LogNum;

    fn buy(var_name: &'static str, level: i32) -> VarBuy {
        VarBuy {
            var_name: var_name.into(),
            level,
            cost: LogNum::from(10.),
            symbol: "rho",
            timestamp: 0.,
        }
    }

    #[test]
    fn last_purchase_is_the_most_recent_one_of_that_variable() {
        let list = [
            buy("q1", 1),
            buy("c1", 1),
            buy("q1", 2),
            buy("q1", 3),
            buy("c1", 2),
        ];
        assert_eq!(get_last_purchase("q1", &list), Some(3));
        assert_eq!(get_last_purchase("c1", &list), Some(2));
        assert_eq!(get_last_purchase("c3", &list), None);
        // matches on the variable name, not the currency symbol
        assert_eq!(get_last_purchase("rho", &list), None);
        assert_eq!(get_last_purchase("q1", &[]), None);
    }
}
