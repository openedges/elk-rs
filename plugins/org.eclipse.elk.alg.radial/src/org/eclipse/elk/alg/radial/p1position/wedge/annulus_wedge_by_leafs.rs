use crate::org::eclipse::elk::alg::radial::p1position::wedge::IAnnulusWedgeCriteria;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

#[derive(Default)]
pub struct AnnulusWedgeByLeafs;

impl IAnnulusWedgeCriteria for AnnulusWedgeByLeafs {
    fn calculate_wedge_space(&self, node: &ElkNodeRef) -> f64 {
        RadialUtil::get_number_of_leaves(node)
    }
}
