use std::cmp::{max, PartialEq};
use num::{Float, Zero};
use crate::utils::lognum::{LogNum, ZERO};
use std::ops::{
    Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign,
};
#[derive(Debug, Clone, PartialEq)]
pub struct LogNumPoly{
    pub coefficients: Vec<LogNum>,
}

impl Zero for LogNumPoly {
    fn zero() -> LogNumPoly {
        LogNumPoly{coefficients: vec![]}
    }
    fn is_zero(&self) -> bool {
        self.coefficients.len() == 0
    }
}

impl LogNumPoly {
    pub fn new(coefficients: Vec<LogNum>) -> Self {
        Self::from_coeffs(coefficients)
    }

    pub fn order(&self) -> i32 {
        if self.is_zero() { return 0; }
        self.coefficients.len() as i32 - 1
    }
    #[inline]
    pub fn get_coefficient(&self, n : usize) -> LogNum {
        if self.coefficients.len() <= n { return ZERO }
        self.coefficients[n]
    }

    #[inline]
    fn cleanup(&mut self) {
        while self.coefficients.last().is_some_and(|c| c.is_zero()) {
            self.coefficients.pop();
        }
    }

    pub fn eval(&self, x: LogNum) -> LogNum {
        if self.is_zero() { return ZERO; }
        let mut rtn = ZERO;
        for i in 0..self.coefficients.len() {
            rtn += self.get_coefficient(i)*x.powi(i as i32);
        }
        rtn
    }
    
    pub fn differentiate(&self) -> LogNumPoly {
        let mut coefficients: Vec<LogNum> = vec![];
        for i in 1..self.coefficients.len() {
            coefficients.push(self.get_coefficient(i) * LogNum::from(i as i32));
        }
        LogNumPoly{coefficients}
    }

    pub fn integrate_definite(&self, lower_bound : LogNum, upper_bound : LogNum) -> LogNum {
        let mut coefficients: Vec<LogNum> = vec![ZERO];
        for i in 0..self.coefficients.len() {
            coefficients.push(self.get_coefficient(i) / LogNum::from(i as i32 + 1));
        }
        let indefinite = LogNumPoly{coefficients};
        indefinite.eval(upper_bound) - indefinite.eval(lower_bound)
    }

    fn from_coeffs(coefficients: Vec<LogNum>) -> Self {
        let mut p = LogNumPoly { coefficients };
        p.cleanup();
        p
    }
}

impl Neg for LogNumPoly {
    type Output = LogNumPoly;
    fn neg(mut self) -> LogNumPoly {
        for c in &mut self.coefficients {
            *c = -*c;
        }
        self.cleanup();
        self
    }
}

impl Add for LogNumPoly {
    type Output = LogNumPoly;

    fn add(self, rhs: Self) -> Self::Output {
        let mut coefficients: Vec<LogNum> = vec![];
        let n = self.coefficients.len().max(rhs.coefficients.len());
        for i in 0..n {
            coefficients.push(
                self.get_coefficient(i)
                    + rhs.get_coefficient(i)
            );
        }
        Self::from_coeffs(coefficients)
    }
}

impl Sub for LogNumPoly {
    type Output = LogNumPoly;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut coefficients: Vec<LogNum> = vec![];
        let n = self.coefficients.len().max(rhs.coefficients.len());
        for i in 0..n {
            coefficients.push(
                self.get_coefficient(i)
                    - rhs.get_coefficient(i)
            );
        }
        Self::from_coeffs(coefficients)
    }
}

impl AddAssign for LogNumPoly {
    fn add_assign(&mut self, rhs: Self) {
        self.coefficients.resize(max(self.coefficients.len(), rhs.coefficients.len()), ZERO);
        for i in 0..rhs.coefficients.len() {
            self.coefficients[i] += rhs.coefficients[i];
        }
        self.cleanup();
    }
}

impl SubAssign for LogNumPoly {
    fn sub_assign(&mut self, rhs: Self) {
        self.coefficients.resize(max(self.coefficients.len(), rhs.coefficients.len()), ZERO);
        for i in 0..rhs.coefficients.len() {
            self.coefficients[i] -= rhs.coefficients[i];
        }
        self.cleanup();
    }
}

impl Mul<LogNumPoly> for LogNumPoly {
    type Output = LogNumPoly;
    fn mul(self, rhs: Self) -> Self::Output {
        if self.is_zero() || rhs.is_zero() {
            return LogNumPoly::zero();
        }
        // Gotta just use this simple method because the polynomial used are at most degree 10
        let mut coefficients: Vec<LogNum> = vec![ZERO; self.coefficients.len() + rhs.coefficients.len() - 1usize];
        for i in 0..self.coefficients.len(){
            for j in 0..rhs.coefficients.len(){
                coefficients[i+j] += self.coefficients[i] * rhs.coefficients[j];
            }
        }
        Self::from_coeffs(coefficients)
    }
}

impl MulAssign<LogNumPoly> for LogNumPoly {
    fn mul_assign(&mut self, rhs: LogNumPoly) {
        let mut coefficients: Vec<LogNum> = vec![ZERO; self.coefficients.len() + rhs.coefficients.len() - 1usize];

        for i in 0..self.coefficients.len() {
            for j in 0..rhs.coefficients.len() {
                coefficients[i + j] += self.coefficients[i] * rhs.coefficients[j];
            }
        }
        *self = Self::from_coeffs(coefficients);
    }
}

impl Mul<LogNum> for LogNumPoly {
    type Output = LogNumPoly;
    fn mul(mut self, rhs: LogNum) -> LogNumPoly {
        if rhs.is_zero() {
            return LogNumPoly::zero();
        }
        for coeff in &mut self.coefficients {
            *coeff = *coeff * rhs;
        }
        self.cleanup();
        self
    }
}

impl MulAssign<LogNum> for LogNumPoly {
    fn mul_assign(&mut self, rhs: LogNum) {
        for coeff in &mut self.coefficients {
            *coeff *= rhs;
        }
        self.cleanup();
    }
}
