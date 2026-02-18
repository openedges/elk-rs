use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::{TGraphRef, TNodeRef};
use crate::org::eclipse::elk::alg::mrtree::options::InternalProperties;

#[derive(Default)]
pub struct NeighborsProcessor {
    number_of_nodes: usize,
}

impl ILayoutProcessor<TGraphRef> for NeighborsProcessor {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Processor set neighbors", 1.0);

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
                        .and_then(|mut node_guard| {
                            node_guard.get_property(InternalProperties::ROOT)
                        })
                        .unwrap_or(false)
                })
                .cloned();
            (nodes, root)
        };

        self.number_of_nodes = if nodes.is_empty() { 1 } else { nodes.len() };

        if let Some(root) = root {
            let children = root
                .lock()
                .ok()
                .map(|node| node.children())
                .unwrap_or_default();
            self.set_neighbors(&children, progress_monitor);
        }

        progress_monitor.done();
    }
}

impl NeighborsProcessor {
    fn set_neighbors(
        &self,
        current_level: &[TNodeRef],
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        if current_level.is_empty() {
            return;
        }

        let mut sub_task =
            progress_monitor.sub_task(current_level.len() as f32 / self.number_of_nodes as f32);
        sub_task.begin("Set neighbors in level", 1.0);

        let mut next_level: Vec<TNodeRef> = Vec::new();
        let mut left_neighbor: Option<TNodeRef> = None;

        for node in current_level {
            if let Ok(mut node_guard) = node.lock() {
                next_level.extend(node_guard.children());
                if let Some(left_node) = left_neighbor.as_ref() {
                    if let Ok(mut left_guard) = left_node.lock() {
                        left_guard.set_property(
                            InternalProperties::RIGHTNEIGHBOR,
                            Some(Some(node.clone())),
                        );
                        node_guard.set_property(
                            InternalProperties::LEFTNEIGHBOR,
                            Some(Some(left_node.clone())),
                        );
                        let same_parent = left_guard
                            .parent()
                            .zip(node_guard.parent())
                            .is_some_and(|(a, b)| std::sync::Arc::ptr_eq(&a, &b));
                        if same_parent {
                            left_guard.set_property(
                                InternalProperties::RIGHTSIBLING,
                                Some(Some(node.clone())),
                            );
                            node_guard.set_property(
                                InternalProperties::LEFTSIBLING,
                                Some(Some(left_node.clone())),
                            );
                        }
                    }
                }
            }
            left_neighbor = Some(node.clone());
        }

        sub_task.done();

        self.set_neighbors(&next_level, progress_monitor);
    }
}
