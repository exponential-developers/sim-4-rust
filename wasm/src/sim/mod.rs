use crate::CONFIG;
use crate::api::{
    condition::ConditionArgs,
    config::{Config, ConfigTheories},
};
use crate::api::{config::TheoryType, query::*, response::*};
use crate::theories::{
    bap::{BapStrat, simulate_bap},
    bd::{BdStrat, simulate_bd},
    bt::{BtStrat, simulate_bt},
    csr2::{Csr2Strat, simulate_csr2},
    ef::{EfStrat, simulate_ef},
    fi::{FiStrat, simulate_fi},
    fp::{FpStrat, simulate_fp},
    fs::{FsStrat, simulate_fs},
    ilc::{IlcStrat, simulate_ilc},
    mf::{MfStrat, simulate_mf},
    nli::{NliStrat, parse_exponential_value, simulate_nli},
    rz::{RzStrat, simulate_rz},
    sl::{SlStrat, simulate_sl},
    t1::{T1Strat, simulate_t1},
    t2::{T2Strat, simulate_t2},
    t3::{T3Strat, simulate_t3},
    t4::{T4Strat, simulate_t4},
    t5::{T5Strat, simulate_t5},
    t6::{T6Strat, simulate_t6},
    t7::{T7Strat, simulate_t7},
    t8::{T8Strat, simulate_t8},
    tc::{TcStrat, simulate_tc},
    traditional::{
        PubTableCollector, SimParams, TheoryInput, default_result, log_to_exp,
        set_current_collector,
    },
    wsp::{WspStrat, simulate_wsp},
};
use crate::utils::{
    lognum::{self, LogNum},
    result::*,
    settings::*,
    var_buy::VarBuy,
};

/** `a > b`, and false if either is NaN (so a NaN result counts as "no progress") */
fn is_greater<T: PartialOrd>(a: T, b: T) -> bool {
    a.partial_cmp(&b) == Some(std::cmp::Ordering::Greater)
}

fn config() -> Result<&'static Config, String> {
    CONFIG
        .get()
        .ok_or_else(|| "Sim config (data.json) was not loaded".to_owned())
}

fn theory_config(theory: TheoryType) -> Result<&'static ConfigTheories, String> {
    config()?
        .theories
        .get(&theory)
        .ok_or_else(|| format!("{theory:?} is missing from data.json"))
}

mod progress;
pub use progress::{FinishedSink, Progress, ProgressSink};

/** Theories that have been ported to the Rust sim, in data.json order */
pub const PORTED_THEORIES: [TheoryType; 23] = [
    TheoryType::T1,
    TheoryType::T2,
    TheoryType::T3,
    TheoryType::T4,
    TheoryType::T5,
    TheoryType::T6,
    TheoryType::T7,
    TheoryType::T8,
    TheoryType::WSP,
    TheoryType::SL,
    TheoryType::EF,
    TheoryType::CSR2,
    TheoryType::FI,
    TheoryType::FP,
    TheoryType::RZ,
    TheoryType::MF,
    TheoryType::BaP,
    TheoryType::BT,
    TheoryType::TC,
    TheoryType::FS,
    TheoryType::BD,
    TheoryType::ILC,
    TheoryType::NLI,
];

/** Whether a theory has been ported to the Rust sim */
pub fn is_ported(theory: TheoryType) -> bool {
    PORTED_THEORIES.contains(&theory)
}

/** Reads the theory inputs that sims use (MF's reset depth, NLI's lifetime α) */
pub fn apply_theory_inputs(params: &mut SimParams, inputs: &TheoryInputs) -> Result<(), String> {
    // sim-3: parseInt(specificInputs["depth"] ?? "0")
    params.mf_reset_depth = inputs
        .get("depth")
        .and_then(|d| d.trim().parse().ok())
        .unwrap_or(0);
    params.nli_lifetime_alpha = match inputs.get("lifetime_alpha") {
        Some(alpha) if !alpha.is_empty() => Some(parse_exponential_value(alpha)?),
        _ => None,
    };
    Ok(())
}

/** The variable purchases of a result, by simulating it again (see `Replay`) */
pub fn replay_bought_vars(replay: &Replay) -> Result<Vec<VarBuy>, String> {
    let mut input = TheoryInput {
        sigma: replay.sigma,
        last_pub_rho: replay.rho,
        last_pub_tau: replay.tau,
        rho_cap: replay.cap.unwrap_or(f64::INFINITY),
        params: SimParams {
            dt: replay.dt,
            ddt: replay.ddt,
            bought_vars_delta: replay.bought_vars_delta,
            show_a23: replay.show_a23,
            ..SimParams::default()
        },
    };
    apply_theory_inputs(&mut input.params, &replay.theory_inputs)?;
    Ok(run_strat(replay.theory, &replay.strat, &input)?.bought_vars)
}

