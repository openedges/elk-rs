use crate::org::eclipse::elk::alg::radial::p1position::wedge::{
    AnnulusWedgeByLeafs, AnnulusWedgeByNodeSpace, IAnnulusWedgeCriteria,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum AnnulusWedgeCriteria {
    LeafNumber,
    NodeSize,
}

impl AnnulusWedgeCriteria {
    pub fn create(&self) -> Box<dyn IAnnulusWedgeCriteria> {
        match self {
            AnnulusWedgeCriteria::LeafNumber => Box::new(AnnulusWedgeByLeafs::default()),
            AnnulusWedgeCriteria::NodeSize => Box::new(AnnulusWedgeByNodeSpace::default()),
        }
    }
}

impl Default for AnnulusWedgeCriteria {
    fn default() -> Self {
        AnnulusWedgeCriteria::NodeSize
    }
}
