use crate::utils::lognum::LogNum;
use num::complex::Complex;

fn _test() {
    let a: Complex<LogNum> = Complex::new(LogNum::from_f64(2.), LogNum::from_f64(3.));
    let b: Complex<LogNum> = Complex::new(LogNum::from_f64(4.), LogNum::from_f64(-1.));
    let c = a.powc(b);
    
}