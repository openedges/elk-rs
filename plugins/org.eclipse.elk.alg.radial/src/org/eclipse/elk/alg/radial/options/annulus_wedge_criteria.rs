use crate::org::eclipse::elk::alg::radial::p1position::wedge::{
    AnnulusWedgeByLeafs, AnnulusWedgeByNodeSpace, IAnnulusWedgeCriteria,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum AnnulusWedgeCriteria {
    LeafNumber,
    #[default]
    NodeSize,
}

impl AnnulusWedgeCriteria {
    pub fn create(&self) -> Box<dyn IAnnulusWedgeCriteria> {
        match self {
            AnnulusWedgeCriteria::LeafNumber => Box::new(AnnulusWedgeByLeafs),
            AnnulusWedgeCriteria::NodeSize => Box::new(AnnulusWedgeByNodeSpace),
        }
    }
}
