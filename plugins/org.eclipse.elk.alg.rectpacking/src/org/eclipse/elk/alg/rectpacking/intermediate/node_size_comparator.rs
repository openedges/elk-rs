use std::cmp::Ordering;

use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

pub struct NodeSizeComparator;

impl NodeSizeComparator {
    pub fn compare(node0: &ElkNodeRef, node1: &ElkNodeRef) -> Ordering {
        let height0 = node0.borrow_mut().connectable().shape().height();
        let height1 = node1.borrow_mut().connectable().shape().height();
        height1
            .partial_cmp(&height0)
            .unwrap_or(Ordering::Equal)
    }
}
