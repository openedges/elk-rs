use std::collections::VecDeque;

use rustc_hash::FxHashMap;
use std::sync::{Arc, LazyLock};

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::networksimplex::{
    NEdge, NEdgeRef, NGraph, NNode, NNodeRef, NetworkSimplex,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::math::elk_margin::ElkMargin;
use org_eclipse_elk_core::org::eclipse::elk::core::options::node_label_placement::NodeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, IElkProgressMonitor};

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdgeRef, LGraph, LLabelRef, LNodeRef, LPortRef, LayerRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions, NodeFlexibility, Spacings,
};
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

static HIERARCHY_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config.add_before(
        LayeredPhases::P5EdgeRouting,
        Arc::new(IntermediateProcessorStrategy::HierarchicalPortPositionProcessor),
    );
    config
});

const EDGE_WEIGHT_BASE: f64 = 4.0;
const SMALL_EDGE_WEIGHT: f64 = 0.1;
const LONG_EDGE_VS_PATH_FACTOR: f64 = 2.0;
const NODE_SIZE_WEIGHT_STATIC: f64 = 10000.0;
const NODE_SIZE_WEIGHT_FLEXIBLE: f64 = 1.0;
const EPSILON: f64 = 0.00001;

const VISITED: i32 = -1;
const OTHER: i32 = 0;
const JUNCTION: i32 = 2;

pub struct NetworkSimplexPlacer {
    spacings: Option<Spacings>,
    n_graph: NGraph,
    node_reps: Vec<Option<NodeRep>>,
    edge_reps: Vec<Option<EdgeRep>>,
    port_map: FxHashMap<usize, NNodeRef>,
    node_count: usize,
    edge_count: usize,
    node_state: Vec<i32>,
    two_paths: Vec<Path>,
    crossing: Vec<bool>,
    flexible_where_space_permits_edges: Vec<NEdgeRef>,
    // Raw graph pointer used to access graph properties without re-locking the graph mutex.
    graph_ptr: Option<usize>,
}

impl NetworkSimplexPlacer {
    pub fn new() -> Self {
        NetworkSimplexPlacer {
            spacings: None,
            n_graph: NGraph::new(),
            node_reps: Vec::new(),
            edge_reps: Vec::new(),
            port_map: FxHashMap::default(),
            node_count: 0,
            edge_count: 0,
            node_state: Vec::new(),
            two_paths: Vec::new(),
            crossing: Vec::new(),
            flexible_where_space_permits_edges: Vec::new(),
            graph_ptr: None,
        }
    }

    fn graph_ref(&self) -> &LGraph {
        let ptr = self.graph_ptr.expect("graph pointer missing") as *const LGraph;
        // SAFETY: `graph_ptr` is set from a valid reference to the LGraph at the start of
        // `process()`. The graph outlives this processor and is not moved during layout.
        unsafe { &*ptr }
    }
}

impl Default for NetworkSimplexPlacer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for NetworkSimplexPlacer {
    fn process(
        &mut self,
        layered_graph: &mut LGraph,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        progress_monitor.begin("Network simplex node placement", 1.0);

        self.graph_ptr = Some(layered_graph as *const LGraph as usize);
        self.spacings = layered_graph.get_property(InternalProperties::SPACINGS);
        if self.spacings.is_none() {
            panic!("Missing spacings configuration for network simplex node placement");
        }

        self.prepare(layered_graph);
        self.build_initial_auxiliary_graph(layered_graph);
        self.insert_north_south_auxiliary_edges(layered_graph);
        self.insert_in_layer_edge_auxiliary_edges(layered_graph);

        if layered_graph
            .get_property(LayeredOptions::NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES)
            .unwrap_or(false)
        {
            let mut pm = progress_monitor.sub_task(1.0);
            pm.begin("Straight Edges Pre-Processing", 1.0);
            self.prefer_straight_edges(layered_graph);
            pm.done();
        }

        self.n_graph.make_connected();

        let thoroughness = layered_graph
            .get_property(LayeredOptions::THOROUGHNESS)
            .unwrap_or(7);
        let iter_limit = thoroughness * (self.n_graph.nodes.len() as i32);

        let mut simplex = NetworkSimplex::for_graph(&mut self.n_graph);
        simplex.with_iteration_limit(iter_limit);
        simplex.with_balancing(false);
        simplex.execute_with_monitor(progress_monitor.sub_task(1.0).as_mut());

        if !self.flexible_where_space_permits_edges.is_empty() {
            let mut pm = progress_monitor.sub_task(1.0);
            pm.begin("Flexible Where Space Processing", 1.0);

            self.insert_flexible_where_space_auxiliary_edges();
            for edge in &self.flexible_where_space_permits_edges {
                if let Ok(mut edge_guard) = edge.lock() {
                    edge_guard.weight = NODE_SIZE_WEIGHT_FLEXIBLE;
                }
            }

            let mut simplex = NetworkSimplex::for_graph(&mut self.n_graph);
            simplex.with_iteration_limit(iter_limit);
            simplex.with_balancing(false);
            simplex.execute_with_monitor(pm.sub_task(1.0).as_mut());
            pm.done();
        }

        if layered_graph
            .get_property(LayeredOptions::NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES)
            .unwrap_or(false)
        {
            let mut pm = progress_monitor.sub_task(1.0);
            pm.begin("Straight Edges Post-Processing", 1.0);
            self.post_process_two_paths();
            pm.done();
        }

        self.apply_positions(layered_graph);
        self.cleanup();
        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        if graph
            .get_property_ref(InternalProperties::GRAPH_PROPERTIES)
            .is_some_and(|props| props.contains(&GraphProperties::ExternalPorts))
        {
            Some(LayoutProcessorConfiguration::create_from(
                &HIERARCHY_PROCESSING_ADDITIONS,
            ))
        } else {
            None
        }
    }
}

impl NetworkSimplexPlacer {
    fn prepare(&mut self, graph: &mut LGraph) {
        self.n_graph = NGraph::new();
        self.port_map.clear();
        self.flexible_where_space_permits_edges.clear();

        let layers = graph.layers().clone();
        let mut node_idx = 0usize;
        let mut edge_idx = 0usize;

        for layer in &layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                if let Ok(mut node_guard) = node.lock() {
                    node_guard.shape().graph_element().id = node_idx as i32;
                }
                node_idx += 1;

                let outgoing_edges = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.outgoing_edges())
                    .unwrap_or_default();
                for edge in outgoing_edges {
                    if let Ok(mut edge_guard) = edge.lock() {
                        edge_guard.graph_element().id = edge_idx as i32;
                    }
                    edge_idx += 1;
                }

