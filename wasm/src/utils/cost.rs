use num::Float;

use crate::utils::lognum::LogNum;

pub trait Cost {
    fn get_cost(&self, level: i32) -> LogNum;
}




#[derive(Debug, Clone, Copy)]
pub struct CompositeCost<T: Cost, U: Cost> {
    pub model1: T,
    pub model2: U,
    pub cutoff: i32,
}

impl<T: Cost, U: Cost> CompositeCost<T, U> {
    pub fn new(model1: T, model2: U, cutoff: i32) -> Self {
        CompositeCost { 
            model1, 
            model2, 
            cutoff 
        }
    } 
}

impl<T: Cost, U: Cost> Cost for CompositeCost<T, U> {
    fn get_cost(&self, level: i32) -> LogNum {
        if level < self.cutoff {
            self.model1.get_cost(level)
        } else {
            self.model2.get_cost(level - self.cutoff)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExponentialCost {
    base: LogNum,
    increase: LogNum
}

impl ExponentialCost {
    pub fn new(base: impl Into<LogNum>, increase: impl Into<LogNum>) -> Self {
        ExponentialCost { base: base.into(), increase: increase.into() }
    }

    pub fn new_log2(base: impl Into<LogNum>, increase: impl Into<LogNum>) -> Self {
        ExponentialCost { base: base.into(), increase: LogNum::from_f64(2.).pow(increase.into()) }
    }
}

impl Cost for ExponentialCost {
    fn get_cost(&self, level: i32) -> LogNum {
        self.base * self.increase.powi(level)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StepwiseCost<T: Cost> {
    model: T,
    step: i32
}

impl<T: Cost> StepwiseCost<T> {
    pub fn new(model: T, step: i32) -> Self {
        StepwiseCost {
            model,
            step
        }
    }
}

impl<T: Cost> Cost for StepwiseCost<T> {
    fn get_cost(&self, level: i32) -> LogNum {
        self.model.get_cost(level / self.step)
    }
}


#[derive(Debug, Clone, Copy)]
pub struct ConstantCost {
    cost: LogNum
}

impl ConstantCost {
    pub fn new(cost: impl Into<LogNum>) -> Self {
        ConstantCost {
            cost: cost.into()
        }
    }
}

impl Cost for ConstantCost {
    fn get_cost(&self, _level: i32) -> LogNum {
        self.cost
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FirstFreeCost<T: Cost> {
    model: T
}

impl<T: Cost> FirstFreeCost<T> {
    pub fn new(model: T) -> Self {
        FirstFreeCost { model }
    }
}

impl<T: Cost> Cost for FirstFreeCost<T> {
    fn get_cost(&self, level: i32) -> LogNum {
        if level <= 0 {
            -LogNum::min_positive_value()
        }
        else {
            self.model.get_cost(level - 1)
        }
    }
}