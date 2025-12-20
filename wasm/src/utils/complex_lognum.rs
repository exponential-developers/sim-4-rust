use crate::utils::lognum::LogNum;
use num::complex::Complex;

fn _test() {
    let a: Complex<LogNum> = Complex::new(LogNum::from(2.), LogNum::from(3.));
    let b: Complex<LogNum> = Complex::new(LogNum::from(4.), LogNum::from(-1.));
}