/** "15,001" */
fn count_text(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/** Runs one named strat of a theory */
pub fn run_strat(
    theory: TheoryType,
    strat: &str,
    input: &TheoryInput,
) -> Result<SimResult, String> {
    match theory {
        TheoryType::T1 => {
            let strat =
                T1Strat::from_name(strat).ok_or_else(|| format!("Unknown T1 strat \"{strat}\""))?;
            Ok(simulate_t1(strat, input))
        }
        TheoryType::T2 => {
            let strat =
                T2Strat::from_name(strat).ok_or_else(|| format!("Unknown T2 strat \"{strat}\""))?;
            Ok(simulate_t2(strat, input))
        }
        TheoryType::T3 => {
            let strat =
                T3Strat::from_name(strat).ok_or_else(|| format!("Unknown T3 strat \"{strat}\""))?;
            Ok(simulate_t3(strat, input))
        }
        TheoryType::T4 => {
            let strat =
                T4Strat::from_name(strat).ok_or_else(|| format!("Unknown T4 strat \"{strat}\""))?;
            Ok(simulate_t4(strat, input))
        }
        TheoryType::T5 => {
            let strat =
                T5Strat::from_name(strat).ok_or_else(|| format!("Unknown T5 strat \"{strat}\""))?;
            Ok(simulate_t5(strat, input))
        }
        TheoryType::T6 => {
            let strat =
                T6Strat::from_name(strat).ok_or_else(|| format!("Unknown T6 strat \"{strat}\""))?;
            Ok(simulate_t6(strat, input))
        }
        TheoryType::T7 => {
            let strat =
                T7Strat::from_name(strat).ok_or_else(|| format!("Unknown T7 strat \"{strat}\""))?;
            Ok(simulate_t7(strat, input))
        }
        TheoryType::T8 => {
            let strat =
                T8Strat::from_name(strat).ok_or_else(|| format!("Unknown T8 strat \"{strat}\""))?;
            Ok(simulate_t8(strat, input))
        }
        TheoryType::WSP => {
            let strat = WspStrat::from_name(strat)
                .ok_or_else(|| format!("Unknown WSP strat \"{strat}\""))?;
            Ok(simulate_wsp(strat, input))
        }
        TheoryType::SL => {
            let strat =
                SlStrat::from_name(strat).ok_or_else(|| format!("Unknown SL strat \"{strat}\""))?;
            Ok(simulate_sl(strat, input))
        }
        TheoryType::EF => {
            let strat =
                EfStrat::from_name(strat).ok_or_else(|| format!("Unknown EF strat \"{strat}\""))?;
            Ok(simulate_ef(strat, input))
        }
        TheoryType::CSR2 => {
            let strat = Csr2Strat::from_name(strat)
                .ok_or_else(|| format!("Unknown CSR2 strat \"{strat}\""))?;
            Ok(simulate_csr2(strat, input))
        }
        TheoryType::BaP => {
            let strat = BapStrat::from_name(strat)
                .ok_or_else(|| format!("Unknown BaP strat \"{strat}\""))?;
            Ok(simulate_bap(strat, input))
        }
        TheoryType::FI => {
            let strat =
                FiStrat::from_name(strat).ok_or_else(|| format!("Unknown FI strat \"{strat}\""))?;
            Ok(simulate_fi(strat, input))
        }
        TheoryType::FP => {
            let strat =
                FpStrat::from_name(strat).ok_or_else(|| format!("Unknown FP strat \"{strat}\""))?;
            Ok(simulate_fp(strat, input))
        }
        TheoryType::BT => {
            let strat =
                BtStrat::from_name(strat).ok_or_else(|| format!("Unknown BT strat \"{strat}\""))?;
            Ok(simulate_bt(strat, input))
        }
        TheoryType::TC => {
            let strat =
                TcStrat::from_name(strat).ok_or_else(|| format!("Unknown TC strat \"{strat}\""))?;
            Ok(simulate_tc(strat, input))
        }
        TheoryType::MF => {
            let strat =
                MfStrat::from_name(strat).ok_or_else(|| format!("Unknown MF strat \"{strat}\""))?;
            Ok(simulate_mf(strat, input))
        }
        TheoryType::RZ => {
            let strat =
                RzStrat::from_name(strat).ok_or_else(|| format!("Unknown RZ strat \"{strat}\""))?;
            Ok(simulate_rz(strat, input))
        }
        TheoryType::FS => {
            let strat =
                FsStrat::from_name(strat).ok_or_else(|| format!("Unknown FS strat \"{strat}\""))?;
            Ok(simulate_fs(strat, input))
        }
        TheoryType::BD => {
            let strat =
                BdStrat::from_name(strat).ok_or_else(|| format!("Unknown BD strat \"{strat}\""))?;
            Ok(simulate_bd(strat, input))
        }
        TheoryType::ILC => {
            let strat = IlcStrat::from_name(strat)
                .ok_or_else(|| format!("Unknown ILC strat \"{strat}\""))?;
            Ok(simulate_ilc(strat, input))
        }
        TheoryType::NLI => {
            let strat = NliStrat::from_name(strat)
                .ok_or_else(|| format!("Unknown NLI strat \"{strat}\""))?;
            simulate_nli(strat, input)
        }
    }
}

/**
 * Returns the strats to simulate for a strat category such as "Best Overall"
 * (sim-3 `getStrats`): strats whose filter condition and forced condition both hold.
 */
fn get_strats(
    theory: TheoryType,
    category: &str,
    tau: f64,
    rho: f64,
    last_strat: &str,
) -> Result<Vec<&'static str>, String> {
    let cats = &config()?.strat_categories;
    // The categories map, in order, to the Very-Active, Active, Semi-Idle and Idle play styles
    let is_cat = |i: usize| cats.get(i).is_some_and(|c| c == category);
    let args = ConditionArgs {
        very_active: is_cat(0),
        active: is_cat(1),
        semi_idle: is_cat(2),
        idle: is_cat(3),
        tau,
        rho,
        last_strat,
    };
    let config = theory_config(theory)?;
    let args = ConditionArgs {
        rho: if config.supports_rho { rho } else { 0. },
        ..args
    };
    Ok(config
        .strats
        .iter()
        .filter(|(_, s)| s.filter.eval(&args) && s.forced.eval(&args))
        .map(|(name, _)| name.as_str())
        .collect())
}

