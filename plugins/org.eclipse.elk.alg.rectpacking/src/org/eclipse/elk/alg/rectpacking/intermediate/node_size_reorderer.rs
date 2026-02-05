use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use super::node_size_comparator::NodeSizeComparator;

pub struct NodeSizeReorderer;

impl ILayoutProcessor<ElkNodeRef> for NodeSizeReorderer {
    fn process(&mut self, graph: &mut ElkNodeRef, _progress_monitor: &mut dyn IElkProgressMonitor) {
        let mut children = collect_children(graph);
        children.sort_by(|a, b| NodeSizeComparator::compare(a, b));
        reorder_children(graph, children);
    }
}

fn collect_children(graph: &ElkNodeRef) -> Vec<ElkNodeRef> {
    let mut graph_mut = graph.borrow_mut();
    graph_mut.children().iter().cloned().collect()
}

fn reorder_children(graph: &ElkNodeRef, new_order: Vec<ElkNodeRef>) {
    let mut graph_mut = graph.borrow_mut();
    let children = graph_mut.children();
    while children.len() > 0 {
        children.remove_at(0);
    }
    for child in new_order {
        children.add(child);
    }
}
