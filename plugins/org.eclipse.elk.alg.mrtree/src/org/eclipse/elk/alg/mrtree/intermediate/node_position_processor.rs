use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::{TGraphRef, TNodeRef};
use crate::org::eclipse::elk::alg::mrtree::options::InternalProperties;

#[derive(Default)]
pub struct NodePositionProcessor {
    number_of_nodes: usize,
}

impl ILayoutProcessor<TGraphRef> for NodePositionProcessor {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Processor set coordinates", 1.0);

        let (nodes, root) = {
            let graph_guard = match graph.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    progress_monitor.done();
                    return;
                }
            };
            let nodes = graph_guard.nodes().clone();
            let root = nodes
                .iter()
                .find(|node| {
                    node.lock()
                        .ok()
                        .and_then(|mut node_guard| node_guard.get_property(InternalProperties::ROOT))
                        .unwrap_or(false)
                })
                .cloned();
            (nodes, root)
        };

        self.number_of_nodes = if nodes.is_empty() { 1 } else { nodes.len() };

        if let Some(root) = root {
            if let Ok(mut root_guard) = root.lock() {
                let x = root_guard.get_property(InternalProperties::XCOOR).unwrap_or(0) as f64;
                let y = root_guard.get_property(InternalProperties::YCOOR).unwrap_or(0) as f64;
                let pos = root_guard.position();
                pos.x = x;
                pos.y = y;
            }

            let mut next_level = root
                .lock()
                .ok()
                .map(|node_guard| node_guard.children_copy())
                .unwrap_or_default();
            let mut sub_tasks = 1.0f32;
            while !next_level.is_empty() {
                next_level = self.set_coordinates(&next_level, progress_monitor.sub_task(sub_tasks));
                sub_tasks = next_level.len() as f32 / self.number_of_nodes as f32;
            }
        }

        let nodes = {
            let graph_guard = match graph.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    progress_monitor.done();
                    return;
                }
            };
            graph_guard.nodes().clone()
        };
        for node in nodes {
            if let Ok(mut node_guard) = node.lock() {
                let size = node_guard.size_ref();
                let offset = KVector::with_values(size.x / 2.0, size.y / 2.0);
                node_guard.position().sub(&offset);
            }
        }

        progress_monitor.done();
    }
}

impl NodePositionProcessor {
    fn set_coordinates(
        &self,
        current_level: &[TNodeRef],
        mut progress_monitor: Box<dyn IElkProgressMonitor>,
    ) -> Vec<TNodeRef> {
        if current_level.is_empty() {
            return Vec::new();
        }

        progress_monitor.begin("Set coordinates", 1.0);
        let mut next_level: Vec<TNodeRef> = Vec::new();

        for node in current_level {
            if let Ok(mut node_guard) = node.lock() {
                next_level.extend(node_guard.children_copy());
                let x = node_guard.get_property(InternalProperties::XCOOR).unwrap_or(0) as f64;
                let y = node_guard.get_property(InternalProperties::YCOOR).unwrap_or(0) as f64;
                let pos = node_guard.position();
                pos.x = x;
                pos.y = y;
            }
        }

        progress_monitor.done();
        next_level
    }
}
