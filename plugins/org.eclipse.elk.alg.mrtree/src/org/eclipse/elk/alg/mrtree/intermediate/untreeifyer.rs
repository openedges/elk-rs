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
            let graph_guard = graph.lock();
            graph_guard
                .get_property(InternalProperties::REMOVABLE_EDGES)
                .unwrap_or_default()
        };

        for edge in edges {
            let source = edge.lock().source();
            let target = edge.lock().target();
            if let Some(source) = source {
                {
                    let mut node_guard = source.lock();
                    node_guard.add_outgoing(edge.clone());
                }
            }
            if let Some(target) = target {
                {
                    let mut node_guard = target.lock();
                    node_guard.add_incoming(edge.clone());
                }
            }
        }

        progress_monitor.done();
    }
}
