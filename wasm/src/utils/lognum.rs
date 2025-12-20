use core::f64;
use std::fmt::Display;
use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};
use std::str::FromStr;
use std::cmp::Ordering;

use num::{Float, Num, NumCast, ToPrimitive, Zero, One};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogNum {
    pub value: f64,
    pub sign: i8,
}

pub const ZERO: LogNum = LogNum {
    value: f64::NEG_INFINITY,
    sign: 1,
};
pub const ONE: LogNum = LogNum { value: 0., sign: 1 };

impl LogNum {
    pub fn from_f64(value: f64) -> Self {
        LogNum {
            value: value.abs().log10(),
            sign: value.signum() as i8,
        }
    }
}

impl From<f64> for LogNum {
    fn from(value: f64) -> Self {
        LogNum::from_f64(value)
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
                    Ok(lhs) => Ok(LogNum::new(rhs, sign) * LogNum::from_f64(lhs)),
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

impl PartialOrd for LogNum {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self.sign, other.sign) {
            (1, 1) => self.value.partial_cmp(&other.value),
            (1, -1) => Some(Ordering::Greater),
            (-1, 1) => Some(Ordering::Less),
            (-1, -1) => other.value.partial_cmp(&self.value),
            _ => None
        }
    }
}

impl LogNum {
    pub fn powf64(self, rhs: f64) -> Self {
        LogNum::new(self.value * rhs, (self.sign as f64).powf(rhs) as i8)
    }
}

impl Zero for LogNum {
    fn zero() -> Self {
        ZERO
    }

    fn is_zero(&self) -> bool {
        self.value == f64::NEG_INFINITY
    }
}

impl One for LogNum {
    fn one() -> Self {
        ONE
    }

    fn is_one(&self) -> bool {
        *self == ONE
    }
}

impl Num for LogNum {
    type FromStrRadixErr = LogNumParseError;

    fn from_str_radix(str: &str, _radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        LogNum::from_str(str)
    }
}

impl ToPrimitive for LogNum {
    fn to_f64(&self) -> Option<f64> {
        Some(10f64.powf(self.value) * self.sign as f64)
    }

    fn to_i64(&self) -> Option<i64> {
        self.to_f64()?.to_i64()
    }

    fn to_u64(&self) -> Option<u64> {
        self.to_f64()?.to_u64()
    }
}


impl NumCast for LogNum {
    fn from<T: num::ToPrimitive>(n: T) -> Option<Self> {
        Some(LogNum::from_f64(n.to_f64()?))
    }
}

impl Float for LogNum {
    fn abs(self) -> Self {
        LogNum::new(self.value, 1)
    }

    fn abs_sub(self, other: Self) -> Self {
        (self - other).abs()
    }

