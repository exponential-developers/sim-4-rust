use core::fmt::{self, Display};
use serde::{Deserialize, Serialize, de};
use std::cmp::Ordering;
use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};
use std::str::FromStr;

use num::{Float, Num, NumCast, One, ToPrimitive, Zero};

use crate::utils::fastmath;

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
            value: fastmath::log10(value.abs()),
            sign: value.signum() as i8,
        }
    }
}

impl From<f64> for LogNum {
    fn from(value: f64) -> Self {
        LogNum::from_f64(value)
    }
}

impl From<i32> for LogNum {
    fn from(value: i32) -> Self {
        LogNum::from_f64(value as f64)
    }
}

impl From<i64> for LogNum {
    fn from(value: i64) -> Self {
        LogNum::from_f64(value as f64)
    }
}

impl From<u64> for LogNum {
    fn from(value: u64) -> Self {
        LogNum::from_f64(value as f64)
    }
}

#[derive(Debug)]
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
            // A plain number is a log10 value ("100" = 1e100); a leading '-' makes the number
            // negative ("-100" = -1e100), like "-e100" and "-2e5"
            1 => match parts[0].parse::<f64>() {
                Ok(value) => Ok(LogNum::new(value.abs(), sign)),
                Err(_) => Err(LogNumParseError {}),
            },
            3.. => Err(LogNumParseError {}),
            _ => match parts[1].parse::<f64>() {
                // The sign is already taken from the leading '-': use the mantissa's magnitude
                Ok(rhs) => match parts[0].parse::<f64>() {
                    Ok(lhs) => Ok(LogNum::new(rhs, sign) * LogNum::from_f64(lhs.abs())),
                    Err(_) => Ok(LogNum::new(rhs, sign)),
                },
                Err(_) => Err(LogNumParseError {}),
            },
        }
    }
}

impl Display for LogNum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let minus = if self.sign < 0 { "-" } else { "" };
        if self.value.is_nan() {
            return write!(f, "NaN");
        }
        if self.value == f64::NEG_INFINITY {
            return write!(f, "0");
        }
        if self.value == f64::INFINITY {
            return write!(f, "{minus}inf");
        }
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
            return LogNum::new(max.value, max.sign);
        }
        if max.value == min.value && max.value == f64::NEG_INFINITY {
            return ZERO;
        }
        if self.sign == rhs.sign {
            LogNum::new(
                max.value + fastmath::log10_1p_exp10(min.value - max.value),
                max.sign,
            )
        } else {
            LogNum::new(
                max.value + fastmath::log10_1m_exp10(min.value - max.value),
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
        if self.is_zero() || rhs.is_zero() {
            return LogNum::zero();
        }
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
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.sign, other.sign) {
            (1, 1) => self.value.partial_cmp(&other.value),
            (1, -1) => Some(Ordering::Greater),
            (-1, 1) => Some(Ordering::Less),
            (-1, -1) => other.value.partial_cmp(&self.value),
            _ => None,
        }
    }
}

impl LogNum {
    pub fn powf64(self, rhs: f64) -> Self {
        LogNum::new(self.value * rhs, (self.sign as f64).powf(rhs) as i8)
    }

    pub fn log10_f64(self) -> f64 {
        self.value
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
    fn to_i64(&self) -> Option<i64> {
        self.to_f64()?.to_i64()
    }

    fn to_u64(&self) -> Option<u64> {
        self.to_f64()?.to_u64()
    }

    fn to_f64(&self) -> Option<f64> {
        Some(10f64.powf(self.value) * self.sign as f64)
    }
}

impl NumCast for LogNum {
    fn from<T: num::ToPrimitive>(n: T) -> Option<Self> {
        Some(LogNum::from_f64(n.to_f64()?))
    }
}

impl Float for LogNum {
    fn nan() -> Self {
        LogNum::new(f64::NAN, 1)
    }

    fn infinity() -> Self {
        LogNum::new(f64::INFINITY, 1)
    }

    fn neg_infinity() -> Self {
        LogNum::new(f64::INFINITY, -1)
    }

    fn neg_zero() -> Self {
        LogNum::new(f64::NEG_INFINITY, -1)
    }

    fn min_value() -> Self {
        LogNum::new(f64::MAX, -1)
    }

    fn min_positive_value() -> Self {
        LogNum::new(f64::MIN, 1)
    }

    fn epsilon() -> Self {
        LogNum::from_f64(f64::EPSILON)
    }

    fn max_value() -> Self {
        LogNum::new(f64::MAX, 1)
    }

    fn is_nan(self) -> bool {
        self.value.is_nan()
    }

    fn is_infinite(self) -> bool {
        self.value == f64::INFINITY
    }

    fn is_finite(self) -> bool {
        self.classify() == std::num::FpCategory::Normal
            || self.classify() == std::num::FpCategory::Zero
    }

