use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::alignment::Alignment;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};

use crate::org::eclipse::elk::alg::layered::options::{
    EdgeConstraint, GraphProperties, InLayerConstraint, InternalProperties, LayerConstraint,
    LayeredOptions, PortType,
};

use super::l_node::NodeType;
use super::{LEdgeRef, LGraphRef, LNodeRef, LPort, LPortRef, LayerRef};

pub struct LGraphUtil;

impl LGraphUtil {
    pub fn to_node_array(nodes: &[LNodeRef]) -> Vec<LNodeRef> {
        nodes.to_vec()
    }

    pub fn to_edge_array(edges: &[LEdgeRef]) -> Vec<LEdgeRef> {
        edges.to_vec()
    }

    pub fn to_port_array(ports: &[LPortRef]) -> Vec<LPortRef> {
        ports.to_vec()
    }

    pub fn resize_node(node: &LNodeRef, new_size: &KVector, move_ports: bool, move_labels: bool) {
        let (old_size, ports, labels, fixed_ports) = match node.lock() {
            Ok(mut node_guard) => (
                *node_guard.shape().size_ref(),
                node_guard.ports().clone(),
                node_guard.labels().clone(),
                node_guard
                    .get_property(LayeredOptions::PORT_CONSTRAINTS)
                    .unwrap_or(PortConstraints::Undefined)
                    == PortConstraints::FixedPos,
            ),
            Err(_) => return,
        };

        let width_ratio = new_size.x / old_size.x;
        let height_ratio = new_size.y / old_size.y;
        let width_diff = new_size.x - old_size.x;
        let height_diff = new_size.y - old_size.y;

        if move_ports {
            for port in ports {
                if let Ok(mut port_guard) = port.lock() {
                    let side = port_guard.side();
                    let pos = port_guard.shape().position();
                    match side {
                        PortSide::North => {
                            if !fixed_ports {
                                pos.x *= width_ratio;
                            }
                        }
                        PortSide::East => {
                            pos.x += width_diff;
                            if !fixed_ports {
                                pos.y *= height_ratio;
                            }
                        }
                        PortSide::South => {
                            if !fixed_ports {
                                pos.x *= width_ratio;
                            }
                            pos.y += height_diff;
                        }
                        PortSide::West => {
                            if !fixed_ports {
                                pos.y *= height_ratio;
                            }
                        }
                        PortSide::Undefined => {}
                    }
                }
            }
        }

        if move_labels {
            for label in labels {
                if let Ok(mut label_guard) = label.lock() {
                    let (pos_x, pos_y, size_x, size_y) = {
                        let shape = label_guard.shape();
                        let pos = *shape.position_ref();
                        let size = *shape.size_ref();
                        (pos.x, pos.y, size.x, size.y)
                    };
                    let midx = pos_x + size_x / 2.0;
                    let midy = pos_y + size_y / 2.0;
                    let width_percent = midx / old_size.x;
                    let height_percent = midy / old_size.y;

                    if width_percent + height_percent >= 1.0 {
                        let pos = label_guard.shape().position();
                        if width_percent - height_percent > 0.0 && midy >= 0.0 {
                            pos.x = pos_x + width_diff;
                            pos.y = pos_y + height_diff * height_percent;
                        } else if width_percent - height_percent < 0.0 && midx >= 0.0 {
                            pos.x = pos_x + width_diff * width_percent;
                            pos.y = pos_y + height_diff;
                        }
                    }
                }
            }
        }

        if let Ok(mut node_guard) = node.lock() {
            let size = node_guard.shape().size();
            size.x = new_size.x;
            size.y = new_size.y;
            node_guard.set_property(
                LayeredOptions::NODE_SIZE_CONSTRAINTS,
                Some(SizeConstraint::fixed()),
            );
        }
    }

    pub fn offset_graphs(graphs: &[LGraphRef], offsetx: f64, offsety: f64) {
        for graph in graphs {
            LGraphUtil::offset_graph(graph, offsetx, offsety);
        }
    }

    pub fn offset_graph(graph: &LGraphRef, offsetx: f64, offsety: f64) {
        let graph_offset = KVector::with_values(offsetx, offsety);
        let layerless_nodes = graph
            .lock()
            .map(|graph_guard| graph_guard.layerless_nodes().clone())
            .unwrap_or_default();

        for node in layerless_nodes {
            let ports = if let Ok(mut node_guard) = node.lock() {
                node_guard.shape().position().add(&graph_offset);
                node_guard.ports().clone()
            } else {
                Vec::new()
            };

            for port in ports {
                let outgoing_edges = port
                    .lock()
                    .map(|port_guard| port_guard.outgoing_edges().clone())
                    .unwrap_or_default();

                for edge in outgoing_edges {
                    let labels = if let Ok(mut edge_guard) = edge.lock() {
                        edge_guard.bend_points().offset(offsetx, offsety);

                        if let Some(mut junction_points) =
                            edge_guard.get_property(LayeredOptions::JUNCTION_POINTS)
                        {
                            junction_points.offset(offsetx, offsety);
                            edge_guard.set_property(
                                LayeredOptions::JUNCTION_POINTS,
                                Some(junction_points),
                            );
                        }

                        edge_guard.labels().clone()
                    } else {
                        Vec::new()
                    };

                    for label in labels {
                        if let Ok(mut label_guard) = label.lock() {
                            label_guard.shape().position().add(&graph_offset);
                        }
                    }
                }
            }
        }
    }

