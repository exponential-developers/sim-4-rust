use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn test(input: &str) -> String {

    input.len().to_string()
}
