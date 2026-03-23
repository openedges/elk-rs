use std::sync::Arc;
use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LGraph, LNode, LNodeRef, Layer, LayerRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;
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

pub struct BreadthFirstModelOrderLayerer;

impl BreadthFirstModelOrderLayerer {
    pub fn new() -> Self {
        BreadthFirstModelOrderLayerer
    }
}

impl Default for BreadthFirstModelOrderLayerer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for BreadthFirstModelOrderLayerer {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Breadth first model order layering", 1.0);

        let nodes = graph.layerless_nodes().clone();
        if nodes.is_empty() {
            monitor.done();
            return;
        }

        let graph_ref = nodes
            .first()
            .and_then(|node| node.lock().graph())
            .unwrap_or_default();

        let mut real_nodes: Vec<(i32, LNodeRef)> = Vec::new();
        for node in &nodes {
            let (node_type, model_order) = {
                let node_guard = node.lock();
                (
                    node_guard.node_type(),
                    node_guard.get_property(InternalProperties::MODEL_ORDER),
                )
            };
            if node_type == NodeType::Normal {
                let order = model_order.unwrap_or_else(|| {
                    panic!(
                        "The BF model order layer assigner requires all real nodes to have a model order."
                    )
                });
                real_nodes.push((order, node.clone()));
            }
        }
        real_nodes.sort_by(|(a, _), (b, _)| a.cmp(b));

        let mut current_layer = Layer::new(&graph_ref);
        let mut current_dummy_layer: Option<LayerRef> = None;
        graph.layers_mut().push(current_layer.clone());

        let mut first_node = true;
        for (_, node) in real_nodes {
            if first_node {
                LNode::set_layer(&node, Some(current_layer.clone()));
                first_node = false;
                continue;
            }

            let incoming_edges = node
                .lock().incoming_edges();

            for edge in &incoming_edges {
                let source_node = edge
                    .lock().source()
                    .and_then(|port| port.lock().node());
                let Some(source_node) = source_node else {
                    continue;
                };
                let source_type = source_node
                    .lock().node_type();

                let mut connected = false;
                if source_type == NodeType::Normal {
                    if let Some(source_layer) = source_node
                        .lock().layer()
                    {
                        if Arc::ptr_eq(&source_layer, &current_layer) {
                            connected = true;
                        }
                    }
                } else if source_type == NodeType::Label {
                    let label_incoming = source_node
                        .lock().incoming_edges();
                    if let Some(label_edge) = label_incoming.first() {
                        let label_source_layer = label_edge
                            .lock().source()
                            .and_then(|port| {
                                port.lock().node()
                            })
                            .and_then(|node| {
                                node.lock().layer()
                            });
                        if let Some(label_source_layer) = label_source_layer {
                            if Arc::ptr_eq(&label_source_layer, &current_layer) {
                                connected = true;
                            }
                        }
                    }
                }

                if connected {
                    let dummy_layer = Layer::new(&graph_ref);
                    graph.layers_mut().push(dummy_layer.clone());
                    current_dummy_layer = Some(dummy_layer);

                    let new_layer = Layer::new(&graph_ref);
                    graph.layers_mut().push(new_layer.clone());
                    current_layer = new_layer;
                }
            }

            for edge in &incoming_edges {
                let source_node = edge
                    .lock().source()
                    .and_then(|port| port.lock().node());
                let Some(source_node) = source_node else {
                    continue;
                };
                let source_type = source_node
                    .lock().node_type();
                if source_type == NodeType::Label
                    && source_node
                        .lock().layer()
                        .is_none()
                {
                    let dummy_layer = current_dummy_layer
                        .as_ref()
                        .expect("dummy layer missing for label dummy placement")
                        .clone();
                    LNode::set_layer(&source_node, Some(dummy_layer));
                }
            }

            LNode::set_layer(&node, Some(current_layer.clone()));
        }

        graph.layerless_nodes_mut().clear();
        graph.layers_mut().retain(|layer| {
            !layer.lock().nodes().is_empty()
        });
        for (index, layer) in graph.layers().iter().enumerate() {
            {
                let mut layer_guard = layer.lock();
                layer_guard.graph_element().id = index as i32;
            }
        }

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
