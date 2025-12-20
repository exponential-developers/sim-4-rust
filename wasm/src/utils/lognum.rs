use std::fmt::Display;
use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct LogNum {
    pub value: f64,
    pub sign: i8,
}

pub const ZERO: LogNum = LogNum {
    value: f64::NEG_INFINITY,
    sign: 1,
};
pub const ONE: LogNum = LogNum { value: 0., sign: 1 };

impl From<f64> for LogNum {
    fn from(value: f64) -> Self {
        LogNum {
            value: value.abs().log10(),
            sign: value.signum() as i8,
        }
    }
}

pub struct LogNumParseError {}

impl FromStr for LogNum {
    type Err = LogNumParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(LogNumParseError {});
        }

        let sign = if s.chars().next().unwrap_or(' ') == '-' {
            -1
        } else {
            1
        };

        let parts: Vec<&str> = s.split("e").collect();

        match parts.len() {
            0 => Err(LogNumParseError {}),
            1 => match parts[0].parse::<f64>() {
                Ok(value) => Ok(LogNum::new(value, sign)),
                Err(_) => Err(LogNumParseError {}),
            },
            _ => match parts[1].parse::<f64>() {
                Ok(rhs) => match parts[0].parse::<f64>() {
                    Ok(lhs) => Ok(LogNum::new(rhs, sign) * LogNum::from(lhs)),
                    Err(_) => Ok(LogNum::new(rhs, sign)),
                },
                Err(_) => Err(LogNumParseError {}),
            },
        }
    }
}

impl Display for LogNum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let floor = self.value.floor();
        let frac = self.value - floor;
        write!(f, "{:.2}e{}", 10f64.powf(frac) * self.sign as f64, floor)
    }
}

impl From<LogNum> for f64 {
    fn from(value: LogNum) -> Self {
        10f64.powf(value.value) * value.sign as f64
    }
}

impl LogNum {
    fn new(value: f64, sign: i8) -> Self {
        LogNum { value, sign }
    }
}

impl Neg for LogNum {
    type Output = LogNum;

    fn neg(self) -> LogNum {
        LogNum::new(self.value, -self.sign)
    }
}

impl Add for LogNum {
    type Output = LogNum;

    fn add(self, rhs: Self) -> LogNum {
        let max = if self.value >= rhs.value { &self } else { &rhs };
        let min = if self.value < rhs.value { &self } else { &rhs };
        if max.value == f64::INFINITY {
            LogNum::new(max.value, max.sign)
        } else if self.sign == rhs.sign {
            LogNum::new(
                max.value + (1. + 10f64.powf(min.value - max.value)).log10(),
                max.sign,
            )
        } else {
            LogNum::new(
                max.value + (1. - 10f64.powf(min.value - max.value)).log10(),
                max.sign,
            )
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
        self + (-rhs)
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
        LogNum::new(self.value + rhs.value, self.sign * rhs.sign)
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
        LogNum::new(self.value - rhs.value, self.sign * rhs.sign)
    }
}

impl DivAssign for LogNum {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs
    }
}

impl Rem for LogNum {
    type Output = LogNum;

    fn rem(self, _rhs: Self) -> LogNum {
        unimplemented!("Rem not implemented for LogNum")
    }
}

impl RemAssign for LogNum {
    fn rem_assign(&mut self, _rhs: Self) {
        unimplemented!("RemAssign not implemented for LogNum")
    }
}

#[allow(dead_code)]
impl LogNum {
    fn pow(self, rhs: f64) -> Self {
        LogNum::new(self.value * rhs, (self.sign as f64).powf(rhs) as i8)
    }

    fn ln(self) -> LogNum {
        LogNum::new(self.value.log10() + std::f64::consts::LN_10.log10(), 1)
    }

    fn exp(self) -> LogNum {
        LogNum::new(
            std::f64::consts::LOG10_E * 10f64.powf(self.value) * self.sign as f64,
            1,
        )
    }
}

impl num::Zero for LogNum {
    fn zero() -> Self {
        ZERO
    }

    fn is_zero(&self) -> bool {
        self.value == f64::NEG_INFINITY
    }
}

impl num::One for LogNum {
    fn one() -> Self {
        ONE
    }

    fn is_one(&self) -> bool {
        *self == ONE
    }
}

impl num::Num for LogNum {
    type FromStrRadixErr = LogNumParseError;

    fn from_str_radix(str: &str, _radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        LogNum::from_str(str)
    }
}

