mod utils;
use utils::lognum::LogNum;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn test(input: &str) -> String {
    let mut x: LogNum = LogNum::from(10.);
    x -= LogNum::from(5.);

    input.to_string() + " " + &(x.to_string())
}
