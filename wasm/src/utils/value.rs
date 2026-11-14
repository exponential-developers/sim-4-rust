/*!
 * Value models
 */

use dyn_clone::DynClone;
use num::Float;
use std::fmt::Debug;

use crate::utils::lognum::LogNum;

/** Trait for cost models */
pub trait ValueTrait: DynClone + Debug {
    /** Computes the value for the given level */
    fn compute_from_zero(&self, level: i32) -> LogNum;

    /** Computes the value for the next level from the previous value*/
    fn compute_next(&self, old_value: LogNum, current_level: i32) -> LogNum;
}
dyn_clone::clone_trait_object!(ValueTrait);

#[derive(Debug, Clone, Copy)]
pub struct StepwisePowerSumValue {
    base: LogNum,
    length: i32,
    offset: LogNum,
    /** base and offset as plain numbers, for the sim-3 compatible formula below */
    base_f: f64,
    offset_f: f64,
}

impl StepwisePowerSumValue {
    pub fn new(base: f64, length: i32, offset: f64) -> Self {
        StepwisePowerSumValue {
            base: base.into(),
            length,
            offset: offset.into(),
            base_f: base,
            offset_f: offset,
        }
    }
}

impl ValueTrait for StepwisePowerSumValue {
    /** Same computation as sim-3's `StepwisePowerSumValue.recomputeValue`, so results match exactly */
    fn compute_from_zero(&self, level: i32) -> LogNum {
        use crate::utils::fastmath::log10;
        if level == 0 {
            return self.offset;
        }
        let int_part = level / self.length;
        let mod_part = level - int_part * self.length;
        let d = self.length as f64 / (self.base_f - 1.);
        // log10(x - y) for log10 values x > y (sim-3 `subtract`)
        let x = log10(d + mod_part as f64) + log10(self.base_f) * int_part as f64;
        let y = log10(d);
        let sub_part = LogNum { value: x, sign: 1 } - LogNum { value: y, sign: 1 };
        LogNum {
            value: log10(self.offset_f),
            sign: 1,
        } + sub_part
    }

    fn compute_next(&self, old_value: LogNum, current_level: i32) -> LogNum {
        old_value + self.base.powi(current_level / self.length)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExponentialValue {
    base: LogNum,
}

impl ExponentialValue {
    pub fn new(power: impl Into<LogNum>) -> Self {
        ExponentialValue { base: power.into() }
    }
}

impl ValueTrait for ExponentialValue {
    fn compute_from_zero(&self, level: i32) -> LogNum {
        self.base.powi(level)
    }

    fn compute_next(&self, _old_value: LogNum, current_level: i32) -> LogNum {
        // Recompute from the level instead of multiplying the old value: repeated
        // multiplication accumulates rounding errors over thousands of levels.
        self.base.powi(current_level + 1)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LinearValue {
    slope: LogNum,
    offset: LogNum,
}

impl LinearValue {
    pub fn new(power: impl Into<LogNum>, offset: impl Into<LogNum>) -> Self {
        LinearValue {
            slope: power.into(),
            offset: offset.into(),
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
    Other(Box<dyn ValueTrait>),
}

/** enum holding different value models */
impl Value {
    pub fn new_stepwise(base: f64, length: i32, offset: f64) -> Self {
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
#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12 || a == b
    }

    #[test]
    fn exponential_value_is_base_to_the_level() {
        let v = Value::new_exponential(2.);
        assert_eq!(v.compute_from_zero(0).value, 0.);
        assert!(close(v.compute_from_zero(5).value, 5. * 2f64.log10()));
        // computed from the level, not from the previous value
        assert!(close(
            v.compute_next(LogNum::from(123.), 4).value,
            5. * 2f64.log10()
        ));
    }

    #[test]
    fn stepwise_value_sums_powers_of_the_base() {
        // q-style variable: each level adds 2^floor(level / 10), starting from 0
        let v = Value::new_stepwise(2., 10, 0.);
        let expected = |level: i32| (0..level).map(|l| 2f64.powi(l / 10)).sum::<f64>();
        assert_eq!(v.compute_from_zero(0).value, f64::NEG_INFINITY);
        for level in [1, 9, 10, 15, 37, 250] {
            assert!(
                close(v.compute_from_zero(level).value, expected(level).log10()),
                "level {level}"
            );
        }
        // stepping one level at a time gives the same values
        let mut value = v.compute_from_zero(0);
        for level in 0..60 {
            value = v.compute_next(value, level);
            assert!(
                (value.value - v.compute_from_zero(level + 1).value).abs() < 1e-9,
                "level {}",
                level + 1
            );
        }
        // with an offset (c1-style variable starting at 1)
        let c1 = Value::new_stepwise(2., 10, 1.);
        assert_eq!(c1.compute_from_zero(0).value, 0.);
        assert!(close(
            c1.compute_from_zero(12).value,
            (1. + expected(12)).log10()
        ));
    }

    #[test]
    fn linear_value_adds_the_slope_each_level() {
        // value = 3 + 2 * level
        let v = Value::new_linear(2., 3.);
        assert!(close(v.compute_from_zero(0).value, 3f64.log10()));
        assert!(close(v.compute_from_zero(10).value, 23f64.log10()));
        assert!(close(
            v.compute_next(LogNum::from(23.), 10).value,
            25f64.log10()
        ));
    }

    #[test]
    fn boxed_models_dispatch_to_the_model() {
        let v = Value::new(ExponentialValue::new(10.));
        assert!(close(v.compute_from_zero(3).value, 3.));
        assert!(close(v.compute_next(LogNum::from(1.), 3).value, 4.));
    }
}
