#[allow(dead_code)]
mod utils;
#[allow(dead_code)]
mod api;

use api::query;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn test(input: &str) -> String {
    let q = serde_json::from_str::<query::SimQuery>(input);

    format!("{:?}", q)
}
