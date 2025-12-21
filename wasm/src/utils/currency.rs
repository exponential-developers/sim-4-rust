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

    pub fn new(symb: &str) -> Self {
        Currency {
            value: lognum::ZERO,
            symbol: symb.to_owned()
        }
    }

    pub fn add(&mut self, value: LogNum) {
        self.value += value
    }

    pub fn subtract(&mut self, value: LogNum) {
        self.value -= value
    }
}