    pub fn place_nodes_horizontally(layer: &LayerRef, xoffset: f64) {
        let (nodes, layer_size) = if let Ok(mut layer_guard) = layer.lock() {
            if layer_guard.size_ref().x <= 0.0 {
                let mut max_width = 0.0;
                for node in layer_guard.nodes() {
                    if let Ok(mut node_guard) = node.lock() {
                        let size_x = node_guard.shape().size_ref().x;
                        let margin = node_guard.margin();
                        let width = size_x + margin.left + margin.right;
                        if width > max_width {
                            max_width = width;
                        }
                    }
                }
                layer_guard.size().x = max_width;
            }
            (layer_guard.nodes().clone(), *layer_guard.size_ref())
        } else {
            return;
        };

        let mut max_left_margin: f64 = 0.0;
        let mut max_right_margin: f64 = 0.0;
        for node in &nodes {
            if let Ok(mut node_guard) = node.lock() {
                let margin = node_guard.margin();
                max_left_margin = max_left_margin.max(margin.left);
                max_right_margin = max_right_margin.max(margin.right);
            }
        }

        for node in nodes {
            let (alignment, node_size, margin, ports) = if let Ok(mut node_guard) = node.lock() {
                (
                    node_guard
                        .get_property(LayeredOptions::ALIGNMENT)
                        .unwrap_or(Alignment::Center),
                    *node_guard.shape().size_ref(),
                    node_guard.margin().clone(),
                    node_guard.ports().clone(),
                )
            } else {
                continue;
            };

            let ratio = match alignment {
                Alignment::Left => 0.0,
                Alignment::Right => 1.0,
                Alignment::Center => 0.5,
                _ => {
                    let mut inports = 0;
                    let mut outports = 0;
                    for port in ports {
                        if let Ok(port_guard) = port.lock() {
                            if !port_guard.incoming_edges().is_empty() {
                                inports += 1;
                            }
                            if !port_guard.outgoing_edges().is_empty() {
                                outports += 1;
                            }
                        }
                    }
                    if inports + outports == 0 {
                        0.5
                    } else {
                        outports as f64 / (inports + outports) as f64
                    }
                }
            };

            let mut xpos = (layer_size.x - node_size.x) * ratio;
            if ratio > 0.5 {
                xpos -= max_right_margin * 2.0 * (ratio - 0.5);
            } else if ratio < 0.5 {
                xpos += max_left_margin * 2.0 * (0.5 - ratio);
            }

            if xpos < margin.left {
                xpos = margin.left;
            }
            let max_x = layer_size.x - margin.right - node_size.x;
            if xpos > max_x {
                xpos = max_x;
            }

            if let Ok(mut node_guard) = node.lock() {
                node_guard.shape().position().x = xoffset + xpos;
            }
        }
    }

    pub fn find_max_non_dummy_node_width(layer: &LayerRef, respect_node_margins: bool) -> f64 {
        let (direction, nodes) = if let Ok(layer_guard) = layer.lock() {
            let direction = layer_guard
                .graph()
                .and_then(|graph| graph.lock().ok().and_then(|mut g| g.get_property(LayeredOptions::DIRECTION)))
                .unwrap_or(Direction::Undefined);
            (direction, layer_guard.nodes().clone())
        } else {
            return 0.0;
        };

        if direction.is_vertical() {
            return 0.0;
        }

        let mut max_width = 0.0;
        for node in nodes {
            if let Ok(mut node_guard) = node.lock() {
                if node_guard.node_type() == NodeType::Normal {
                    let mut width = node_guard.shape().size_ref().x;
                    if respect_node_margins {
                        let margin = node_guard.margin();
                        width += margin.left + margin.right;
                    }
                    if width > max_width {
                        max_width = width;
                    }
                }
            }
        }

        max_width
    }