/** `progress` gets which strat runs when there are several (a "Best ..." category) */
fn single_sim(query: SingleSimQuery, progress: &Progress) -> Result<SingleSimResponse, String> {
    if !is_ported(query.theory) {
        return Err(format!(
            "{:?} has not been ported to the Rust sim yet",
            query.theory
        ));
    }

    let input = TheoryInput {
        sigma: query.sigma,
        last_pub_rho: query.rho.value,
        last_pub_tau: query.tau,
        rho_cap: query.cap.map_or(f64::INFINITY, |c| c.value),
        params: SimParams {
            dt: query.settings.dt,
            ddt: query.settings.ddt,
            bought_vars_delta: query.settings.bought_vars_delta as f64,
            show_a23: query.settings.show_a23,
            ..SimParams::default()
        },
    };
    let mut input = input;
    apply_theory_inputs(&mut input.params, &query.theory_inputs)?;

    let strats: Vec<&str> = if config()?.strat_categories.contains(&query.strat) {
        let tau = query
            .tau
            .unwrap_or(query.rho.value * theory_config(query.theory)?.tau_factor);
        get_strats(
            query.theory,
            &query.strat,
            tau,
            query.rho.value,
            query.last_strat.as_deref().unwrap_or(""),
        )?
    } else {
        vec![query.strat.as_str()]
    };

    // Keep the result with the highest tau/h; on ties the first strat wins (sim-3 `singleSim`)
    let mut best = default_result();
    let count = strats.len();
    for (i, strat) in strats.into_iter().enumerate() {
        if count > 1 {
            progress.report(
                &format!("{strat} ({}/{count})", i + 1),
                i as f64 / count as f64,
            );
        }
        let mut res = run_strat(query.theory, strat, &input)?;
        res.replay = Some(Box::new(Replay {
            theory: query.theory,
            strat: strat.to_owned(),
            sigma: query.sigma,
            rho: query.rho.value,
            tau: query.tau,
            cap: query.cap.map(|c| c.value),
            theory_inputs: query.theory_inputs.clone(),
            dt: query.settings.dt,
            ddt: query.settings.ddt,
            bought_vars_delta: query.settings.bought_vars_delta as f64,
            show_a23: query.settings.show_a23,
        }));
        progress.finished(&serde_json::json!({ "kind": "result", "result": &res }));
        if res.tau_h > best.tau_h {
            best = res;
        }
    }

    Ok(SingleSimResponse { result: best })
}

fn chain_sim(query: ChainSimQuery, progress: &Progress) -> Result<ChainSimResponse, String> {
    let tau_factor = theory_config(query.theory)?.tau_factor;
    chain_from_tau(
        ChainFromTau {
            theory: query.theory,
            strat: &query.strat,
            sigma: query.sigma,
            start_tau: query.rho.value * tau_factor,
            cap: query.cap,
            hard_cap: query.hard_cap,
            settings: &query.settings,
            theory_inputs: &query.theory_inputs,
        },
        progress,
    )
}

/** Where a chain starts (sim-3 chains work with tau values, which they pass on to each sim) */
struct ChainFromTau<'a> {
    theory: TheoryType,
    strat: &'a str,
    sigma: i64,
    start_tau: f64,
    cap: LogNum,
    hard_cap: bool,
    settings: &'a SimSettings,
    theory_inputs: &'a TheoryInputs,
}

/** sim-3 `chainSim`: simulates publications until tau reaches the cap */
fn chain_from_tau(query: ChainFromTau, progress: &Progress) -> Result<ChainSimResponse, String> {
    let tau_factor = theory_config(query.theory)?.tau_factor;
    let start = query.start_tau;
    let cap = query.cap.value * tau_factor;
    let mut tau = start;
    let mut time = 0f64;
    let mut last_strat = String::new();

    let mut results: Vec<SimResult> = Vec::new();
    let mut theory_inputs = query.theory_inputs.clone();

    while tau < cap {
        let rho = LogNum {
            value: tau / tau_factor,
            sign: 1,
        };
        let text = format!(
            "Pub {} · {}/{}",
            results.len() + 1,
            log_to_exp(rho.value, 0),
            log_to_exp(query.cap.value, 0)
        );
        let done = (tau - start) / (cap - start);
        progress.report(&text, done);
        let res = single_sim(
            SingleSimQuery {
                theory: query.theory,
                strat: query.strat.to_owned(),
                theory_inputs: theory_inputs.clone(),
                rho,
                tau: Some(tau),
                sigma: query.sigma,
                settings: query.settings.clone(),
                cap: if query.hard_cap {
                    Some(query.cap)
                } else {
                    None
                },
                last_strat: Some(last_strat.clone()),
            },
            &progress.part(&text, done, done),
        )?
        .result;

        let pub_tau = res.pub_rho.value * tau_factor;
        if !is_greater(pub_tau, tau) {
            return Err(format!(
                "Chain stopped at {rho}: no strat could make progress (last result: \"{}\")",
                res.strat
            ));
        }

        tau = pub_tau;
        time += res.time;
        last_strat.clear();
        last_strat.push_str(res.strat.split(' ').next().unwrap_or(""));
        finished_pub(progress, &res, start, tau, time);
        if let Some(next_inputs) = &res.theory_inputs {
            theory_inputs.extend(next_inputs.clone());
        }
        results.push(res);
    }

    chain_response(start, tau, time, results)
}

