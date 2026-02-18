use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::{TGraphRef, TNodeRef};
use crate::org::eclipse::elk::alg::mrtree::options::{InternalProperties, MrTreeOptions};

#[derive(Default)]
pub struct LevelHeightProcessor {
    number_of_nodes: usize,
}

impl ILayoutProcessor<TGraphRef> for LevelHeightProcessor {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Processor determine the height for each level", 1.0);

        let (nodes, root, direction) = {
            let mut graph_guard = match graph.lock() {
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
            let direction = graph_guard
                .get_property(MrTreeOptions::DIRECTION)
                .unwrap_or(Direction::Undefined);
            (nodes, root, direction)
        };

        self.number_of_nodes = if nodes.is_empty() { 1 } else { nodes.len() };

        if let Some(root) = root {
            let mut current = vec![root];
            self.set_level_height(&mut current, progress_monitor, direction);
        }

        progress_monitor.done();
    }
}

impl LevelHeightProcessor {
    fn set_level_height(
        &self,
        current_level: &mut [TNodeRef],
        progress_monitor: &mut dyn IElkProgressMonitor,
        direction: Direction,
    ) {
        if current_level.is_empty() {
            return;
        }

        let mut sub_task =
            progress_monitor.sub_task(current_level.len() as f32 / self.number_of_nodes as f32);
        sub_task.begin("Set neighbors in level", 1.0);

        let mut next_level: Vec<TNodeRef> = Vec::new();
        let mut height: f64 = 0.0;

        for node in current_level.iter() {
            if let Ok(node_guard) = node.lock() {
                next_level.extend(node_guard.children());
                let size = node_guard.size_ref();
                if direction.is_horizontal() {
                    height = height.max(size.x);
                } else {
                    height = height.max(size.y);
                }
            }
        }

        for node in current_level.iter() {
            if let Ok(mut node_guard) = node.lock() {
                node_guard.set_property(InternalProperties::LEVELHEIGHT, Some(height));
            }
        }

        sub_task.done();

        self.set_level_height(&mut next_level, progress_monitor, direction);
    }
}
