use std::sync::Arc;
use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LGraphRef, LNodeRef, Layer};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

static BASELINE_PROCESSING_CONFIGURATION: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .add_before(
            LayeredPhases::P1CycleBreaking,
            Arc::new(IntermediateProcessorStrategy::EdgeAndLayerConstraintEdgeReverser),
        )
        .add_before(
            LayeredPhases::P2Layering,
            Arc::new(IntermediateProcessorStrategy::LayerConstraintPreprocessor),
        )
        .add_before(
            LayeredPhases::P3NodeOrdering,
            Arc::new(IntermediateProcessorStrategy::LayerConstraintPostprocessor),
        );
    config
});

pub struct LongestPathSourceLayerer {
    node_heights: Vec<i32>,
}

impl LongestPathSourceLayerer {
    pub fn new() -> Self {
        LongestPathSourceLayerer {
            node_heights: Vec::new(),
        }
    }

    fn visit(&mut self, node: &LNodeRef, graph: &mut LGraph, graph_ref: &LGraphRef) -> i32 {
        let index = node_index(node);
        if index < self.node_heights.len() && self.node_heights[index] >= 0 {
            return self.node_heights[index];
        }

        let ports = node.lock().ports().clone();

        let mut max_height = 1;
        for port in ports {
            let incoming = port.lock().incoming_edges().clone();
            for edge in incoming {
                let source_node = edge
                    .lock()
                    .source()
                    .and_then(|port| port.lock().node());
                let Some(source_node) = source_node else {
                    continue;
                };
                if Arc::ptr_eq(&source_node, node) {
                    continue;
                }
                let source_height = self.visit(&source_node, graph, graph_ref);
                max_height = max_height.max(source_height + 1);
            }
        }

        self.put_node(node, max_height, graph, graph_ref);
        max_height
    }

    fn put_node(
        &mut self,
        node: &LNodeRef,
        height: i32,
        graph: &mut LGraph,
        graph_ref: &LGraphRef,
    ) {
        let height = height.max(1) as usize;
        let current_layers = graph.layers().len();
        for _ in current_layers..height {
            graph.layers_mut().push(Layer::new(graph_ref));
        }

        let layer_index = height.saturating_sub(1);
        if let Some(layer) = graph.layers().get(layer_index).cloned() {
            crate::org::eclipse::elk::alg::layered::graph::LNode::set_layer(node, Some(layer));
        }

        let index = node_index(node);
        if index < self.node_heights.len() {
            self.node_heights[index] = height as i32;
        }
    }
}

impl Default for LongestPathSourceLayerer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for LongestPathSourceLayerer {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Longest path to source layering", 1.0);

        let nodes = graph.layerless_nodes().clone();
        if nodes.is_empty() {
            monitor.done();
            return;
        }

        let graph_ref = nodes
            .first()
            .and_then(|node| node.lock().graph())
            .unwrap_or_default();

        self.node_heights = vec![-1; nodes.len()];
        for (index, node) in nodes.iter().enumerate() {
            { let mut node_guard = node.lock();
                node_guard.shape().graph_element().id = index as i32;
            }
        }

        for node in &nodes {
            self.visit(node, graph, &graph_ref);
        }

        graph.layerless_nodes_mut().clear();
        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        Some(LayoutProcessorConfiguration::create_from(
            &BASELINE_PROCESSING_CONFIGURATION,
        ))
    }
}

fn node_index(node: &LNodeRef) -> usize {
    node.lock().shape().graph_element().id as usize
}