                let anchor_must_be_integer = is_flexible_node(self.graph_ref(), &node);
                let ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.ports().clone())
                    .unwrap_or_default();
                for port in ports {
                    if let Ok(mut port_guard) = port.lock() {
                        if anchor_must_be_integer {
                            let y = port_guard.anchor_ref().y;
                            if y != y.floor() {
                                let offset = y - y.round();
                                port_guard.anchor().y -= offset;
                            }
                        }

                        let y = port_guard.shape().position_ref().y + port_guard.anchor_ref().y;
                        if y != y.floor() {
                            let offset = y - y.round();
                            port_guard.shape().position().y -= offset;
                        }
                    }
                }
            }
        }

        self.node_count = node_idx;
        self.edge_count = edge_idx;
        self.node_reps = vec![None; self.node_count];
        self.edge_reps = vec![None; self.edge_count];
        self.node_state.clear();
        self.two_paths.clear();
        self.crossing.clear();
    }

    fn cleanup(&mut self) {
        self.spacings = None;
        self.n_graph = NGraph::new();
        self.node_reps.clear();
        self.edge_reps.clear();
        self.port_map.clear();
        self.node_state.clear();
        self.two_paths.clear();
        self.crossing.clear();
        self.flexible_where_space_permits_edges.clear();
        self.node_count = 0;
        self.edge_count = 0;
        self.graph_ptr = None;
    }

    fn build_initial_auxiliary_graph(&mut self, graph: &LGraph) {
        let layers = graph.layers().clone();
        for layer in &layers {
            self.transform_layer(layer);
        }
        self.transform_edges(graph);
    }

    fn transform_layer(&mut self, layer: &LayerRef) {
        let mut last_rep: Option<NodeRep> = None;
        let nodes = layer
            .lock()
            .ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();

        for node in nodes {
            let rep = if is_flexible_node(self.graph_ref(), &node) {
                self.transform_fixed_order_node(&node)
            } else {
                self.transform_fixed_pos_node(&node)
            };

            let node_id = node_id(&node);
            if node_id < self.node_reps.len() {
                self.node_reps[node_id] = Some(rep.clone());
            }

            if let Some(last) = last_rep.as_ref() {
                let spacing = {
                    let last_bottom = node_margin_bottom(&last.origin);
                    let current_top = node_margin_top(&node);
                    let vertical = self
                        .spacings
                        .as_ref()
                        .unwrap()
                        .get_vertical_spacing(&last.origin, &node);
                    let mut value = last_bottom + vertical + current_top;
                    if !last.is_flexible {
                        value += node_size_y(&last.origin);
                    }
                    value
                };

                NEdge::of()
                    .delta(spacing.ceil() as i32)
                    .weight(0.0)
                    .source(last.tail.clone())
                    .target(rep.head.clone())
                    .create();
            }

            last_rep = Some(rep);
        }
    }

    fn transform_fixed_pos_node(&mut self, node: &LNodeRef) -> NodeRep {
        let origin: Arc<dyn std::any::Any + Send + Sync> = Arc::new(node.clone());
        let single = NNode::of()
            .origin(origin)
            .type_label("non-flexible")
            .create(&mut self.n_graph);

        let ports = node
            .lock()
            .ok()
            .map(|node_guard| node_guard.ports().clone())
            .unwrap_or_default();
        for port in ports {
            let side = port.lock().ok().map(|port_guard| port_guard.side());
            if matches!(side, Some(PortSide::East | PortSide::West)) {
                self.port_map.insert(port_key(&port), single.clone());
            }
        }

        NodeRep {
            origin: node.clone(),
            is_flexible: false,
            head: single.clone(),
            tail: single,
        }
    }

    fn transform_fixed_order_node(&mut self, node: &LNodeRef) -> NodeRep {
        let origin: Arc<dyn std::any::Any + Send + Sync> = Arc::new(node.clone());
        let top_left = NNode::of()
            .origin(origin.clone())
            .type_label("flexible-head")
            .create(&mut self.n_graph);
        let bottom_left = NNode::of()
            .origin(origin)
            .type_label("flexible-tail")
            .create(&mut self.n_graph);

        let corners = NodeRep {
            origin: node.clone(),
            is_flexible: true,
            head: top_left.clone(),
            tail: bottom_left.clone(),
        };

        let min_height = node_size_y(node);
        let nf = get_node_flexibility(self.graph_ref(), node);
        let mut size_weight = NODE_SIZE_WEIGHT_STATIC;
        if nf.is_flexible_size() {
            size_weight = NODE_SIZE_WEIGHT_FLEXIBLE;
        }

        let node_size_edge = NEdge::of()
            .weight(size_weight)
            .delta(min_height.ceil() as i32)
            .source(top_left)
            .target(bottom_left)
            .create();

        if nf == NodeFlexibility::NodeSizeWhereSpacePermits {
            self.flexible_where_space_permits_edges.push(node_size_edge);
        }

        let west_ports = node
            .lock()
            .ok()
            .map(|mut node_guard| node_guard.port_side_view(PortSide::West))
            .unwrap_or_default();
        let mut west_ports_rev = west_ports;
        west_ports_rev.reverse();
        self.transform_ports(&west_ports_rev, &corners);

        let east_ports = node
            .lock()
            .ok()
            .map(|mut node_guard| node_guard.port_side_view(PortSide::East))
            .unwrap_or_default();
        self.transform_ports(&east_ports, &corners);

        corners
    }

    fn transform_ports(&mut self, ports: &[LPortRef], corners: &NodeRep) {
        if ports.is_empty() {
            return;
        }

        let graph = self.graph_ref();
        let port_spacing: f64 = Spacings::get_individual_or_default_with_graph(
            graph,
            &corners.origin,
            LayeredOptions::SPACING_PORT_PORT,
        );
        let port_surrounding: ElkMargin = Spacings::get_individual_or_default_with_graph(
            graph,
            &corners.origin,
            LayeredOptions::SPACING_PORTS_SURROUNDING,
        );

        let mut last_nnode = corners.head.clone();
        let mut last_port: Option<LPortRef> = None;

        for port in ports {
            let spacing = if let Some(ref last) = last_port {
                let last_size = last
                    .lock()
                    .ok()
                    .map(|mut port_guard| port_guard.shape().size_ref().y)
                    .unwrap_or(0.0);
                port_spacing + last_size
            } else {
                port_surrounding.top
            };

            let origin: Arc<dyn std::any::Any + Send + Sync> = Arc::new(port.clone());
            let nnode = NNode::of()
                .origin(origin)
                .type_label("port")
                .create(&mut self.n_graph);
            self.port_map.insert(port_key(port), nnode.clone());

            NEdge::of()
                .weight(0.0)
                .delta(spacing.ceil() as i32)
                .source(last_nnode.clone())
                .target(nnode.clone())
                .create();

            last_port = Some(port.clone());
            last_nnode = nnode;
        }

        if let Some(last_port) = last_port {
            let last_size = last_port
                .lock()
                .ok()
                .map(|mut port_guard| port_guard.shape().size_ref().y)
                .unwrap_or(0.0);
            NEdge::of()
                .weight(0.0)
                .delta((port_surrounding.bottom + last_size).ceil() as i32)
                .source(last_nnode)
                .target(corners.tail.clone())
                .create();
        }
    }

    fn transform_edges(&mut self, graph: &LGraph) {
        let layers = graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let outgoing_edges = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.outgoing_edges())
                    .unwrap_or_default();
                for edge in outgoing_edges {
                    if !is_handled_edge(&edge) {
                        continue;
                    }
                    self.transform_edge(&edge);
                }
            }
        }
    }

    fn transform_edge(&mut self, edge: &LEdgeRef) {
        let dummy = NNode::of().type_label("edge").create(&mut self.n_graph);

        let source_port = edge.lock().ok().and_then(|edge_guard| edge_guard.source());
        let target_port = edge.lock().ok().and_then(|edge_guard| edge_guard.target());
        let (Some(source_port), Some(target_port)) = (source_port, target_port) else {
            return;
        };

        let source_node = source_port
            .lock()
            .ok()
            .and_then(|port_guard| port_guard.node());
        let target_node = target_port
            .lock()
            .ok()
            .and_then(|port_guard| port_guard.node());
        let (Some(source_node), Some(target_node)) = (source_node, target_node) else {
            return;
        };

        let src_rep = self.node_reps[node_id(&source_node)]
            .as_ref()
            .cloned()
            .expect("Missing node rep");
        let tgt_rep = self.node_reps[node_id(&target_node)]
            .as_ref()
            .cloned()
            .expect("Missing node rep");

        let mut src_offset = source_port
            .lock()
            .ok()
            .map(|port_guard| port_guard.anchor_ref().y)
            .unwrap_or(0.0);
        let mut tgt_offset = target_port
            .lock()
            .ok()
            .map(|port_guard| port_guard.anchor_ref().y)
            .unwrap_or(0.0);

        if !src_rep.is_flexible {
            src_offset += source_port
                .lock()
                .ok()
                .map(|mut port_guard| port_guard.shape().position_ref().y)
                .unwrap_or(0.0);
        }
        if !tgt_rep.is_flexible {
            tgt_offset += target_port
                .lock()
                .ok()
                .map(|mut port_guard| port_guard.shape().position_ref().y)
                .unwrap_or(0.0);
        }

        debug_assert!(
            ((src_offset - tgt_offset) - (src_offset - tgt_offset).round()).abs() < EPSILON,
            "Port positions must be integral"
        );

        let tgt_delta = (src_offset - tgt_offset).max(0.0).round() as i32;
        let src_delta = (tgt_offset - src_offset).max(0.0).round() as i32;

        let weight = self.get_edge_weight(edge);
        let origin: Arc<dyn std::any::Any + Send + Sync> = Arc::new(edge.clone());

        let left = NEdge::of_origin(origin.clone())
            .weight(weight)
            .delta(src_delta)
            .source(dummy.clone())
            .target(self.port_map[&port_key(&source_port)].clone())
            .create();

        let right = NEdge::of_origin(origin)
            .weight(weight)
            .delta(tgt_delta)
            .source(dummy)
            .target(self.port_map[&port_key(&target_port)].clone())
            .create();

        let edge_id = edge_id(edge);
        if edge_id < self.edge_reps.len() {
            self.edge_reps[edge_id] = Some(EdgeRep { left, right });
        }
    }

    fn insert_in_layer_edge_auxiliary_edges(&mut self, graph: &LGraph) {
        let layers = graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                if node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.node_type() != NodeType::Normal)
                    .unwrap_or(false)
                {
                    continue;
                }

                let edges = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.connected_edges())
                    .unwrap_or_default();
                for edge in edges {
                    let in_layer = {
                        let (sp, tp) = edge
                            .lock()
                            .ok()
                            .map(|eg| (eg.source(), eg.target()))
                            .unwrap_or((None, None));
                        check_in_layer_edge_ports(&sp, &tp)
                    };
                    if !in_layer {
                        continue;
                    }

                    let src_is_dummy = edge
                        .lock()
                        .ok()
                        .and_then(|edge_guard| edge_guard.source())
                        .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
                        .and_then(|node| {
                            node.lock()
                                .ok()
                                .map(|node_guard| node_guard.node_type() != NodeType::Normal)
                        })
                        .unwrap_or(false);

                    let source_port = edge.lock().ok().and_then(|edge_guard| edge_guard.source());
                    let target_port = edge.lock().ok().and_then(|edge_guard| edge_guard.target());
                    let (Some(source_port), Some(target_port)) = (source_port, target_port) else {
                        continue;
                    };

                    let port = if src_is_dummy {
                        target_port
                    } else {
                        source_port
                    };
                    let dummy_port = edge
                        .lock()
                        .ok()
                        .map(|edge_guard| edge_guard.other_port(&port));
                    let Some(dummy_port) = dummy_port else {
                        continue;
                    };
                    let dummy_node = dummy_port
                        .lock()
                        .ok()
                        .and_then(|port_guard| port_guard.node());
                    let Some(dummy_node) = dummy_node else {
                        continue;
                    };

                    let port_rep = self.port_map.get(&port_key(&port)).cloned();
                    let dummy_rep = self.node_reps[node_id(&dummy_node)]
                        .as_ref()
                        .map(|rep| rep.head.clone());
                    let (Some(port_rep), Some(dummy_rep)) = (port_rep, dummy_rep) else {
                        continue;
                    };

                    let port_index = port
                        .lock()
                        .ok()
                        .and_then(|port_guard| port_guard.node())
                        .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.index()))
                        .unwrap_or(0);
                    let dummy_index = dummy_node
                        .lock()
                        .ok()
                        .and_then(|node_guard| node_guard.index())
                        .unwrap_or(0);

                    let (src, tgt) = if port_index < dummy_index {
                        (port_rep, dummy_rep)
                    } else {
                        (dummy_rep, port_rep)
                    };

                    NEdge::of()
                        .delta(0)
                        .weight(EDGE_WEIGHT_BASE)
                        .source(src)
                        .target(tgt)
                        .create();
                }
            }
        }
    }

    fn insert_north_south_auxiliary_edges(&mut self, graph: &LGraph) {
        let layers = graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let node_id_val = node_id(&node);
                let mut node_guard = match node.lock() {
                    Ok(guard) => guard,
                    Err(_) => continue,
                };

                let south_ports = node_guard.port_side_view(PortSide::South);
                for port in south_ports {
                    let dummy = port.lock().ok().and_then(|mut port_guard| {
                        port_guard.get_property(InternalProperties::PORT_DUMMY)
                    });
                    if let Some(dummy) = dummy {
                        let dummy_id = node_id(&dummy);
                        NEdge::of()
                            .delta(0)
                            .weight(SMALL_EDGE_WEIGHT)
                            .source(self.node_reps[node_id_val].as_ref().unwrap().tail.clone())
                            .target(self.node_reps[dummy_id].as_ref().unwrap().head.clone())
                            .create();
                    }
                }

                let north_ports = node_guard.port_side_view(PortSide::North);
                for port in north_ports {
                    let dummy = port.lock().ok().and_then(|mut port_guard| {
                        port_guard.get_property(InternalProperties::PORT_DUMMY)
                    });
                    if let Some(dummy) = dummy {
                        let dummy_id = node_id(&dummy);
                        NEdge::of()
                            .delta(0)
                            .weight(SMALL_EDGE_WEIGHT)
                            .source(self.node_reps[dummy_id].as_ref().unwrap().tail.clone())
                            .target(self.node_reps[node_id_val].as_ref().unwrap().head.clone())
                            .create();
                    }
                }
            }
        }
    }

    fn insert_flexible_where_space_auxiliary_edges(&mut self) {
        let min_layer = self
            .n_graph
            .nodes
            .iter()
            .filter_map(|node| node.lock().ok().map(|node_guard| node_guard.layer))
            .min()
            .unwrap_or(0);
        let max_layer = self
            .n_graph
            .nodes
            .iter()
            .filter_map(|node| node.lock().ok().map(|node_guard| node_guard.layer))
            .max()
            .unwrap_or(0);
        let used_layers = max_layer - min_layer;

        let global_source = NNode::of().create(&mut self.n_graph);
        let global_sink = NNode::of().create(&mut self.n_graph);

        NEdge::of()
            .weight(NODE_SIZE_WEIGHT_STATIC * 2.0)
            .delta(used_layers)
            .source(global_source.clone())
            .target(global_sink.clone())
            .create();

        for rep in &self.node_reps {
            let Some(rep) = rep.as_ref() else {
                continue;
            };
            if rep
                .origin
                .lock()
                .ok()
                .map(|node_guard| node_guard.node_type() != NodeType::Normal)
                .unwrap_or(true)
            {
                continue;
            }
            let port_count = rep
                .origin
                .lock()
                .ok()
                .map(|node_guard| node_guard.ports().len())
                .unwrap_or(0);
            if port_count <= 1 {
                continue;
            }

            let tail_layer = rep
                .tail
                .lock()
                .ok()
                .map(|node_guard| node_guard.layer)
                .unwrap_or(0);
            let head_layer = rep
                .head
                .lock()
                .ok()
                .map(|node_guard| node_guard.layer)
                .unwrap_or(0);

            NEdge::of()
                .weight(0.0)
                .delta(tail_layer - min_layer)
                .source(global_source.clone())
                .target(rep.tail.clone())
                .create();
            NEdge::of()
                .weight(0.0)
                .delta(used_layers - head_layer)
                .source(rep.head.clone())
                .target(global_sink.clone())
                .create();
        }
    }

    fn apply_positions(&mut self, graph: &LGraph) {
        let layers = graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let node_id_val = node_id(&node);
                let rep = self.node_reps[node_id_val].as_ref().cloned().unwrap();
                let min_y = rep
                    .head
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.layer)
                    .unwrap_or(0) as f64;
                let max_y = rep
                    .tail
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.layer)
                    .unwrap_or(0) as f64;
                let nf = get_node_flexibility(self.graph_ref(), &node);
                let flexible_node = is_flexible_node(self.graph_ref(), &node);

                if let Ok(mut node_guard) = node.lock() {
                    node_guard.shape().position().y = min_y;

                    let size_delta = (max_y - min_y) - node_guard.shape().size_ref().y;

                    if flexible_node && nf.is_flexible_size_where_space_permits() {
                        node_guard.shape().size().y += size_delta;
                    }

                    if flexible_node && nf.is_flexible_ports() {
                        let ports = node_guard.ports().clone();
                        for port in ports {
                            let side = port.lock().ok().map(|port_guard| port_guard.side());
                            if matches!(side, Some(PortSide::East | PortSide::West)) {
                                if let Some(n_node) = self.port_map.get(&port_key(&port)) {
                                    let layer_val = n_node
                                        .lock()
                                        .ok()
                                        .map(|node_guard| node_guard.layer)
                                        .unwrap_or(0)
                                        as f64;
                                    if let Ok(mut port_guard) = port.lock() {
                                        port_guard.shape().position().y = layer_val - min_y;
                                    }
                                }
                            }
                        }

                        let placement = node_guard
                            .get_property(LayeredOptions::NODE_LABELS_PLACEMENT)
                            .unwrap_or_else(NodeLabelPlacement::fixed);
                        let labels = node_guard.labels().clone();
                        for label in labels {
                            adjust_label_position(&placement, &label, size_delta);
                        }

                        if nf.is_flexible_size_where_space_permits() {
                            let south_ports = node_guard.port_side_view(PortSide::South);
                            drop(node_guard);
                            for port in south_ports {
                                if let Ok(mut port_guard) = port.lock() {
                                    port_guard.shape().position().y += size_delta;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn get_edge_weight(&self, edge: &LEdgeRef) -> f64 {
        let priority = edge
            .lock()
            .ok()
            .and_then(|mut edge_guard| {
                edge_guard.get_property(LayeredOptions::PRIORITY_STRAIGHTNESS)
            })
            .unwrap_or(1)
            .max(1);
        let (source_type, target_type) = edge
            .lock()
            .ok()
            .and_then(|edge_guard| {
                let source = edge_guard.source()?;
                let target = edge_guard.target()?;
                let source_node = source
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.node())?;
                let target_node = target
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.node())?;
                let source_type = source_node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.node_type())
                    .unwrap_or(NodeType::Normal);
                let target_type = target_node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.node_type())
                    .unwrap_or(NodeType::Normal);
                Some((source_type, target_type))
            })
            .unwrap_or((NodeType::Normal, NodeType::Normal));

        priority as f64 * edge_type_weight(source_type, target_type)
    }

    fn prefer_straight_edges(&mut self, graph: &LGraph) {
        self.node_state = vec![0; self.node_count];
        self.two_paths.clear();

        let layers = graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let id = node_id(&node);
                if id < self.node_state.len() {
                    self.node_state[id] = get_node_state(&node);
                }
            }
        }

        self.mark_edge_crossings(graph);
        let paths = self.identify_paths(graph);

        for path in paths {
            if path.edges.len() <= 1 {
                continue;
            }
            if path.edges.len() == 2 {
                let mut path = path;
                path.order_two_path();
                if !path.is_two_path_center_node_flexible(self.graph_ref()) {
                    self.two_paths.push(path);
                }
                continue;
            }

            if path.contains_long_edge_dummy()
                || path.contains_flexible_node(self.graph_ref(), |nf| {
                    nf.is_flexible_size_where_space_permits()
                })
            {
                continue;
            }

            for (index, edge) in path.edges.iter().enumerate() {
                let weight = if index == 0 || index + 1 == path.edges.len() {
                    edge_type_weight(NodeType::Normal, NodeType::LongEdge)
                } else {
                    edge_type_weight(NodeType::LongEdge, NodeType::LongEdge)
                } * LONG_EDGE_VS_PATH_FACTOR;

                if let Some(rep) = self
                    .edge_reps
                    .get_mut(edge_id(edge))
                    .and_then(|rep| rep.as_mut())
                {
                    let mut left_weight = rep
                        .left
                        .lock()
                        .ok()
                        .map(|edge_guard| edge_guard.weight)
                        .unwrap_or(0.0);
                    let mut right_weight = rep
                        .right
                        .lock()
                        .ok()
                        .map(|edge_guard| edge_guard.weight)
                        .unwrap_or(0.0);
                    left_weight = left_weight.max(weight);
                    right_weight = right_weight.max(weight);
                    if let Ok(mut left_guard) = rep.left.lock() {
                        left_guard.weight = left_weight;
                    }
                    if let Ok(mut right_guard) = rep.right.lock() {
                        right_guard.weight = right_weight;
                    }
                }
            }
        }
    }

    fn post_process_two_paths(&mut self) {
        let mut queue: VecDeque<Path> = self.two_paths.drain(..).collect();
        let mut stack: Vec<Path> = Vec::new();

        while let Some(path) = queue.pop_front() {
            if self.improve_two_path(&path, true) {
                stack.push(path);
            }
        }

        while let Some(path) = stack.pop() {
            let _ = self.improve_two_path(&path, false);
        }
    }

    fn improve_two_path(&mut self, path: &Path, probe: bool) -> bool {
        let left_edge = self.edge_reps[edge_id(&path.edges[0])]
            .as_ref()
            .cloned()
            .unwrap();
        let right_edge = self.edge_reps[edge_id(&path.edges[1])]
            .as_ref()
            .cloned()
            .unwrap();

        if left_edge.is_straight() && right_edge.is_straight() {
            return false;
        }

        let center_origin = left_edge.right.lock().ok().and_then(|edge_guard| {
            edge_guard
                .target
                .lock()
                .ok()
                .and_then(|node_guard| node_guard.origin.clone())
        });
        let Some(center_origin) = center_origin else {
            return false;
        };
        let Some(center_node) = center_origin.as_ref().downcast_ref::<LNodeRef>() else {
            return false;
        };
        let center_id = node_id(center_node);
        let n_node = self.node_reps[center_id].as_ref().cloned().unwrap();

        let mut above_dist = f64::INFINITY;
        let mut below_dist = f64::INFINITY;

        let node_index = center_node
            .lock()
            .ok()
            .and_then(|node_guard| node_guard.index())
            .unwrap_or(0);
        let layer_nodes = center_node
            .lock()
            .ok()
            .and_then(|node_guard| node_guard.layer())
            .and_then(|layer| {
                layer
                    .lock()
                    .ok()
                    .map(|layer_guard| layer_guard.nodes().clone())
            })
            .unwrap_or_default();

        if node_index > 0 {
            let above = layer_nodes[node_index - 1].clone();
            let above_rep = self.node_reps[node_id(&above)].as_ref().cloned().unwrap();
            let spacing = self
                .spacings
                .as_ref()
                .unwrap()
                .get_vertical_spacing(&above, center_node)
                .ceil();
            above_dist = (n_node
                .head
                .lock()
                .ok()
                .map(|node_guard| node_guard.layer)
                .unwrap_or(0) as f64
                - node_margin_top(center_node))
                - (above_rep
                    .head
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.layer)
                    .unwrap_or(0) as f64
                    + node_size_y(&above)
                    + node_margin_bottom(&above))
                - spacing;
        }
        if node_index + 1 < layer_nodes.len() {
            let below = layer_nodes[node_index + 1].clone();
            let below_rep = self.node_reps[node_id(&below)].as_ref().cloned().unwrap();
            let spacing = self
                .spacings
                .as_ref()
                .unwrap()
                .get_vertical_spacing(&below, center_node)
                .ceil();
            below_dist = (below_rep
                .head
                .lock()
                .ok()
                .map(|node_guard| node_guard.layer)
                .unwrap_or(0) as f64
                - node_margin_top(&below))
                - (n_node
                    .head
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.layer)
                    .unwrap_or(0) as f64
                    + node_size_y(center_node)
                    + node_margin_bottom(center_node))
                - spacing;
        }

        if probe && (above_dist - below_dist).abs() < EPSILON {
            return true;
        }

        let a = length(&left_edge.left) as i32;
        let b = -length(&left_edge.right) as i32;
        let c = -length(&right_edge.left) as i32;
        let d = length(&right_edge.right) as i32;

        let case_d = left_edge.not_straight_by() > 0 && right_edge.not_straight_by() < 0;
        let case_c = left_edge.not_straight_by() < 0 && right_edge.not_straight_by() > 0;
        let left_value = left_edge.left_target_layer() + left_edge.right_delta();
        let right_value = right_edge.right_target_layer() + right_edge.left_delta();
        let case_b = left_value < right_value;
        let case_a = left_value > right_value;

        let mut move_by = 0;
        if !case_d && !case_c {
            if case_a {
                if above_dist + (c as f64) > 0.0 {
                    move_by = c;
                } else if below_dist - (a as f64) > 0.0 {
                    move_by = a;
                }
            } else if case_b {
                if above_dist + (b as f64) > 0.0 {
                    move_by = b;
                } else if below_dist - (d as f64) > 0.0 {
                    move_by = d;
                }
            }
        }

        if let Ok(mut head_guard) = n_node.head.lock() {
            head_guard.layer += move_by;
        }
        if n_node.is_flexible {
            if let Ok(mut tail_guard) = n_node.tail.lock() {
                tail_guard.layer += move_by;
            }
        }

        false
    }

    fn identify_paths(&mut self, graph: &LGraph) -> Vec<Path> {
        let mut paths = Vec::new();
        let layers = graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                if self.node_state[node_id(&node)] != JUNCTION {
                    continue;
                }
                let edges = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.connected_edges())
                    .unwrap_or_default();
                for edge in edges {
                    if !is_handled_edge(&edge) {
                        continue;
                    }
                    let path = self.follow(&edge, &node, Path::new());
                    if path.edges.len() > 1 {
                        paths.push(path);
                    }
                }
            }
        }
        paths
    }

    fn follow(&mut self, edge: &LEdgeRef, current: &LNodeRef, mut path: Path) -> Path {
        // Extract ports first, drop edge lock, then resolve other_node
        // (other_node locks ports which could deadlock if edge lock is held)
        let other = {
            let (sp, tp) = edge
                .lock()
                .ok()
                .map(|eg| (eg.source(), eg.target()))
                .unwrap_or((None, None));
            let source_node = sp.and_then(|p| p.lock().ok().and_then(|p| p.node()));
            let target_node = tp.and_then(|p| p.lock().ok().and_then(|p| p.node()));
            if source_node
                .as_ref()
                .is_some_and(|n| Arc::ptr_eq(n, current))
            {
                target_node
            } else {
                source_node
            }
        };
        path.edges.push(edge.clone());

        let Some(other) = other else {
            return path;
        };
        let other_id = node_id(&other);

        if self.node_state.get(other_id).copied().unwrap_or(OTHER) == VISITED
            || self.node_state.get(other_id).copied().unwrap_or(OTHER) == JUNCTION
            || self.crossing.get(edge_id(edge)).copied().unwrap_or(false)
        {
            return path;
        }

        if other_id < self.node_state.len() {
            self.node_state[other_id] = VISITED;
        }

        let incident_edges = other
            .lock()
            .ok()
            .map(|node_guard| node_guard.connected_edges())
            .unwrap_or_default();
        for incident in incident_edges {
            if !is_handled_edge(&incident) {
                continue;
            }
            if Arc::ptr_eq(&incident, edge) {
                continue;
            }
            return self.follow(&incident, &other, path);
        }

        path
    }

    fn mark_edge_crossings(&mut self, graph: &LGraph) {
        self.crossing = vec![false; self.edge_count];
        let layers = graph.layers().clone();
        for i in 0..layers.len().saturating_sub(1) {
            let left = layers[i].clone();
            let right = layers[i + 1].clone();
            self.mark_crossing_edges(&left, &right);
        }
    }

    fn mark_crossing_edges(&mut self, left: &LayerRef, right: &LayerRef) {
        let mut open_edges: Vec<LEdgeRef> = Vec::new();

        let left_nodes = left
            .lock()
            .ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();
        for node in &left_nodes {
            let east_ports = node
                .lock()
                .ok()
                .map(|mut node_guard| node_guard.port_side_view(PortSide::East))
                .unwrap_or_default();
            for port in east_ports {
                let outgoing = port
                    .lock()
                    .ok()
                    .map(|port_guard| port_guard.outgoing_edges().clone())
                    .unwrap_or_default();
                for edge in outgoing {
                    // Extract ports WITHOUT holding edge lock during property checks
                    let (source_port, target_port) = {
                        let edge_guard = match edge.lock() {
                            Ok(g) => g,
                            Err(_) => {
                                continue;
                            }
                        };
                        (edge_guard.source(), edge_guard.target())
                        // edge_guard (MutexGuard) dropped here
                    };

                    // Check skip conditions with NO edge lock held
                    let is_self = check_self_loop_ports(&source_port, &target_port);
                    let is_in_layer = if is_self {
                        false
                    } else {
                        check_in_layer_edge_ports(&source_port, &target_port)
                    };
                    let target_not_in_right = target_port
                        .as_ref()
                        .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
                        .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.layer()))
                        .map(|layer| !Arc::ptr_eq(&layer, right))
                        .unwrap_or(true);

                    if is_self || is_in_layer || target_not_in_right {
                        continue;
                    }
                    open_edges.push(edge);
                }
            }
        }

        let right_nodes = right
            .lock()
            .ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();
        for node in right_nodes.into_iter().rev() {
            let west_ports = node
                .lock()
                .ok()
                .map(|mut node_guard| node_guard.port_side_view(PortSide::West))
                .unwrap_or_default();
            for port in west_ports {
                let incoming = port
                    .lock()
                    .ok()
                    .map(|port_guard| port_guard.incoming_edges().clone())
                    .unwrap_or_default();
                for edge in incoming {
                    // Extract ports WITHOUT holding edge lock during property checks
                    let (source_port, target_port) = {
                        let edge_guard = match edge.lock() {
                            Ok(g) => g,
                            Err(_) => {
                                continue;
                            }
                        };
                        (edge_guard.source(), edge_guard.target())
                        // edge_guard (MutexGuard) dropped here
                    };

                    // Check skip conditions with NO edge lock held
                    let is_self = check_self_loop_ports(&source_port, &target_port);
                    let is_in_layer = if is_self {
                        false
                    } else {
                        check_in_layer_edge_ports(&source_port, &target_port)
                    };
                    let source_not_in_left = source_port
                        .as_ref()
                        .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
                        .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.layer()))
                        .map(|layer| !Arc::ptr_eq(&layer, left))
                        .unwrap_or(true);

                    if is_self || is_in_layer || source_not_in_left {
                        continue;
                    }

                    if !open_edges.is_empty() {
                        let mut idx = open_edges.len();
                        while idx > 0 {
                            idx -= 1;
                            let last = open_edges[idx].clone();
                            if Arc::ptr_eq(&last, &edge) {
                                if idx < open_edges.len() {
                                    open_edges.remove(idx);
                                }
                                break;
                            } else {
                                let last_id = edge_id(&last);
                                let edge_id_val = edge_id(&edge);
                                if last_id < self.crossing.len() {
                                    self.crossing[last_id] = true;
                                }
                                if edge_id_val < self.crossing.len() {
                                    self.crossing[edge_id_val] = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
struct NodeRep {
    origin: LNodeRef,
    head: NNodeRef,
    tail: NNodeRef,
    is_flexible: bool,
}

#[derive(Clone)]
struct EdgeRep {
    left: NEdgeRef,
    right: NEdgeRef,
}

impl EdgeRep {
    fn is_straight(&self) -> bool {
        self.not_straight_by() == 0
    }

    fn not_straight_by(&self) -> i32 {
        (self.left_target_layer() - self.left_delta())
            - (self.right_target_layer() - self.right_delta())
    }

    fn left_target_layer(&self) -> i32 {
        self.left
            .lock()
            .ok()
            .map(|edge_guard| {
                edge_guard
                    .target
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.layer)
                    .unwrap_or(0)
            })
            .unwrap_or(0)
    }

    fn right_target_layer(&self) -> i32 {
        self.right
            .lock()
            .ok()
            .map(|edge_guard| {
                edge_guard
                    .target
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.layer)
                    .unwrap_or(0)
            })
            .unwrap_or(0)
    }

    fn left_delta(&self) -> i32 {
        self.left
            .lock()
            .ok()
            .map(|edge_guard| edge_guard.delta)
            .unwrap_or(0)
    }

    fn right_delta(&self) -> i32 {
        self.right
            .lock()
            .ok()
            .map(|edge_guard| edge_guard.delta)
            .unwrap_or(0)
    }
}

#[derive(Clone)]
struct Path {
    edges: Vec<LEdgeRef>,
}

impl Path {
    fn new() -> Self {
        Path { edges: Vec::new() }
    }

    fn contains_long_edge_dummy(&self) -> bool {
        if self.edges.is_empty() {
            return false;
        }
        let first = &self.edges[0];
        let first_source = first
            .lock()
            .ok()
            .and_then(|edge_guard| edge_guard.source())
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
        if let Some(node) = first_source {
            if node
                .lock()
                .ok()
                .map(|node_guard| node_guard.node_type() == NodeType::LongEdge)
                .unwrap_or(false)
            {
                return true;
            }
        }
        for edge in &self.edges {
            let target_node = edge
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.target())
                .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
            if let Some(node) = target_node {
                if node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.node_type() == NodeType::LongEdge)
                    .unwrap_or(false)
                {
                    return true;
                }
            }
        }
        false
    }

    fn contains_flexible_node<F>(&self, graph: &LGraph, predicate: F) -> bool
    where
        F: Fn(NodeFlexibility) -> bool,
    {
        if self.edges.is_empty() {
            return false;
        }
        let first_source = self.edges[0]
            .lock()
            .ok()
            .and_then(|edge_guard| edge_guard.source())
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
        if let Some(node) = first_source {
            if predicate(get_node_flexibility(graph, &node)) {
                return true;
            }
        }
        for edge in &self.edges {
            let target_node = edge
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.target())
                .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
            if let Some(node) = target_node {
                if predicate(get_node_flexibility(graph, &node)) {
                    return true;
                }
            }
        }
        false
    }

    fn order_two_path(&mut self) {
        if self.edges.len() != 2 {
            panic!("Order only allowed for two paths.");
        }
        let first = self.edges[0].clone();
        let second = self.edges[1].clone();
        let first_target = first
            .lock()
            .ok()
            .and_then(|edge_guard| edge_guard.target())
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
        let second_source = second
            .lock()
            .ok()
            .and_then(|edge_guard| edge_guard.source())
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
        if first_target
            .zip(second_source)
            .map(|(a, b)| !Arc::ptr_eq(&a, &b))
            .unwrap_or(true)
        {
            self.edges.clear();
            self.edges.push(second);
            self.edges.push(first);
        }
    }

    fn is_two_path_center_node_flexible(&self, graph: &LGraph) -> bool {
        if self.edges.len() != 2 {
            return false;
        }
        let target = self.edges[0]
            .lock()
            .ok()
            .and_then(|edge_guard| edge_guard.target())
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
        target
            .map(|node| is_flexible_node(graph, &node))
            .unwrap_or(false)
    }
}