    fn is_normal(self) -> bool {
        self.classify() == std::num::FpCategory::Normal
    }

    fn is_subnormal(self) -> bool {
        false
    }

    fn classify(self) -> std::num::FpCategory {
        if self.value == f64::INFINITY {
            std::num::FpCategory::Infinite
        } else if self.is_zero() {
            std::num::FpCategory::Zero
        } else if self.value.is_nan() {
            std::num::FpCategory::Nan
        } else {
            std::num::FpCategory::Normal
        }
    }

    fn floor(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().floor())
    }

    fn ceil(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().ceil())
    }

    fn round(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().round())
    }

    fn trunc(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().trunc())
    }

    fn fract(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().fract())
    }

    fn abs(self) -> Self {
        LogNum::new(self.value, 1)
    }

    fn signum(self) -> Self {
        if self.is_nan() {
            LogNum::new(f64::NAN, 1)
        } else {
            LogNum::from_f64(self.sign as f64)
        }
    }

    fn is_sign_positive(self) -> bool {
        self.sign == 1
    }

    fn is_sign_negative(self) -> bool {
        self.sign == -1
    }

    fn mul_add(self, a: Self, b: Self) -> Self {
        (self * a) + b
    }

    fn recip(self) -> Self {
        LogNum::new(-self.value, self.sign)
    }

    fn powi(self, n: i32) -> Self {
        LogNum::new(self.value * n as f64, self.sign.pow(n.unsigned_abs()))
    }

    fn powf(self, n: Self) -> Self {
        self.powf64(n.to_f64().unwrap())
    }

    fn sqrt(self) -> Self {
        self.powf64(1. / 2.)
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

    fn ln(self) -> Self {
        LogNum::new(self.value.log10() + std::f64::consts::LN_10.log10(), 1)
    }

    fn log(self, base: Self) -> Self {
        LogNum::new(
            self.value.log10() + 10f64.log(base.to_f64().unwrap()).log10(),
            1,
        )
    }

    fn log2(self) -> Self {
        LogNum::new(self.value.log10() + std::f64::consts::LOG2_10.log10(), 1)
    }

    fn log10(self) -> Self {
        LogNum::from_f64(self.value)
    }

    fn to_degrees(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().to_degrees())
    }

    fn max(self, other: Self) -> Self {
        if self >= other { self } else { other }
    }

    fn min(self, other: Self) -> Self {
        if self <= other { self } else { other }
    }

    fn clamp(self, min: Self, max: Self) -> Self {
        self.max(min).min(max)
    }

    /** Positive difference: `self - other` if `self > other`, else 0 (the `Float::abs_sub` contract) */
    fn abs_sub(self, other: Self) -> Self {
        if self <= other { ZERO } else { self - other }
    }

    fn cbrt(self) -> Self {
        self.powf64(1. / 3.)
    }

    fn hypot(self, other: Self) -> Self {
        (self.powi(2) + other.powi(2)).sqrt()
    }

    fn sin(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().sin())
    }

    fn cos(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().cos())
    }

    fn tan(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().tan())
    }

    fn asin(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().asin())
    }

    fn acos(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().acos())
    }

    fn atan(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().atan())
    }

    fn atan2(self, other: Self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().atan2(other.to_f64().unwrap()))
    }

    fn sin_cos(self) -> (Self, Self) {
        (self.sin(), self.cos())
    }

    fn exp_m1(self) -> Self {
        self.exp() - ONE
    }

    fn ln_1p(self) -> Self {
        (self + ONE).ln()
    }

    fn sinh(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().sinh())
    }

    fn cosh(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().cosh())
    }

    fn tanh(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().tanh())
    }

    fn asinh(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().asinh())
    }

    fn acosh(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().acosh())
    }

    fn atanh(self) -> Self {
        LogNum::from_f64(self.to_f64().unwrap().atanh())
    }

    fn integer_decode(self) -> (u64, i16, i8) {
        self.to_f64().unwrap().integer_decode()
    }

    fn copysign(self, sign: Self) -> Self {
        LogNum::new(self.value, sign.sign)
    }
}

impl LogNum {
    pub fn pow(self, other: Self) -> Self {
        self.powf(other)
    }
}

impl<'de> Deserialize<'de> for LogNum {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct LogNumVisitor;

        impl<'de> de::Visitor<'de> for LogNumVisitor {
            type Value = LogNum;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a number")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(LogNum::new(v as f64, 1))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(LogNum::new(v as f64, 1))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(LogNum::new(v, 1))
            }
        }

        deserializer.deserialize_any(LogNumVisitor)
    }
}