    fn acos(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().acos())
    }

    fn acosh(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().acosh())
    }

    fn asin(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().asin())
    }

    fn asinh(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().asinh())
    }

    fn atan(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().atan())
    }

    fn atan2(self, other: Self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().atan2(other.to_f64().unwrap()))
    }

    fn atanh(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().atanh())
    }

    fn powf(self, n: Self) -> Self {
        self.powf64(n.to_f64().unwrap())
    }

    fn powi(self, n: i32) -> Self {
        LogNum::new(self.value * n as f64, self.sign.pow(n.abs() as u32) as i8)
    }

    fn cbrt(self) -> Self {
        self.powf64(1. / 3.)
    }

    fn ceil(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().ceil())
    }

    fn min(self, other: Self) -> Self {
        if self <= other { self } else { other }
    }

    fn max(self, other: Self) -> Self {
        if self >= other { self } else { other }
    }

    fn clamp(self, min: Self, max: Self) -> Self {
        self.max(min).min(max)
    }

    fn classify(self) -> std::num::FpCategory {
        if self.value == f64::INFINITY {
            std::num::FpCategory::Infinite
        }
        else if self.is_zero() {
            std::num::FpCategory::Zero
        }
        else if self.value.is_nan() {
            std::num::FpCategory::Nan
        }
        else {
            std::num::FpCategory::Normal
        }
    }

    fn copysign(self, sign: Self) -> Self {
        LogNum::new(self.value, sign.sign)
    }

    fn cos(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().cos())
    }

    fn cosh(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().cosh())
    }

    fn epsilon() -> Self {
        LogNum::from_f64(f64::EPSILON)
    }

    fn exp(self) -> Self {
        LogNum::new(
            std::f64::consts::LOG10_E * 10f64.powf(self.value) * self.sign as f64,
            1,
        )
    }

    fn exp2(self) -> Self {
        LogNum::new(
            std::f64::consts::LOG10_2 * 10f64.powf(self.value) * self.sign as f64,
            1,
        )
    }

    fn exp_m1(self) -> Self {
        self.exp() - ONE
    }

    fn floor(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().floor())
    }

    fn fract(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().fract())
    }

    fn sqrt(self) -> Self {
        self.powf64(1./2.)
    }

    fn hypot(self, other: Self) -> Self {
        (self.powi(2) + other.powi(2)).sqrt()
    }

    fn infinity() -> Self {
        LogNum::new(f64::INFINITY, 1)
    }

    fn integer_decode(self) -> (u64, i16, i8) {
        self.to_f64().unwrap().integer_decode()
    }

    fn is_finite(self) -> bool {
        self.classify() == std::num::FpCategory::Normal || self.classify() == std::num::FpCategory::Zero
    }

    fn is_infinite(self) -> bool {
        self.value == f64::INFINITY
    }

    fn is_nan(self) -> bool {
        self.value.is_nan()
    }

    fn is_normal(self) -> bool {
        self.classify() == std::num::FpCategory::Normal
    }

    fn is_sign_negative(self) -> bool {
        self.sign == -1
    }

    fn is_sign_positive(self) -> bool {
        self.sign == 1
    }

    fn is_subnormal(self) -> bool {
        false
    }

    fn ln(self) -> Self {
        LogNum::new(self.value.log10() + std::f64::consts::LN_10.log10(), 1)
    }

    fn ln_1p(self) -> Self {
        (self + ONE).ln()
    }

    fn log(self, base: Self) -> Self {
        LogNum::new(self.value.log10() + 10f64.log(base.to_f64().unwrap()).log10(), 1)
    }

    fn log10(self) -> Self {
        LogNum::from_f64(self.value)
    }

    fn log2(self) -> Self {
        LogNum::new(self.value.log10() + std::f64::consts::LOG2_10.log10(), 1)
    }

    fn max_value() -> Self {
        LogNum::new(f64::MAX, 1)
    }

    fn min_positive_value() -> Self {
        LogNum::new(f64::MIN, 1)
    }

    fn min_value() -> Self {
        LogNum::new(f64::MAX, -1)
    }

    fn mul_add(self, a: Self, b: Self) -> Self {
        (self * a) + b
    }

    fn nan() -> Self {
        LogNum::new(f64::NAN, 1)
    }

    fn neg_infinity() -> Self {
        LogNum::new(f64::INFINITY, -1)
    }

    fn neg_zero() -> Self {
        LogNum::new(f64::NEG_INFINITY, -1)
    }

    fn recip(self) -> Self {
        LogNum::new(-self.value, self.sign)
    }

    fn round(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().round())
    }

    fn signum(self) -> Self {
        if self.is_nan() { LogNum::new(f64::NAN, 1) } else { LogNum::from_f64(self.sign as f64) }
    }

    fn sin(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().sin())
    }

    fn sin_cos(self) -> (Self, Self) {
        (self.sin(), self.cos())
    }

    fn sinh(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().sinh())
    }

    fn tan(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().tan())
    }

    fn tanh(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().tanh())
    }

    fn to_degrees(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().to_degrees())
    }

    fn trunc(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().trunc())
    }
}

impl LogNum {
    pub fn pow(self, other: Self) -> Self {
        self.powf(other)
    }
}