    pub fn compute_graph_properties(layered_graph: &LGraphRef) {
        let direction = LGraphUtil::get_direction(layered_graph);
        let nodes = layered_graph
            .lock()
            .map(|graph_guard| graph_guard.layerless_nodes().clone())
            .unwrap_or_default();

        let mut props = EnumSet::none_of();

        for node in nodes {
            let (node_type, is_comment, is_hypernode, ports, port_constraints) =
                if let Ok(mut node_guard) = node.lock() {
                    (
                        node_guard.node_type(),
                        node_guard
                            .get_property(LayeredOptions::COMMENT_BOX)
                            .unwrap_or(false),
                        node_guard
                            .get_property(LayeredOptions::HYPERNODE)
                            .unwrap_or(false),
                        node_guard.ports().clone(),
                        node_guard
                            .get_property(LayeredOptions::PORT_CONSTRAINTS)
                            .unwrap_or(PortConstraints::Undefined),
                    )
                } else {
                    continue;
                };

            if is_comment {
                props.insert(GraphProperties::Comments);
            } else if is_hypernode {
                props.insert(GraphProperties::Hypernodes);
                props.insert(GraphProperties::Hyperedges);
            } else if node_type == NodeType::ExternalPort {
                props.insert(GraphProperties::ExternalPorts);
            }

            let normalized_constraints = if port_constraints == PortConstraints::Undefined {
                if let Ok(mut node_guard) = node.lock() {
                    node_guard.set_property(
                        LayeredOptions::PORT_CONSTRAINTS,
                        Some(PortConstraints::Free),
                    );
                }
                PortConstraints::Free
            } else {
                port_constraints
            };

            if normalized_constraints != PortConstraints::Free {
                props.insert(GraphProperties::NonFreePorts);
            }

            for port in ports {
                let (port_side, outgoing_edges, incident_edges) = if let Ok(port_guard) = port.lock()
                {
                    (
                        port_guard.side(),
                        port_guard.outgoing_edges().clone(),
                        port_guard.incoming_edges().len() + port_guard.outgoing_edges().len(),
                    )
                } else {
                    continue;
                };

                if incident_edges > 1 {
                    props.insert(GraphProperties::Hyperedges);
                }

                match direction {
                    Direction::Up | Direction::Down => {
                        if port_side == PortSide::East || port_side == PortSide::West {
                            props.insert(GraphProperties::NorthSouthPorts);
                        }
                    }
                    _ => {
                        if port_side == PortSide::North || port_side == PortSide::South {
                            props.insert(GraphProperties::NorthSouthPorts);
                        }
                    }
                }

                for edge in outgoing_edges {
                    let labels = if let Ok(edge_guard) = edge.lock() {
                        if let Some(target) = edge_guard.target() {
                            if let Ok(target_guard) = target.lock() {
                                if let Some(target_node) = target_guard.node() {
                                    if Arc::ptr_eq(&target_node, &node) {
                                        props.insert(GraphProperties::SelfLoops);
                                    }
                                }
                            }
                        }

                        edge_guard.labels().clone()
                    } else {
                        Vec::new()
                    };

                    for label in labels {
                        if let Ok(mut label_guard) = label.lock() {
                            match label_guard
                                .get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
                                .unwrap_or(EdgeLabelPlacement::Center)
                            {
                                EdgeLabelPlacement::Center => {
                                    props.insert(GraphProperties::CenterLabels);
                                }
                                EdgeLabelPlacement::Head | EdgeLabelPlacement::Tail => {
                                    props.insert(GraphProperties::EndLabels);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Ok(mut graph_guard) = layered_graph.lock() {
            graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(props));
        }
    }

    pub fn create_port(
        node: &LNodeRef,
        end_point: Option<KVector>,
        port_type: PortType,
        layered_graph: &LGraphRef,
    ) -> LPortRef {
        let direction = LGraphUtil::get_direction(layered_graph);
        let merge_ports = layered_graph
            .lock()
            .ok()
            .and_then(|mut graph_guard| graph_guard.get_property(LayeredOptions::MERGE_EDGES))
            .unwrap_or(false);
        let node_is_hyper = node
            .lock()
            .ok()
            .and_then(|mut node_guard| node_guard.get_property(LayeredOptions::HYPERNODE))
            .unwrap_or(false);
        let port_constraints = node
            .lock()
            .ok()
            .and_then(|mut node_guard| node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS))
            .unwrap_or(PortConstraints::Undefined);

        if (merge_ports || node_is_hyper) && !port_constraints.is_side_fixed() {
            let default_side = PortSide::from_direction(direction);
            let side = match port_type {
                PortType::Output => default_side,
                PortType::Input => default_side.opposed(),
                PortType::Undefined => default_side,
            };
            return LGraphUtil::provide_collector_port(layered_graph, node, port_type, side);
        }

        let port = LPort::new();
        LPort::set_node(&port, Some(node.clone()));

        if let Some(end_point) = end_point {
            if let Ok(mut port_guard) = port.lock() {
                if let Ok(mut node_guard) = node.lock() {
                    let node_pos = *node_guard.shape().position_ref();
                    let node_size = *node_guard.shape().size_ref();
                    let pos = port_guard.shape().position();
                    pos.x = end_point.x - node_pos.x;
                    pos.y = end_point.y - node_pos.y;
                    pos.bound(0.0, 0.0, node_size.x, node_size.y);
                }
            }
            let side = LGraphUtil::calc_port_side(&port, direction);
            if let Ok(mut port_guard) = port.lock() {
                port_guard.set_side(side);
            }
        } else {
            let default_side = PortSide::from_direction(direction);
            let side = match port_type {
                PortType::Output => default_side,
                PortType::Input => default_side.opposed(),
                PortType::Undefined => default_side,
            };
            if let Ok(mut port_guard) = port.lock() {
                port_guard.set_side(side);
            }
        }

        if let Ok(mut graph_guard) = layered_graph.lock() {
            let mut graph_props = graph_guard
                .get_property(InternalProperties::GRAPH_PROPERTIES)
                .unwrap_or(EnumSet::none_of());

            if let Ok(port_guard) = port.lock() {
                let port_side = port_guard.side();
                match direction {
                    Direction::Left | Direction::Right => {
                        if port_side == PortSide::North || port_side == PortSide::South {
                            graph_props.insert(GraphProperties::NorthSouthPorts);
                        }
                    }
                    Direction::Up | Direction::Down => {
                        if port_side == PortSide::East || port_side == PortSide::West {
                            graph_props.insert(GraphProperties::NorthSouthPorts);
                        }
                    }
                    _ => {}
                }
            }

            graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(graph_props));
        }

        port
    }

    pub fn calc_port_side(port: &LPortRef, direction: Direction) -> PortSide {
        let (node, pos, size) = if let Ok(mut port_guard) = port.lock() {
            (
                port_guard.node(),
                *port_guard.shape().position_ref(),
                *port_guard.shape().size_ref(),
            )
        } else {
            return PortSide::Undefined;
        };

        let node = match node {
            Some(node) => node,
            None => return PortSide::Undefined,
        };

        let node_size = if let Ok(mut node_guard) = node.lock() {
            *node_guard.shape().size_ref()
        } else {
            return PortSide::Undefined;
        };

        if node_size.x <= 0.0 && node_size.y <= 0.0 {
            return PortSide::Undefined;
        }

        match direction {
            Direction::Left | Direction::Right => {
                if pos.x < 0.0 {
                    return PortSide::West;
                }
                if pos.x + size.x > node_size.x {
                    return PortSide::East;
                }
            }
            Direction::Up | Direction::Down => {
                if pos.y < 0.0 {
                    return PortSide::North;
                }
                if pos.y + size.y > node_size.y {
                    return PortSide::South;
                }
            }
            _ => {}
        }

        let width_percent = (pos.x + size.x / 2.0) / node_size.x;
        let height_percent = (pos.y + size.y / 2.0) / node_size.y;
        if width_percent + height_percent <= 1.0 && width_percent - height_percent <= 0.0 {
            PortSide::West
        } else if width_percent + height_percent >= 1.0 && width_percent - height_percent >= 0.0 {
            PortSide::East
        } else if height_percent < 0.5 {
            PortSide::North
        } else {
            PortSide::South
        }
    }

    pub fn calc_port_offset(port: &LPortRef, side: PortSide) -> f64 {
        let (node, pos, size) = if let Ok(mut port_guard) = port.lock() {
            (
                port_guard.node(),
                *port_guard.shape().position_ref(),
                *port_guard.shape().size_ref(),
            )
        } else {
            return 0.0;
        };

        let node = match node {
            Some(node) => node,
            None => return 0.0,
        };

        let node_size = if let Ok(mut node_guard) = node.lock() {
            *node_guard.shape().size_ref()
        } else {
            return 0.0;
        };

        match side {
            PortSide::North => -(pos.y + size.y),
            PortSide::East => pos.x - node_size.x,
            PortSide::South => pos.y - node_size.y,
            PortSide::West => -(pos.x + size.x),
            PortSide::Undefined => 0.0,
        }
    }

    fn center_point(point: &mut KVector, boundary: &KVector, side: PortSide) {
        match side {
            PortSide::North => {
                point.x = boundary.x / 2.0;
                point.y = 0.0;
            }
            PortSide::East => {
                point.x = boundary.x;
                point.y = boundary.y / 2.0;
            }
            PortSide::South => {
                point.x = boundary.x / 2.0;
                point.y = boundary.y;
            }
            PortSide::West => {
                point.x = 0.0;
                point.y = boundary.y / 2.0;
            }
            PortSide::Undefined => {}
        }
    }

    pub fn provide_collector_port(
        _layered_graph: &LGraphRef,
        node: &LNodeRef,
        port_type: PortType,
        side: PortSide,
    ) -> LPortRef {
        let mut port: Option<LPortRef> = None;

        match port_type {
            PortType::Input => {
                if let Ok(node_guard) = node.lock() {
                    for candidate in node_guard.ports() {
                        if candidate
                            .lock()
                            .ok()
                            .and_then(|mut port| port.get_property(InternalProperties::INPUT_COLLECT))
                            .unwrap_or(false)
                        {
                            return candidate.clone();
                        }
                    }
                }
                let created = LPort::new();
                if let Ok(mut port) = created.lock() {
                    port.set_property(InternalProperties::INPUT_COLLECT, Some(true));
                }
                port = Some(created);
            }
            PortType::Output => {
                if let Ok(node_guard) = node.lock() {
                    for candidate in node_guard.ports() {
                        if candidate
                            .lock()
                            .ok()
                            .and_then(|mut port| port.get_property(InternalProperties::OUTPUT_COLLECT))
                            .unwrap_or(false)
                        {
                            return candidate.clone();
                        }
                    }
                }
                let created = LPort::new();
                if let Ok(mut port) = created.lock() {
                    port.set_property(InternalProperties::OUTPUT_COLLECT, Some(true));
                }
                port = Some(created);
            }
            PortType::Undefined => {}
        }

        if let Some(port_ref) = port {
            LPort::set_node(&port_ref, Some(node.clone()));
            if let Ok(mut port_guard) = port_ref.lock() {
                port_guard.set_side(side);
                let size = node
                    .lock()
                    .map(|mut node| *node.shape().size_ref())
                    .unwrap_or_default();
                let mut pos = KVector::new();
                LGraphUtil::center_point(&mut pos, &size, side);
                *port_guard.shape().position() = pos;
            }
            return port_ref;
        }

        LPort::new()
    }

    pub fn initialize_port(
        port: &LPortRef,
        port_constraints: PortConstraints,
        direction: Direction,
        anchor_pos: Option<KVector>,
    ) {
        let mut port_side = port
            .lock()
            .ok()
            .map(|port_guard| port_guard.side())
            .unwrap_or(PortSide::Undefined);

        if port_side == PortSide::Undefined && port_constraints.is_side_fixed() {
            port_side = LGraphUtil::calc_port_side(port, direction);
            if let Ok(mut port_guard) = port.lock() {
                port_guard.set_side(port_side);

                let has_border_offset = port_guard
                    .shape()
                    .graph_element()
                    .properties()
                    .has_property(LayeredOptions::PORT_BORDER_OFFSET);
                let pos = *port_guard.shape().position_ref();

                if !has_border_offset
                    && port_side != PortSide::Undefined
                    && (pos.x != 0.0 || pos.y != 0.0)
                {
                    let offset = LGraphUtil::calc_port_offset(port, port_side);
                    port_guard.set_property(LayeredOptions::PORT_BORDER_OFFSET, Some(offset));
                }
            }
        }

        if port_constraints.is_ratio_fixed() {
            let ratio = if let Ok(mut port_guard) = port.lock() {
                let node_size = port_guard
                    .node()
                    .and_then(|node| node.lock().ok().map(|mut node| *node.shape().size_ref()))
                    .unwrap_or(KVector::new());
                let pos = *port_guard.shape().position_ref();
                match port_side {
                    PortSide::North | PortSide::South => {
                        if node_size.x > 0.0 {
                            pos.x / node_size.x
                        } else {
                            0.0
                        }
                    }
                    PortSide::East | PortSide::West => {
                        if node_size.y > 0.0 {
                            pos.y / node_size.y
                        } else {
                            0.0
                        }
                    }
                    PortSide::Undefined => 0.0,
                }
            } else {
                0.0
            };

            if let Ok(mut port_guard) = port.lock() {
                port_guard.set_property(InternalProperties::PORT_RATIO_OR_POSITION, Some(ratio));
            }
        }

        if let Ok(mut port_guard) = port.lock() {
            let port_size = *port_guard.shape().size_ref();
            let port_anchor = port_guard.anchor();

            if let Some(anchor_pos) = anchor_pos {
                port_anchor.x = anchor_pos.x;
                port_anchor.y = anchor_pos.y;
                port_guard.set_explicitly_supplied_port_anchor(true);
            } else if port_constraints.is_side_fixed() && port_side != PortSide::Undefined {
                match port_side {
                    PortSide::North => {
                        port_anchor.x = port_size.x / 2.0;
                    }
                    PortSide::East => {
                        port_anchor.x = port_size.x;
                        port_anchor.y = port_size.y / 2.0;
                    }
                    PortSide::South => {
                        port_anchor.x = port_size.x / 2.0;
                        port_anchor.y = port_size.y;
                    }
                    PortSide::West => {
                        port_anchor.y = port_size.y / 2.0;
                    }
                    PortSide::Undefined => {}
                }
            } else {
                port_anchor.x = port_size.x / 2.0;
                port_anchor.y = port_size.y / 2.0;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_external_port_dummy(
        property_holder: &mut MapPropertyHolder,
        port_constraints: PortConstraints,
        port_side: PortSide,
        net_flow: i32,
        port_node_size: &KVector,
        port_position: &KVector,
        port_size: &KVector,
        layout_direction: Direction,
        layered_graph: &LGraphRef,
    ) -> LNodeRef {
        let mut final_external_port_side = port_side;

        let dummy = super::LNode::new(layered_graph);
        if let Ok(mut dummy_guard) = dummy.lock() {
            dummy_guard.set_node_type(NodeType::ExternalPort);
            dummy_guard.set_property(InternalProperties::EXT_PORT_SIZE, Some(*port_size));
            dummy_guard.set_property(
                LayeredOptions::PORT_CONSTRAINTS,
                Some(PortConstraints::FixedPos),
            );
        }

        let port_border_offset = property_holder
            .get_property(LayeredOptions::PORT_BORDER_OFFSET)
            .unwrap_or(0.0);
        if let Ok(mut dummy_guard) = dummy.lock() {
            dummy_guard.set_property(
                LayeredOptions::PORT_BORDER_OFFSET,
                Some(port_border_offset),
            );
        }

        let dummy_port = LPort::new();
        LPort::set_node(&dummy_port, Some(dummy.clone()));

        if !port_constraints.is_side_fixed() {
            if net_flow >= 0 {
                final_external_port_side = PortSide::from_direction(layout_direction);
            } else {
                final_external_port_side = PortSide::from_direction(layout_direction).opposed();
            }
            property_holder.set_property(LayeredOptions::PORT_SIDE, Some(final_external_port_side));
        }

        let mut anchor = if property_holder.has_property(LayeredOptions::PORT_ANCHOR) {
            property_holder
                .get_property(LayeredOptions::PORT_ANCHOR)
                .unwrap_or_default()
        } else {
            KVector::with_values(port_size.x / 2.0, port_size.y / 2.0)
        };
        let explicit_anchor = property_holder.has_property(LayeredOptions::PORT_ANCHOR);

        match final_external_port_side {
            PortSide::West => {
                if let Ok(mut dummy_guard) = dummy.lock() {
                    dummy_guard.set_property(
                        LayeredOptions::LAYERING_LAYER_CONSTRAINT,
                        Some(LayerConstraint::FirstSeparate),
                    );
                    dummy_guard.set_property(
                        InternalProperties::EDGE_CONSTRAINT,
                        Some(EdgeConstraint::OutgoingOnly),
                    );
                    dummy_guard.shape().size().y = port_size.y;
                    if port_border_offset < 0.0 {
                        dummy_guard.shape().size().x = -port_border_offset;
                    }
                }
                if let Ok(mut dummy_port_guard) = dummy_port.lock() {
                    dummy_port_guard.set_side(PortSide::East);
                }
                if !explicit_anchor {
                    anchor.x = port_size.x;
                }
                anchor.x -= port_size.x;
            }
            PortSide::East => {
                if let Ok(mut dummy_guard) = dummy.lock() {
                    dummy_guard.set_property(
                        LayeredOptions::LAYERING_LAYER_CONSTRAINT,
                        Some(LayerConstraint::LastSeparate),
                    );
                    dummy_guard.set_property(
                        InternalProperties::EDGE_CONSTRAINT,
                        Some(EdgeConstraint::IncomingOnly),
                    );
                    dummy_guard.shape().size().y = port_size.y;
                    if port_border_offset < 0.0 {
                        dummy_guard.shape().size().x = -port_border_offset;
                    }
                }
                if let Ok(mut dummy_port_guard) = dummy_port.lock() {
                    dummy_port_guard.set_side(PortSide::West);
                }
                if !explicit_anchor {
                    anchor.x = 0.0;
                }
            }
            PortSide::North => {
                if let Ok(mut dummy_guard) = dummy.lock() {
                    dummy_guard.set_property(
                        InternalProperties::IN_LAYER_CONSTRAINT,
                        Some(InLayerConstraint::Top),
                    );
                    dummy_guard.shape().size().x = port_size.x;
                    if port_border_offset < 0.0 {
                        dummy_guard.shape().size().y = -port_border_offset;
                    }
                }
                if let Ok(mut dummy_port_guard) = dummy_port.lock() {
                    dummy_port_guard.set_side(PortSide::South);
                }
                if !explicit_anchor {
                    anchor.y = port_size.y;
                }
                anchor.y -= port_size.y;
            }
            PortSide::South => {
                if let Ok(mut dummy_guard) = dummy.lock() {
                    dummy_guard.set_property(
                        InternalProperties::IN_LAYER_CONSTRAINT,
                        Some(InLayerConstraint::Bottom),
                    );
                    dummy_guard.shape().size().x = port_size.x;
                    if port_border_offset < 0.0 {
                        dummy_guard.shape().size().y = -port_border_offset;
                    }
                }
                if let Ok(mut dummy_port_guard) = dummy_port.lock() {
                    dummy_port_guard.set_side(PortSide::North);
                }
                if !explicit_anchor {
                    anchor.y = 0.0;
                }
            }
            PortSide::Undefined => {}
        }

        if let Ok(mut dummy_port_guard) = dummy_port.lock() {
            dummy_port_guard.shape().position().set(&anchor);
        }
        if let Ok(mut dummy_guard) = dummy.lock() {
            dummy_guard.set_property(LayeredOptions::PORT_ANCHOR, Some(anchor));
        }

        if port_constraints.is_order_fixed() {
            let mut info_value = 0.0;
            if port_constraints == PortConstraints::FixedOrder
                && property_holder.has_property(LayeredOptions::PORT_INDEX)
            {
                let index = property_holder
                    .get_property(LayeredOptions::PORT_INDEX)
                    .unwrap_or(0) as f64;
                match final_external_port_side {
                    PortSide::North | PortSide::East => info_value = index,
                    PortSide::South | PortSide::West => info_value = -index,
                    PortSide::Undefined => {}
                }
            } else {
                match final_external_port_side {
                    PortSide::West | PortSide::East => {
                        info_value = port_position.y;
                        if port_constraints.is_ratio_fixed() && port_node_size.y > 0.0 {
                            info_value /= port_node_size.y;
                        }
                    }
                    PortSide::North | PortSide::South => {
                        info_value = port_position.x;
                        if port_constraints.is_ratio_fixed() && port_node_size.x > 0.0 {
                            info_value /= port_node_size.x;
                        }
                    }
                    PortSide::Undefined => {}
                }
            }

            if let Ok(mut dummy_guard) = dummy.lock() {
                dummy_guard.set_property(
                    InternalProperties::PORT_RATIO_OR_POSITION,
                    Some(info_value),
                );
            }
        }

        if let Ok(mut dummy_guard) = dummy.lock() {
            dummy_guard.set_property(
                InternalProperties::EXT_PORT_SIDE,
                Some(final_external_port_side),
            );
        }

        dummy
    }

    pub fn get_external_port_position(
        graph: &LGraphRef,
        port_dummy: &LNodeRef,
        port_width: f64,
        port_height: f64,
    ) -> KVector {
        let (mut port_pos, port_offset, _, ext_side) = if let Ok(mut dummy_guard) =
            port_dummy.lock()
        {
            let pos = *dummy_guard.shape().position_ref();
            let size = *dummy_guard.shape().size_ref();
            let mut port_pos = KVector::from_vector(&pos);
            port_pos.x += size.x / 2.0;
            port_pos.y += size.y / 2.0;
            (
                port_pos,
                dummy_guard
                    .get_property(LayeredOptions::PORT_BORDER_OFFSET)
                    .unwrap_or(0.0),
                size,
                dummy_guard
                    .get_property(InternalProperties::EXT_PORT_SIDE)
                    .unwrap_or(PortSide::Undefined),
            )
        } else {
            return KVector::new();
        };

        let (graph_size, padding, graph_offset) = if let Ok(graph_guard) = graph.lock() {
            (
                *graph_guard.size_ref(),
                graph_guard.padding_ref().clone(),
                *graph_guard.offset_ref(),
            )
        } else {
            return port_pos;
        };

        match ext_side {
            PortSide::North => {
                port_pos.x += padding.left + graph_offset.x - (port_width / 2.0);
                port_pos.y = -port_height - port_offset;
                if let Ok(mut dummy_guard) = port_dummy.lock() {
                    dummy_guard.shape().position().y =
                        -(padding.top + port_offset + graph_offset.y);
                }
            }
            PortSide::East => {
                port_pos.x = graph_size.x + padding.left + padding.right + port_offset;
                port_pos.y += padding.top + graph_offset.y - (port_height / 2.0);
                if let Ok(mut dummy_guard) = port_dummy.lock() {
                    dummy_guard.shape().position().x =
                        graph_size.x + padding.right + port_offset - graph_offset.x;
                }
            }
            PortSide::South => {
                port_pos.x += padding.left + graph_offset.x - (port_width / 2.0);
                port_pos.y = graph_size.y + padding.top + padding.bottom + port_offset;
                if let Ok(mut dummy_guard) = port_dummy.lock() {
                    dummy_guard.shape().position().y =
                        graph_size.y + padding.bottom + port_offset - graph_offset.y;
                }
            }
            PortSide::West => {
                port_pos.x = -port_width - port_offset;
                port_pos.y += padding.top + graph_offset.y - (port_height / 2.0);
                if let Ok(mut dummy_guard) = port_dummy.lock() {
                    dummy_guard.shape().position().x =
                        -(padding.left + port_offset + graph_offset.x);
                }
            }
            PortSide::Undefined => {}
        }

        port_pos
    }

    pub fn is_descendant(child: &LNodeRef, parent: &LNodeRef) -> bool {
        let mut current = child.clone();
        loop {
            let next = current
                .lock()
                .ok()
                .and_then(|node| node.graph())
                .and_then(|graph| graph.lock().ok().and_then(|graph| graph.parent_node()));

            let next = match next {
                Some(next) => next,
                None => return false,
            };

            if Arc::ptr_eq(&next, parent) {
                return true;
            }
            current = next;
        }
    }

    pub fn change_coord_system(point: &mut KVector, old_graph: &LGraphRef, new_graph: &LGraphRef) {
        if Arc::ptr_eq(old_graph, new_graph) {
            return;
        }

        let mut graph = old_graph.clone();
        loop {
            let (offset, padding, parent_node) = if let Ok(graph_guard) = graph.lock() {
                (
                    *graph_guard.offset_ref(),
                    graph_guard.padding_ref().clone(),
                    graph_guard.parent_node(),
                )
            } else {
                return;
            };

            point.add(&offset);
            if let Some(node) = parent_node {
                point.add_values(padding.left, padding.top);
                if let Ok(mut node_guard) = node.lock() {
                    let pos = *node_guard.shape().position_ref();
                    point.add(&pos);
                    if let Some(parent_graph) = node_guard.graph() {
                        graph = parent_graph;
                        continue;
                    }
                }
            }
            break;
        }

        graph = new_graph.clone();
        loop {
            let (offset, padding, parent_node) = if let Ok(graph_guard) = graph.lock() {
                (
                    *graph_guard.offset_ref(),
                    graph_guard.padding_ref().clone(),
                    graph_guard.parent_node(),
                )
            } else {
                return;
            };

            point.sub(&offset);
            if let Some(node) = parent_node {
                point.sub_values(padding.left, padding.top);
                if let Ok(mut node_guard) = node.lock() {
                    let pos = *node_guard.shape().position_ref();
                    point.sub(&pos);
                    if let Some(parent_graph) = node_guard.graph() {
                        graph = parent_graph;
                        continue;
                    }
                }
            }
            break;
        }
    }

    pub fn get_individual_or_inherited<T: Clone + Send + Sync + 'static>(
        node: &LNodeRef,
        property: &Property<T>,
    ) -> Option<T> {
        let (mut result, graph) = if let Ok(mut node_guard) = node.lock() {
            let has_individual = node_guard
                .shape()
                .graph_element()
                .properties()
                .has_property(CoreOptions::SPACING_INDIVIDUAL);
            let mut value = None;
            if has_individual {
                if let Some(mut individual) =
                    node_guard.get_property(CoreOptions::SPACING_INDIVIDUAL)
                {
                    let has_prop = individual.properties().has_property(property);
                    if has_prop {
                        value = individual.properties_mut().get_property(property);
                    }
                }
            }
            (value, node_guard.graph())
        } else {
            (None, None)
        };

        if result.is_none() {
            if let Some(graph) = graph {
                if let Ok(mut graph_guard) = graph.lock() {
                    result = graph_guard.get_property(property);
                }
            }
        }

        result
    }

    pub fn get_direction(graph: &LGraphRef) -> Direction {
        let (direction, aspect_ratio) = if let Ok(mut graph_guard) = graph.lock() {
            (
                graph_guard
                    .get_property(LayeredOptions::DIRECTION)
                    .unwrap_or(Direction::Undefined),
                graph_guard
                    .get_property(LayeredOptions::ASPECT_RATIO)
                    .unwrap_or(1.0),
            )
        } else {
            return Direction::Undefined;
        };

        if direction == Direction::Undefined {
            if aspect_ratio >= 1.0 {
                Direction::Right
            } else {
                Direction::Down
            }
        } else {
            direction
        }
    }

    pub fn get_minimal_model_order(graph: &LGraphRef) -> i32 {
        let nodes = graph
            .lock()
            .map(|graph_guard| graph_guard.layerless_nodes().clone())
            .unwrap_or_default();

        let mut order = i32::MAX;
        for node in nodes {
            if let Ok(mut node_guard) = node.lock() {
                if let Some(value) = node_guard.get_property(InternalProperties::MODEL_ORDER) {
                    if value < order {
                        order = value;
                    }
                }
            }
        }

        order
    }
}