impl Serialize for LogNum {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_f64(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{E, PI};

    /** Plain number from a LogNum */
    fn real(x: LogNum) -> f64 {
        x.to_f64().unwrap()
    }

    fn n(x: f64) -> LogNum {
        LogNum::from_f64(x)
    }

    fn close(a: f64, b: f64) -> bool {
        a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.)
    }

    #[track_caller]
    fn assert_real(x: LogNum, expected: f64) {
        assert!(close(real(x), expected), "{} != {expected}", real(x));
    }

    #[test]
    fn parsing_and_display() {
        assert_real("2e5".parse().unwrap(), 2e5);
        assert_real("-2e5".parse().unwrap(), -2e5);
        assert_real("e5".parse().unwrap(), 1e5);
        assert_real("-e5".parse().unwrap(), -1e5);
        // a plain number is read as a log10 value, like inputs elsewhere in the sim
        let plain: LogNum = "100".parse().unwrap();
        assert_eq!((plain.value, plain.sign), (100., 1));
        // and a leading '-' makes it negative, as with the other formats
        let negative: LogNum = "-100".parse().unwrap();
        assert_eq!((negative.value, negative.sign), (100., -1));
        assert!("".parse::<LogNum>().is_err());
        assert!("2e".parse::<LogNum>().is_err());
        assert!("abc".parse::<LogNum>().is_err());
        assert!("1e2e3".parse::<LogNum>().is_err());
        assert_real(LogNum::from_str_radix("3e2", 10).unwrap(), 300.);

        assert_eq!(n(2e5).to_string(), "2.00e5");
        assert_eq!(n(-31400.).to_string(), "-3.14e4");
        assert_eq!(ZERO.to_string(), "0");
        assert_eq!(<LogNum as Float>::nan().to_string(), "NaN");
        assert_eq!(<LogNum as Float>::infinity().to_string(), "inf");
        assert_eq!(<LogNum as Float>::neg_infinity().to_string(), "-inf");
    }

    #[test]
    fn arithmetic_with_signs_and_special_values() {
        assert_real(n(1000.) + n(100.), 1100.);
        assert_real(n(1000.) + n(-100.), 900.);
        assert_real(n(-1000.) + n(100.), -900.);
        assert_real(n(100.) - n(1000.), -900.);
        assert_real(n(-3.) - n(-3.), 0.);
        assert_eq!(ZERO + ZERO, ZERO);
        assert_real(n(42.) + ZERO, 42.);
        assert!((<LogNum as Float>::infinity() + n(5.)).is_infinite());

        assert_real(n(20.) * n(-5.), -100.);
        assert_eq!(n(20.) * ZERO, ZERO);
        assert_eq!(ZERO * n(20.), ZERO);
        assert_real(n(1000.) / n(-10.), -100.);
        assert_real(-n(7.), -7.);

        let mut x = n(10.);
        x += n(5.);
        x -= n(3.);
        x *= n(4.);
        x /= n(8.);
        assert_real(x, 6.);
    }

    #[test]
    fn ordering_respects_signs() {
        assert!(n(-5.) < n(3.));
        assert!(n(3.) > n(-5.));
        assert!(n(-5.) < n(-3.));
        assert!(n(2.) < n(3.));
        let unsigned = LogNum { value: 1., sign: 0 };
        assert_eq!(unsigned.partial_cmp(&n(1.)), None);

        assert_real(Float::max(n(-5.), n(3.)), 3.);
        assert_real(Float::min(n(-5.), n(3.)), -5.);
        assert_real(n(50.).clamp(n(1.), n(10.)), 10.);
        assert_real(n(-50.).clamp(n(1.), n(10.)), 1.);
        assert_real(n(5.).clamp(n(1.), n(10.)), 5.);
    }

    #[test]
    fn conversions() {
        assert_eq!(n(1234.6).to_i64(), Some(1234));
        assert_eq!(n(1234.6).to_u64(), Some(1234));
        assert_eq!(n(-3.5).to_u64(), None);
        assert_real(<LogNum as NumCast>::from(250u32).unwrap(), 250.);
        assert_real(<LogNum as From<i32>>::from(-7), -7.);
        assert_real(<LogNum as From<i64>>::from(70), 70.);
        assert_real(<LogNum as From<u64>>::from(700), 700.);
        assert_real(n(10.).powf64(3.), 1000.);
        assert_eq!(n(1000.).log10_f64(), 3.);
        assert!(ONE.is_one() && !n(2.).is_one());
        assert!(LogNum::zero().is_zero() && LogNum::one() == ONE);
    }

