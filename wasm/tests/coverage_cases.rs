//! Coverage run: runs every case saved from sim-3 (tests/fixtures/sim3_results.json) natively,
//! through the same entry points as the website, so `cargo llvm-cov` measures the theory and
//! sim-mode code that the parity tests exercise.
//!
//! It only checks that each case runs and gives a sensible result. Exact equality with sim-3 is
//! checked by `sim3_parity.rs`, which has to run as WebAssembly: natively, log10/pow come from the
//! platform's math library and can differ from JavaScript's in the last digit.
//!
//! Ignored by default (it takes a while); the coverage job runs it with:
//!   cargo llvm-cov --release -- --include-ignored
#![cfg(not(target_arch = "wasm32"))]

use serde_json::{Value, json};

const FIXTURES: &str = include_str!("fixtures/sim3_results.json");
const DATA_JSON: &str = include_str!("../../src/Data/data.json");

fn settings() -> Value {
    json!({
        "dt": 1.5, "ddt": 1.0001, "bought_vars_delta": 5,
        "sim_all_strats": "all", "completed_cts": "no", "show_a23": false, "show_unofficials": false
    })
}

/** Runs a query through the website's JSON API and returns the response data */
/** Adds a case's theory inputs (only saved when it has some) to a query */
fn with_inputs(mut data: Value, case: &Value) -> Value {
    if !case["theory_inputs"].is_null() {
        data["theory_inputs"] = case["theory_inputs"].clone();
    }
    data
}

fn api(query_type: &str, mut data: Value) -> Value {
    wasm::set_config(DATA_JSON);
    data["settings"] = settings();
    let query = json!({ "type": query_type, "data": data }).to_string();
    let res: Value = serde_json::from_str(&wasm::run_query(&query)).unwrap();
    assert_eq!(
        res["response_type"], "success",
        "{query_type} failed: {}",
        res["data"]
    );
    res["data"]["data"].clone()
}

/** A finished publication: positive rate and time, pub rho above the starting rho */
fn assert_sensible(label: &str, start: f64, tau_h: f64, pub_rho: f64, time: f64) {
    assert!(tau_h.is_finite() && tau_h > 0., "{label}: tau/h {tau_h}");
    assert!(time.is_finite() && time > 0., "{label}: time {time}");
    assert!(
        pub_rho > start,
        "{label}: pub rho {pub_rho} not above {start}"
    );
}