fn step_sim(query: StepSimQuery, progress: &Progress) -> Result<StepSimResponse, String> {
    let tau_factor = theory_config(query.theory)?.tau_factor;
    let cap = query.cap.value * tau_factor;
    let mut tau = query.rho.value * tau_factor;
    let start = tau;
    let mut last_strat = String::new();
    if !is_greater(query.step.value, 0.) {
        return Err("The step length must be greater than 0".to_owned());
    }
    // Same tolerance as sim-3 (cap + 0.00001 in log10)
    let next = |tau: f64| (tau / tau_factor + query.step.value) * tau_factor;
    let total = std::iter::successors(Some(tau), |&t| Some(next(t)))
        .take_while(|&t| t < cap + 0.00001)
        .count();

    let mut results: Vec<SimResult> = Vec::new();

    while tau < cap + 0.00001 {
        let rho = LogNum {
            value: tau / tau_factor,
            sign: 1,
        };
        let text = format!(
            "Step {}/{} · {}",
            count_text(results.len() + 1),
            count_text(total),
            log_to_exp(rho.value, 0)
        );
        let done = results.len() as f64 / total as f64;
        progress.report(&text, done);
        let res = single_sim(
            SingleSimQuery {
                theory: query.theory,
                strat: query.strat.clone(),
                theory_inputs: query.theory_inputs.clone(),
                rho,
                tau: Some(tau),
                sigma: query.sigma,
                settings: query.settings.clone(),
                cap: None,
                last_strat: Some(last_strat.clone()),
            },
            &progress.part(&text, done, done),
        )?
        .result;

        // The next step adds the step to rho
        tau = next(tau);
        last_strat.clear();
        last_strat.push_str(res.strat.split(' ').next().unwrap_or(""));
        progress.finished(&serde_json::json!({ "kind": "result", "result": &res }));
        results.push(res);
    }

    Ok(StepSimResponse { results })
}

/** sim-3 `convertTime`: "1y2d03h04m" style duration */
pub(crate) fn convert_time(secs: f64) -> String {
    let mins = ((secs / 60.) % 60.).floor();
    let hrs = ((secs / 3600.) % 24.).floor();
    let days = ((secs / 86400.) % 365.).floor();
    let years = (secs / 31536000.).floor();
    let mut out = String::new();
    if years > 0. {
        out += &if years < 1e6 {
            format!("{years}")
        } else {
            log_to_exp(years.log10(), 3)
        };
        out.push('y');
    }
    if days > 0. {
        out += &format!("{days}d");
    }
    out += &format!("{hrs:02}h");
    if years == 0. {
        out += &format!("{mins:02}m");
    }
    out
}

/** Chain of exactly `amount` publications (sim-3 `amountSim`) */
fn amount_sim(query: AmountSimQuery, progress: &Progress) -> Result<ChainSimResponse, String> {
    let tau_factor = theory_config(query.theory)?.tau_factor;
    let start = query.rho.value * tau_factor;
    let mut tau = start;
    let mut time = 0f64;
    let mut last_strat = String::new();
    let mut results: Vec<SimResult> = Vec::new();

    for i in 0..query.amount {
        let text = format!("Pub {}/{}", i + 1, query.amount);
        let (done, next) = (
            i as f64 / query.amount as f64,
            (i + 1) as f64 / query.amount as f64,
        );
        progress.report(&text, done);
        let res = single_sim(
            SingleSimQuery {
                theory: query.theory,
                strat: query.strat.clone(),
                theory_inputs: query.theory_inputs.clone(),
                rho: LogNum {
                    value: tau / tau_factor,
                    sign: 1,
                },
                tau: Some(tau),
                sigma: query.sigma,
                settings: query.settings.clone(),
                cap: None,
                last_strat: Some(last_strat.clone()),
            },
            &progress.part(&text, done, next),
        )?
        .result;
        tau = res.pub_rho.value * tau_factor;
        time += res.time;
        last_strat = res.strat.split(' ').next().unwrap_or("").to_owned();
        finished_pub(progress, &res, start, tau, time);
        results.push(res);
    }

    chain_response(start, tau, time, results)
}

/** Chain until the total time is reached (sim-3 `timeSim`) */
fn time_sim(query: TimeSimQuery, progress: &Progress) -> Result<ChainSimResponse, String> {
    let tau_factor = theory_config(query.theory)?.tau_factor;
    let start = query.rho.value * tau_factor;
    let mut tau = start;
    let mut time = 0f64;
    let mut last_strat = String::new();
    let mut results: Vec<SimResult> = Vec::new();
    let stop = convert_time(query.time);

    while time < query.time {
        let text = format!(
            "Pub {} · {}/{}",
            results.len() + 1,
            convert_time(time),
            stop
        );
        let done = time / query.time;
        progress.report(&text, done);
        let rho = LogNum {
            value: tau / tau_factor,
            sign: 1,
        };
        let res = single_sim(
            SingleSimQuery {
                theory: query.theory,
                strat: query.strat.clone(),
                theory_inputs: query.theory_inputs.clone(),
                rho,
                tau: Some(tau),
                sigma: query.sigma,
                settings: query.settings.clone(),
                cap: None,
                last_strat: Some(last_strat.clone()),
            },
            &progress.part(&text, done, done),
        )?
        .result;
        let pub_tau = res.pub_rho.value * tau_factor;
        if !is_greater(pub_tau, tau) || !is_greater(res.time, 0.) {
            return Err(format!(
                "Stopped at {rho}: no strat could make progress (last result: \"{}\")",
                res.strat
            ));
        }
        tau = pub_tau;
        time += res.time;
        last_strat = res.strat.split(' ').next().unwrap_or("").to_owned();
        finished_pub(progress, &res, start, tau, time);
        results.push(res);
    }

    chain_response(start, tau, time, results)
}

/** Sends a finished publication of a chain, with the chain's totals so far */
fn finished_pub(progress: &Progress, res: &SimResult, start_tau: f64, tau: f64, time: f64) {
    let delta_tau = tau - start_tau;
    progress.finished(&serde_json::json!({
        "kind": "pub",
        "result": res,
        "delta_tau": delta_tau,
        "average_rate": delta_tau / (time / 3600.),
        "total_time": time,
    }));
}

fn chain_response(
    start_tau: f64,
    end_tau: f64,
    time: f64,
    results: Vec<SimResult>,
) -> Result<ChainSimResponse, String> {
    let delta_tau = end_tau - start_tau;
    Ok(ChainSimResponse {
        results,
        delta_tau: LogNum {
            value: delta_tau,
            sign: 1,
        },
        average_rate: delta_tau / (time / 3600.),
        total_time: time,
    })
}

