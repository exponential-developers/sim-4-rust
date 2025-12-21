#[derive(Debug, Clone)]
pub enum SimAllStrats {
    ALL,
    ACTIVE,
    IDLE
}

#[derive(Debug, Clone)]
pub enum CompletedCTs {
    IN,
    END,
    NO
}

#[derive(Debug, Clone)]
pub struct SimSettings {
    dt: f64,
    ddt: f64,
    mf_reset_depth: i32,
    bought_vars_delta: i32,
    theme: String,
    sim_all_strats: SimAllStrats,
    completed_cts: CompletedCTs,
    show_a23: bool,
    show_unofficials: bool
}