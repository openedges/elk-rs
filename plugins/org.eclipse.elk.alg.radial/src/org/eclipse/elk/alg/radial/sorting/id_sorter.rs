use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::options::RadialOptions;
use crate::org::eclipse::elk::alg::radial::sorting::IRadialSorter;

#[derive(Default)]
pub struct IDSorter;

impl IDSorter {
    fn order_id(node: &ElkNodeRef) -> i32 {
        let mut node_mut = node.borrow_mut();
        node_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .get_property(RadialOptions::ORDER_ID)
            .unwrap_or(0)
    }
}

impl IRadialSorter for IDSorter {
    fn sort(&mut self, nodes: &mut Vec<ElkNodeRef>) {
        nodes.sort_by(|a, b| {
            let a_id = Self::order_id(a);
            let b_id = Self::order_id(b);
            a_id.cmp(&b_id)
        });
    }

    fn initialize(&mut self, _root: &ElkNodeRef) {
        // nothing to do
    }
}
