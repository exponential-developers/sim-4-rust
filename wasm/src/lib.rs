#[allow(dead_code)]
mod utils;
#[allow(dead_code)]
mod api;
#[allow(dead_code, unused_variables)]
mod sim;

use std::sync::OnceLock;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use api::query::SimQuery;
use api::response::SimResponse;
use api::config::Config;
use sim::simulate;

static CONFIG: OnceLock<Config> = OnceLock::new();
const DEFAULT_ERR: &str = "{\"response_type\": \"failure\", \"data\":\"API Error\"}";

#[derive(Serialize)]
#[serde(tag = "response_type", content = "data", rename_all = "lowercase")]
enum ApiResponse {
    Success(SimResponse),
    Failure(String)
}

#[wasm_bindgen] 
pub fn set_config(config: &str) -> String { 
    let cfg: Config = serde_json::from_str(config).unwrap(); 
    CONFIG.set(cfg).expect("Config already set"); 

    format!("{:?}", CONFIG.get())
}

fn create_error(msg: &str) -> String {
    serde_json::to_string(&ApiResponse::Failure(msg.to_owned()))
        .unwrap_or(DEFAULT_ERR.to_owned())
}

#[wasm_bindgen]
#[allow(unused_variables)]
pub fn main(input: &str) -> String {
    let query = match serde_json::from_str::<SimQuery>(input) {
        Ok(query) => query,
        Err(err) => return create_error(&format!("Error parsing query: {}", err.to_string()))
    };

    let res = match simulate(query) {
        Ok(res) => res,
        Err(err) => return create_error(&err) 
    };

    serde_json::to_string(&ApiResponse::Success(res))
        .unwrap_or(DEFAULT_ERR.to_owned())
}