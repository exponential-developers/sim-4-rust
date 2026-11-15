use crate::CONFIG;
use crate::api::{
    condition::ConditionArgs,
    config::{Config, ConfigTheories},
};
use crate::api::{config::TheoryType, query::*, response::*};
use crate::theories::{
    bap::{BapStrat, simulate_bap},
    bt::{BtStrat, simulate_bt},
    csr2::{Csr2Strat, simulate_csr2},
    ef::{EfStrat, simulate_ef},
    fi::{FiStrat, simulate_fi},
    fp::{FpStrat, simulate_fp},
    mf::{MfStrat, simulate_mf},
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

/** Theories that have been ported to the Rust sim, in data.json order */
pub const PORTED_THEORIES: [TheoryType; 19] = [
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
    TheoryType::TC,
    TheoryType::BT,
];

/** Whether a theory has been ported to the Rust sim */
pub fn is_ported(theory: TheoryType) -> bool {
    PORTED_THEORIES.contains(&theory)
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
    Ok(theory_config(theory)?
        .strats
        .iter()
        .filter(|(_, s)| s.filter.eval(&args) && s.forced.eval(&args))
        .map(|(name, _)| name.as_str())
        .collect())
}

fn single_sim(query: SingleSimQuery) -> Result<SingleSimResponse, String> {
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
            mf_reset_depth: query.settings.mf_reset_depth,
        },
    };

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
    for strat in strats {
        let res = run_strat(query.theory, strat, &input)?;
        if res.tau_h > best.tau_h {
            best = res;
        }
    }

    Ok(SingleSimResponse { result: best })
}

fn chain_sim(query: ChainSimQuery, progress: Progress) -> Result<ChainSimResponse, String> {
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
}

/** sim-3 `chainSim`: simulates publications until tau reaches the cap */
fn chain_from_tau(query: ChainFromTau, progress: Progress) -> Result<ChainSimResponse, String> {
    let tau_factor = theory_config(query.theory)?.tau_factor;
    let start = query.start_tau;
    let cap = query.cap.value * tau_factor;
    let mut tau = start;
    let mut time = 0f64;
    let mut last_strat = String::new();

    let mut results: Vec<SimResult> = Vec::new();

    while tau < cap {
        let rho = LogNum {
            value: tau / tau_factor,
            sign: 1,
        };
        progress(&format!(
            "Simulating {}/{}",
            log_to_exp(rho.value, 0),
            log_to_exp(query.cap.value, 0)
        ));
        let res = single_sim(SingleSimQuery {
            theory: query.theory,
            strat: query.strat.to_owned(),
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
        })?
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
        results.push(res);
    }

    chain_response(start, tau, time, results)
}

fn step_sim(query: StepSimQuery, progress: Progress) -> Result<StepSimResponse, String> {
    let tau_factor = theory_config(query.theory)?.tau_factor;
    let cap = query.cap.value * tau_factor;
    let mut tau = query.rho.value * tau_factor;
    let mut last_strat = String::new();

    let mut results: Vec<SimResult> = Vec::new();

    // Same tolerance as sim-3 (cap + 0.00001 in log10)
    while tau < cap + 0.00001 {
        let rho = LogNum {
            value: tau / tau_factor,
            sign: 1,
        };
        progress(&format!(
            "Simulating {}/{}",
            log_to_exp(rho.value, 0),
            log_to_exp(query.cap.value, 0)
        ));
        let res = single_sim(SingleSimQuery {
            theory: query.theory,
            strat: query.strat.clone(),
            rho,
            tau: Some(tau),
            sigma: query.sigma,
            settings: query.settings.clone(),
            cap: None,
            last_strat: Some(last_strat.clone()),
        })?
        .result;

        // The next step adds the step to rho
        tau = (tau / tau_factor + query.step.value) * tau_factor;
        last_strat.clear();
        last_strat.push_str(res.strat.split(' ').next().unwrap_or(""));
        results.push(res);
    }

    Ok(StepSimResponse { results })
}

