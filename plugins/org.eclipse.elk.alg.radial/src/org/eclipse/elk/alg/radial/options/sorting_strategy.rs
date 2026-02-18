use crate::org::eclipse::elk::alg::radial::sorting::{
    IDSorter, IRadialSorter, PolarCoordinateSorter,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum SortingStrategy {
    #[default]
    None,
    PolarCoordinate,
    Id,
}

impl SortingStrategy {
    pub fn create(&self) -> Option<Box<dyn IRadialSorter>> {
        match self {
            SortingStrategy::None => None,
            SortingStrategy::PolarCoordinate => Some(Box::new(PolarCoordinateSorter::default())),
            SortingStrategy::Id => Some(Box::new(IDSorter)),
        }
    }
}
