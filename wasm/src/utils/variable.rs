use crate::utils::{
    cost::Cost,
    lognum::{self, LogNum},
    value::Value,
    //currency::Currency
};

/** Holds a representation of a upgradable variable */
#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub cost_model: Cost,
    pub value_model: Value,
    // may or may not be implemented
    //pub currency: &'a Currency,
    pub level: i32,
    pub cost: LogNum,
    pub value: LogNum,

    // Hotab coasting (mirrors sim-3's Variable coasting fields)
    /** Max level bought when the strat was run without coasting (f64::INFINITY if unset) */
    pub original_cap: f64,
    /** Level at which coasting rules start applying (f64::INFINITY if unset) */
    pub start_cap_at: f64,
    /** Whether the variable should be bought according to coasting rules */
    pub should_buy: bool,
    /** Whether a fork should be created to test not buying this variable anymore */
    pub should_fork: bool,
}

impl Variable {
    /** Creates a new variable */
    pub fn new(name: &str, cost_model: Cost, value_model: Value) -> Self {
        let mut var = Variable {
            name: name.to_owned(),
            cost_model,
            value_model,
            level: 0,
            cost: lognum::ZERO,
            value: lognum::ZERO,
            original_cap: f64::INFINITY,
            start_cap_at: f64::INFINITY,
            should_buy: true,
            should_fork: false,
        };
        var.compute_from_zero();

        var
    }

    /** Updates the cost and the value */
    pub fn compute_from_zero(&mut self) {
        self.cost = self.cost_model.get_cost(self.level);
        self.value = self.value_model.compute_from_zero(self.level);
    }

    /** Performs a purchase of the variable, updating its cost and value */
    pub fn buy(&mut self) {
        self.value = self.value_model.compute_next(self.value, self.level);
        self.level += 1;
        self.cost = self.cost_model.get_cost(self.level);
    }

    /** Sets the variable to the given level, updating its cost and value */
    pub fn set(&mut self, level: i32) {
        self.level = level;
        self.compute_from_zero();
    }

    /** Sets the original cap (level reached when running the strat without coasting) */
    pub fn set_original_cap(&mut self, original_cap: i32) {
        self.original_cap = original_cap as f64;
    }

    /** Sets the coasting cap to original cap - cap_delta (at least 1) */
    pub fn configure_cap(&mut self, cap_delta: i32) {
        self.start_cap_at = (self.original_cap - cap_delta as f64).max(1.);
    }

    /** True if the coasting cap is reached */
    pub fn coasting_cap_reached(&self) -> bool {
        self.level as f64 >= self.start_cap_at
    }

    /** True if the level is strictly above the original cap */
    pub fn above_original_cap(&self) -> bool {
        self.level as f64 > self.original_cap
    }

    /** Stop buying this variable (coasting) */
    pub fn stop_buying(&mut self) {
        self.should_buy = false;
    }

    /** Text shown in the strat column for the last level bought */
    pub fn prepare_extra_for_cap(&self, last_level: i32) -> String {
        let actual_last = if last_level != 0 {
            last_level
        } else {
            self.level
        };
        format!(" {}: {}", self.name, actual_last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var() -> Variable {
        Variable::new(
            "q2",
            Cost::new_exponential(100., 10.),
            Value::new_exponential(2.),
        )
    }

    #[test]
    fn buying_updates_level_cost_and_value() {
        let mut v = var();
        assert_eq!(v.level, 0);
        v.buy();
        v.buy();
        assert_eq!(v.level, 2);
        assert!((v.cost.value - 4.).abs() < 1e-12);
        assert!((v.value.value - 2. * 2f64.log10()).abs() < 1e-12);
        v.set(5);
        assert_eq!(v.level, 5);
        assert!((v.cost.value - 7.).abs() < 1e-12);
    }

    #[test]
    fn coasting_caps() {
        let mut v = var();
        // unset: no cap is ever reached
        assert!(!v.coasting_cap_reached() && !v.above_original_cap());

        v.set_original_cap(10);
        v.configure_cap(3);
        assert_eq!(v.start_cap_at, 7.);
        v.set(6);
        assert!(!v.coasting_cap_reached());
        v.set(7);
        assert!(v.coasting_cap_reached());
        v.set(10);
        assert!(!v.above_original_cap());
        v.set(11);
        assert!(v.above_original_cap());

        // the cap never goes below level 1
        v.configure_cap(50);
        assert_eq!(v.start_cap_at, 1.);

        assert!(v.should_buy);
        v.stop_buying();
        assert!(!v.should_buy);
    }

    #[test]
    fn strat_text_shows_last_level() {
        let mut v = var();
        v.set(42);
        assert_eq!(v.prepare_extra_for_cap(30), " q2: 30");
        assert_eq!(v.prepare_extra_for_cap(0), " q2: 42"); // 0 = never bought: use the current level
    }
}
