use crate::org::eclipse::elk::alg::radial::intermediate::optimization::{
    CrossingMinimizationPosition, EdgeLengthOptimization, EdgeLengthPositionOptimization, IEvaluation,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum RadialTranslationStrategy {
    #[default]
    None,
    EdgeLength,
    EdgeLengthByPosition,
    CrossingMinimizationByPosition,
}

impl RadialTranslationStrategy {
    pub fn create(&self) -> Option<Box<dyn IEvaluation>> {
        match self {
            RadialTranslationStrategy::None => None,
            RadialTranslationStrategy::EdgeLength => Some(Box::new(EdgeLengthOptimization)),
            RadialTranslationStrategy::EdgeLengthByPosition => {
                Some(Box::new(EdgeLengthPositionOptimization))
            }
            RadialTranslationStrategy::CrossingMinimizationByPosition => {
                Some(Box::new(CrossingMinimizationPosition))
            }
        }
    }
}