fn get_node_flexibility(graph: &LGraph, node: &LNodeRef) -> NodeFlexibility {
    if let Ok(mut node_guard) = node.lock() {
        if let Some(value) =
            node_guard.get_property(LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY)
        {
            return value;
        }
    }
    if let Some(value) = graph
        .get_property_ref(LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT)
    {
        return value;
    }
    LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT
        .get_default()
        .unwrap_or(NodeFlexibility::None)
}

fn is_flexible_node(graph: &LGraph, node: &LNodeRef) -> bool {
    let node_type = node
        .lock()
        .ok()
        .map(|node_guard| node_guard.node_type())
        .unwrap_or(NodeType::Normal);
    if node_type != NodeType::Normal {
        return false;
    }

    let port_count = node
        .lock()
        .ok()
        .map(|node_guard| node_guard.ports().len())
        .unwrap_or(0);
    if port_count <= 1 {
        return false;
    }

    let port_constraints = node
        .lock()
        .ok()
        .and_then(|mut node_guard| node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS))
        .unwrap_or(PortConstraints::Undefined);
    if port_constraints.is_pos_fixed() {
        return false;
    }

    let nf = get_node_flexibility(graph, node);
    if nf == NodeFlexibility::None {
        return false;
    }

    if !nf.is_flexible_size_where_space_permits() {
        let port_spacing: f64 = Spacings::get_individual_or_default_with_graph(
            graph,
            node,
            LayeredOptions::SPACING_PORT_PORT,
        );
        let mut additional_port_spacing: ElkMargin =
            Spacings::get_individual_or_default_with_graph(
                graph,
                node,
                LayeredOptions::SPACING_PORTS_SURROUNDING,
            );
        if additional_port_spacing == ElkMargin::new() {
            additional_port_spacing = ElkMargin::with_any(port_spacing);
        }

        let west_ports = node
            .lock()
            .ok()
            .map(|mut node_guard| node_guard.port_side_view(PortSide::West))
            .unwrap_or_default();
        let required_west_height = additional_port_spacing.top
            + additional_port_spacing.bottom
            + (west_ports.len().saturating_sub(1) as f64) * port_spacing;
        if required_west_height > node_size_y(node) {
            return false;
        }

        let east_ports = node
            .lock()
            .ok()
            .map(|mut node_guard| node_guard.port_side_view(PortSide::East))
            .unwrap_or_default();
        let required_east_height = additional_port_spacing.top
            + additional_port_spacing.bottom
            + (east_ports.len().saturating_sub(1) as f64) * port_spacing;
        if required_east_height > node_size_y(node) {
            return false;
        }
    }

    true
}

