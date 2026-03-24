use std::sync::Arc;
use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphRef, LNode, LNodeRef, Layer, LayerRef, NodeType,
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

pub struct DepthFirstModelOrderLayerer {
    current_layer: Option<LayerRef>,
    current_layer_id: i32,
    current_dummy_layer: Option<LayerRef>,
    nodes_to_place: Vec<LNodeRef>,
    max_to_place: i32,
}

impl DepthFirstModelOrderLayerer {
    pub fn new() -> Self {
        DepthFirstModelOrderLayerer {
            current_layer: None,
            current_layer_id: 0,
            current_dummy_layer: None,
            nodes_to_place: Vec::new(),
            max_to_place: 0,
        }
    }

    fn is_connected_to_current_layer(&self, node: &LNodeRef) -> bool {
        let incoming_edges = node
            .lock().incoming_edges();

        for edge in incoming_edges {
            let source_node = edge
                .lock().source()
                .and_then(|port| port.lock().node());
            let Some(source_node) = source_node else {
                continue;
            };
            let source_type = source_node
                .lock().node_type();

            let mut directly_connected = false;
            let mut connected_via_label_dummy = false;
            if self.nodes_to_place.is_empty() {
                if source_type == NodeType::Normal {
                    if let Some(layer) = source_node
                        .lock().layer()
                    {
                        directly_connected = layer
                            .lock()
                            .graph_element().id == self.current_layer_id;
                    }
                }

                if source_type == NodeType::Label {
                    let label_incoming = source_node
                        .lock().incoming_edges();
                    if let Some(label_edge) = label_incoming.first() {
                        if let Some(layer) = label_edge
                            .lock().source()
                            .and_then(|port| {
                                port.lock().node()
                            })
                            .and_then(|node| {
                                node.lock().layer()
                            })
                        {
                            connected_via_label_dummy = layer
                                .lock()
                                .graph_element().id == self.current_layer_id;
                        }
                    }
                }
            } else {
                if source_type == NodeType::Normal {
                    directly_connected = node_id(&source_node) == self.current_layer_id;
                }

                if source_type == NodeType::Label {
                    let label_incoming = source_node
                        .lock().incoming_edges();
                    if let Some(label_edge) = label_incoming.first() {
                        if let Some(label_source) = label_edge
                            .lock().source()
                            .and_then(|port| {
                                port.lock().node()
                            })
                        {
                            connected_via_label_dummy =
                                node_id(&label_source) == self.current_layer_id;
                        }
                    }
                }
            }

            if directly_connected || connected_via_label_dummy {
                return true;
            }
        }
        false
    }

    fn add_node_to_layer(
        &mut self,
        layer_id: i32,
        node: &LNodeRef,
        graph: &mut LGraph,
        graph_ref: &LGraphRef,
    ) {
        let layer_index = layer_id as usize;
        if layer_index < graph.layers().len() {
            self.current_layer = graph.layers().get(layer_index).cloned();
            let dummy_index = layer_index.saturating_sub(1);
            self.current_dummy_layer = graph.layers().get(dummy_index).cloned();
            self.current_layer_id = layer_id;
        } else {
            let dummy_layer = Layer::new(graph_ref);
            {
                let mut dummy_guard = dummy_layer.lock();
                dummy_guard.graph_element().id = layer_id - 1;
            }
            graph.layers_mut().push(dummy_layer.clone());

            let new_layer = Layer::new(graph_ref);
            {
                let mut new_guard = new_layer.lock();
                new_guard.graph_element().id = layer_id;
            }
            graph.layers_mut().push(new_layer.clone());

            self.current_dummy_layer = Some(dummy_layer);
            self.current_layer = Some(new_layer);
            self.current_layer_id = layer_id;
        }

        if let Some(current_layer) = self.current_layer.clone() {
            LNode::set_layer(node, Some(current_layer));
        }

        let incoming_edges = node
            .lock().incoming_edges();
        for edge in incoming_edges {
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
                if let Some(dummy_layer) = self.current_dummy_layer.clone() {
                    LNode::set_layer(&source_node, Some(dummy_layer));
                }
            }
        }
    }

    fn get_max_connected_layer(&self, layer_id: i32, node: &LNodeRef) -> i32 {
        let mut max_layer = layer_id;
        let incoming_edges = node
            .lock().incoming_edges();
        for edge in incoming_edges {
            if let Some(source_layer) = edge
                .lock().source()
                .and_then(|port| port.lock().node())
                .and_then(|node| node.lock().layer())
            {
                {
                    let mut layer_guard = source_layer.lock();
                    max_layer = max_layer.max(layer_guard.graph_element().id);
                }
            }
        }
        max_layer
    }

    fn place_nodes_to_place(&mut self, graph: &mut LGraph, graph_ref: &LGraphRef) {
        self.max_to_place = 0;
        for node_to_place in &self.nodes_to_place {
            let desired_layer = node_id(node_to_place);
            if desired_layer as usize >= graph.layers().len() {
                let dummy_layer = Layer::new(graph_ref);
                {
                    let mut dummy_guard = dummy_layer.lock();
                    dummy_guard.graph_element().id = desired_layer - 1;
                }
                graph.layers_mut().push(dummy_layer);

                let new_layer = Layer::new(graph_ref);
                {
                    let mut new_guard = new_layer.lock();
                    new_guard.graph_element().id = desired_layer;
                }
                graph.layers_mut().push(new_layer);
            }
            if let Some(layer) = graph.layers().get(desired_layer as usize).cloned() {
                LNode::set_layer(node_to_place, Some(layer));
            }
        }
    }
}