/**
 * For each step from rho up to cap - step, chains to cap and keeps the number of pubs with the
 * best tau/h (sim-3 `stepChainSim`)
 */
fn step_chain_sim(
    query: StepChainSimQuery,
    progress: &Progress,
) -> Result<StepSimResponse, String> {
    let tau_factor = theory_config(query.theory)?.tau_factor;
    let cap = query.cap.value * tau_factor;
    // sim-3 `calculateNextStep`: adds the step (times mult) to rho
    let next_step = |tau: f64, mult: f64| (tau / tau_factor + query.step.value * mult) * tau_factor;
    let mut tau = query.rho.value * tau_factor;
    let start = tau;
    let last_step = next_step(cap, -1.);
    if !is_greater(query.step.value, 0.) {
        return Err("The step length must be greater than 0".to_owned());
    }
    let total = std::iter::successors(Some(tau), |&t| Some(t + query.step.value))
        .take_while(|&t| t < last_step + 0.000001)
        .count();
    let mut results: Vec<SimResult> = Vec::new();

    while tau < last_step + 0.000001 {
        let rho = LogNum {
            value: tau / tau_factor,
            sign: 1,
        };
        let text = format!(
            "Step {}/{} · {}",
            count_text(results.len() + 1),
            count_text(total),
            log_to_exp(rho.value, 0)
        );
        // Each step's chain fills the share up to the next step
        let share = |tau: f64| (tau - start) / (last_step + query.step.value - start);
        let done = share(tau);
        progress.report(&text, done);
        let chain = chain_from_tau(
            ChainFromTau {
                theory: query.theory,
                strat: &query.strat,
                sigma: query.sigma,
                start_tau: tau,
                cap: query.cap,
                hard_cap: query.hard_cap,
                settings: &query.settings,
                theory_inputs: &query.theory_inputs,
            },
            &progress.part(&text, done, share(tau + query.step.value)),
        )?;

        let (mut tau_acc, mut time_acc) = (0f64, 0f64);
        let mut best = default_result();
        for (n, result) in chain.results.iter().enumerate() {
            tau_acc += result.delta_tau.value;
            time_acc += result.time;
            let pubs = n + 1;
            let candidate = SimResult {
                theory: query.theory,
                sigma: query.sigma,
                last_pub: rho,
                pub_rho: result.pub_rho,
                delta_tau: LogNum {
                    value: tau_acc,
                    sign: 1,
                },
                pub_multi: 1.,
                strat: format!("{pubs} pub{}", if pubs > 1 { "s" } else { "" }),
                tau_h: tau_acc / (time_acc / 3600.),
                time: time_acc,
                bought_vars: Vec::new(),
                theory_inputs: None,
                replay: None,
            };
            // getBestResult(bestRes, curRes): the earlier result wins ties
            if candidate.tau_h > best.tau_h {
                best = candidate;
            }
        }
        progress.finished(&serde_json::json!({ "kind": "result", "result": &best }));
        results.push(best);
        // sim-3 adds the step to tau
        tau += query.step.value;
    }

    Ok(StepSimResponse { results })
}

