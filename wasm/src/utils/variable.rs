use crate::utils::{
    lognum::{self, LogNum},
    cost::Cost,
    value::Value,
    //currency::Currency
};

#[derive(Debug, Clone)]
pub struct Variable<C: Cost, V: Value> {
    pub name: String,
    pub cost_model: C,
    pub value_model: V,
    // may or may not be implemented
    //pub currency: &'a Currency,
    pub level: i32,
    pub cost: LogNum,
    pub value: LogNum,

    // Also implement hotab cost stuff if needed
}

impl<C: Cost, V: Value> Variable<C, V> {
    pub fn new(name: &str, cost_model: C, value_model: V) -> Self {
        let mut var = Variable {
            name: name.to_owned(),
            cost_model: cost_model,
            value_model: value_model,
            level: 0,
            cost: lognum::ZERO,
            value: lognum::ZERO
        };
        var.recompute();

        var
    }

    pub fn recompute(&mut self) {
        self.cost = self.cost_model.get_cost(self.level);
        self.value = self.value_model.recompute(self.level);
    }

    pub fn buy(&mut self) {
        self.value = self.value_model.compute_new(self.value, self.level);
        self.level += 1;
        self.cost = self.cost_model.get_cost(self.level);
    }

    pub fn set(&mut self, level: i32) {
        self.level = level;
        self.recompute();
    }
}