fn edge_type_weight(node_type1: NodeType, node_type2: NodeType) -> f64 {
    if node_type1 == NodeType::Normal && node_type2 == NodeType::Normal {
        EDGE_WEIGHT_BASE
    } else if node_type1 == NodeType::Normal || node_type2 == NodeType::Normal {
        2.0 * EDGE_WEIGHT_BASE
    } else {
        8.0 * EDGE_WEIGHT_BASE
    }
}

fn is_handled_edge(edge: &LEdgeRef) -> bool {
    // Extract ports first, drop edge lock, then check (avoids edge→port nested locking)
    let (sp, tp) = edge
        .lock()
        .ok()
        .map(|eg| (eg.source(), eg.target()))
        .unwrap_or((None, None));
    !check_self_loop_ports(&sp, &tp) && !check_in_layer_edge_ports(&sp, &tp)
}

fn get_node_state(node: &LNodeRef) -> i32 {
    let ports = node
        .lock()
        .ok()
        .map(|node_guard| node_guard.ports().clone())
        .unwrap_or_default();
    let mut inco = 0usize;
    let mut ouco = 0usize;
    for port in ports {
        // Extract edge lists, then DROP port lock before checking is_self_loop
        // (is_self_loop locks source/target ports, which would deadlock if this port is held)
        let (incoming, outgoing) = {
            if let Ok(port_guard) = port.lock() {
                (
                    port_guard.incoming_edges().clone(),
                    port_guard.outgoing_edges().clone(),
                )
            } else {
                continue;
            }
            // port lock dropped here
        };
        inco += incoming
            .iter()
            .filter(|edge| {
                let (sp, tp) = edge
                    .lock()
                    .ok()
                    .map(|eg| (eg.source(), eg.target()))
                    .unwrap_or((None, None));
                !check_self_loop_ports(&sp, &tp)
            })
            .count();
        ouco += outgoing
            .iter()
            .filter(|edge| {
                let (sp, tp) = edge
                    .lock()
                    .ok()
                    .map(|eg| (eg.source(), eg.target()))
                    .unwrap_or((None, None));
                !check_self_loop_ports(&sp, &tp)
            })
            .count();
        if inco > 1 || ouco > 1 {
            return JUNCTION;
        }
    }
    if inco + ouco == 1 {
        return JUNCTION;
    }
    OTHER
}

