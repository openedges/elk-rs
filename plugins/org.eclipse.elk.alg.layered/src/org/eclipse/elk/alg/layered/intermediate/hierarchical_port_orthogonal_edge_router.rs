#![allow(clippy::mutable_key_type)]

use std::collections::HashSet;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_label_placement::PortLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::GraphAdapter;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, IElkProgressMonitor};

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::nodespacing::node_dimension_calculation::NodeDimensionCalculation;
use crate::org::eclipse::elk::alg::layered::graph::transform::LGraphAdapters;
use crate::org::eclipse::elk::alg::layered::graph::{
    ArenaSync, LEdge, LGraph, LGraphUtil, LNode, LNodeRef, LPort, LayerRef, NodeRefKey, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::orthogonal_routing_generator::OrthogonalRoutingGenerator;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::direction::routing_direction::RoutingDirection;
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

pub struct HierarchicalPortOrthogonalEdgeRouter {
    northern_ext_port_edge_routing_height: f64,
}

fn trace_step(message: &str) {
    if ElkTrace::global().hier_port_ortho {
        eprintln!("[hier-port-ortho] {message}");
    }
}

impl Default for HierarchicalPortOrthogonalEdgeRouter {
    fn default() -> Self {
        Self {
            northern_ext_port_edge_routing_height: 0.0,
        }
    }
}

impl ILayoutProcessor<LGraph> for HierarchicalPortOrthogonalEdgeRouter {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Orthogonally routing hierarchical port edges", 1.0);
        self.northern_ext_port_edge_routing_height = 0.0;

        trace_step("restore_north_south_dummies");
        let north_south_dummies = self.restore_north_south_dummies(layered_graph);
        trace_step("set_north_south_dummy_coordinates");
        self.set_north_south_dummy_coordinates(layered_graph, &north_south_dummies);
        trace_step("route_edges");
        self.route_edges(monitor, layered_graph, &north_south_dummies);
        trace_step("remove_temporary_north_south_dummies");
        self.remove_temporary_north_south_dummies(layered_graph);
        trace_step("fix_coordinates");
        self.fix_coordinates(layered_graph);
        trace_step("correct_slanted_edge_segments");
        self.correct_slanted_edge_segments(layered_graph);

        monitor.done();
    }
}

impl HierarchicalPortOrthogonalEdgeRouter {
    fn restore_north_south_dummies(&self, layered_graph: &mut LGraph) -> Vec<LNodeRef> {
        let mut restored = Vec::new();
        let Some(dummies) =
            layered_graph.get_property(InternalProperties::EXT_PORT_REPLACED_DUMMIES)
        else {
            return restored;
        };

        for dummy in &dummies {
            self.restore_dummy(dummy, layered_graph);
            restored.push(dummy.clone());
        }

        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock().nodes().clone();
            for node in nodes {
                let is_external = {
                    let node_guard = node.lock();
                    node_guard.node_type() == NodeType::ExternalPort
                };
                if !is_external {
                    continue;
                }

                let replaced_dummy = {
                    let node_guard = node.lock();
                    node_guard.get_property(InternalProperties::EXT_PORT_REPLACED_DUMMY)
                };
                if let Some(replaced_dummy) = replaced_dummy {
                    self.connect_node_to_dummy(layered_graph, &node, &replaced_dummy);
                }
            }
        }

        if let Some(last_layer) = layered_graph.layers().last().cloned() {
            for dummy in &restored {
                LNode::set_layer(dummy, Some(last_layer.clone()));
            }
        }

