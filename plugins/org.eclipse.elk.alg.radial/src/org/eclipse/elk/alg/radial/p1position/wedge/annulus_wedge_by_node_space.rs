use crate::org::eclipse::elk::alg::radial::p1position::wedge::IAnnulusWedgeCriteria;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

#[derive(Default)]
pub struct AnnulusWedgeByNodeSpace;

impl IAnnulusWedgeCriteria for AnnulusWedgeByNodeSpace {
    fn calculate_wedge_space(&self, node: &ElkNodeRef) -> f64 {
        let successors = RadialUtil::get_successors(node);
        let (width, height) = {
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            (shape.width(), shape.height())
        };
        let node_size = (width * width + height * height).sqrt();

        let mut child_space = 0.0;
        for child in successors {
            child_space += self.calculate_wedge_space(&child);
        }
        child_space.max(node_size)
    }
}
