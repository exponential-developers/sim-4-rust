use crate::utils::lognum::{self,LogNum};

#[derive(Debug, Clone)]
pub struct Currency {
    value: LogNum,
    symbol: String
}

impl Currency {
    pub fn new_rho() -> Self {
        Currency {
            value: lognum::ZERO,
            symbol: "ρ".to_owned()
        }
    }

    pub fn new(symbol: &str) -> Self {
        Currency {
            value: lognum::ZERO,
            symbol: symbol.to_owned()
        }
    }

    pub fn add(&mut self, value: LogNum) {
        self.value += value
    }

    pub fn subtract(&mut self, value: LogNum) {
        self.value -= value
    }
}