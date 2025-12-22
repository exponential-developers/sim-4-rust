use crate::api::{
    query::*, response::*
};
use crate::utils::lognum::LogNum;
use crate::utils::result::*;

fn single_sim(query: SingleSimQuery) -> Result<SingleSimResponse, String> {
    Err("Not implemented".to_owned())
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

        rho = res.pub_rho;
        time += res.time;
        last_strat.clear();
        last_strat.push_str(res.strat.split_whitespace().next().unwrap_or(""));
        results.push(res);
    }

    //...

    Err("Not Implemented".to_owned())
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
    Err("Not implemented".to_owned())
}

pub fn simulate(query: SimQuery) -> Result<SimResponse, String> {
    match query {
        SimQuery::Single(q) => Ok(SimResponse::Single(single_sim(q)?)),
        SimQuery::Chain(q) => Ok(SimResponse::Chain(chain_sim(q)?)),
        SimQuery::Step(q) => Ok(SimResponse::Step(step_sim(q)?)),
        SimQuery::All(q) => Ok(SimResponse::All(sim_all(q)?))
    }
}