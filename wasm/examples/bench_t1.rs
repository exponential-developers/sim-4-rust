//! Native benchmark for the Rust T1 port.
//!
//! Usage: cargo run --release --example bench_t1 -- [iterations]
//! Prints one JSON line per (rho, strat) case with the result and the average time per sim.

use std::time::Instant;
use wasm::theories::{
    t1::{T1Strat, simulate_t1},
    traditional::{SimParams, TheoryInput},
};

fn main() {
    let iterations: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    // Same cases as bench/run.js: [log10 rho of last pub, students]
    let cases = [
        (25., 30),
        (75., 60),
        (150., 90),
        (300., 140),
        (500., 180),
        (700., 220),
        (1000., 280),
    ];

    let mut total_ms = 0.;
    for (rho, sigma) in cases {
        for strat in T1Strat::ALL {
            let input = TheoryInput {
                sigma,
                last_pub_rho: rho,
                last_pub_tau: None,
                rho_cap: f64::INFINITY,
                params: SimParams::default(),
            };
            // warm-up + result
            let res = simulate_t1(strat, &input);
            let start = Instant::now();
            for _ in 0..iterations {
                std::hint::black_box(simulate_t1(strat, &input));
            }
            let ms = start.elapsed().as_secs_f64() * 1000. / iterations as f64;
            total_ms += ms;
            println!(
                "{{\"rho\":{rho},\"sigma\":{sigma},\"strat\":\"{}\",\"resStrat\":{:?},\"tauH\":{},\"pubRho\":{},\"time\":{},\"boughtVars\":{},\"ms\":{ms}}}",
                strat.name(),
                res.strat,
                res.tau_h,
                res.pub_rho.value,
                res.time,
                res.bought_vars.len()
            );
        }
    }
    eprintln!("total ms per full pass: {total_ms:.2}");
}
