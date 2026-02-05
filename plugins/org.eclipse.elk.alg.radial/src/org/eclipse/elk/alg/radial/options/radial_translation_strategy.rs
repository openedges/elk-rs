use crate::org::eclipse::elk::alg::radial::intermediate::optimization::{
    CrossingMinimizationPosition, EdgeLengthOptimization, EdgeLengthPositionOptimization, IEvaluation,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum RadialTranslationStrategy {
    None,
    EdgeLength,
    EdgeLengthByPosition,
    CrossingMinimizationByPosition,
}

impl RadialTranslationStrategy {
    pub fn create(&self) -> Option<Box<dyn IEvaluation>> {
        match self {
            RadialTranslationStrategy::None => None,
            RadialTranslationStrategy::EdgeLength => Some(Box::new(EdgeLengthOptimization::default())),
            RadialTranslationStrategy::EdgeLengthByPosition => {
                Some(Box::new(EdgeLengthPositionOptimization::default()))
            }
            RadialTranslationStrategy::CrossingMinimizationByPosition => {
                Some(Box::new(CrossingMinimizationPosition::default()))
            }
        }
    }
}

impl Default for RadialTranslationStrategy {
    fn default() -> Self {
        RadialTranslationStrategy::None
    }
}
