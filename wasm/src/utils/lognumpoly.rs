use crate::utils::lognum::{LogNum, ONE, ZERO};
use num::{Complex, Zero};
use std::cmp::{PartialEq, max};
use std::f32::consts::PI;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Debug, Clone, PartialEq)]
pub struct LogNumPoly {
    coefficients: Vec<LogNum>,
}

impl Zero for LogNumPoly {
    fn zero() -> LogNumPoly {
        LogNumPoly {
            coefficients: vec![],
        }
    }
    fn is_zero(&self) -> bool {
        self.coefficients.is_empty()
    }
}

impl LogNumPoly {
    pub fn order(&self) -> i32 {
        if self.is_zero() {
            return 0;
        }
        self.coefficients.len() as i32 - 1
    }
    #[inline]
    pub fn get_coefficient(&self, n: usize) -> LogNum {
        if self.coefficients.len() <= n {
            return ZERO;
        }
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

    pub fn definite_int(&self, lower_bound: LogNum, upper_bound: LogNum) -> LogNum {
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
        if self.is_zero() {
            return;
        }
        *self /= *self.coefficients.last().unwrap();
    }

    pub fn solve(&self) -> Vec<Complex<LogNum>> {
        let mut local = self.clone();
        local.normalize();
        match local.order() {
            0 => Vec::new(),
            1 => vec![Complex::from(-local.coefficients[0])],
            2 => {
                let b = Complex::from(local.coefficients[1]);
                let c = Complex::from(local.coefficients[0]);
                let delta = b * b - c * LogNum::from(4);
                vec![
                    (-b + delta.sqrt()) / LogNum::from(2),
                    (-b - delta.sqrt()) / LogNum::from(2),
                ]
            }
            n => {
                //https://en.wikipedia.org/wiki/Durand%E2%80%93Kerner_method
                //Initial Guess
                let mut roots = Vec::new();
                for k in 0..n {
                    roots.push(Complex::from_polar(
                        LogNum::from(1.3),
                        LogNum::from(k as f64 / n as f64 * 2. * PI as f64),
                    ));
                }
                // Iteration
                const MAX_ITER: usize = 100;
                let eps = LogNum::from(1e-6);

                for _ in 0..MAX_ITER {
                    let mut converged = true;

                    for i in 0..n {
                        let mut denom = Complex::new(ONE, ZERO);

                        for j in 0..n {
                            if i != j {
                                denom *= roots[i as usize] - roots[j as usize];
                            }
                        }

                        let correction = local.eval_complex(roots[i as usize]) / denom;

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
            coefficients.push(self.get_coefficient(i) + rhs.get_coefficient(i));
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
            coefficients.push(self.get_coefficient(i) - rhs.get_coefficient(i));
        }
        Self::from_coeffs(coefficients)
    }
}

impl AddAssign for LogNumPoly {
    fn add_assign(&mut self, rhs: Self) {
        self.coefficients
            .resize(max(self.coefficients.len(), rhs.coefficients.len()), ZERO);
        for i in 0..rhs.coefficients.len() {
            self.coefficients[i] += rhs.coefficients[i];
        }
        self.cleanup();
    }
}

impl SubAssign for LogNumPoly {
    fn sub_assign(&mut self, rhs: Self) {
        self.coefficients
            .resize(max(self.coefficients.len(), rhs.coefficients.len()), ZERO);
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
        let mut coefficients: Vec<LogNum> =
            vec![ZERO; self.coefficients.len() + rhs.coefficients.len() - 1usize];
        for i in 0..self.coefficients.len() {
            for j in 0..rhs.coefficients.len() {
                coefficients[i + j] += self.coefficients[i] * rhs.coefficients[j];
            }
        }
        Self::from_coeffs(coefficients)
    }
}

impl MulAssign<LogNumPoly> for LogNumPoly {
    fn mul_assign(&mut self, rhs: LogNumPoly) {
        // Same guard as `mul`: with two zero polynomials, the size below would underflow
        if self.is_zero() || rhs.is_zero() {
            *self = LogNumPoly::zero();
            return;
        }
        let mut coefficients: Vec<LogNum> =
            vec![ZERO; self.coefficients.len() + rhs.coefficients.len() - 1usize];

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
            *coeff *= rhs;
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
            *coeff /= rhs;
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
    fn acceptable(estimation: Complex<LogNum>, value: Complex<LogNum>) -> bool {
        (value - estimation).norm() < LogNum::from(1e-6)
    }
    #[test]
    fn poly_solve_order_1() {
        let p = LogNumPoly::from_coeffs(vec![LogNum::from(1.0), LogNum::from(2.0)]);
        let roots = p.solve();
        assert!(acceptable(roots[0], Complex::from(LogNum::from(-1. / 2.))));
        assert_eq!(roots.len(), 1);
    }
    #[test]
    fn poly_solve_order_2() {
        let p = LogNumPoly::from_coeffs(vec![
            LogNum::from(-1.0),
            LogNum::from(0.0),
            LogNum::from(1.0),
        ]);
        let roots = p.solve();
        assert!(acceptable(roots[0], Complex::from(ONE)));
        assert!(acceptable(roots[1], Complex::from(-ONE)));
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn poly_solve_order_2_im() {
        let p = LogNumPoly::from_coeffs(vec![
            LogNum::from(1.0),
            LogNum::from(0.0),
            LogNum::from(1.0),
        ]);
        let roots = p.solve();
        assert!(acceptable(roots[0], Complex::i()));
        assert!(acceptable(roots[1], -Complex::i()));
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn poly_solve_order_4_mixed() {
        for a in -2..=2 {
            for b in -2..=2 {
                for c in -2..=2 {
                    for d in -2..=2 {
                        for e in -2..=2 {
                            let p = LogNumPoly::from_coeffs(vec![
                                LogNum::from(a), // constant
                                LogNum::from(b), // x
                                LogNum::from(c), // x^2
                                LogNum::from(d), // x^3
                                LogNum::from(e), // x^4
                            ]);

                            let roots = p.solve();
                            assert_eq!(roots.len() as i32, p.order());

                            for r in roots {
                                assert!(acceptable(p.eval_complex(r), Complex::zero()));
                            }
                        }
                    }
                }
            }
        }
    }

    /** Polynomial from plain-number coefficients, constant term first */
    fn poly(coeffs: &[f64]) -> LogNumPoly {
        LogNumPoly::from_coeffs(coeffs.iter().map(|&c| LogNum::from(c)).collect())
    }

    /** Plain-number coefficients of a polynomial, constant term first */
    fn coeffs(p: &LogNumPoly) -> Vec<f64> {
        use num::ToPrimitive;
        (0..=p.order() as usize)
            .take(if p.is_zero() { 0 } else { usize::MAX })
            .map(|i| p.get_coefficient(i).to_f64().unwrap())
            .collect()
    }

    #[track_caller]
    fn assert_coeffs(p: &LogNumPoly, expected: &[f64]) {
        let actual = coeffs(p);
        assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
        for (a, e) in actual.iter().zip(expected) {
            assert!((a - e).abs() < 1e-9, "{actual:?} vs {expected:?}");
        }
    }

    fn real(x: LogNum) -> f64 {
        use num::ToPrimitive;
        x.to_f64().unwrap()
    }

    #[test]
    fn order_coefficients_and_trailing_zeros() {
        let zero = LogNumPoly::zero();
        assert!(zero.is_zero());
        assert_eq!(zero.order(), 0);
        // trailing zero coefficients are dropped: 1 + 2x + 0x^2 + 0x^3 has order 1
        let p = poly(&[1., 2., 0., 0.]);
        assert_eq!(p.order(), 1);
        assert_eq!(p.get_coefficient(5), ZERO);
        assert!(poly(&[0., 0.]).is_zero());
    }

    #[test]
    fn evaluation_derivative_and_integral() {
        let p = poly(&[1., 2., 3.]); // 1 + 2x + 3x^2
        assert!((real(p.eval(LogNum::from(2.))) - 17.).abs() < 1e-9);
        assert!((real(p.eval(LogNum::from(-1.))) - 2.).abs() < 1e-9);
        // at x = i: 1 + 2i - 3 = -2 + 2i
        let v = p.eval_complex(Complex::new(ZERO, ONE));
        assert!((real(v.re) + 2.).abs() < 1e-9 && (real(v.im) - 2.).abs() < 1e-9);

        assert_coeffs(&p.derivative(), &[2., 6.]);
        assert!(poly(&[5.]).derivative().is_zero());

        // integral of 1 + 2x + 3x^2 from 0 to 2 = [x + x^2 + x^3] = 14
        assert!((real(p.definite_int(ZERO, LogNum::from(2.))) - 14.).abs() < 1e-9);
        assert!((real(p.definite_int(LogNum::from(1.), LogNum::from(2.))) - 11.).abs() < 1e-9);
    }

    #[test]
    fn normalize_makes_the_leading_coefficient_one() {
        let mut p = poly(&[4., 2.]); // 2x + 4 -> x + 2
        p.normalize();
        assert_coeffs(&p, &[2., 1.]);
        let mut zero = LogNumPoly::zero();
        zero.normalize();
        assert!(zero.is_zero());
        // a constant has no roots
        assert!(poly(&[3.]).solve().is_empty());
    }

    #[test]
    fn polynomial_arithmetic() {
        let a = poly(&[1., 1.]); // x + 1
        let b = poly(&[-1., 1.]); // x - 1
        assert_coeffs(&(a.clone() + b.clone()), &[0., 2.]);
        assert_coeffs(&(a.clone() - b.clone()), &[2.]); // x terms cancel: order drops
        assert_coeffs(&(-a.clone()), &[-1., -1.]);
        assert_coeffs(&(a.clone() * b.clone()), &[-1., 0., 1.]); // x^2 - 1
        assert!((a.clone() * LogNumPoly::zero()).is_zero());
        assert!((LogNumPoly::zero() * a.clone()).is_zero());

        let mut c = a.clone();
        c += poly(&[0., 0., 3.]);
        assert_coeffs(&c, &[1., 1., 3.]);
        c -= poly(&[0., 0., 3.]);
        assert_coeffs(&c, &[1., 1.]);
        c -= a.clone();
        assert!(c.is_zero());

        let mut d = a.clone();
        d *= b.clone();
        assert_coeffs(&d, &[-1., 0., 1.]);
        let mut z = LogNumPoly::zero();
        z *= LogNumPoly::zero(); // must not underflow
        assert!(z.is_zero());
        d *= LogNumPoly::zero();
        assert!(d.is_zero());
    }

    #[test]
    fn scalar_multiplication_and_division() {
        let p = poly(&[1., 2.]);
        assert_coeffs(&(p.clone() * LogNum::from(3.)), &[3., 6.]);
        assert!((p.clone() * ZERO).is_zero());
        assert_coeffs(&(p.clone() / LogNum::from(2.)), &[0.5, 1.]);
        let mut q = p.clone();
        q *= LogNum::from(-2.);
        assert_coeffs(&q, &[-2., -4.]);
        q /= LogNum::from(-2.);
        assert_coeffs(&q, &[1., 2.]);
        q *= ZERO;
        assert!(q.is_zero());
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn dividing_by_zero_panics() {
        let _ = poly(&[1., 2.]) / ZERO;
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn dividing_in_place_by_zero_panics() {
        let mut p = poly(&[1., 2.]);
        p /= ZERO;
    }

    #[test]
    fn poly_solve_order_high() {
        let mut v: Vec<LogNum> = Vec::new();
        v.resize(20, ZERO);
        v[10] = ONE;
        v[0] = -ONE;

        let p = LogNumPoly::from_coeffs(v);

        let roots = p.solve();
        assert_eq!(roots.len() as i32, p.order());

        for r in roots {
            assert!(acceptable(p.eval_complex(r), Complex::zero()));
        }
    }
}
