/*!
 * Cost models
 */

use dyn_clone::DynClone;
use num::Float;
use std::fmt::Debug;

use crate::utils::lognum::{self, LogNum};

/** Trait for cost models */
pub trait CostTrait: DynClone + Debug {
    /**
     * Cost to get from (level-1) to (level)
     */
    fn get_cost_to(&self, level: i32) -> LogNum;
    /**
     * Cost to get from level 0 to (level)
     */
    fn get_total_cost_to(&self, level: i32) -> LogNum;
    /**
     * Cost to get from (level) to (level+1)
     */
    #[inline]
    fn get_cost(&self, current_level: i32) -> LogNum {
        self.get_cost_to(current_level + 1)
    }
}
dyn_clone::clone_trait_object!(CostTrait);

#[derive(Debug, Clone, Copy)]
pub struct CompositeCost<T: CostTrait, U: CostTrait> {
    pub model1: T,
    pub model2: U,
    pub cutoff: i32,
}

impl<T: CostTrait, U: CostTrait> CompositeCost<T, U> {
    pub fn new(model1: T, model2: U, cutoff: i32) -> Self {
        CompositeCost {
            model1,
            model2,
            cutoff,
        }
    }
}

impl<T: CostTrait + Clone, U: CostTrait + Clone> CostTrait for CompositeCost<T, U> {
    fn get_cost_to(&self, level: i32) -> LogNum {
        if level <= 0 {
            lognum::ZERO
        } else if level <= self.cutoff {
            self.model1.get_cost_to(level)
        } else {
            self.model2.get_cost_to(level - self.cutoff)
        }
    }

    fn get_total_cost_to(&self, level: i32) -> LogNum {
        if level <= 0 {
            lognum::ZERO
        } else if level <= self.cutoff {
            self.model1.get_total_cost_to(level)
        } else {
            self.model2.get_total_cost_to(level - self.cutoff)
                + self.model1.get_total_cost_to(self.cutoff)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExponentialCost {
    coefficient: LogNum,
    base: LogNum,
}

impl ExponentialCost {
    pub fn new(coefficient: impl Into<LogNum>, base: impl Into<LogNum>) -> Self {
        ExponentialCost {
            coefficient: coefficient.into(),
            base: base.into(),
        }
    }

    pub fn new_log2(coefficient: impl Into<LogNum>, base: impl Into<LogNum>) -> Self {
        ExponentialCost {
            coefficient: coefficient.into(),
            base: LogNum::from_f64(2.).pow(base.into()),
        }
    }
}

impl CostTrait for ExponentialCost {
    fn get_cost_to(&self, level: i32) -> LogNum {
        if level <= 0 {
            lognum::ZERO
        } else {
            self.coefficient * self.base.powi(level - 1)
        }
    }
    fn get_total_cost_to(&self, level: i32) -> LogNum {
        if level <= 0 {
            lognum::ZERO
        } else {
            self.coefficient * (self.base.powi(level) - lognum::ONE) / (self.base - lognum::ONE)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StepwiseCost<T: CostTrait> {
    model: T,
    step: i32,
}

impl<T: CostTrait> StepwiseCost<T> {
    pub fn new(model: T, step: i32) -> Self {
        StepwiseCost { model, step }
    }
}

impl<T: CostTrait + Clone> CostTrait for StepwiseCost<T> {
    fn get_cost_to(&self, level: i32) -> LogNum {
        if level <= 0 {
            lognum::ZERO
        } else {
            self.model.get_cost_to((level - 1) / self.step + 1)
        }
    }
    fn get_total_cost_to(&self, level: i32) -> LogNum {
        if level <= 0 {
            lognum::ZERO
        } else {
            let int_part = (level - 1) / self.step;
            let mod_part = (level - 1) % self.step + 1;
            self.model.get_total_cost_to(int_part) * self.step.into()
                + self.model.get_cost_to(int_part + 1) * mod_part.into()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConstantCost {
    cost: LogNum,
}

impl ConstantCost {
    pub fn new(cost: impl Into<LogNum>) -> Self {
        ConstantCost { cost: cost.into() }
    }
}

impl CostTrait for ConstantCost {
    fn get_cost_to(&self, level: i32) -> LogNum {
        if level <= 0 { lognum::ZERO } else { self.cost }
    }
    fn get_total_cost_to(&self, level: i32) -> LogNum {
        if level <= 0 {
            lognum::ZERO
        } else {
            self.cost * level.into()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FirstFreeCost<T: CostTrait> {
    model: T,
}

impl<T: CostTrait> FirstFreeCost<T> {
    pub fn new(model: T) -> Self {
        FirstFreeCost { model }
    }
}

impl<T: CostTrait + Clone> CostTrait for FirstFreeCost<T> {
    fn get_cost_to(&self, level: i32) -> LogNum {
        self.model.get_cost_to(level - 1)
    }
    fn get_total_cost_to(&self, level: i32) -> LogNum {
        self.model.get_total_cost_to(level - 1)
    }
}

/** enum holding different cost models */
#[derive(Debug, Clone)]
pub enum Cost {
    Exponential(ExponentialCost),
    Constant(ConstantCost),
    Other(Box<dyn CostTrait>),
}

impl Cost {
    pub fn new_exponential(coefficient: impl Into<LogNum>, base: impl Into<LogNum>) -> Self {
        Self::Exponential(ExponentialCost::new(coefficient, base))
    }

    pub fn new_constant(cost: impl Into<LogNum>) -> Self {
        Self::Constant(ConstantCost::new(cost))
    }

    pub fn new(model: impl CostTrait + 'static) -> Self {
        Self::Other(Box::new(model))
    }

    pub fn get_cost_to(&self, level: i32) -> LogNum {
        match self {
            Self::Exponential(cost) => cost.get_cost_to(level),
            Self::Constant(cost) => cost.get_cost_to(level),
            Self::Other(cost) => cost.get_cost_to(level),
        }
    }

    pub fn get_total_cost_to(&self, level: i32) -> LogNum {
        match self {
            Self::Exponential(cost) => cost.get_total_cost_to(level),
            Self::Constant(cost) => cost.get_total_cost_to(level),
            Self::Other(cost) => cost.get_total_cost_to(level),
        }
    }

    pub fn get_cost(&self, current_level: i32) -> LogNum {
        match self {
            Self::Exponential(cost) => cost.get_cost(current_level),
            Self::Constant(cost) => cost.get_cost(current_level),
            Self::Other(cost) => cost.get_cost(current_level),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12 || a == b
    }

    #[test]
    fn exponential_cost_grows_by_the_base_each_level() {
        // cost to go from level L to L+1 = 100 * 10^L
        let c = Cost::new_exponential(100., 10.);
        assert!(close(c.get_cost(0).value, 2.));
        assert!(close(c.get_cost(7).value, 9.));
    }

    #[test]
    fn first_free_cost_makes_the_first_level_free() {
        let c = Cost::new(FirstFreeCost::new(ExponentialCost::new(5., 2.)));
        assert_eq!(c.get_cost(0).value, f64::NEG_INFINITY); // level 0 -> 1 costs nothing
        assert!(close(c.get_cost(1).value, 5f64.log10()));
        assert!(close(c.get_cost(3).value, 5f64.log10() + 2. * 2f64.log10()));
    }

    #[test]
    fn constant_cost_never_changes() {
        let c = Cost::new_constant(1e6);
        assert!(close(c.get_cost(0).value, 6.));
        assert!(close(c.get_cost(500).value, 6.));
    }
}
