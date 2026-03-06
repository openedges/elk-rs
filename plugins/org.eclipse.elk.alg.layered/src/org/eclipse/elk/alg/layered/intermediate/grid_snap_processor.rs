use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, NodeType};
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;

#[inline]
fn snap_round(coord: f64, grid_size: f64) -> f64 {
    (coord / grid_size).round() * grid_size
}

#[inline]
fn snap_ceil(size: f64, grid_size: f64) -> f64 {
    (size / grid_size).ceil() * grid_size
}

// Node size ceil snap (BEFORE_P4).
// P4 uses snapped sizes for port distribution.
pub struct GridSnapSizeProcessor;

impl ILayoutProcessor<LGraph> for GridSnapSizeProcessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Grid snap node sizes", 1.0);

        let grid_size = layered_graph
            .get_property(LayeredOptions::GRID_SNAP_GRID_SIZE)
            .unwrap_or(0.0);

        if grid_size <= 0.0 {
            monitor.done();
            return;
        }

        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            for node in nodes {
                let Ok(mut node_guard) = node.lock() else {
                    continue;
                };
                if node_guard.node_type() != NodeType::Normal {
                    continue;
                }
                let size = node_guard.shape().size();
                size.x = snap_ceil(size.x, grid_size);
                size.y = snap_ceil(size.y, grid_size);
            }
        }

        monitor.done();
    }
}

// Node position round snap (AFTER_P4).
// P5 edge routing and HierarchicalNodeResizer use snapped coordinates.
pub struct GridSnapPositionProcessor;

impl ILayoutProcessor<LGraph> for GridSnapPositionProcessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Grid snap node positions", 1.0);

        let grid_size = layered_graph
            .get_property(LayeredOptions::GRID_SNAP_GRID_SIZE)
            .unwrap_or(0.0);

        if grid_size <= 0.0 {
            monitor.done();
            return;
        }

        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            for node in nodes {
                let Ok(mut node_guard) = node.lock() else {
                    continue;
                };
                if node_guard.node_type() != NodeType::Normal {
                    continue;
                }
                let pos = node_guard.shape().position();
                pos.x = snap_round(pos.x, grid_size);
                pos.y = snap_round(pos.y, grid_size);
            }
        }

        monitor.done();
    }
}

// Graph size/offset snap (AFTER_P5, before DirectionPostprocessor).
// Ensures Direction mirror transforms preserve grid alignment.
pub struct GridSnapGraphSizeProcessor;

impl ILayoutProcessor<LGraph> for GridSnapGraphSizeProcessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Grid snap graph size", 1.0);

        let grid_size = layered_graph
            .get_property(LayeredOptions::GRID_SNAP_GRID_SIZE)
            .unwrap_or(0.0);

        if grid_size <= 0.0 {
            monitor.done();
            return;
        }

        let size = layered_graph.size();
        size.x = snap_ceil(size.x, grid_size);
        size.y = snap_ceil(size.y, grid_size);

        let offset = layered_graph.offset();
        offset.x = snap_round(offset.x, grid_size);
        offset.y = snap_round(offset.y, grid_size);

        monitor.done();
    }
}