    #[test]
    fn special_values_and_classification() {
        use std::num::FpCategory;
        let nan: LogNum = Float::nan();
        let inf: LogNum = Float::infinity();
        assert!(nan.is_nan() && nan.classify() == FpCategory::Nan && !nan.is_finite());
        assert!(
            inf.is_infinite()
                && inf.classify() == FpCategory::Infinite
                && !inf.is_finite()
                && !inf.is_normal()
        );
        assert_eq!(real(Float::neg_infinity()), f64::NEG_INFINITY);
        let neg_zero: LogNum = Float::neg_zero();
        assert!(neg_zero.is_zero() && neg_zero.is_sign_negative());
        assert_eq!(ZERO.classify(), FpCategory::Zero);
        assert!(ZERO.is_finite() && !ZERO.is_normal());
        assert!(n(5.).is_normal() && n(5.).is_finite() && !n(5.).is_subnormal());
        assert!(
            real(Float::max_value()).is_infinite() && real(Float::min_value()) == f64::NEG_INFINITY
        );
        assert_eq!(real(Float::min_positive_value()), 0.); // 10^(most negative f64)
        assert_real(Float::epsilon(), f64::EPSILON);
    }

    #[test]
    fn rounding_and_sign_functions() {
        assert_real(n(2.7).floor(), 2.);
        assert_real(n(2.2).ceil(), 3.);
        assert_real(n(2.5).round(), 3.);
        assert_real(n(-2.7).trunc(), -2.);
        assert_real(n(2.75).fract(), 0.75);
        assert_real(n(-5.).abs(), 5.);
        assert_real(n(-5.).signum(), -1.);
        assert_real(n(5.).signum(), 1.);
        assert!(<LogNum as Float>::nan().signum().is_nan());
        assert!(n(5.).is_sign_positive() && n(-5.).is_sign_negative());
        assert_real(n(5.).copysign(n(-1.)), -5.);
        // positive difference
        assert_real(n(5.).abs_sub(n(3.)), 2.);
        assert_eq!(n(3.).abs_sub(n(5.)), ZERO);
    }

    #[test]
    fn powers_roots_logs_and_exponentials() {
        assert_real(n(2.).powi(10), 1024.);
        assert_real(n(-2.).powi(3), -8.);
        assert_real(n(-2.).powi(2), 4.);
        assert_real(n(4.).powf(n(0.5)), 2.);
        assert_real(n(4.).pow(n(1.5)), 8.);
        assert_real(n(16.).sqrt(), 4.);
        assert_real(n(27.).cbrt(), 3.);
        assert_real(n(4.).recip(), 0.25);
        assert_real(n(2.).mul_add(n(3.), n(4.)), 10.);
        assert_real(n(3.).hypot(n(4.)), 5.);
        assert_real(n(1.).exp(), E);
        assert_real(n(3.).exp2(), 8.);
        assert_real(n(0.5).exp_m1(), 0.5f64.exp_m1());
        assert_real(n(E * E).ln(), 2.);
        assert_real(n(1.).ln_1p(), 2f64.ln());
        assert_real(n(8.).log(n(2.)), 3.);
        assert_real(n(1024.).log2(), 10.);
        assert_real(n(1e7).log10(), 7.);
    }

    #[test]
    fn trigonometry_on_the_underlying_number() {
        assert_real(n(PI / 2.).sin(), 1.);
        assert_real(n(PI).cos(), -1.);
        assert_real(n(PI / 4.).tan(), 1.);
        let (s, c) = n(PI / 6.).sin_cos();
        assert_real(s, 0.5);
        assert_real(c, (PI / 6.).cos());
        assert_real(n(1.).asin(), PI / 2.);
        assert_real(ZERO.acos(), PI / 2.);
        assert_real(n(1.).atan(), PI / 4.);
        assert_real(n(1.).atan2(n(1.)), PI / 4.);
        assert_real(n(0.5).sinh(), 0.5f64.sinh());
        assert_real(n(0.5).cosh(), 0.5f64.cosh());
        assert_real(n(0.5).tanh(), 0.5f64.tanh());
        assert_real(n(2.).asinh(), 2f64.asinh());
        assert_real(n(2.).acosh(), 2f64.acosh());
        assert_real(n(0.5).atanh(), 0.5f64.atanh());
        assert_real(n(PI).to_degrees(), 180.);
        assert_eq!(n(2.).integer_decode(), 2f64.integer_decode());
    }

    #[test]
    fn json_numbers_are_log10_values() {
        let x: LogNum = serde_json::from_str("12").unwrap();
        assert_eq!((x.value, x.sign), (12., 1));
        let y: LogNum = serde_json::from_str("12.5").unwrap();
        assert_eq!(y.value, 12.5);
        let z: LogNum = serde_json::from_str(&u64::MAX.to_string()).unwrap();
        assert_eq!(z.value, u64::MAX as f64);
        assert!(serde_json::from_str::<LogNum>("\"abc\"").is_err());
        assert_eq!(
            serde_json::to_string(&LogNum {
                value: 3.5,
                sign: 1
            })
            .unwrap(),
            "3.5"
        );
    }
}
