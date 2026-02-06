use crate::org::eclipse::elk::alg::radial::intermediate::overlaps::{
    IOverlapRemoval, RadiusExtensionOverlapRemoval,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum OverlapRemovalStrategy {
    #[default]
    ExtentRadii,
}

impl OverlapRemovalStrategy {
    pub fn create(&self) -> Box<dyn IOverlapRemoval> {
        match self {
            OverlapRemovalStrategy::ExtentRadii => Box::new(RadiusExtensionOverlapRemoval::default()),
        }
    }
}
