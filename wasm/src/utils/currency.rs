/*!
 * Currency structure
 */

use crate::utils::lognum::{self, LogNum};

/** Struct representing a currency with a value and a symbol */
#[derive(Debug, Clone)]
pub struct Currency {
    value: LogNum,
    symbol: String,
}

impl Currency {
    pub fn new_rho() -> Self {
        Currency {
            value: lognum::ZERO,
            symbol: "ρ".to_owned(),
        }
    }

    pub fn new(symbol: &str) -> Self {
        Currency {
            value: lognum::ZERO,
            symbol: symbol.to_owned(),
        }
    }

    pub fn add(&mut self, value: LogNum) {
        self.value += value
    }

    pub fn subtract(&mut self, value: LogNum) {
        self.value -= value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_starts_at_zero_and_adds_and_subtracts_amounts() {
        let mut rho = Currency::new_rho();
        assert_eq!(rho.symbol, "ρ");
        assert_eq!(rho.value, lognum::ZERO);
        rho.add(LogNum::from(1000.));
        rho.add(LogNum::from(1000.));
        assert!((rho.value.value - 2000f64.log10()).abs() < 1e-12);
        rho.subtract(LogNum::from(1500.));
        assert!((rho.value.value - 500f64.log10()).abs() < 1e-12);

        let lambda = Currency::new("lambda");
        assert_eq!(lambda.symbol, "lambda");
        assert_eq!(lambda.value, lognum::ZERO);
    }
}
