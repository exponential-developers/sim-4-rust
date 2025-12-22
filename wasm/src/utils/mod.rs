pub mod lognum;
pub mod complex_lognum;

pub mod cost;
pub mod value;
pub mod variable;
pub mod currency;

pub mod var_buy;
pub mod settings;
pub mod result;

use var_buy::VarBuy;

pub fn get_last_purchase(var_name: &str, var_buy_list: &[VarBuy]) -> Option<i32> {
    for buy in var_buy_list {
        if buy.symbol == var_name {
            return Some(buy.level)
        }
    }

    None
}