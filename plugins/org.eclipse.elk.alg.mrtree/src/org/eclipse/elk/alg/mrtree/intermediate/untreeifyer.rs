use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::TGraphRef;
use crate::org::eclipse::elk::alg::mrtree::options::InternalProperties;

#[derive(Default)]
pub struct Untreeifyer;

impl ILayoutProcessor<TGraphRef> for Untreeifyer {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Untreeify", 1.0);

        let edges = {
            let mut graph_guard = match graph.lock_ok() {
            Some(guard) => guard,
            None => {
                    progress_monitor.done();
                    return;
                }
            };
            graph_guard
                .get_property(InternalProperties::REMOVABLE_EDGES)
                .unwrap_or_default()
        };

        for edge in edges {
            let source = edge.lock_ok().and_then(|edge_guard| edge_guard.source());
            let target = edge.lock_ok().and_then(|edge_guard| edge_guard.target());
            if let Some(source) = source {
                if let Some(mut node_guard) = source.lock_ok() {
                    node_guard.add_outgoing(edge.clone());
                }
            }
            if let Some(target) = target {
                if let Some(mut node_guard) = target.lock_ok() {
                    node_guard.add_incoming(edge.clone());
                }
            }
        }

        progress_monitor.done();
    }
}