impl Default for DepthFirstModelOrderLayerer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for DepthFirstModelOrderLayerer {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Depth first model order layering", 1.0);

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
                        "The DF model order layer assigner requires all real nodes to have a model order."
                    )
                });
                real_nodes.push((order, node.clone()));
            }
        }
        real_nodes.sort_by(|(a, _), (b, _)| a.cmp(b));

        let first_layer = Layer::new(&graph_ref);
        {
            let mut layer_guard = first_layer.lock();
            layer_guard.graph_element().id = 0;
        }
        graph.layers_mut().push(first_layer.clone());
        self.current_layer = Some(first_layer);
        self.current_layer_id = 0;
        self.current_dummy_layer = None;
        self.nodes_to_place.clear();
        self.max_to_place = 0;

        let mut first_node = true;
        for (_, node) in real_nodes {
            if first_node {
                if let Some(current_layer) = self.current_layer.clone() {
                    LNode::set_layer(&node, Some(current_layer));
                }
                first_node = false;
                continue;
            }

            if self.is_connected_to_current_layer(&node) {
                let mut max_layer = self.current_layer_id;
                max_layer = self.get_max_connected_layer(max_layer, &node);
                let desired_layer = max_layer + 2;
                let layer_diff = max_layer - self.current_layer_id;

                if !self.nodes_to_place.is_empty() {
                    if layer_diff > 0 {
                        for to_place in &self.nodes_to_place {
                            let new_id = node_id(to_place) + (max_layer - self.max_to_place);
                            set_node_id(to_place, new_id);
                        }
                        self.place_nodes_to_place(graph, &graph_ref);
                        self.nodes_to_place.clear();
                        self.add_node_to_layer(desired_layer, &node, graph, &graph_ref);
                    } else {
                        self.nodes_to_place.push(node.clone());
                        set_node_id(&node, desired_layer);
                        self.max_to_place = self.max_to_place.max(desired_layer);

                        let incoming_edges = node
                            .lock().incoming_edges();
                        for edge in incoming_edges {
                            let source_node = edge
                                .lock().source()
                                .and_then(|port| {
                                    port.lock().node()
                                });
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
                                self.nodes_to_place.push(source_node.clone());
                                set_node_id(&source_node, desired_layer - 1);
                            }
                        }

                        self.current_layer_id = desired_layer;
                    }
                } else {
                    self.add_node_to_layer(desired_layer, &node, graph, &graph_ref);
                }
            } else {
                self.place_nodes_to_place(graph, &graph_ref);
                self.nodes_to_place.clear();

                if node.lock().incoming_edges().is_empty()
                {
                    self.nodes_to_place.push(node.clone());
                    set_node_id(&node, 0);
                    self.max_to_place = self.max_to_place.max(0);
                    self.current_layer = graph.layers().first().cloned();
                    self.current_layer_id = 0;
                } else {
                    let mut max_layer = 0;
                    max_layer = self.get_max_connected_layer(max_layer, &node);
                    let desired_layer = max_layer + 2;
                    self.add_node_to_layer(desired_layer, &node, graph, &graph_ref);
                }
            }
        }

        if !self.nodes_to_place.is_empty() {
            self.place_nodes_to_place(graph, &graph_ref);
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

fn node_id(node: &LNodeRef) -> i32 {
    node.lock().shape().graph_element().id
}

fn set_node_id(node: &LNodeRef, value: i32) {
    {
        let mut node_guard = node.lock();
        node_guard.shape().graph_element().id = value;
    }
}
