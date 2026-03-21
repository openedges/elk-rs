use std::collections::HashMap;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::{TGraphRef, TNodeRef};
use crate::org::eclipse::elk::alg::mrtree::options::{InternalProperties, MrTreeOptions};

#[derive(Default)]
pub struct LevelProcessor {
    level_map: HashMap<i32, i32>,
}

impl ILayoutProcessor<TGraphRef> for LevelProcessor {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Processor compute level", 1.0);
        self.level_map.clear();

        let roots: Vec<TNodeRef> = {
            let graph_guard = match graph.lock_ok() {
            Some(guard) => guard,
            None => {
                    progress_monitor.done();
                    return;
                }
            };
            graph_guard
                .nodes()
                .iter()
                .filter_map(|node| {
                    node.lock_ok().and_then(|mut node_guard| {
                        if node_guard
                            .get_property(InternalProperties::ROOT)
                            .unwrap_or(false)
                        {
                            Some(node.clone())
                        } else {
                            None
                        }
                    })
                })
                .collect()
        };

        self.set_level(&roots, 0);

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

        for node in nodes {
            if let Some(mut node_guard) = node.lock_ok() {
                let level = self.level_map.get(&node_guard.id()).cloned().unwrap_or(0);
                node_guard.set_property(MrTreeOptions::TREE_LEVEL, Some(level));
            }
        }

        progress_monitor.done();
    }
}

impl LevelProcessor {
    fn set_level(&mut self, current_level: &[TNodeRef], level: i32) {
        if current_level.is_empty() {
            return;
        }

        let mut next_level: Vec<TNodeRef> = Vec::new();
        for node in current_level {
            if let Some(node_guard) = node.lock_ok() {
                self.level_map.insert(node_guard.id(), level);
                next_level.extend(node_guard.children());
            }
        }

        self.set_level(&next_level, level + 1);
    }
}
