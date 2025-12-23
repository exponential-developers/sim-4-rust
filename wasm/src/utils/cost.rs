use num::Float;
use crate::utils::lognum;
use crate::utils::lognum::LogNum;

pub trait Cost {
    fn get_cost_to(&self, level: i32) -> LogNum;
    fn get_total_cost_to(&self, level: i32) -> LogNum;
    #[inline]
    fn get_cost(&self, current_level: i32) -> LogNum{
        self.get_cost_to(current_level+1)
    }
}




#[derive(Debug, Clone, Copy)]
pub struct CompositeCost<T: Cost, U: Cost> {
    pub model1: T,
    pub model2: U,
    pub cutoff: i32,
}

impl<T: Cost, U: Cost> CompositeCost<T, U> {
    pub fn new(model1: T, model2: U, cutoff: i32) -> Self {
        CompositeCost { 
            model1, 
            model2, 
            cutoff 
        }
    } 
}

impl<T: Cost, U: Cost> Cost for CompositeCost<T, U> {
    fn get_cost_to(&self, level: i32) -> LogNum {
        if level<=0 {
            lognum::ZERO
        } else if level <= self.cutoff{
            self.model1.get_cost_to(level)
        } else {
            self.model2.get_cost_to(level - self.cutoff)
        }
    }

    fn get_total_cost_to(&self, level: i32) -> LogNum {
        if level<=0 {
            lognum::ZERO
        }
        else if level <= self.cutoff{
            self.model1.get_total_cost_to(level)
        }else{
            self.model2.get_total_cost_to(level - self.cutoff)+self.model1.get_total_cost_to(self.cutoff)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExponentialCost {
    coefficient: LogNum,
    base: LogNum
}

impl ExponentialCost {
    pub fn new(coefficient: impl Into<LogNum>, base: impl Into<LogNum>) -> Self {
        ExponentialCost { coefficient: coefficient.into(), base: base.into() }
    }

    pub fn new_log2(coefficient: impl Into<LogNum>, base: impl Into<LogNum>) -> Self {
        ExponentialCost { coefficient: coefficient.into(), base: LogNum::from_f64(2.).pow(base.into()) }
    }
}

impl Cost for ExponentialCost {

    fn get_cost_to(&self, level: i32) -> LogNum {
        if level<=0 {
            lognum::ZERO
        }else{
            self.coefficient * self.base.powi(level-1)
        }
    }
    fn get_total_cost_to(&self, level: i32) -> LogNum {
        if level<=0 {
            lognum::ZERO
        }else{
            self.coefficient * (self.base.powi(level)-lognum::ONE)/(self.base-lognum::ONE)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StepwiseCost<T: Cost> {
    model: T,
    step: i32
}

impl<T: Cost> StepwiseCost<T> {
    pub fn new(model: T, step: i32) -> Self {
        StepwiseCost {
            model,
            step
        }
    }
}

impl<T: Cost> Cost for StepwiseCost<T> {
    fn get_cost_to(&self, level: i32) -> LogNum {
        if level<=0 {
            lognum::ZERO
        }else{
            self.model.get_cost_to((level-1) / self.step + 1)
        }
    }
    fn get_total_cost_to(&self, level: i32) -> LogNum {
        if level<=0 {
            lognum::ZERO
        }else {
            let int_part = (level - 1) / self.step;
            let mod_part = (level - 1) % self.step + 1;
            self.model.get_total_cost_to(int_part) * self.step.into() + self.model.get_cost_to(int_part + 1) * mod_part.into()
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub struct ConstantCost {
    cost: LogNum
}

impl ConstantCost {
    pub fn new(cost: impl Into<LogNum>) -> Self {
        ConstantCost {
            cost: cost.into()
        }
    }
}

impl Cost for ConstantCost {
    fn get_cost_to(&self, level: i32) -> LogNum {
        if level<=0 {
            lognum::ZERO
        }else{
            self.cost
        }
    }
    fn get_total_cost_to(&self, level: i32) -> LogNum {
        if level<=0 {
            lognum::ZERO
        }else{
            self.cost*level.into()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FirstFreeCost<T: Cost> {
    model: T
}

impl<T: Cost> FirstFreeCost<T> {
    pub fn new(model: T) -> Self {
        FirstFreeCost { model }
    }
}

impl<T: Cost> Cost for FirstFreeCost<T> {
    fn get_cost_to(&self, level: i32) -> LogNum {
        self.model.get_cost_to(level - 1)
    }
    fn get_total_cost_to(&self, level: i32) -> LogNum {
        self.model.get_total_cost_to(level - 1)
    }
}