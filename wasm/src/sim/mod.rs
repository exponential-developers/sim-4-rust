use crate::CONFIG;
use crate::api::{
    config::TheoryType,
    query::*, 
    response::*
};
use crate::utils::{
    lognum::{self,LogNum},
    result::*,
    settings::*
};


fn single_sim(query: SingleSimQuery) -> Result<SingleSimResponse, String> {
    Ok(SingleSimResponse { result: SimResult::default() })
}

fn chain_sim(query: ChainSimQuery) -> Result<ChainSimResponse, String> {
    let mut rho = query.rho;
    let mut time = 0f64;
    let mut last_strat = String::new();

    let mut results: Vec<SimResult> = Vec::new();

    while rho < query.cap {
        let res = single_sim(SingleSimQuery {
            theory: query.theory.clone(),
            strat: query.strat.clone(),
            rho: rho,
            sigma: query.sigma,
            settings: query.settings.clone(),
            cap: if query.hard_cap { Some(query.cap) } else { None },
            last_strat: Some(last_strat.clone())
        })?.result;

        // Test result to test functionnality
        let res = SimResult {
            theory: query.theory,
            sigma: query.sigma,
            last_pub: rho,
            pub_rho: rho * LogNum::from(10.),
            delta_tau: LogNum::from(10.),
            pub_multi: 2.718,
            strat: "TEST".to_owned(),
            tau_h: 3.1415,
            time: 3600.,
            bought_vars: Vec::new()
        };

        rho = res.pub_rho;
        time += res.time;
        last_strat.clear();
        last_strat.push_str(res.strat.split_whitespace().next().unwrap_or(""));
        results.push(res);
    }

    let delta_tau = (rho / query.rho).powf64(
        CONFIG.get().unwrap().theories.get(&query.theory).unwrap().tau_factor
    );

    Ok(ChainSimResponse {
        results,
        delta_tau,
        average_rate: delta_tau.value / (time / 3600.),
        total_time: time
    })
}

fn step_sim(query: StepSimQuery) -> Result<StepSimResponse, String> {
    let mut rho = query.rho;
    let mut last_strat = String::new();

    let mut results: Vec<SimResult> = Vec::new();

    while rho < query.cap * LogNum::from(1.001) {
        let res = single_sim(SingleSimQuery {
            theory: query.theory.clone(),
            strat: query.strat.clone(),
            rho: rho,
            sigma: query.sigma,
            settings: query.settings.clone(),
            cap: None,
            last_strat: Some(last_strat.clone())
        })?.result;

        rho *= query.step;
        last_strat.clear();
        last_strat.push_str(res.strat.split_whitespace().next().unwrap_or(""));
        results.push(res);
    }

    Ok(StepSimResponse { results })
}

fn sim_all(query: SimAllQuery) -> Result<SimAllResponse, String> {
    let mut results: Vec<SimAllResult> = Vec::new();

    for (i, rho) in query.values.iter().enumerate() {
        let theory = 
            if let Ok(theory) = TheoryType::try_from(i) { theory } 
            else { continue; };
        if *rho <= lognum::ONE { continue; };

        let active_res = if query.settings.sim_all_strats != SimAllStrats::Idle {
            single_sim(SingleSimQuery {
                theory,
                strat: if query.very_active { "Best Overall".to_owned() } 
                        else { "Best Active".to_owned() },
                sigma: query.sigma,
                rho: *rho,
                cap: None,
                last_strat: None,
                settings: query.settings.clone()
            })?.result
        } else { SimResult::default() };

        let idle_res = if query.settings.sim_all_strats != SimAllStrats::Active {
            single_sim(SingleSimQuery {
                theory,
                strat: if query.semi_idle { "Best Semi-Idle".to_owned() } 
                        else { "Best Idle".to_owned() },
                sigma: query.sigma,
                rho: *rho,
                cap: None,
                last_strat: None,
                settings: query.settings.clone()
            })?.result
        } else { SimResult::default() };

        results.push(SimAllResult {
            theory,
            ratio: if query.settings.sim_all_strats == SimAllStrats::All {
                active_res.tau_h / idle_res.tau_h
            } else { 1. },
            last_pub: *rho,
            active: active_res,
            idle: idle_res
        })
    }

    Ok(SimAllResponse {
        sigma: query.sigma,
        strat_type: query.settings.sim_all_strats,
        completed_cts: query.settings.completed_cts,
        results
    })
}

pub fn simulate(query: SimQuery) -> Result<SimResponse, String> {
    match query {
        SimQuery::Single(q) => Ok(SimResponse::Single(single_sim(q)?)),
        SimQuery::Chain(q) => Ok(SimResponse::Chain(chain_sim(q)?)),
        SimQuery::Step(q) => Ok(SimResponse::Step(step_sim(q)?)),
        SimQuery::All(q) => Ok(SimResponse::All(sim_all(q)?))
    }
}