fn length(edge: &NEdgeRef) -> i32 {
    let guard = edge.lock().ok();
    let Some(edge_guard) = guard else {
        return 0;
    };
    let src_layer = edge_guard
        .source
        .lock()
        .ok()
        .map(|node_guard| node_guard.layer)
        .unwrap_or(0);
    let tgt_layer = edge_guard
        .target
        .lock()
        .ok()
        .map(|node_guard| node_guard.layer)
        .unwrap_or(0);
    (src_layer - tgt_layer).abs() - edge_guard.delta
}

fn adjust_label_position(
    placement: &EnumSet<NodeLabelPlacement>,
    label: &LLabelRef,
    size_delta: f64,
) {
    if placement.contains(&NodeLabelPlacement::VBottom) {
        if let Ok(mut label_guard) = label.lock() {
            label_guard.shape().position().y += size_delta;
        }
    } else if placement.contains(&NodeLabelPlacement::VCenter) {
        if let Ok(mut label_guard) = label.lock() {
            label_guard.shape().position().y += size_delta / 2.0;
        }
    }
}

fn node_id(node: &LNodeRef) -> usize {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}

fn edge_id(edge: &LEdgeRef) -> usize {
    edge.lock()
        .ok()
        .map(|mut edge_guard| edge_guard.graph_element().id as usize)
        .unwrap_or(0)
}

