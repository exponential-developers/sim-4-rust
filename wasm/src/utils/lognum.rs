use std::fmt::Display;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct LogNum {
    pub value: f64
}

pub const ZERO: LogNum = LogNum { value: f64::NEG_INFINITY };
pub const ONE: LogNum = LogNum { value: 0. };

impl From<f64> for LogNum {
    fn from(value: f64) -> Self {
        LogNum { value: value.log10() }
    }
}

pub struct LogNumParseError {}

impl FromStr for LogNum {
    type Err = LogNumParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split("e").collect();
        
        match parts.len() {
            0 => Err(LogNumParseError {}),
            1 => match parts[0].parse::<f64>() {
                Ok(value) => Ok(LogNum::new(value)),
                Err(_) => Err(LogNumParseError {})
            },
            _ => {
                match parts[1].parse::<f64>() {
                    Ok(rhs) => match parts[0].parse::<f64>() {
                        Ok(lhs) => Ok(LogNum::new(rhs) * LogNum::from(lhs)),
                        Err(_) => Ok(LogNum::new(rhs))
                    },
                    Err(_) => Err(LogNumParseError {})
                }
            }
        }
    }
}

impl Display for LogNum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let floor = self.value.floor();
        let frac = self.value - floor;
        write!(f, "{:.2}e{}", 10f64.powf(frac), floor)
    }
}


impl Into<f64> for LogNum {
    fn into(self) -> f64 {
        10f64.powf(self.value)
    }
}

impl LogNum {
    fn new(value: f64) -> Self {
        LogNum { value }
    }
}

impl Add for LogNum {
    type Output = LogNum;

    fn add(self, rhs: Self) -> LogNum {
        let max = self.value.max(rhs.value);
        let min = self.value.min(rhs.value);
        if max == f64::INFINITY {
            LogNum::new(max)
        }
        else {
            LogNum::new(max + (1. + 10f64.powf(min - max)).log10())
        }
    }
}

impl AddAssign for LogNum {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs
    }
}

impl Sub for LogNum {
    type Output = LogNum;

    fn sub(self, rhs: Self) -> LogNum {
        let max = self.value.max(rhs.value);
        let min = self.value.min(rhs.value);
        if max == f64::INFINITY {
            LogNum::new(max)
        }
        else {
            LogNum::new(max + (1. - 10f64.powf(min - max)).log10())
        }
    }
}

impl SubAssign for LogNum {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs
    }
}

impl Mul for LogNum {
    type Output = LogNum;

    fn mul(self, rhs: Self) -> LogNum {
        LogNum::new(self.value + rhs.value)
    }
}

impl MulAssign for LogNum {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs
    }
}

impl Div for LogNum {
    type Output = LogNum;

    fn div(self, rhs: Self) -> LogNum {
        LogNum::new(self.value - rhs.value)
    }
}

impl DivAssign for LogNum {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs
    }
}

impl LogNum {
    fn pow(self, rhs: f64) -> Self {
        LogNum::new(self.value * rhs)
    }
}