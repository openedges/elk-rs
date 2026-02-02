use std::any::TypeId;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct NormalizedHeuristics<T> {
    pub target: T,
    pub values: HashMap<TypeId, f64>,
}

pub trait IDecider<T: Clone> {
    fn make_attachment_decision(&self, heuristics: &[NormalizedHeuristics<T>]) -> Option<T>;
}
