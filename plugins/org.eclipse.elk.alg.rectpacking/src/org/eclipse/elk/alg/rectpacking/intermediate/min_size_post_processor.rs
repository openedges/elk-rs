use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::options::InternalProperties;

pub struct MinSizePostProcessor;

impl ILayoutProcessor<ElkNodeRef> for MinSizePostProcessor {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Min Size Postprocessing", 1.0);
        let min_width = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(InternalProperties::MIN_WIDTH)
                .unwrap_or(0.0)
        };
        let target_width = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(InternalProperties::TARGET_WIDTH)
                .unwrap_or(0.0)
        };
        let mut graph_mut = graph.borrow_mut();
        graph_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(
                InternalProperties::TARGET_WIDTH,
                Some(target_width.max(min_width)),
            );
        progress_monitor.done();
    }
}
