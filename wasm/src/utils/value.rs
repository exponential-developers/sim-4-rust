use std::fmt::Debug;
use num::Float;

use crate::utils::lognum::{self, LogNum};

pub trait Value: Debug + Copy {
    fn recompute(self, level: i32) -> LogNum;

    fn compute_new(self, _old_value: LogNum, current_level: i32) -> LogNum {
        self.recompute(current_level + 1)
    }
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
    fn recompute(self, level: i32) -> LogNum {
        let int_part = level / self.length;
        let mod_part = level % self.length;
        let d = LogNum::from(self.length) / (self.base - lognum::ONE);

        (d + LogNum::from(mod_part)) * self.base.powi(int_part) - d + self.offset
    }

    fn compute_new(self, old_value: LogNum, current_level: i32) -> LogNum {
        old_value + self.base.powi(current_level / self.length)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExponentialValue {
    power: LogNum
}

impl ExponentialValue {
    pub fn new(power: impl Into<LogNum>) -> Self {
        ExponentialValue {
            power: power.into()
        }
    }
}

impl Value for ExponentialValue {
    fn recompute(self, level: i32) -> LogNum {
        self.power.powi(level)
    }

    fn compute_new(self, old_value: LogNum, _current_level: i32) -> LogNum {
        old_value * self.power
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LinearValue {
    power: LogNum,
    offset: LogNum
}

impl LinearValue {
    pub fn new(power: impl Into<LogNum>, offset: impl Into<LogNum>) -> Self {
        LinearValue {
            power: power.into(),
            offset: offset.into()
        }
    }
}

impl Value for LinearValue {
    fn recompute(self, level: i32) -> LogNum {
        self.offset + LogNum::from(level) * self.power
    }

    fn compute_new(self, old_value: LogNum, _current_level: i32) -> LogNum {
        old_value + self.power
    }
}