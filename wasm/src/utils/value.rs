use num::Float;

use crate::utils::lognum::{self, LogNum};

pub trait Value {
    fn compute_from_zero(&self, level: i32) -> LogNum;

    fn compute_next(&self, old_value: LogNum, current_level: i32) -> LogNum;
}

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

impl Value for StepwisePowerSumValue {
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

impl Value for ExponentialValue {
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

impl Value for LinearValue {
    fn compute_from_zero(&self, level: i32) -> LogNum {
        self.offset + LogNum::from(level) * self.slope
    }

    fn compute_next(&self, old_value: LogNum, _current_level: i32) -> LogNum {
        old_value + self.slope
    }
}