#[test]
#[ignore = "coverage run (slow); exact results are checked by sim3_parity in WebAssembly"]
fn run_all_saved_cases() {
    let f: Value = serde_json::from_str(FIXTURES).unwrap();

    for c in f["strats"].as_array().unwrap() {
        let (theory, strat) = (c["theory"].as_str().unwrap(), c["strat"].as_str().unwrap());
        let rho = c["rho"].as_f64().unwrap();
        let inputs = if c["theory_inputs"].is_null() {
            "{}".to_owned()
        } else {
            c["theory_inputs"].to_string()
        };
        let r: Value = serde_json::from_str(&wasm::sim_strat_with_inputs(
            theory,
            strat,
            c["sigma"].as_u64().unwrap() as u32,
            rho,
            &inputs,
        ))
        .unwrap();
        let label = format!("{theory} {strat} e{rho}");
        // No result is only fine where sim-3 has none either (T8's SingleMS strats with no swap
        // point near the last pub, ILC's pub table strats past the table)
        if r["strat"] == "Result undefined" {
            assert_eq!(
                c["expected"]["strat"], "Result undefined",
                "{label}: no result"
            );
            assert_eq!(r["tauH"].as_f64().unwrap(), 0., "{label}");
            continue;
        }
        // sim-3 names T6's idle recovery stops "...IdleRecoveryCoastM3": the stop comes after the
        // strat name, and here it is part of the requested name (T6SnaxIdleRecoveryM3Coast)
        let expected_name = match strat.split_once("IdleRecoveryM") {
            Some((head, rest)) => format!("{head}IdleRecovery{}", &rest[1..]),
            None => strat.to_owned(),
        };
        // FI's PT strats can return the result of the strat without PT (the continuity run)
        let without_pt = expected_name.replacen("PT", "", 1);
        let name = r["strat"].as_str().unwrap();
        assert!(
            name.starts_with(&expected_name) || name.starts_with(&without_pt),
            "{label}: result strat {}",
            r["strat"]
        );
        assert_sensible(
            &label,
            rho,
            r["tauH"].as_f64().unwrap(),
            r["pubRho"].as_f64().unwrap(),
            r["time"].as_f64().unwrap(),
        );
    }

    for c in f["best"].as_array().unwrap() {
        let rho = c["rho"].as_f64().unwrap();
        let data = json!({ "theory": c["theory"], "strat": c["strat"], "sigma": c["sigma"], "rho": rho, "cap": null, "last_strat": null });
        let r = &api("single", with_inputs(data, c))["result"];
        let label = format!("{} {} e{rho}", c["theory"], c["strat"]);
        assert_sensible(
            &label,
            rho,
            r["tau_h"].as_f64().unwrap(),
            r["pub_rho"].as_f64().unwrap(),
            r["time"].as_f64().unwrap(),
        );
    }

    for c in f["chains"].as_array().unwrap() {
        let data = json!({ "theory": c["theory"], "strat": c["strat"], "sigma": c["sigma"], "rho": c["rho"], "cap": c["cap"], "hard_cap": c["hard_cap"] });
        let r = api("chain", with_inputs(data, c));
        assert!(
            !r["results"].as_array().unwrap().is_empty(),
            "chain {}: no results",
            c["strat"]
        );
        assert!(r["delta_tau"].as_f64().unwrap() > 0.);
    }

    for c in f["modes"].as_array().unwrap() {
        let (mode, extra) = (c["mode"].as_str().unwrap(), c["extra"].as_array().unwrap());
        let mut data = with_inputs(
            json!({ "theory": c["theory"], "strat": c["strat"], "sigma": c["sigma"], "rho": c["rho"] }),
            c,
        );
        let keys: &[&str] = match mode {
            "step" | "pub_table" => &["cap", "step"],
            "amount" => &["amount"],
            "time" => &["time"],
            "step_chain" => &["cap", "step", "hard_cap"],
            _ => &[],
        };
        for (k, v) in keys.iter().zip(extra) {
            data[*k] = v.clone();
        }
        let r = api(mode, data);
        let rows = if mode == "pub_table" {
            &r["pub_table"]
        } else {
            &r["results"]
        };
        assert!(
            rows.as_array().is_some_and(|a| !a.is_empty()),
            "{mode}: no results"
        );
    }

    // "All" mode, plus errors for bad input
    let all = api(
        "all",
        json!({ "values": [500.0, 500.0], "sigma": 500, "very_active": false, "semi_idle": true }),
    );
    let theories: Vec<&str> = all["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["theory"].as_str().unwrap())
        .collect();
    assert_eq!(theories, ["T1", "T2"], "All mode should skip 0 values");
    let err: Value = serde_json::from_str(&wasm::run_query("not json")).unwrap();
    assert_eq!(err["response_type"], "failure");
    let err: Value = serde_json::from_str(&wasm::sim_strat("T1", "NotAStrat", 100, 100.)).unwrap();
    assert_eq!(err["response_type"], "failure");
    let err: Value = serde_json::from_str(&wasm::sim_strat("T99", "T1", 100, 100.)).unwrap();
    assert!(err["data"].as_str().unwrap().contains("Unknown theory"));
    assert_eq!(
        wasm::ported_theories(),
        r#"["T1","T2","T3","T4","T5","T6","T7","T8","WSP","SL","EF","CSR2","FI","FP","RZ","MF","BaP","BT","TC","FS","BD","ILC","NLI"]"#
    );
}