/** sim-3 `convertTime`: "1y2d03h04m" style duration */
fn convert_time(secs: f64) -> String {
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
fn amount_sim(query: AmountSimQuery, progress: Progress) -> Result<ChainSimResponse, String> {
    let tau_factor = theory_config(query.theory)?.tau_factor;
    let start = query.rho.value * tau_factor;
    let mut tau = start;
    let mut time = 0f64;
    let mut last_strat = String::new();
    let mut results: Vec<SimResult> = Vec::new();

    for i in 0..query.amount {
        progress(&format!("Simulating {}/{} pubs", i + 1, query.amount));
        let res = single_sim(SingleSimQuery {
            theory: query.theory,
            strat: query.strat.clone(),
            rho: LogNum {
                value: tau / tau_factor,
                sign: 1,
            },
            tau: Some(tau),
            sigma: query.sigma,
            settings: query.settings.clone(),
            cap: None,
            last_strat: Some(last_strat.clone()),
        })?
        .result;
        tau = res.pub_rho.value * tau_factor;
        time += res.time;
        last_strat = res.strat.split(' ').next().unwrap_or("").to_owned();
        results.push(res);
    }

    chain_response(start, tau, time, results)
}

/** Chain until the total time is reached (sim-3 `timeSim`) */
fn time_sim(query: TimeSimQuery, progress: Progress) -> Result<ChainSimResponse, String> {
    let tau_factor = theory_config(query.theory)?.tau_factor;
    let start = query.rho.value * tau_factor;
    let mut tau = start;
    let mut time = 0f64;
    let mut last_strat = String::new();
    let mut results: Vec<SimResult> = Vec::new();
    let stop = convert_time(query.time);

    while time < query.time {
        progress(&format!("Simulating {}/{}", convert_time(time), stop));
        let rho = LogNum {
            value: tau / tau_factor,
            sign: 1,
        };
        let res = single_sim(SingleSimQuery {
            theory: query.theory,
            strat: query.strat.clone(),
            rho,
            tau: Some(tau),
            sigma: query.sigma,
            settings: query.settings.clone(),
            cap: None,
            last_strat: Some(last_strat.clone()),
        })?
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
        results.push(res);
    }

    chain_response(start, tau, time, results)
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
fn step_chain_sim(query: StepChainSimQuery, progress: Progress) -> Result<StepSimResponse, String> {
    let tau_factor = theory_config(query.theory)?.tau_factor;
    let cap = query.cap.value * tau_factor;
    // sim-3 `calculateNextStep`: adds the step (times mult) to rho
    let next_step = |tau: f64, mult: f64| (tau / tau_factor + query.step.value * mult) * tau_factor;
    let mut tau = query.rho.value * tau_factor;
    let mut results: Vec<SimResult> = Vec::new();

    while tau < next_step(cap, -1.) + 0.000001 {
        let rho = LogNum {
            value: tau / tau_factor,
            sign: 1,
        };
        progress(&format!(
            "Simulating {}/{}",
            log_to_exp(rho.value, 0),
            log_to_exp(query.cap.value, 0)
        ));
        let chain = chain_from_tau(
            ChainFromTau {
                theory: query.theory,
                strat: &query.strat,
                sigma: query.sigma,
                start_tau: tau,
                cap: query.cap,
                hard_cap: query.hard_cap,
                settings: &query.settings,
            },
            &|_| {},
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
            };
            // getBestResult(bestRes, curRes): the earlier result wins ties
            if candidate.tau_h > best.tau_h {
                best = candidate;
            }
        }
        results.push(best);
        // sim-3 adds the step to tau
        tau += query.step.value;
    }

    Ok(StepSimResponse { results })
}

/** Every strat allowed at this rho (forced conditions only), sorted by tau/h (sim-3 `comparisonSim`) */
fn comparison_sim(query: ComparisonSimQuery) -> Result<StepSimResponse, String> {
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
    for strat in strats {
        results.push(
            single_sim(SingleSimQuery {
                theory: query.theory,
                strat: strat.to_owned(),
                sigma: query.sigma,
                tau: None,
                rho: query.rho,
                cap: None,
                last_strat: None,
                settings: query.settings.clone(),
            })?
            .result,
        );
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
fn pub_table_sim(query: PubTableSimQuery, progress: Progress) -> Result<PubTableResponse, String> {
    let (start, cap, step) = (query.rho.value, query.cap.value, query.step);
    if !is_greater(step, 0.) {
        return Err("The step length must be greater than 0".to_owned());
    }
    let mut current = cap - step;
    let mut last_strat = String::new();
    // Cap is reachable in 0 time
    let mut pub_table: Vec<[f64; 2]> = vec![[0., 0.]];

    let result = (|| -> Result<(), String> {
        while current > start - 0.00001 {
            progress(&format!(
                "Simulating {}/{}",
                log_to_exp(current, 0),
                log_to_exp(start, 0)
            ));
            let collector = std::rc::Rc::new(std::cell::RefCell::new(PubTableCollector::new(
                current, step, cap,
            )));
            set_current_collector(Some(collector.clone()));
            let res = single_sim(SingleSimQuery {
                theory: query.theory,
                strat: query.strat.clone(),
                sigma: query.sigma,
                tau: None,
                rho: LogNum {
                    value: current,
                    sign: 1,
                },
                cap: None,
                last_strat: Some(last_strat.clone()),
                settings: query.settings.clone(),
            })?
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

fn sim_all(query: SimAllQuery, progress: Progress) -> Result<SimAllResponse, String> {
    let mut results: Vec<SimAllResult> = Vec::new();

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
        progress(&format!("Simulating {theory:?}"));

        let active_res = if query.settings.sim_all_strats != SimAllStrats::Idle {
            single_sim(SingleSimQuery {
                theory,
                strat: if query.very_active {
                    "Best Overall".to_owned()
                } else {
                    "Best Active".to_owned()
                },
                sigma: query.sigma,
                tau: None,
                rho: *rho,
                cap: None,
                last_strat: None,
                settings: query.settings.clone(),
            })?
            .result
        } else {
            SimResult::default()
        };

        let idle_res = if query.settings.sim_all_strats != SimAllStrats::Active {
            single_sim(SingleSimQuery {
                theory,
                strat: if query.semi_idle {
                    "Best Semi-Idle".to_owned()
                } else {
                    "Best Idle".to_owned()
                },
                sigma: query.sigma,
                tau: None,
                rho: *rho,
                cap: None,
                last_strat: None,
                settings: query.settings.clone(),
            })?
            .result
        } else {
            SimResult::default()
        };

        results.push(SimAllResult {
            theory,
            ratio: if query.settings.sim_all_strats == SimAllStrats::All {
                active_res.tau_h / idle_res.tau_h
            } else {
                1.
            },
            last_pub: *rho,
            active: active_res,
            idle: idle_res,
        })
    }

    Ok(SimAllResponse {
        sigma: query.sigma,
        strat_type: query.settings.sim_all_strats,
        completed_cts: query.settings.completed_cts,
        results,
    })
}

/** Receives progress messages ("Simulating 1e85/1e150") during long simulations */
pub type Progress<'a> = &'a dyn Fn(&str);

pub fn simulate(query: SimQuery, progress: Progress) -> Result<SimResponse, String> {
    match query {
        SimQuery::Single(q) => Ok(SimResponse::Single(single_sim(q)?)),
        SimQuery::Chain(q) => Ok(SimResponse::Chain(chain_sim(q, progress)?)),
        SimQuery::Step(q) => Ok(SimResponse::Step(step_sim(q, progress)?)),
        SimQuery::All(q) => Ok(SimResponse::All(sim_all(q, progress)?)),
        SimQuery::StepChain(q) => Ok(SimResponse::Step(step_chain_sim(q, progress)?)),
        SimQuery::Amount(q) => Ok(SimResponse::Chain(amount_sim(q, progress)?)),
        SimQuery::Time(q) => Ok(SimResponse::Chain(time_sim(q, progress)?)),
        SimQuery::Comparison(q) => Ok(SimResponse::Step(comparison_sim(q)?)),
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
            "dt": 1.5, "ddt": 1.0001, "mf_reset_depth": 0, "bought_vars_delta": 5,
            "sim_all_strats": sim_all_strats, "completed_cts": "no", "show_a23": false, "show_unofficials": false
        })
    }

    fn query(query_type: &str, data: serde_json::Value) -> SimQuery {
        crate::set_config(DATA_JSON);
        serde_json::from_value(serde_json::json!({ "type": query_type, "data": data })).unwrap()
    }

    /** Single sim of a strat category at e500, sigma 500 */
    fn best(theory: &str, strat: &str) -> SimResult {
        let q = query(
            "single",
            serde_json::json!({ "theory": theory, "strat": strat, "sigma": 500, "rho": 500.0, "settings": settings("all") }),
        );
        match simulate(q, &|_| {}).unwrap() {
            SimResponse::Single(r) => r.result,
            _ => panic!("expected a single sim response"),
        }
    }

    fn all(very_active: bool, semi_idle: bool, strats: &str) -> SimAllResponse {
        // 20 values: more than there are theories (the extra one is ignored); 0 = don't simulate
        let mut values = vec![0.0; 20];
        values[0] = 500.;
        values[1] = 500.;
        values[19] = 500.;
        let q = query(
            "all",
            serde_json::json!({
                "values": values, "sigma": 500, "very_active": very_active, "semi_idle": semi_idle,
                "settings": settings(strats)
            }),
        );
        let progress = std::cell::RefCell::new(Vec::new());
        let res =
            match simulate(q, &|msg: &str| progress.borrow_mut().push(msg.to_owned())).unwrap() {
                SimResponse::All(r) => r,
                _ => panic!("expected an all response"),
            };
        assert_eq!(*progress.borrow(), ["Simulating T1", "Simulating T2"]);
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
        let err = simulate(q, &|_| {}).unwrap_err();
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
