use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::TGraphRef;
use crate::org::eclipse::elk::alg::mrtree::options::{InternalProperties, MrTreeOptions};

#[derive(Default)]
pub struct LevelCoordinatesProcessor;

impl ILayoutProcessor<TGraphRef> for LevelCoordinatesProcessor {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Processor determine the coords for each level", 1.0);

        let (nodes, direction) = {
            let mut graph_guard = match graph.lock_ok() {
            Some(guard) => guard,
            None => {
                    progress_monitor.done();
                    return;
                }
            };
            let nodes = graph_guard.nodes().clone();
            let direction = graph_guard
                .get_property(MrTreeOptions::DIRECTION)
                .unwrap_or(Direction::Down);
            (nodes, direction)
        };

        let mut levels: Vec<(f64, f64)> = Vec::new();

        // Pass 1: extract level + bounds, cache levels to avoid re-reading property
        let mut node_levels: Vec<usize> = Vec::with_capacity(nodes.len());
        for node in &nodes {
            if let Some(mut node_guard) = node.lock_ok() {
                let level = node_guard
                    .get_property(MrTreeOptions::TREE_LEVEL)
                    .unwrap_or(0) as usize;
                node_levels.push(level);
                while level >= levels.len() {
                    levels.push((f64::MAX, -f64::MAX));
                }
                let pos = node_guard.position_ref();
                let size = node_guard.size_ref();
                let (start, end) = if direction.is_horizontal() {
                    (pos.x, pos.x + size.x)
                } else {
                    (pos.y, pos.y + size.y)
                };
                let (min_val, max_val) = levels[level];
                levels[level] = (min_val.min(start), max_val.max(end));
            } else {
                node_levels.push(0);
            }
        }

        // Pass 2: write back using cached levels — skip property read
        for (i, node) in nodes.iter().enumerate() {
            let level = node_levels[i];
            if level < levels.len() {
                if let Some(mut node_guard) = node.lock_ok() {
                    let (min_val, max_val) = levels[level];
                    node_guard.set_property(InternalProperties::LEVELMIN, Some(min_val));
                    node_guard.set_property(InternalProperties::LEVELMAX, Some(max_val));
                }
            }
        }

        progress_monitor.done();
    }
}