fn check_self_loop_ports(source: &Option<LPortRef>, target: &Option<LPortRef>) -> bool {
    match (source, target) {
        (Some(source), Some(target)) => {
            let source_node = source.lock().ok().and_then(|port| port.node());
            let target_node = target.lock().ok().and_then(|port| port.node());
            if let (Some(source_node), Some(target_node)) = (source_node, target_node) {
                Arc::ptr_eq(&source_node, &target_node)
            } else {
                false
            }
        }
        _ => false,
    }
}

fn check_in_layer_edge_ports(source: &Option<LPortRef>, target: &Option<LPortRef>) -> bool {
    if check_self_loop_ports(source, target) {
        return false;
    }
    if let (Some(source), Some(target)) = (source, target) {
        let source_layer = source
            .lock()
            .ok()
            .and_then(|port| port.node())
            .and_then(|node| node.lock().ok().and_then(|node| node.layer()));
        let target_layer = target
            .lock()
            .ok()
            .and_then(|port| port.node())
            .and_then(|node| node.lock().ok().and_then(|node| node.layer()));
        if let (Some(source_layer), Some(target_layer)) = (source_layer, target_layer) {
            return Arc::ptr_eq(&source_layer, &target_layer);
        }
    }
    false
}

fn port_key(port: &LPortRef) -> usize {
    Arc::as_ptr(port) as usize
}

fn node_margin_top(node: &LNodeRef) -> f64 {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.margin().top)
        .unwrap_or(0.0)
}

fn node_margin_bottom(node: &LNodeRef) -> f64 {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.margin().bottom)
        .unwrap_or(0.0)
}

fn node_size_y(node: &LNodeRef) -> f64 {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.shape().size_ref().y)
        .unwrap_or(0.0)
}
