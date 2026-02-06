use crate::org::eclipse::elk::alg::radial::intermediate::compaction::{
    AnnulusWedgeCompaction, IRadialCompactor, RadialCompaction,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum CompactionStrategy {
    #[default]
    None,
    RadialCompaction,
    WedgeCompaction,
}

impl CompactionStrategy {
    pub fn create(&self) -> Box<dyn IRadialCompactor> {
        match self {
            CompactionStrategy::RadialCompaction => Box::new(RadialCompaction::default()),
            CompactionStrategy::WedgeCompaction => Box::new(AnnulusWedgeCompaction::default()),
            CompactionStrategy::None => {
                panic!("No implementation is available for the layout option {:?}", self)
            }
        }
    }
}