        restored
    }

    fn restore_dummy(&self, dummy: &LNodeRef, graph: &mut LGraph) {
        let (port_side, dummy_port) = {
            let dummy_guard = dummy.lock();
            let side = dummy_guard
                .get_property(InternalProperties::EXT_PORT_SIDE)
                .unwrap_or(PortSide::Undefined);
            let port = dummy_guard.ports().first().cloned();
            match port {
                Some(port) => (side, port),
                None => return,
            }
        };

        {
            let mut port_guard = dummy_port.lock();
            match port_side {
                PortSide::North => port_guard.set_side(PortSide::South),
                PortSide::South => port_guard.set_side(PortSide::North),
                _ => {}
            }
        }

        let size_constraints = graph
            .get_property(LayeredOptions::NODE_SIZE_CONSTRAINTS)
            .unwrap_or_else(EnumSet::none_of);
        if !size_constraints.contains(&SizeConstraint::PortLabels) {
            return;
        }

        let port_label_spacing_horizontal = {
            let dummy_guard = dummy.lock();
            dummy_guard.get_property(LayeredOptions::SPACING_LABEL_PORT_HORIZONTAL)
        }
            .or_else(|| LayeredOptions::SPACING_LABEL_PORT_HORIZONTAL.get_default())
            .unwrap_or(0.0);
        let port_label_spacing_vertical = {
            let dummy_guard = dummy.lock();
            dummy_guard.get_property(LayeredOptions::SPACING_LABEL_PORT_VERTICAL)
        }
            .or_else(|| LayeredOptions::SPACING_LABEL_PORT_VERTICAL.get_default())
            .unwrap_or(0.0);
        let label_label_spacing = {
            let dummy_guard = dummy.lock();
            dummy_guard.get_property(LayeredOptions::SPACING_LABEL_LABEL)
        }
            .or_else(|| LayeredOptions::SPACING_LABEL_LABEL.get_default())
            .unwrap_or(0.0);

        let port_label_placement = graph
            .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
            .unwrap_or_else(PortLabelPlacement::outside);

        if port_label_placement.contains(&PortLabelPlacement::Inside) {
            let mut current_y = port_label_spacing_vertical;
            let (dummy_width, port_pos_x, labels) = {
                let mut dummy_guard = dummy.lock();
                (
                    dummy_guard.shape().size_ref().x,
                    {
                        let mut port_guard = dummy_port.lock();
                        port_guard.shape().position_ref().x
                    },
                    dummy_port
                        .lock().labels().clone(),
                )
            };
            let x_center_relative = dummy_width / 2.0 - port_pos_x;

            for label in labels {
                {
                    let mut label_guard = label.lock();
                    label_guard.shape().position().y = current_y;
                    label_guard.shape().position().x =
                        x_center_relative - label_guard.shape().size_ref().x / 2.0;
                    current_y += label_guard.shape().size_ref().y + label_label_spacing;
                }
            }
        } else if port_label_placement.contains(&PortLabelPlacement::Outside) {
            let (dummy_width, port_pos_x, labels) = {
                let mut dummy_guard = dummy.lock();
                (
                    dummy_guard.shape().size_ref().x,
                    {
                        let mut port_guard = dummy_port.lock();
                        port_guard.shape().position_ref().x
                    },
                    dummy_port
                        .lock().labels().clone(),
                )
            };

            for label in labels {
                {
                    let mut label_guard = label.lock();
                    label_guard.shape().position().x =
                        port_label_spacing_horizontal + dummy_width - port_pos_x;
                }
            }
        }

        let adapter = LGraphAdapters::adapt(graph, false, false, |_| true);
        let mut calculator = NodeDimensionCalculation::get_node_margin_calculator(&adapter);
        for node_adapter in adapter.get_nodes() {
            if std::sync::Arc::ptr_eq(node_adapter.element(), dummy) {
                calculator.process_node(&node_adapter);
                break;
            }
        }
    }

    fn connect_node_to_dummy(&self, _graph: &mut LGraph, node: &LNodeRef, dummy: &LNodeRef) {
        let out_port = LPort::new();
        LPort::set_node(&out_port, Some(node.clone()));

        let ext_port_side = {
            let node_guard = node.lock();
            node_guard.get_property(InternalProperties::EXT_PORT_SIDE)
                .unwrap_or(PortSide::Undefined)
        };
        {
            let mut out_guard = out_port.lock();
            out_guard.set_side(ext_port_side);
        }

        let in_port = {
            let dummy_guard = dummy.lock();
            dummy_guard.ports().first().cloned()
        };
        let Some(in_port) = in_port else {
            return;
        };

        let edge = LEdge::new();
        LEdge::set_source(&edge, Some(out_port));
        LEdge::set_target(&edge, Some(in_port));
    }

    fn set_north_south_dummy_coordinates(
        &self,
        layered_graph: &mut LGraph,
        north_south_dummies: &[LNodeRef],
    ) {
        let constraints = layered_graph
            .get_property(LayeredOptions::PORT_CONSTRAINTS)
            .unwrap_or(PortConstraints::Undefined);

        let graph_size = *layered_graph.size_ref();
        let graph_padding = layered_graph.padding_ref().clone();
        let graph_width = graph_size.x + graph_padding.left + graph_padding.right;
        let north_y = 0.0 - graph_padding.top - layered_graph.offset_ref().y;
        let south_y =
            graph_size.y + graph_padding.top + graph_padding.bottom - layered_graph.offset_ref().y;

        let mut northern_dummies = Vec::new();
        let mut southern_dummies = Vec::new();

        for dummy in north_south_dummies {
            match constraints {
                PortConstraints::Free
                | PortConstraints::FixedSide
                | PortConstraints::FixedOrder => {
                    self.calculate_north_south_dummy_positions(dummy);
                }
                PortConstraints::FixedRatio => {
                    self.apply_north_south_dummy_ratio(dummy, graph_width);
                    {
                        let mut dummy_guard = dummy.lock();
                        let padding_left = layered_graph.padding_ref().left;
                        let offset_x = layered_graph.offset_ref().x;
                        dummy_guard.shape().position().x -= padding_left + offset_x;
                    }
                }
                PortConstraints::FixedPos => {
                    self.apply_north_south_dummy_position(dummy);
                    {
                        let mut dummy_guard = dummy.lock();
                        let padding_left = layered_graph.padding_ref().left;
                        let offset_x = layered_graph.offset_ref().x;
                        dummy_guard.shape().position().x -= padding_left + offset_x;
                    }
                    {
                        let mut dummy_guard = dummy.lock();
                        let required_x = dummy_guard.shape().position_ref().x
                            + dummy_guard.shape().size_ref().x / 2.0;
                        let graph_size = layered_graph.size();
                        graph_size.x = graph_size.x.max(required_x);
                    }
                }
                PortConstraints::Undefined => {}
            }

            let ext_side = {
                let dummy_guard = dummy.lock();
                dummy_guard.get_property(InternalProperties::EXT_PORT_SIDE)
                    .unwrap_or(PortSide::Undefined)
            };

            match ext_side {
                PortSide::North => {
                    {
                        let mut dummy_guard = dummy.lock();
                        dummy_guard.shape().position().y = north_y;
                    }
                    northern_dummies.push(dummy.clone());
                }
                PortSide::South => {
                    {
                        let mut dummy_guard = dummy.lock();
                        dummy_guard.shape().position().y = south_y;
                    }
                    southern_dummies.push(dummy.clone());
                }
                _ => {}
            }
        }

        match constraints {
            PortConstraints::Free | PortConstraints::FixedSide => {
                self.ensure_unique_positions(&northern_dummies, layered_graph);
                self.ensure_unique_positions(&southern_dummies, layered_graph);
            }
            PortConstraints::FixedOrder => {
                self.restore_proper_order(&northern_dummies, layered_graph);
                self.restore_proper_order(&southern_dummies, layered_graph);
            }
            _ => {}
        }
    }

    fn calculate_north_south_dummy_positions(&self, dummy: &LNodeRef) {
        let dummy_port = {
            let dummy_guard = dummy.lock();
            dummy_guard.ports().first().cloned()
        };
        let Some(dummy_port) = dummy_port else {
            return;
        };

        let connected_ports = dummy_port
            .lock().connected_ports();

        if connected_ports.is_empty() {
            {
                let mut dummy_guard = dummy.lock();
                dummy_guard.shape().position().x = 0.0;
            }
            return;
        }

        let mut pos_sum = 0.0;
        for port in &connected_ports {
            {
                let mut port_guard = port.lock();
                if let Some(node) = port_guard.node() {
                    {
                        let mut node_guard = node.lock();
                        pos_sum += node_guard.shape().position_ref().x
                            + port_guard.shape().position_ref().x
                            + port_guard.anchor_ref().x;
                    }
                }
            }
        }

        let offset = {
            let dummy_guard = dummy.lock();
            dummy_guard.get_property(LayeredOptions::PORT_ANCHOR)
                .unwrap_or_default()
                .x
        };

        {
            let mut dummy_guard = dummy.lock();
            dummy_guard.shape().position().x = pos_sum / (connected_ports.len() as f64) - offset;
        }
    }

    fn apply_north_south_dummy_ratio(&self, dummy: &LNodeRef, width: f64) {
        let offset = {
            let dummy_guard = dummy.lock();
            dummy_guard.get_property(LayeredOptions::PORT_ANCHOR)
                .unwrap_or_default()
                .x
        };
        let ratio = {
            let dummy_guard = dummy.lock();
            dummy_guard.get_property(InternalProperties::PORT_RATIO_OR_POSITION)
                .unwrap_or(0.0)
        };

        {
            let mut dummy_guard = dummy.lock();
            dummy_guard.shape().position().x = width * ratio - offset;
        }
    }

    fn apply_north_south_dummy_position(&self, dummy: &LNodeRef) {
        let offset = {
            let dummy_guard = dummy.lock();
            dummy_guard.get_property(LayeredOptions::PORT_ANCHOR)
                .unwrap_or_default()
                .x
        };
        let position = {
            let dummy_guard = dummy.lock();
            dummy_guard.get_property(InternalProperties::PORT_RATIO_OR_POSITION)
                .unwrap_or(0.0)
        };

        {
            let mut dummy_guard = dummy.lock();
            dummy_guard.shape().position().x = position - offset;
        }
    }

    fn ensure_unique_positions(&self, dummies: &[LNodeRef], graph: &mut LGraph) {
        if dummies.is_empty() {
            return;
        }

        let mut dummy_array = LGraphUtil::to_node_array(dummies);
        dummy_array.sort_by(|a, b| {
            let ax = {
                let mut node_guard = a.lock();
                node_guard.shape().position_ref().x
            };
            let bx = {
                let mut node_guard = b.lock();
                node_guard.shape().position_ref().x
            };
            ax.partial_cmp(&bx).unwrap_or(std::cmp::Ordering::Equal)
        });

        self.assign_ascending_coordinates(&dummy_array, graph);
    }

    fn restore_proper_order(&self, dummies: &[LNodeRef], graph: &mut LGraph) {
        if dummies.is_empty() {
            return;
        }

        let mut dummy_array = LGraphUtil::to_node_array(dummies);
        dummy_array.sort_by(|a, b| {
            let ax = {
                let node_guard = a.lock();
                node_guard.get_property(InternalProperties::PORT_RATIO_OR_POSITION)
                    .unwrap_or(0.0)
            };
            let bx = {
                let node_guard = b.lock();
                node_guard.get_property(InternalProperties::PORT_RATIO_OR_POSITION)
                    .unwrap_or(0.0)
            };
            ax.partial_cmp(&bx).unwrap_or(std::cmp::Ordering::Equal)
        });

        self.assign_ascending_coordinates(&dummy_array, graph);
    }

    fn assign_ascending_coordinates(&self, dummies: &[LNodeRef], graph: &mut LGraph) {
        if dummies.is_empty() {
            return;
        }

        let spacing = graph
            .get_property(LayeredOptions::SPACING_PORT_PORT)
            .unwrap_or(0.0);

        let first = {
            let mut node_guard = dummies[0].lock();
            (
                node_guard.shape().position_ref().x,
                node_guard.shape().size_ref().x,
                node_guard.margin().right,
            )
        };
        let mut next_valid = first.0 + first.1 + first.2 + spacing;

        for dummy in dummies.iter().skip(1) {
            let (pos_x, size_x, margin_left, margin_right) = {
                let mut node_guard = dummy.lock();
                (
                    node_guard.shape().position_ref().x,
                    node_guard.shape().size_ref().x,
                    node_guard.margin().left,
                    node_guard.margin().right,
                )
            };
            let delta = pos_x - margin_left - next_valid;
            if delta < 0.0 {
                {
                    let mut dummy_guard = dummy.lock();
                    dummy_guard.shape().position().x -= delta;
                }
            }

            let current_pos = {
                let mut node_guard = dummy.lock();
                node_guard.shape().position_ref().x
            };
            let graph_size = graph.size();
            graph_size.x = graph_size.x.max(current_pos + size_x);

            let new_pos = {
                let mut node_guard = dummy.lock();
                node_guard.shape().position_ref().x
            };
            next_valid = new_pos + size_x + margin_right + spacing;
        }
    }

    fn route_edges(
        &mut self,
        monitor: &mut dyn IElkProgressMonitor,
        layered_graph: &mut LGraph,
        north_south_dummies: &[LNodeRef],
    ) {
        let mut northern_source_layer: Vec<LNodeRef> = Vec::new();
        let mut northern_target_layer: Vec<LNodeRef> = Vec::new();
        let mut southern_source_layer: Vec<LNodeRef> = Vec::new();
        let mut southern_target_layer: Vec<LNodeRef> = Vec::new();

        let mut northern_source_seen: HashSet<NodeRefKey> = HashSet::new();
        let mut northern_target_seen: HashSet<NodeRefKey> = HashSet::new();
        let mut southern_source_seen: HashSet<NodeRefKey> = HashSet::new();
        let mut southern_target_seen: HashSet<NodeRefKey> = HashSet::new();

        let node_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_NODE_NODE)
            .unwrap_or(0.0);
        let edge_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_EDGE_EDGE)
            .unwrap_or(0.0);
        if ElkTrace::global().hier_port_ortho {
            eprintln!(
                "[hier-port-ortho] route_edges prep north(src={},tgt={}) south(src={},tgt={}) spacing(node={},edge={}) graph(size=({:.1},{:.1}) offset=({:.1},{:.1}))",
                northern_source_layer.len(),
                northern_target_layer.len(),
                southern_source_layer.len(),
                southern_target_layer.len(),
                node_spacing,
                edge_spacing,
                layered_graph.size_ref().x,
                layered_graph.size_ref().y,
                layered_graph.offset_ref().x,
                layered_graph.offset_ref().y
            );
        }

        for dummy in north_south_dummies {
            let ext_side = {
                let dummy_guard = dummy.lock();
                dummy_guard.get_property(InternalProperties::EXT_PORT_SIDE)
                    .unwrap_or(PortSide::Undefined)
            };

            if ext_side == PortSide::North {
                let key = NodeRefKey(dummy.clone());
                if northern_target_seen.insert(key) {
                    northern_target_layer.push(dummy.clone());
                }

                let incoming_edges = dummy
                    .lock().incoming_edges();
                for edge in incoming_edges {
                    if let Some(source_node) = edge
                        .lock().source()
                        .and_then(|port| port.lock().node())
                    {
                        let key = NodeRefKey(source_node.clone());
                        if northern_source_seen.insert(key) {
                            northern_source_layer.push(source_node);
                        }
                    }
                }
            } else if ext_side == PortSide::South {
                let key = NodeRefKey(dummy.clone());
                if southern_target_seen.insert(key) {
                    southern_target_layer.push(dummy.clone());
                }

                let incoming_edges = dummy
                    .lock().incoming_edges();
                for edge in incoming_edges {
                    if let Some(source_node) = edge
                        .lock().source()
                        .and_then(|port| port.lock().node())
                    {
                        let key = NodeRefKey(source_node.clone());
                        if southern_source_seen.insert(key) {
                            southern_source_layer.push(source_node);
                        }
                    }
                }
            }
        }

        if !northern_source_layer.is_empty() {
            let sync = ArenaSync::from_lgraph(layered_graph);
            let mut routing_generator = OrthogonalRoutingGenerator::new(
                RoutingDirection::SouthToNorth,
                edge_spacing,
                Some("extnorth".to_string()),
            );
            let slots = routing_generator.route_edges(
                monitor,
                layered_graph,
                Some(&northern_source_layer),
                0,
                Some(&northern_target_layer),
                -node_spacing - layered_graph.offset_ref().y,
                &sync,
            );
            if slots > 0 {
                self.northern_ext_port_edge_routing_height =
                    node_spacing + (slots as f64 - 1.0) * edge_spacing;
                layered_graph.offset().y += self.northern_ext_port_edge_routing_height;
                layered_graph.size().y += self.northern_ext_port_edge_routing_height;
            }
            if ElkTrace::global().hier_port_ortho {
                eprintln!(
                    "[hier-port-ortho] north slots={} added_height={:.1} graph(size=({:.1},{:.1}) offset=({:.1},{:.1}))",
                    slots,
                    self.northern_ext_port_edge_routing_height,
                    layered_graph.size_ref().x,
                    layered_graph.size_ref().y,
                    layered_graph.offset_ref().x,
                    layered_graph.offset_ref().y
                );
            }
        }

        if !southern_source_layer.is_empty() {
            let sync = ArenaSync::from_lgraph(layered_graph);
            let mut routing_generator = OrthogonalRoutingGenerator::new(
                RoutingDirection::NorthToSouth,
                edge_spacing,
                Some("extsouth".to_string()),
            );
            let slots = routing_generator.route_edges(
                monitor,
                layered_graph,
                Some(&southern_source_layer),
                0,
                Some(&southern_target_layer),
                layered_graph.size_ref().y + node_spacing - layered_graph.offset_ref().y,
                &sync,
            );
            if slots > 0 {
                layered_graph.size().y += node_spacing + (slots as f64 - 1.0) * edge_spacing;
            }
            if ElkTrace::global().hier_port_ortho {
                eprintln!(
                    "[hier-port-ortho] south slots={} graph(size=({:.1},{:.1}) offset=({:.1},{:.1}))",
                    slots,
                    layered_graph.size_ref().x,
                    layered_graph.size_ref().y,
                    layered_graph.offset_ref().x,
                    layered_graph.offset_ref().y
                );
            }
        }
    }

    fn remove_temporary_north_south_dummies(&self, layered_graph: &mut LGraph) {
        let mut nodes_to_remove: Vec<LNodeRef> = Vec::new();

        for layer in layered_graph.layers().clone() {
            let nodes = layer
                .lock().nodes().clone();
            for node in nodes {
                let is_external = {
                    let node_guard = node.lock();
                    node_guard.node_type() == NodeType::ExternalPort
                };
                if !is_external {
                    continue;
                }

                let has_replaced = {
                    let mut node_guard = node.lock();
                    node_guard
                        .shape()
                        .graph_element()
                        .properties()
                        .has_property(InternalProperties::EXT_PORT_REPLACED_DUMMY)
                };
                if !has_replaced {
                    continue;
                }

                let (node_in_port, node_out_port, node_origin_port) = {
                    let node_guard = node.lock();
                    let mut in_port = None;
                    let mut out_port = None;
                    let mut origin_port = None;
                    for port in node_guard.ports() {
                        let side = port
                            .lock().side();
                        match side {
                            PortSide::West => in_port = Some(port.clone()),
                            PortSide::East => out_port = Some(port.clone()),
                            _ => origin_port = Some(port.clone()),
                        }
                    }
                    (in_port, out_port, origin_port)
                };

                let (Some(node_in_port), Some(node_out_port), Some(node_origin_port)) =
                    (node_in_port, node_out_port, node_origin_port)
                else {
                    continue;
                };

                let node_to_origin_edge = {
                    let port_guard = node_origin_port.lock();
                    port_guard.outgoing_edges().first().cloned()
                };
                let Some(node_to_origin_edge) = node_to_origin_edge else {
                    continue;
                };

                let origin_bends = {
                    let edge_guard = node_to_origin_edge.lock();
                    edge_guard
                        .bend_points_ref()
                        .iter()
                        .copied()
                        .collect::<Vec<_>>()
                };
                let mut incoming_bends = KVectorChain::from_vectors(&origin_bends);
                let mut first_bend = {
                    let mut port_guard = node_origin_port.lock();
                    *port_guard.shape().position_ref()
                };
                {
                    let mut node_guard = node.lock();
                    first_bend.add(node_guard.shape().position_ref());
                }
                incoming_bends.insert(0, first_bend);

                let mut outgoing_bends =
                    KVectorChain::reverse(&KVectorChain::from_vectors(&origin_bends));
                let mut last_bend = {
                    let mut port_guard = node_origin_port.lock();
                    *port_guard.shape().position_ref()
                };
                {
                    let mut node_guard = node.lock();
                    last_bend.add(node_guard.shape().position_ref());
                }
                outgoing_bends.add_vector(last_bend);

                let replaced_dummy = {
                    let node_guard = node.lock();
                    node_guard.get_property(InternalProperties::EXT_PORT_REPLACED_DUMMY)
                };
                let Some(replaced_dummy) = replaced_dummy else {
                    continue;
                };
                let replaced_dummy_port = {
                    let dummy_guard = replaced_dummy.lock();
                    dummy_guard.ports().first().cloned()
                };
                let Some(replaced_dummy_port) = replaced_dummy_port else {
                    continue;
                };

                let incoming_edges = node_in_port
                    .lock().incoming_edges().clone();
                for edge in incoming_edges {
                    LEdge::set_target(&edge, Some(replaced_dummy_port.clone()));
                    {
                        let mut edge_guard = edge.lock();
                        let mut existing: Vec<KVector> =
                            edge_guard.bend_points_ref().iter().copied().collect();
                        let mut extra: Vec<KVector> = incoming_bends.iter().copied().collect();
                        existing.append(&mut extra);
                        edge_guard.bend_points().clear();
                        edge_guard.bend_points().add_all(&existing);
                    }
                }

                let outgoing_edges = node_out_port
                    .lock().outgoing_edges().clone();
                for edge in outgoing_edges {
                    LEdge::set_source(&edge, Some(replaced_dummy_port.clone()));
                    {
                        let mut edge_guard = edge.lock();
                        let mut existing: Vec<KVector> =
                            edge_guard.bend_points_ref().iter().copied().collect();
                        let mut extra: Vec<KVector> = outgoing_bends.iter().copied().collect();
                        extra.append(&mut existing);
                        edge_guard.bend_points().clear();
                        edge_guard.bend_points().add_all(&extra);
                    }
                }

                LEdge::set_source(&node_to_origin_edge, None);
                LEdge::set_target(&node_to_origin_edge, None);

                nodes_to_remove.push(node);
            }
        }

        for node in nodes_to_remove {
            LNode::set_layer(&node, None);
        }
    }

    fn fix_coordinates(&self, layered_graph: &mut LGraph) {
        let constraints = layered_graph
            .get_property(LayeredOptions::PORT_CONSTRAINTS)
            .unwrap_or(PortConstraints::Undefined);
        let layers = layered_graph.layers().clone();
        if layers.is_empty() {
            return;
        }

        self.fix_coordinates_for_layer(&layers[0], constraints, layered_graph);
        if layers.len() > 1 {
            self.fix_coordinates_for_layer(&layers[layers.len() - 1], constraints, layered_graph);
        }
    }

    fn fix_coordinates_for_layer(
        &self,
        layer: &LayerRef,
        constraints: PortConstraints,
        graph: &mut LGraph,
    ) {
        let padding = graph.padding_ref().clone();
        let offset = *graph.offset_ref();
        let graph_actual_size = graph.actual_size();
        let mut new_actual_height = graph_actual_size.y;

        let nodes = layer
            .lock().nodes().clone();

        for node in &nodes {
            let (node_type, ext_side, ext_size, port_anchor) = {
                let node_guard = node.lock();
                (
                    node_guard.node_type(),
                    node_guard
                        .get_property(InternalProperties::EXT_PORT_SIDE)
                        .unwrap_or(PortSide::Undefined),
                    node_guard
                        .get_property(InternalProperties::EXT_PORT_SIZE)
                        .unwrap_or_default(),
                    node_guard
                        .get_property(LayeredOptions::PORT_ANCHOR)
                        .unwrap_or_default(),
                )
            };

            if node_type != NodeType::ExternalPort {
                continue;
            }

            {
                let mut node_guard = node.lock();
                match ext_side {
                    PortSide::East => {
                        node_guard.shape().position().x =
                            graph.size_ref().x + padding.right - offset.x;
                    }
                    PortSide::West => {
                        node_guard.shape().position().x = -offset.x - padding.left;
                    }
                    _ => {}
                }
            }

            let mut required_height = 0.0;
            match ext_side {
                PortSide::East | PortSide::West => {
                    if constraints == PortConstraints::FixedRatio {
                        let ratio = {
                            let node_guard = node.lock();
                            node_guard.get_property(InternalProperties::PORT_RATIO_OR_POSITION)
                                .unwrap_or(0.0)
                        };
                        {
                            let mut node_guard = node.lock();
                            node_guard.shape().position().y =
                                graph_actual_size.y * ratio - port_anchor.y;
                            required_height = node_guard.shape().position_ref().y + ext_size.y;
                            node_guard.shape().position().y -= padding.top + offset.y;
                        }
                    } else if constraints == PortConstraints::FixedPos {
                        let pos = {
                            let node_guard = node.lock();
                            node_guard.get_property(InternalProperties::PORT_RATIO_OR_POSITION)
                                .unwrap_or(0.0)
                        };
                        {
                            let mut node_guard = node.lock();
                            node_guard.shape().position().y = pos - port_anchor.y;
                            required_height = node_guard.shape().position_ref().y + ext_size.y;
                            node_guard.shape().position().y -= padding.top + offset.y;
                        }
                    }
                }
                _ => {}
            }

            new_actual_height = new_actual_height.max(required_height);
        }

        graph.size().y += new_actual_height - graph_actual_size.y;

        for node in nodes {
            let (node_type, ext_side) = {
                let node_guard = node.lock();
                (
                    node_guard.node_type(),
                    node_guard
                        .get_property(InternalProperties::EXT_PORT_SIDE)
                        .unwrap_or(PortSide::Undefined),
                )
            };
            if node_type != NodeType::ExternalPort {
                continue;
            }

            match ext_side {
                PortSide::North => {
                    {
                        let mut node_guard = node.lock();
                        node_guard.shape().position().y = -offset.y - padding.top;
                    }
                }
                PortSide::South => {
                    {
                        let mut node_guard = node.lock();
                        node_guard.shape().position().y =
                            graph.size_ref().y + padding.bottom - offset.y;
                    }
                }
                _ => {}
            }
        }
    }

    fn correct_slanted_edge_segments(&self, layered_graph: &mut LGraph) {
        let layers = layered_graph.layers().clone();
        if layers.is_empty() {
            return;
        }
        self.correct_slanted_edge_segments_layer(&layers[0]);
        if layers.len() > 1 {
            self.correct_slanted_edge_segments_layer(&layers[layers.len() - 1]);
        }
    }

    fn correct_slanted_edge_segments_layer(&self, layer: &LayerRef) {
        let nodes = layer
            .lock().nodes().clone();
        for node in nodes {
            let (node_type, ext_side) = {
                let node_guard = node.lock();
                (
                    node_guard.node_type(),
                    node_guard
                        .get_property(InternalProperties::EXT_PORT_SIDE)
                        .unwrap_or(PortSide::Undefined),
                )
            };
            if node_type != NodeType::ExternalPort {
                continue;
            }
            if ext_side != PortSide::East && ext_side != PortSide::West {
                continue;
            }

            let connected_edges = node
                .lock().connected_edges();
            for edge in connected_edges {
                {
                    let mut edge_guard = edge.lock();
                    if edge_guard.bend_points_ref().is_empty() {
                        continue;
                    }

                    if let Some(source_port) = edge_guard.source() {
                        let belongs = source_port
                            .lock().node()
                            .map(|port_node| std::sync::Arc::ptr_eq(&port_node, &node))
                            .unwrap_or(false);
                        if belongs {
                            {
                                let port_guard = source_port.lock();
                                if let Some(anchor) = port_guard.absolute_anchor() {
                                    let mut first = edge_guard.bend_points_ref().get_first();
                                    first.y = anchor.y;
                                    edge_guard.bend_points().set(0, first);
                                }
                            }
                        }
                    }

                    if let Some(target_port) = edge_guard.target() {
                        let belongs = target_port
                            .lock().node()
                            .map(|port_node| std::sync::Arc::ptr_eq(&port_node, &node))
                            .unwrap_or(false);
                        if belongs {
                            {
                                let port_guard = target_port.lock();
                                if let Some(anchor) = port_guard.absolute_anchor() {
                                    let last_index = edge_guard.bend_points_ref().len() - 1;
                                    let mut last = edge_guard.bend_points_ref().get_last();
                                    last.y = anchor.y;
                                    edge_guard.bend_points().set(last_index, last);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
