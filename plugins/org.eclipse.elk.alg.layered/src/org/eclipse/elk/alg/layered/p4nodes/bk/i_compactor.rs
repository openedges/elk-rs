use super::aligned_layout::BKAlignedLayout;
use super::neighborhood_information::NeighborhoodInformation;

pub trait ICompactor {
    fn horizontal_compaction(&mut self, bal: &mut BKAlignedLayout, ni: &NeighborhoodInformation);
}