/** Every strat allowed at this rho (forced conditions only), sorted by tau/h (sim-3 `comparisonSim`) */
fn comparison_sim(
    query: ComparisonSimQuery,
    progress: &Progress,
) -> Result<StepSimResponse, String> {
    let tau = query.rho.value * theory_config(query.theory)?.tau_factor;
    let args = ConditionArgs {
        very_active: false,
        active: false,
        semi_idle: false,
        idle: false,
        tau,
        rho: query.rho.value,
        last_strat: "",
    };
    let strats: Vec<&str> = theory_config(query.theory)?
        .strats
        .iter()
        .filter(|(_, s)| s.forced.eval(&args))
        .map(|(name, _)| name.as_str())
        .collect();

    let mut results = Vec::new();
    let count = strats.len();
    for (i, strat) in strats.into_iter().enumerate() {
        progress.report(
            &format!("{strat} ({}/{count})", i + 1),
            i as f64 / count as f64,
        );
        let res = single_sim(
            SingleSimQuery {
                theory: query.theory,
                strat: strat.to_owned(),
                theory_inputs: query.theory_inputs.clone(),
                sigma: query.sigma,
                tau: None,
                rho: query.rho,
                cap: None,
                last_strat: None,
                settings: query.settings.clone(),
            },
            &progress.part("", 0., 0.),
        )?
        .result;
        progress.finished(&serde_json::json!({ "kind": "result", "result": &res }));
        results.push(res);
    }
    // Stable sort, highest tau/h first
    results.sort_by(|a, b| {
        b.tau_h
            .partial_cmp(&a.tau_h)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(StepSimResponse { results })
}

/** Fastest time to reach cap from each step below it (sim-3 `pubTableSim`) */
fn pub_table_sim(query: PubTableSimQuery, progress: &Progress) -> Result<PubTableResponse, String> {
    if !theory_config(query.theory)?.supports_rho {
        return Err("This mode is not supported for this theory".to_owned());
    }
    let (start, cap, step) = (query.rho.value, query.cap.value, query.step);
    if !is_greater(step, 0.) {
        return Err("The step length must be greater than 0".to_owned());
    }
    let mut current = cap - step;
    let first = current;
    let total = std::iter::successors(Some(current), |&c| Some(c - step))
        .take_while(|&c| c > start - 0.00001)
        .count();
    let mut last_strat = String::new();
    // Cap is reachable in 0 time
    let mut pub_table: Vec<[f64; 2]> = vec![[0., 0.]];

    let result = (|| -> Result<(), String> {
        while current > start - 0.00001 {
            let text = format!(
                "Row {}/{} · {}",
                count_text(pub_table.len()),
                count_text(total),
                log_to_exp(current, 0)
            );
            let done = (first - current) / (first - start + step);
            progress.report(&text, done);
            let collector = std::rc::Rc::new(std::cell::RefCell::new(PubTableCollector::new(
                current, step, cap,
            )));
            set_current_collector(Some(collector.clone()));
            let res = single_sim(
                SingleSimQuery {
                    theory: query.theory,
                    strat: query.strat.clone(),
                    theory_inputs: query.theory_inputs.clone(),
                    sigma: query.sigma,
                    tau: None,
                    rho: LogNum {
                        value: current,
                        sign: 1,
                    },
                    cap: None,
                    last_strat: Some(last_strat.clone()),
                    settings: query.settings.clone(),
                },
                &progress.part(&text, done, done),
            )?
            .result;
            set_current_collector(None);
            current -= step;
            last_strat = res.strat.split(' ').next().unwrap_or("").to_owned();

            let timings = &collector.borrow().timings;
            let (mut min_time, mut min_target) = (f64::INFINITY, f64::INFINITY);
            let len = pub_table.len();
            for i in (1..timings.len()).rev() {
                // We overshot
                if current + i as f64 * step > cap + 0.0000001 || i > len {
                    continue;
                }
                let candidate = pub_table[len - i][0] + timings[i];
                if candidate < min_time {
                    min_time = candidate;
                    min_target = cap - (len - i) as f64 * step;
                }
            }
            pub_table.push([min_time, min_target]);
            progress.finished(&serde_json::json!({
                "kind": "pub_table_row", "row": [min_time, min_target], "cap": cap, "start": start, "step": step,
            }));
        }
        Ok(())
    })();
    set_current_collector(None);
    result?;

    Ok(PubTableResponse {
        cap,
        start,
        step,
        pub_table,
    })
}

fn sim_all(query: SimAllQuery, progress: &Progress) -> Result<SimAllResponse, String> {
    let mut results: Vec<SimAllResult> = Vec::new();

    // The theories to simulate, first, so the progress can count them
    let mut todo: Vec<(TheoryType, LogNum)> = Vec::new();
    for (i, rho) in query.values.iter().enumerate() {
        let theory = if let Ok(theory) = TheoryType::try_from(i) {
            theory
        } else {
            continue;
        };
        if *rho <= lognum::ONE {
            continue;
        };
        // Completed custom theories (tau of e600 or more) can be left out (sim-3 `simAll`)
        let is_main = matches!(
            theory,
            TheoryType::T1
                | TheoryType::T2
                | TheoryType::T3
                | TheoryType::T4
                | TheoryType::T5
                | TheoryType::T6
                | TheoryType::T7
                | TheoryType::T8
        );
        if matches!(query.settings.completed_cts, CompletedCTs::No)
            && !is_main
            && rho.value * theory_config(theory)?.tau_factor >= 600.
        {
            continue;
        }
        todo.push((theory, *rho));
    }

    let count = todo.len() as f64;
    let both = query.settings.sim_all_strats == SimAllStrats::All;
    for (k, (theory, rho)) in todo.into_iter().enumerate() {
        let text = format!("{theory:?} ({}/{})", k + 1, count);
        let (done, next) = (k as f64 / count, (k + 1) as f64 / count);
        progress.report(&text, done);
        // With both active and idle strats, each gets half of the theory's share
        let half = if both { 0.5 } else { 1. };
        let theory_progress = progress.part(&text, done, next);
        let theory_inputs = query
            .theory_inputs
            .get(&theory)
            .cloned()
            .unwrap_or_default();

        let active_res = if query.settings.sim_all_strats != SimAllStrats::Idle {
            single_sim(
                SingleSimQuery {
                    theory,
                    theory_inputs: theory_inputs.clone(),
                    strat: if query.very_active {
                        "Best Overall".to_owned()
                    } else {
                        "Best Active".to_owned()
                    },
                    sigma: query.sigma,
                    tau: None,
                    rho,
                    cap: None,
                    last_strat: None,
                    settings: query.settings.clone(),
                },
                &theory_progress.part("active", 0., half),
            )?
            .result
        } else {
            SimResult::default()
        };

        let idle_res = if query.settings.sim_all_strats != SimAllStrats::Active {
            single_sim(
                SingleSimQuery {
                    theory,
                    theory_inputs: theory_inputs.clone(),
                    strat: if query.semi_idle {
                        "Best Semi-Idle".to_owned()
                    } else {
                        "Best Idle".to_owned()
                    },
                    sigma: query.sigma,
                    tau: None,
                    rho,
                    cap: None,
                    last_strat: None,
                    settings: query.settings.clone(),
                },
                &theory_progress.part("idle", 1. - half, 1.),
            )?
            .result
        } else {
            SimResult::default()
        };

        let res = SimAllResult {
            theory,
            ratio: if query.settings.sim_all_strats == SimAllStrats::All {
                active_res.tau_h / idle_res.tau_h
            } else {
                1.
            },
            last_pub: rho,
            active: active_res,
            idle: idle_res,
        };
        progress.finished(&serde_json::json!({
            "kind": "all_result", "result": &res, "sigma": query.sigma,
            "strat_type": query.settings.sim_all_strats, "completed_cts": query.settings.completed_cts,
        }));
        results.push(res);
    }

    Ok(SimAllResponse {
        sigma: query.sigma,
        strat_type: query.settings.sim_all_strats,
        completed_cts: query.settings.completed_cts,
        results,
    })
}

/**
 * Runs a query. `sink` receives progress during long simulations: a status text
 * ("Step 12/40 · 1e85 · T1Ratio (2/5)") and the fraction done (NaN when unknown).
 * `finished` receives each finished result as soon as it is done (see `Progress::finished`).
 */
pub fn simulate(
    query: SimQuery,
    sink: ProgressSink,
    finished: Option<FinishedSink>,
) -> Result<SimResponse, String> {
    let progress = &Progress::new(sink, finished);
    match query {
        SimQuery::Single(q) => Ok(SimResponse::Single(single_sim(q, progress)?)),
        SimQuery::Chain(q) => Ok(SimResponse::Chain(chain_sim(q, progress)?)),
        SimQuery::Step(q) => Ok(SimResponse::Step(step_sim(q, progress)?)),
        SimQuery::All(q) => Ok(SimResponse::All(sim_all(q, progress)?)),
        SimQuery::StepChain(q) => Ok(SimResponse::Step(step_chain_sim(q, progress)?)),
        SimQuery::Amount(q) => Ok(SimResponse::Chain(amount_sim(q, progress)?)),
        SimQuery::Time(q) => Ok(SimResponse::Chain(time_sim(q, progress)?)),
        SimQuery::Comparison(q) => Ok(SimResponse::Step(comparison_sim(q, progress)?)),
        SimQuery::PubTable(q) => Ok(SimResponse::PubTable(pub_table_sim(q, progress)?)),
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_time_formats_like_sim3() {
        assert_eq!(convert_time(0.), "00h00m");
        assert_eq!(convert_time(3661.), "01h01m");
        assert_eq!(convert_time(86400. + 3600. + 60.), "1d01h01m");
        // with years, minutes are left out
        assert_eq!(convert_time(31536000. + 2. * 86400.), "1y2d00h");
        // a million years and more are written in scientific notation
        assert_eq!(convert_time(1e6 * 31536000.), "1e6y00h");
    }

    const DATA_JSON: &str = include_str!("../../../src/Data/data.json");

    fn settings(sim_all_strats: &str) -> serde_json::Value {
        serde_json::json!({
            "dt": 1.5, "ddt": 1.0001, "bought_vars_delta": 5,
            "sim_all_strats": sim_all_strats, "completed_cts": "no", "show_a23": false, "show_unofficials": false
        })
    }

    fn query(query_type: &str, data: serde_json::Value) -> SimQuery {
        crate::set_config(DATA_JSON);
        serde_json::from_value(serde_json::json!({ "type": query_type, "data": data })).unwrap()
    }

    /** Runs a query and returns its response and the finished parts it sent while running */
    fn with_finished(q: SimQuery) -> (SimResponse, Vec<serde_json::Value>) {
        let finished = std::cell::RefCell::new(Vec::new());
        let on_finished = |json: &str| {
            finished
                .borrow_mut()
                .push(serde_json::from_str(json).unwrap())
        };
        let res = simulate(q, &|_, _| {}, Some(&on_finished)).unwrap();
        (res, finished.into_inner())
    }

    /** Purchases as comparable values */
    fn buys(list: &[VarBuy]) -> Vec<(String, i32, f64, &'static str, f64)> {
        list.iter()
            .map(|b| {
                (
                    b.var_name.to_string(),
                    b.level,
                    b.cost.value,
                    b.symbol,
                    b.timestamp,
                )
            })
            .collect()
    }

    #[test]
    fn results_are_sent_without_purchases_and_replay_to_the_same_ones() {
        // A chain: each publication starts from the tau of the previous one
        let q = query(
            "chain",
            serde_json::json!({
                "theory": "T1", "strat": "Best Overall", "sigma": 100, "rho": 60, "cap": 80,
                "hard_cap": true, "settings": settings("all")
            }),
        );
        let SimResponse::Chain(chain) = simulate(q, &|_, _| {}, None).unwrap() else {
            panic!("expected a chain response")
        };
        assert!(chain.results.len() > 1);
        for res in &chain.results {
            let replay = res.replay.as_ref().expect("replay info");
            assert!(!res.bought_vars.is_empty());
            assert_eq!(
                buys(&replay_bought_vars(replay).unwrap()),
                buys(&res.bought_vars)
            );
            // Sent without the purchases, but with their count and the replay info
            let sent = serde_json::to_value(res).unwrap();
            assert_eq!(sent["bought_vars"], serde_json::json!([]));
            assert_eq!(sent["bought_vars_count"], res.bought_vars.len());
            let replay_back: Replay = serde_json::from_value(sent["replay"].clone()).unwrap();
            assert_eq!(
                buys(&replay_bought_vars(&replay_back).unwrap()),
                buys(&res.bought_vars)
            );
        }
        // A theory input (MF's reset depth) is replayed too
        let q = query(
            "single",
            serde_json::json!({
                "theory": "MF", "strat": "MFd", "sigma": 100, "rho": 30, "cap": null,
                "last_strat": null, "settings": settings("all"), "theory_inputs": { "depth": "1" }
            }),
        );
        let SimResponse::Single(single) = simulate(q, &|_, _| {}, None).unwrap() else {
            panic!("expected a single response")
        };
        let res = single.result;
        assert!(res.strat.contains("Depth: 1"), "{}", res.strat);
        let replayed = replay_bought_vars(res.replay.as_ref().unwrap()).unwrap();
        assert_eq!(buys(&replayed), buys(&res.bought_vars));
    }

    #[test]
    fn steps_need_a_positive_step() {
        let q = query(
            "step",
            serde_json::json!({
                "theory": "T1", "strat": "T1", "sigma": 100, "rho": 60, "cap": 80, "step": 0,
                "settings": settings("all")
            }),
        );
        let err = simulate(q, &|_, _| {}, None).unwrap_err();
        assert!(err.contains("greater than 0"), "{err}");
    }

    #[test]
    fn finished_results_are_sent_while_simulating() {
        // Comparison: one part per strat, the same results as the response (before sorting)
        let (res, parts) = with_finished(query(
            "comparison",
            serde_json::json!({ "theory": "T1", "sigma": 100, "rho": 60, "settings": settings("all") }),
        ));
        let SimResponse::Step(res) = res else {
            panic!("expected a step response")
        };
        assert_eq!(parts.len(), res.results.len());
        assert!(parts.iter().all(|p| p["kind"] == "result"));
        let mut sent: Vec<String> = parts
            .iter()
            .map(|p| p["result"]["strat"].to_string())
            .collect();
        let mut got: Vec<String> = res
            .results
            .iter()
            .map(|r| format!("{:?}", r.strat))
            .collect();
        sent.sort();
        got.sort();
        assert_eq!(sent, got);

        // Chain: one part per publication, the last one with the chain's totals
        let (res, parts) = with_finished(query(
            "chain",
            serde_json::json!({
                "theory": "T1", "strat": "Best Overall", "sigma": 100, "rho": 60, "cap": 75,
                "hard_cap": false, "settings": settings("all")
            }),
        ));
        let SimResponse::Chain(res) = res else {
            panic!("expected a chain response")
        };
        assert_eq!(parts.len(), res.results.len());
        let last = parts.last().unwrap();
        assert_eq!(last["kind"], "pub");
        assert_eq!(last["total_time"].as_f64(), Some(res.total_time));
        assert_eq!(last["average_rate"].as_f64(), Some(res.average_rate));
    }

    /** Single sim of a strat category at e500, sigma 500 */
    fn best(theory: &str, strat: &str) -> SimResult {
        let q = query(
            "single",
            serde_json::json!({ "theory": theory, "strat": strat, "sigma": 500, "rho": 500.0, "settings": settings("all") }),
        );
        match simulate(q, &|_, _| {}, None).unwrap() {
            SimResponse::Single(r) => r.result,
            _ => panic!("expected a single sim response"),
        }
    }

    fn all(very_active: bool, semi_idle: bool, strats: &str) -> SimAllResponse {
        // One more value than there are theories (the extra one is ignored); 0 = don't simulate
        let mut values = vec![0.0; PORTED_THEORIES.len() + 1];
        values[0] = 500.;
        values[1] = 500.;
        values[PORTED_THEORIES.len()] = 500.;
        let q = query(
            "all",
            serde_json::json!({
                "values": values, "sigma": 500, "very_active": very_active, "semi_idle": semi_idle,
                "settings": settings(strats)
            }),
        );
        let progress = std::cell::RefCell::new(Vec::new());
        let res = match simulate(
            q,
            &|msg: &str, fraction: f64| progress.borrow_mut().push((msg.to_owned(), fraction)),
            None,
        )
        .unwrap()
        {
            SimResponse::All(r) => r,
            _ => panic!("expected an all response"),
        };
        let progress = progress.borrow();
        // One message per theory, then one per strat of its "Best" categories
        let theories: Vec<&str> = progress
            .iter()
            .map(|(msg, _)| msg.as_str())
            .filter(|msg| !msg.contains('·'))
            .collect();
        assert_eq!(theories, ["T1 (1/2)", "T2 (2/2)"]);
        assert!(
            progress
                .iter()
                .any(|(msg, _)| msg.starts_with("T2 (2/2) · ") && msg.ends_with(')')),
            "{progress:?}"
        );
        // The fraction done only grows, from 0 to under 1
        assert_eq!(progress[0].1, 0.);
        assert!(
            progress.windows(2).all(|w| w[0].1 <= w[1].1),
            "{progress:?}"
        );
        assert!(progress.last().unwrap().1 < 1.);
        res
    }

    #[test]
    fn all_mode_uses_the_chosen_strat_categories() {
        let res = all(true, false, "all");
        let theories: Vec<TheoryType> = res.results.iter().map(|r| r.theory).collect();
        assert_eq!(theories, [TheoryType::T1, TheoryType::T2]);
        let t1 = &res.results[0];
        // very active -> "Best Overall"; not semi-idle -> "Best Idle"
        assert_eq!(t1.active.strat, best("T1", "Best Overall").strat);
        assert_eq!(t1.idle.strat, best("T1", "Best Idle").strat);
        assert_eq!(t1.ratio, t1.active.tau_h / t1.idle.tau_h);

        let res = all(false, true, "all");
        assert_eq!(res.results[0].active.strat, best("T1", "Best Active").strat);
        assert_eq!(
            res.results[0].idle.strat,
            best("T1", "Best Semi-Idle").strat
        );
    }

    #[test]
    fn all_mode_can_skip_active_or_idle_strats() {
        let res = all(false, true, "active");
        let t1 = &res.results[0];
        assert_eq!(t1.idle.strat, SimResult::default().strat);
        assert_eq!(t1.ratio, 1.);
        let res = all(false, true, "idle");
        assert_eq!(res.results[0].active.strat, SimResult::default().strat);
        assert_eq!(res.results[0].ratio, 1.);
    }

    #[test]
    fn pub_table_rejects_a_non_positive_step() {
        let q = query(
            "pub_table",
            serde_json::json!({
                "theory": "T1", "strat": "T1", "sigma": 90, "rho": 150.0, "cap": 160.0, "step": 0.0,
                "settings": settings("all")
            }),
        );
        let err = simulate(q, &|_, _| {}, None).unwrap_err();
        assert!(err.contains("step length"), "{err}");
    }

    #[test]
    fn ported_theories_are_runnable() {
        let input = TheoryInput {
            sigma: 60,
            last_pub_rho: 50.,
            last_pub_tau: None,
            rho_cap: f64::INFINITY,
            params: SimParams::default(),
        };
        for theory in PORTED_THEORIES {
            assert!(is_ported(theory));
            let strat = format!("{theory:?}");
            let res = run_strat(theory, &strat, &input).unwrap();
            assert!(
                res.tau_h > 0. && res.pub_rho.value > 50.,
                "{theory:?}: {res:?}"
            );
        }
        assert!(run_strat(TheoryType::T1, "NotAStrat", &input).is_err());
    }
}
