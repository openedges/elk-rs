use crate::org::eclipse::elk::alg::radial::intermediate::overlaps::{
    IOverlapRemoval, RadiusExtensionOverlapRemoval,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum OverlapRemovalStrategy {
    ExtentRadii,
}

impl OverlapRemovalStrategy {
    pub fn create(&self) -> Box<dyn IOverlapRemoval> {
        match self {
            OverlapRemovalStrategy::ExtentRadii => Box::new(RadiusExtensionOverlapRemoval::default()),
        }
    }
}

impl Default for OverlapRemovalStrategy {
    fn default() -> Self {
        OverlapRemovalStrategy::ExtentRadii
    }
}
