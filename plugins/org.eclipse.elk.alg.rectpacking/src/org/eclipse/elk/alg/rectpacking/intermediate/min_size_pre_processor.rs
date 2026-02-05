use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, IElkProgressMonitor};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::options::InternalProperties;

pub struct MinSizePreProcessor;

impl ILayoutProcessor<ElkNodeRef> for MinSizePreProcessor {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Min Size Preprocessing", 1.0);
        let min_size = ElkUtil::effective_min_size_constraint_for(graph);
        let mut graph_mut = graph.borrow_mut();
        graph_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(InternalProperties::MIN_WIDTH, Some(min_size.x));
        graph_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(InternalProperties::MIN_HEIGHT, Some(min_size.y));
        progress_monitor.done();
    }
}
