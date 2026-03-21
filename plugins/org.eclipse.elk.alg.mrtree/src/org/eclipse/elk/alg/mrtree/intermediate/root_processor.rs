use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::{TGraphRef, TNode};
use crate::org::eclipse::elk::alg::mrtree::options::InternalProperties;

#[derive(Default)]
pub struct RootProcessor {
    roots: Vec<crate::org::eclipse::elk::alg::mrtree::graph::TNodeRef>,
}

impl ILayoutProcessor<TGraphRef> for RootProcessor {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Find roots", 1.0);
        self.roots.clear();

        let nodes: Vec<_> = {
            let graph_guard = match graph.lock_ok() {
            Some(guard) => guard,
            None => {
                    progress_monitor.done();
                    return;
                }
            };
            graph_guard.nodes().clone()
        };

        for node in &nodes {
            if let Some(mut node_guard) = node.lock_ok() {
                if node_guard.incoming_edges().is_empty() {
                    node_guard.set_property(InternalProperties::ROOT, Some(true));
                    self.roots.push(node.clone());
                }
            }
        }

        match self.roots.len() {
            0 => {
                // empty graph; create a dummy root
                let root = TNode::new_with_label(0, Some(graph.clone()), "DUMMY_ROOT");
                if let Some(mut root_guard) = root.lock_ok() {
                    root_guard.set_property(InternalProperties::ROOT, Some(true));
                    root_guard.set_property(InternalProperties::DUMMY, Some(true));
                };
            }
            1 => {
                // already have a root
            }
            _ => {
                let super_root = TNode::new_with_label(0, Some(graph.clone()), "SUPER_ROOT");
                for root in &self.roots {
                    TNode::add_child(&super_root, root);
                    if let Some(mut root_guard) = root.lock_ok() {
                        root_guard.set_property(InternalProperties::ROOT, Some(false));
                    }
                }
                if let Some(mut root_guard) = super_root.lock_ok() {
                    root_guard.set_property(InternalProperties::ROOT, Some(true));
                    root_guard.set_property(InternalProperties::DUMMY, Some(true));
                };
            }
        }

        progress_monitor.done();
    }
}
