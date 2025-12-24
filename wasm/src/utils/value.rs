use std::fmt::Debug;
use num::Float;
use dyn_clone::DynClone;

use crate::utils::lognum::{self, LogNum};

pub trait ValueTrait: DynClone + Debug {
    fn compute_from_zero(&self, level: i32) -> LogNum;

    fn compute_next(&self, old_value: LogNum, current_level: i32) -> LogNum;
}
dyn_clone::clone_trait_object!(ValueTrait);

#[derive(Debug, Clone, Copy)]
pub struct StepwisePowerSumValue {
    base: LogNum,
    length: i32,
    offset: LogNum
}

impl StepwisePowerSumValue {
    pub fn new(base: impl Into<LogNum>, length: i32, offset: impl Into<LogNum>) -> Self {
        StepwisePowerSumValue { 
            base: base.into(), 
            length, 
            offset: offset.into() 
        }
    }
}

impl ValueTrait for StepwisePowerSumValue {
    fn compute_from_zero(&self, level: i32) -> LogNum {
        let int_part = level / self.length;
        let mod_part = level % self.length;
        let d = LogNum::from(self.length) / (self.base - lognum::ONE);

        (d + LogNum::from(mod_part)) * self.base.powi(int_part) - d + self.offset
    }

    fn compute_next(&self, old_value: LogNum, current_level: i32) -> LogNum {
        old_value + self.base.powi(current_level / self.length)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExponentialValue {
    base: LogNum
}

impl ExponentialValue {
    pub fn new(power: impl Into<LogNum>) -> Self {
        ExponentialValue {
            base: power.into()
        }
    }
}

impl ValueTrait for ExponentialValue {
    fn compute_from_zero(&self, level: i32) -> LogNum {
        self.base.powi(level)
    }

    fn compute_next(&self, old_value: LogNum, _current_level: i32) -> LogNum {
        old_value * self.base
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LinearValue {
    slope: LogNum,
    offset: LogNum
}

impl LinearValue {
    pub fn new(power: impl Into<LogNum>, offset: impl Into<LogNum>) -> Self {
        LinearValue {
            slope: power.into(),
            offset: offset.into()
        }
    }
}

impl ValueTrait for LinearValue {
    fn compute_from_zero(&self, level: i32) -> LogNum {
        self.offset + LogNum::from(level) * self.slope
    }

    fn compute_next(&self, old_value: LogNum, _current_level: i32) -> LogNum {
        old_value + self.slope
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Stepwise(StepwisePowerSumValue),
    Exponential(ExponentialValue),
    Linear(LinearValue),
    Other(Box<dyn ValueTrait>)
}

impl Value {
    pub fn new_stepwise(base: impl Into<LogNum>, length: i32, offset: impl Into<LogNum>) -> Self {
        Self::Stepwise(StepwisePowerSumValue::new(base, length, offset))
    }

    pub fn new_exponential(power: impl Into<LogNum>) -> Self {
        Self::Exponential(ExponentialValue::new(power))
    }

    pub fn new_linear(power: impl Into<LogNum>, offset: impl Into<LogNum>) -> Self {
        Self::Linear(LinearValue::new(power, offset))
    }

    pub fn new(model: impl ValueTrait + 'static) -> Self {
        Self::Other(Box::new(model))
    }

    pub fn compute_from_zero(&self, level: i32) -> LogNum {
        match self {
            Self::Stepwise(val) => val.compute_from_zero(level),
            Self::Exponential(val) => val.compute_from_zero(level),
            Self::Linear(val) => val.compute_from_zero(level),
            Self::Other(val) => val.compute_from_zero(level),
        }
    }

    pub fn compute_next(&self, old_value: LogNum, current_level: i32) -> LogNum {
        match self {
            Self::Stepwise(val) => val.compute_next(old_value, current_level),
            Self::Exponential(val) => val.compute_next(old_value, current_level),
            Self::Linear(val) => val.compute_next(old_value, current_level),
            Self::Other(val) => val.compute_next(old_value, current_level),
        }
    }
}