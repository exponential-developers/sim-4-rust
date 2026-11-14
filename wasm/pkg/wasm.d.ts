/* tslint:disable */
/* eslint-disable */

/**
 * Names of the theories ported to the Rust sim, as a JSON array (e.g. ["T1","T2"]) 
 */
export function ported_theories(): string;

/**
 *
 * * Main API function of the wasm module: runs a JSON sim query and returns a JSON response
 * 
 */
export function run_query(input: string): string;

/**
 *
 * * Same as `run_query`, calling `progress(message)` with status text such as
 * * "Simulating 1e85/1e150" during long simulations (chain, steps, all).
 * 
 */
export function run_query_with_progress(input: string, progress: Function): string;

/**
 *
 * * Sets the config (data.json) for the wasm module
 * 
 */
export function set_config(config: string): string;

/**
 *
 * * Benchmark/test entry point: simulates one named strat of a ported theory (e.g. "T1", "T1Ratio")
 * * with default settings (dt 1.5, ddt 1.0001). `rho` is the log10 of the last publication rho.
 * * Returns a JSON summary of the result.
 * 
 */
export function sim_strat(theory: string, strat: string, sigma: number, rho: number): string;
