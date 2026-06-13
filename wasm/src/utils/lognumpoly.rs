use std::cmp::{max, PartialEq};
use std::f32::consts::PI;
use num::{Complex, Zero};
use crate::utils::lognum::{LogNum, ONE, ZERO};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Debug, Clone, PartialEq)]
pub struct LogNumPoly{
    coefficients: Vec<LogNum>,
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
        let mut result = ZERO;
        for coeff in self.coefficients.iter().rev() {
            result = result * x + *coeff;
        }
        result
    }

    pub fn eval_complex(&self, x: Complex<LogNum>) -> Complex<LogNum> {
        let mut result = Complex::new(ZERO, ZERO);

        for coeff in self.coefficients.iter().rev() {
            result = result * x + Complex::new(*coeff, ZERO);
        }
        result
    }

    pub fn derivative(&self) -> LogNumPoly {
        let mut coefficients: Vec<LogNum> = vec![];
        for i in 1..self.coefficients.len() {
            coefficients.push(self.get_coefficient(i) * LogNum::from(i as i32));
        }
        Self::from_coeffs(coefficients)
    }

    pub fn definite_int(&self, lower_bound : LogNum, upper_bound : LogNum) -> LogNum {
        let mut coefficients: Vec<LogNum> = vec![ZERO];
        for i in 0..self.coefficients.len() {
            coefficients.push(self.get_coefficient(i) / LogNum::from(i as i32 + 1));
        }
        let indefinite = Self::from_coeffs(coefficients);
        indefinite.eval(upper_bound) - indefinite.eval(lower_bound)
    }

    pub fn from_coeffs(coefficients: Vec<LogNum>) -> Self {
        let mut p = LogNumPoly { coefficients };
        p.cleanup();
        p
    }

    pub fn normalize(&mut self) {
        if self.is_zero() { return; }
        *self /= *self.coefficients.last().unwrap();
    }

    pub fn solve(&mut self) -> Vec<Complex<LogNum>> {
        let mut local = self.clone();
        local.normalize();
        match local.order(){
            0 => Vec::new(),
            1 => vec!(Complex::from(-local.coefficients[0])),
            2 => {
                let b = Complex::from(local.coefficients[1]);
                let c = Complex::from(local.coefficients[0]);
                let delta = b * b - c * LogNum::from(4);
                vec!((-b + delta.sqrt()) / LogNum::from(2),(-b - delta.sqrt()) / LogNum::from(2))
            },
            n => {
                //Durand–Kerner
                //Initial Guess
                let mut roots = Vec::new();
                for k in 0..n {
                    roots.push(Complex::from_polar(LogNum::from(1.3), LogNum::from(k as f64 / n as f64 * 2. * PI as f64)));
                }
                // Iteration
                const MAX_ITER: usize = 100;
                let eps = LogNum::from(1e-6);

                for _ in 0..MAX_ITER {
                    let mut converged = true;

                    let old = roots.clone();

                    for i in 0..n {
                        let mut denom = Complex::new(ONE, ZERO);

                        for j in 0..n {
                            if i != j {
                                denom *= old[i as usize] - old[j as usize];
                            }
                        }

                        let correction =
                            local.eval_complex(old[i as usize]) / denom;

                        roots[i as usize] -= correction;

                        if correction.norm() > eps {
                            converged = false;
                        }
                    }

                    if converged {
                        return roots;
                    }
                }
                roots
            }
        }
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
                coefficients[i + j] += self.coefficients[i] * rhs.coefficients[j];
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

impl Div<LogNum> for LogNumPoly {
    type Output = LogNumPoly;
    fn div(mut self, rhs: LogNum) -> LogNumPoly {
        if rhs.is_zero() {
            panic!("division by zero");
        }
        for coeff in &mut self.coefficients {
            *coeff = *coeff / rhs;
        }
        self.cleanup();
        self
    }
}

impl DivAssign<LogNum> for LogNumPoly {
    fn div_assign(&mut self, rhs: LogNum) {
        if rhs.is_zero() {
            panic!("division by zero");
        }
        for coeff in &mut self.coefficients {
            *coeff /= rhs;
        }
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn acceptable(estimation : Complex<LogNum>, value : Complex<LogNum>) -> bool{
        (value-estimation).norm() < LogNum::from(1e-6)
    }
    #[test]
    fn poly_solve_order_1() {
        let mut p = LogNumPoly::from_coeffs(vec![LogNum::from(1.0), LogNum::from(2.0)]);
        let roots = p.solve();
        assert!(acceptable(roots[0], Complex::from(LogNum::from(-1./2.))));
        assert_eq!(roots.len(), 1);
    }
    #[test]
    fn poly_solve_order_2() {
        let mut p = LogNumPoly::from_coeffs(vec![LogNum::from(-1.0), LogNum::from(0.0), LogNum::from(1.0)]);
        let roots = p.solve();
        assert!(acceptable(roots[0], Complex::from(ONE)));
        assert!(acceptable(roots[1], Complex::from(-ONE)));
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn poly_solve_order_2_im() {
        let mut p = LogNumPoly::from_coeffs(vec![LogNum::from(1.0), LogNum::from(0.0), LogNum::from(1.0)]);
        let roots = p.solve();
        assert!(acceptable(roots[0], Complex::i()));
        assert!(acceptable(roots[1], -Complex::i()));
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn poly_solve_order_4_mixed() {
        for a in -2..=2{
            for b in -2..=2{
                for c in -2..=2{
                    for d in -2..=2{
                        for e in -2..=2{
                            let mut p = LogNumPoly::from_coeffs(vec![
                                LogNum::from(a), // constant
                                LogNum::from(b),  // x
                                LogNum::from(c),  // x^2
                                LogNum::from(d),  // x^3
                                LogNum::from(e),  // x^4
                            ]);
                            p.cleanup();

                            let roots = p.solve();
                            assert_eq!(roots.len() as i32, p.order());

                            for r in roots{
                                assert!(acceptable(p.eval_complex(r),Complex::zero()));
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn poly_solve_order_high() {
        let mut v: Vec<LogNum> = Vec::new();
        v.resize(20,ZERO);
        v[10] = ONE;
        v[0] = -ONE;

        let mut p = LogNumPoly::from_coeffs(v);

        let roots = p.solve();
        assert_eq!(roots.len() as i32, p.order());

        for r in roots{
            println!("{}",r);
            assert!(acceptable(p.eval_complex(r),Complex::zero()));
        }
    }
}