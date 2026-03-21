use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::TGraphRef;
use crate::org::eclipse::elk::alg::mrtree::options::InternalProperties;

#[derive(Default)]
pub struct GraphBoundsProcessor;

impl ILayoutProcessor<TGraphRef> for GraphBoundsProcessor {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Process graph bounds", 1.0);

        let nodes = {
            let graph_guard = match graph.lock_ok() {
            Some(guard) => guard,
            None => {
                    progress_monitor.done();
                    return;
                }
            };
            graph_guard.nodes().clone()
        };

        let mut xmin = f64::MAX;
        let mut ymin = f64::MAX;
        let mut xmax = f64::MIN;
        let mut ymax = f64::MIN;

        for node in nodes {
            if let Some(node_guard) = node.lock_ok() {
                let pos = node_guard.position_ref();
                let size = node_guard.size_ref();
                xmin = xmin.min(pos.x);
                ymin = ymin.min(pos.y);
                xmax = xmax.max(pos.x + size.x);
                ymax = ymax.max(pos.y + size.y);
            }
        }

        if let Some(mut graph_guard) = graph.lock_ok() {
            graph_guard.set_property(InternalProperties::GRAPH_XMIN, Some(xmin));
            graph_guard.set_property(InternalProperties::GRAPH_YMIN, Some(ymin));
            graph_guard.set_property(InternalProperties::GRAPH_XMAX, Some(xmax));
            graph_guard.set_property(InternalProperties::GRAPH_YMAX, Some(ymax));
        }

        progress_monitor.done();
    }
}
