use crate::api::{
    query::*, response::*
};

pub fn simulate(query: SimQuery) -> Result<SimResponse, String> {
    match query {
        SimQuery::Single(q) => Err("Not implemented".to_owned()),
        SimQuery::Chain(q) => Err("Not implemented".to_owned()),
        SimQuery::Step(q) => Err("Not implemented".to_owned()),
        SimQuery::All(q) => Err("Not implemented".to_owned())
    }
}