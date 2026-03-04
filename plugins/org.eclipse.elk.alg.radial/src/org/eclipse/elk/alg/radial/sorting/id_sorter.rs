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
        // Pre-extract ORDER_ID to avoid O(N log N) borrows in comparator
        let mut keyed: Vec<(i32, ElkNodeRef)> = nodes
            .drain(..)
            .map(|n| {
                let key = Self::order_id(&n);
                (key, n)
            })
            .collect();
        keyed.sort_by(|a, b| a.0.cmp(&b.0));
        nodes.extend(keyed.into_iter().map(|(_, n)| n));
    }

    fn initialize(&mut self, _root: &ElkNodeRef) {
        // nothing to do
    }
}
