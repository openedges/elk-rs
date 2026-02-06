use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::elk_padding::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::alignment::Alignment;
use org_eclipse_elk_core::org::eclipse::elk::core::options::node_label_placement::NodeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, IElkProgressMonitor};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::alg::layered::graph::l_node::NodeType;
use crate::org::eclipse::elk::alg::layered::graph::{
    LEdgeRef, LGraph, LLabelRef, LNodeRef, LPadding, LPortRef, LShape,
};
use crate::org::eclipse::elk::alg::layered::options::{
    DirectionCongruency, InLayerConstraint, InternalProperties, LayerConstraint, LayeredOptions,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    ToInternalLtr,
    ToInputDirection,
}

pub struct GraphTransformer {
    mode: Mode,
}

impl GraphTransformer {
    pub fn new(mode: Mode) -> Self {
        GraphTransformer { mode }
    }

    fn rotate90_clockwise(&self, graph: &mut LGraph, nodes: &[LNodeRef]) {
        self.transpose_all(graph, nodes);
        self.mirror_all_x(graph, nodes);
    }

    fn rotate90_counter_clockwise(&self, graph: &mut LGraph, nodes: &[LNodeRef]) {
        self.mirror_all_x(graph, nodes);
        self.transpose_all(graph, nodes);
    }

    fn mirror_all_x(&self, graph: &mut LGraph, nodes: &[LNodeRef]) {
        self.mirror_x(nodes, graph);
        {
            let padding = graph.padding();
            mirror_padding_x(padding);
        }
        if let Some(mut padding) = graph.get_property(LayeredOptions::NODE_LABELS_PADDING) {
            mirror_elk_padding_x(&mut padding);
            graph.set_property(LayeredOptions::NODE_LABELS_PADDING, Some(padding));
        }
    }

    fn mirror_all_y(&self, graph: &mut LGraph, nodes: &[LNodeRef]) {
        self.mirror_y(nodes, graph);
        {
            let padding = graph.padding();
            mirror_padding_y(padding);
        }
        if let Some(mut padding) = graph.get_property(LayeredOptions::NODE_LABELS_PADDING) {
            mirror_elk_padding_y(&mut padding);
            graph.set_property(LayeredOptions::NODE_LABELS_PADDING, Some(padding));
        }
    }

    fn transpose_all(&self, graph: &mut LGraph, nodes: &[LNodeRef]) {
        self.transpose(nodes);
        self.transpose_edge_label_placement(graph);
        {
            let offset = graph.offset();
            transpose_vector(offset);
        }
        {
            let size = graph.size();
            transpose_vector(size);
        }
        {
            let padding = graph.padding();
            transpose_padding(padding);
        }
        if let Some(mut padding) = graph.get_property(LayeredOptions::NODE_LABELS_PADDING) {
            transpose_elk_padding(&mut padding);
            graph.set_property(LayeredOptions::NODE_LABELS_PADDING, Some(padding));
        }
    }

    fn mirror_x(&self, nodes: &[LNodeRef], graph: &LGraph) {
        let mut offset: f64 = 0.0;
        if graph.size_ref().x == 0.0 {
            for node in nodes {
                if let Ok(mut node_guard) = node.lock() {
                    let pos = *node_guard.shape().position_ref();
                    let size = *node_guard.shape().size_ref();
                    let margin = node_guard.margin();
                    offset = offset.max(pos.x + size.x + margin.right);
                }
            }
        } else {
            offset = graph.size_ref().x - graph.offset_ref().x;
        }
        offset -= graph.offset_ref().x;

        for node in nodes {
            self.mirror_node_x(node, offset);
        }
    }

    fn mirror_y(&self, nodes: &[LNodeRef], graph: &LGraph) {
        let mut offset: f64 = 0.0;
        if graph.size_ref().y == 0.0 {
            for node in nodes {
                if let Ok(mut node_guard) = node.lock() {
                    let pos = *node_guard.shape().position_ref();
                    let size = *node_guard.shape().size_ref();
                    let margin = node_guard.margin();
                    offset = offset.max(pos.y + size.y + margin.bottom);
                }
            }
        } else {
            offset = graph.size_ref().y - graph.offset_ref().y;
        }
        offset -= graph.offset_ref().y;

        for node in nodes {
            self.mirror_node_y(node, offset);
        }
    }

    fn mirror_node_x(&self, node: &LNodeRef, offset: f64) {
        let (node_size, node_type, ports, labels) = {
            let mut node_guard = match node.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };

            let (node_size, has_position) = {
                let shape = node_guard.shape();
                let size = *shape.size_ref();
                mirror_vector_x(shape.position(), offset - size.x);
                self.mirror_node_label_placement_x(shape);
                let has_position = shape_has_property(shape, LayeredOptions::POSITION);
                (size, has_position)
            };

            {
                let padding = node_guard.padding();
                mirror_padding_x(padding);
            }

            if has_position {
                if let Some(mut pos) = node_guard.get_property(LayeredOptions::POSITION) {
                    mirror_vector_x(&mut pos, offset - node_size.x);
                    node_guard.set_property(LayeredOptions::POSITION, Some(pos));
                }
            }

            let alignment = node_guard
                .get_property(LayeredOptions::ALIGNMENT)
                .unwrap_or(Alignment::Center);
            let new_alignment = match alignment {
                Alignment::Left => Alignment::Right,
                Alignment::Right => Alignment::Left,
                _ => alignment,
            };
            if new_alignment != alignment {
                node_guard.set_property(LayeredOptions::ALIGNMENT, Some(new_alignment));
            }

            let ports = node_guard.ports().clone();
            let labels = node_guard.labels().clone();
            (node_size, node_guard.node_type(), ports, labels)
        };

        for port in ports {
            self.mirror_port_x(&port, offset, node_size.x);
        }

        if node_type == NodeType::ExternalPort {
            self.mirror_external_port_side_x(node);
            self.mirror_layer_constraint_x(node);
        }

        for label in labels {
            self.mirror_node_label_x(&label, node_size.x);
        }
    }

    fn mirror_node_y(&self, node: &LNodeRef, offset: f64) {
        let (node_size, node_type, ports, labels) = {
            let mut node_guard = match node.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };

            let (node_size, has_position) = {
                let shape = node_guard.shape();
                let size = *shape.size_ref();
                mirror_vector_y(shape.position(), offset - size.y);
                self.mirror_node_label_placement_y(shape);
                let has_position = shape_has_property(shape, LayeredOptions::POSITION);
                (size, has_position)
            };

            {
                let padding = node_guard.padding();
                mirror_padding_y(padding);
            }

            if has_position {
                if let Some(mut pos) = node_guard.get_property(LayeredOptions::POSITION) {
                    mirror_vector_y(&mut pos, offset - node_size.y);
                    node_guard.set_property(LayeredOptions::POSITION, Some(pos));
                }
            }

            let alignment = node_guard
                .get_property(LayeredOptions::ALIGNMENT)
                .unwrap_or(Alignment::Center);
            let new_alignment = match alignment {
                Alignment::Top => Alignment::Bottom,
                Alignment::Bottom => Alignment::Top,
                _ => alignment,
            };
            if new_alignment != alignment {
                node_guard.set_property(LayeredOptions::ALIGNMENT, Some(new_alignment));
            }

            let ports = node_guard.ports().clone();
            let labels = node_guard.labels().clone();
            (node_size, node_guard.node_type(), ports, labels)
        };

        for port in ports {
            self.mirror_port_y(&port, offset, node_size.y);
        }

        if node_type == NodeType::ExternalPort {
            self.mirror_external_port_side_y(node);
            self.mirror_in_layer_constraint_y(node);
        }

        for label in labels {
            self.mirror_node_label_y(&label, node_size.y);
        }
    }

    fn mirror_port_x(&self, port: &LPortRef, offset: f64, node_size_x: f64) {
        let (port_size, labels, edges) = {
            let mut port_guard = match port.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };

            let port_size = {
                let shape = port_guard.shape();
                let size = *shape.size_ref();
                mirror_vector_x(shape.position(), node_size_x - size.x);
                size
            };

            mirror_vector_x(port_guard.anchor(), port_size.x);
            let side = port_guard.side();
            port_guard.set_side(get_mirrored_port_side_x(side));
            reverse_index(&mut port_guard);

            let labels = port_guard.labels().clone();
            let edges = port_guard.outgoing_edges().clone();
            (port_size, labels, edges)
        };

        for edge in edges {
            self.mirror_edge_x(&edge, offset);
        }

        for label in labels {
            mirror_label_x(&label, port_size.x);
        }
    }

    fn mirror_port_y(&self, port: &LPortRef, offset: f64, node_size_y: f64) {
        let (port_size, labels, edges) = {
            let mut port_guard = match port.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };

            let port_size = {
                let shape = port_guard.shape();
                let size = *shape.size_ref();
                mirror_vector_y(shape.position(), node_size_y - size.y);
                size
            };

            mirror_vector_y(port_guard.anchor(), port_size.y);
            let side = port_guard.side();
            port_guard.set_side(get_mirrored_port_side_y(side));
            reverse_index(&mut port_guard);

            let labels = port_guard.labels().clone();
            let edges = port_guard.outgoing_edges().clone();
            (port_size, labels, edges)
        };

        for edge in edges {
            self.mirror_edge_y(&edge, offset);
        }

        for label in labels {
            mirror_label_y(&label, port_size.y);
        }
    }

    fn mirror_edge_x(&self, edge: &LEdgeRef, offset: f64) {
        let labels = {
            let mut edge_guard = match edge.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };

            for bend_point in edge_guard.bend_points().iter_mut() {
                mirror_vector_x(bend_point, offset);
            }

            let has_junction_points = edge_guard
                .graph_element()
                .properties()
                .has_property(LayeredOptions::JUNCTION_POINTS);

            if has_junction_points {
                if let Some(mut junction_points) =
                    edge_guard.get_property(LayeredOptions::JUNCTION_POINTS)
                {
                    for jp in junction_points.iter_mut() {
                        mirror_vector_x(jp, offset);
                    }
                    edge_guard.set_property(
                        LayeredOptions::JUNCTION_POINTS,
                        Some(junction_points),
                    );
                }
            }

            edge_guard.labels().clone()
        };

        for label in labels {
            mirror_label_x(&label, offset);
        }
    }

    fn mirror_edge_y(&self, edge: &LEdgeRef, offset: f64) {
        let labels = {
            let mut edge_guard = match edge.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };

            for bend_point in edge_guard.bend_points().iter_mut() {
                mirror_vector_y(bend_point, offset);
            }

            let has_junction_points = edge_guard
                .graph_element()
                .properties()
                .has_property(LayeredOptions::JUNCTION_POINTS);

            if has_junction_points {
                if let Some(mut junction_points) =
                    edge_guard.get_property(LayeredOptions::JUNCTION_POINTS)
                {
                    for jp in junction_points.iter_mut() {
                        mirror_vector_y(jp, offset);
                    }
                    edge_guard.set_property(
                        LayeredOptions::JUNCTION_POINTS,
                        Some(junction_points),
                    );
                }
            }

            edge_guard.labels().clone()
        };

        for label in labels {
            mirror_label_y(&label, offset);
        }
    }

    fn mirror_node_label_x(&self, label: &LLabelRef, node_size_x: f64) {
        if let Ok(mut label_guard) = label.lock() {
            let shape = label_guard.shape();
            self.mirror_node_label_placement_x(shape);
            let size = *shape.size_ref();
            mirror_vector_x(shape.position(), node_size_x - size.x);
        }
    }

    fn mirror_node_label_y(&self, label: &LLabelRef, node_size_y: f64) {
        if let Ok(mut label_guard) = label.lock() {
            let shape = label_guard.shape();
            self.mirror_node_label_placement_y(shape);
            let size = *shape.size_ref();
            mirror_vector_y(shape.position(), node_size_y - size.y);
        }
    }

    fn transpose(&self, nodes: &[LNodeRef]) {
        for node in nodes {
            self.transpose_node(node);
        }
    }

    fn transpose_node(&self, node: &LNodeRef) {
        let (node_type, ports, labels) = {
            let mut node_guard = match node.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };

            {
                let shape = node_guard.shape();
                transpose_vector(shape.position());
                transpose_vector(shape.size());
                self.transpose_node_label_placement(shape);
            }

            {
                let padding = node_guard.padding();
                transpose_padding(padding);
            }

            self.transpose_properties(&mut node_guard);

            let ports = node_guard.ports().clone();
            let labels = node_guard.labels().clone();
            (node_guard.node_type(), ports, labels)
        };

        for port in ports {
            self.transpose_port(&port);
        }

        if node_type == NodeType::ExternalPort {
            self.transpose_external_port_side(node);
            self.transpose_layer_constraint(node);
        }

        for label in labels {
            if let Ok(mut label_guard) = label.lock() {
                let shape = label_guard.shape();
                self.transpose_node_label_placement(shape);
                transpose_vector(shape.size());
                transpose_vector(shape.position());
            }
        }
    }

    fn transpose_port(&self, port: &LPortRef) {
        let (labels, edges) = {
            let mut port_guard = match port.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };

            {
                let shape = port_guard.shape();
                transpose_vector(shape.position());
                transpose_vector(shape.size());
            }
            transpose_vector(port_guard.anchor());
            let side = port_guard.side();
            port_guard.set_side(transpose_port_side(side));
            reverse_index(&mut port_guard);

            let labels = port_guard.labels().clone();
            let edges = port_guard.outgoing_edges().clone();
            (labels, edges)
        };

        for edge in edges {
            self.transpose_edge(&edge);
        }

        for label in labels {
            transpose_label(&label);
        }
    }

    fn transpose_edge(&self, edge: &LEdgeRef) {
        let labels = {
            let mut edge_guard = match edge.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };

            for bend_point in edge_guard.bend_points().iter_mut() {
                transpose_vector(bend_point);
            }

            let has_junction_points = edge_guard
                .graph_element()
                .properties()
                .has_property(LayeredOptions::JUNCTION_POINTS);

            if has_junction_points {
                if let Some(mut junction_points) =
                    edge_guard.get_property(LayeredOptions::JUNCTION_POINTS)
                {
                    for jp in junction_points.iter_mut() {
                        transpose_vector(jp);
                    }
                    edge_guard.set_property(
                        LayeredOptions::JUNCTION_POINTS,
                        Some(junction_points),
                    );
                }
            }

            edge_guard.labels().clone()
        };

        for label in labels {
            transpose_label(&label);
        }
    }

    fn transpose_edge_label_placement(&self, graph: &mut LGraph) {
        if let Some(old_side) = graph.get_property(LayeredOptions::EDGE_LABELS_SIDE_SELECTION) {
            graph.set_property(
                LayeredOptions::EDGE_LABELS_SIDE_SELECTION,
                Some(old_side.transpose()),
            );
        }
    }

    fn transpose_external_port_side(&self, node: &LNodeRef) {
        if let Ok(mut node_guard) = node.lock() {
            if let Some(side) = node_guard.get_property(InternalProperties::EXT_PORT_SIDE) {
                node_guard.set_property(
                    InternalProperties::EXT_PORT_SIDE,
                    Some(transpose_port_side(side)),
                );
            }
        }
    }

    fn mirror_external_port_side_x(&self, node: &LNodeRef) {
        if let Ok(mut node_guard) = node.lock() {
            if let Some(side) = node_guard.get_property(InternalProperties::EXT_PORT_SIDE) {
                node_guard
                    .set_property(InternalProperties::EXT_PORT_SIDE, Some(get_mirrored_port_side_x(side)));
            }
        }
    }

    fn mirror_external_port_side_y(&self, node: &LNodeRef) {
        if let Ok(mut node_guard) = node.lock() {
            if let Some(side) = node_guard.get_property(InternalProperties::EXT_PORT_SIDE) {
                node_guard
                    .set_property(InternalProperties::EXT_PORT_SIDE, Some(get_mirrored_port_side_y(side)));
            }
        }
    }

    fn mirror_layer_constraint_x(&self, node: &LNodeRef) {
        if let Ok(mut node_guard) = node.lock() {
            let constraint = node_guard
                .get_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT)
                .unwrap_or(LayerConstraint::None);
            let new_constraint = match constraint {
                LayerConstraint::First => LayerConstraint::Last,
                LayerConstraint::FirstSeparate => LayerConstraint::LastSeparate,
                LayerConstraint::Last => LayerConstraint::First,
                LayerConstraint::LastSeparate => LayerConstraint::FirstSeparate,
                _ => constraint,
            };
            if new_constraint != constraint {
                node_guard.set_property(
                    LayeredOptions::LAYERING_LAYER_CONSTRAINT,
                    Some(new_constraint),
                );
            }
        }
    }

    fn mirror_in_layer_constraint_y(&self, node: &LNodeRef) {
        if let Ok(mut node_guard) = node.lock() {
            let constraint = node_guard
                .get_property(InternalProperties::IN_LAYER_CONSTRAINT)
                .unwrap_or(InLayerConstraint::None);
            let new_constraint = match constraint {
                InLayerConstraint::Top => InLayerConstraint::Bottom,
                InLayerConstraint::Bottom => InLayerConstraint::Top,
                _ => constraint,
            };
            if new_constraint != constraint {
                node_guard.set_property(
                    InternalProperties::IN_LAYER_CONSTRAINT,
                    Some(new_constraint),
                );
            }
        }
    }

    fn transpose_layer_constraint(&self, node: &LNodeRef) {
        if let Ok(mut node_guard) = node.lock() {
            let layer_constraint = node_guard
                .get_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT)
                .unwrap_or(LayerConstraint::None);
            let in_layer_constraint = node_guard
                .get_property(InternalProperties::IN_LAYER_CONSTRAINT)
                .unwrap_or(InLayerConstraint::None);

            if layer_constraint == LayerConstraint::FirstSeparate {
                node_guard.set_property(
                    LayeredOptions::LAYERING_LAYER_CONSTRAINT,
                    Some(LayerConstraint::None),
                );
                node_guard.set_property(
                    InternalProperties::IN_LAYER_CONSTRAINT,
                    Some(InLayerConstraint::Top),
                );
            } else if layer_constraint == LayerConstraint::LastSeparate {
                node_guard.set_property(
                    LayeredOptions::LAYERING_LAYER_CONSTRAINT,
                    Some(LayerConstraint::None),
                );
                node_guard.set_property(
                    InternalProperties::IN_LAYER_CONSTRAINT,
                    Some(InLayerConstraint::Bottom),
                );
            } else if in_layer_constraint == InLayerConstraint::Top {
                node_guard.set_property(
                    LayeredOptions::LAYERING_LAYER_CONSTRAINT,
                    Some(LayerConstraint::FirstSeparate),
                );
                node_guard.set_property(
                    InternalProperties::IN_LAYER_CONSTRAINT,
                    Some(InLayerConstraint::None),
                );
            } else if in_layer_constraint == InLayerConstraint::Bottom {
                node_guard.set_property(
                    LayeredOptions::LAYERING_LAYER_CONSTRAINT,
                    Some(LayerConstraint::LastSeparate),
                );
                node_guard.set_property(
                    InternalProperties::IN_LAYER_CONSTRAINT,
                    Some(InLayerConstraint::None),
                );
            }
        }
    }

    fn transpose_properties(&self, node: &mut crate::org::eclipse::elk::alg::layered::graph::LNode) {
        let min_size = node
            .get_property(LayeredOptions::NODE_SIZE_MINIMUM)
            .unwrap_or_default();
        node.set_property(
            LayeredOptions::NODE_SIZE_MINIMUM,
            Some(KVector::with_values(min_size.y, min_size.x)),
        );

        let alignment = node
            .get_property(LayeredOptions::ALIGNMENT)
            .unwrap_or(Alignment::Center);
        let new_alignment = match alignment {
            Alignment::Left => Alignment::Top,
            Alignment::Right => Alignment::Bottom,
            Alignment::Top => Alignment::Left,
            Alignment::Bottom => Alignment::Right,
            _ => alignment,
        };
        if new_alignment != alignment {
            node.set_property(LayeredOptions::ALIGNMENT, Some(new_alignment));
        }

        let has_position = {
            let shape = node.shape();
            shape_has_property(shape, LayeredOptions::POSITION)
        };
        if has_position {
            if let Some(mut pos) = node.get_property(LayeredOptions::POSITION) {
                transpose_vector(&mut pos);
                node.set_property(LayeredOptions::POSITION, Some(pos));
            }
        }
    }

    fn mirror_node_label_placement_x(&self, shape: &mut LShape) {
        if !shape_has_property(shape, LayeredOptions::NODE_LABELS_PLACEMENT) {
            return;
        }

        let mut placement = shape
            .get_property(LayeredOptions::NODE_LABELS_PLACEMENT)
            .unwrap_or_default();

        if placement.contains(&NodeLabelPlacement::HLeft) {
            placement.remove(&NodeLabelPlacement::HLeft);
            placement.insert(NodeLabelPlacement::HRight);
        } else if placement.contains(&NodeLabelPlacement::HRight) {
            placement.remove(&NodeLabelPlacement::HRight);
            placement.insert(NodeLabelPlacement::HLeft);
        }

        shape.set_property(LayeredOptions::NODE_LABELS_PLACEMENT, Some(placement));
    }

    fn mirror_node_label_placement_y(&self, shape: &mut LShape) {
        if !shape_has_property(shape, LayeredOptions::NODE_LABELS_PLACEMENT) {
            return;
        }

        let mut placement = shape
            .get_property(LayeredOptions::NODE_LABELS_PLACEMENT)
            .unwrap_or_default();

        if placement.contains(&NodeLabelPlacement::VTop) {
            placement.remove(&NodeLabelPlacement::VTop);
            placement.insert(NodeLabelPlacement::VBottom);
        } else if placement.contains(&NodeLabelPlacement::VBottom) {
            placement.remove(&NodeLabelPlacement::VBottom);
            placement.insert(NodeLabelPlacement::VTop);
        }

        shape.set_property(LayeredOptions::NODE_LABELS_PLACEMENT, Some(placement));
    }

    fn transpose_node_label_placement(&self, shape: &mut LShape) {
        if !shape_has_property(shape, LayeredOptions::NODE_LABELS_PLACEMENT) {
            return;
        }

        let placement = shape
            .get_property(LayeredOptions::NODE_LABELS_PLACEMENT)
            .unwrap_or_default();
        if placement.is_empty() {
            return;
        }

        let mut new_placement = EnumSet::none_of();

        if placement.contains(&NodeLabelPlacement::Inside) {
            new_placement.insert(NodeLabelPlacement::Inside);
        } else {
            new_placement.insert(NodeLabelPlacement::Outside);
        }

        if !placement.contains(&NodeLabelPlacement::HPriority) {
            new_placement.insert(NodeLabelPlacement::HPriority);
        }

        if placement.contains(&NodeLabelPlacement::HLeft) {
            new_placement.insert(NodeLabelPlacement::VTop);
        } else if placement.contains(&NodeLabelPlacement::HCenter) {
            new_placement.insert(NodeLabelPlacement::VCenter);
        } else if placement.contains(&NodeLabelPlacement::HRight) {
            new_placement.insert(NodeLabelPlacement::VBottom);
        }

        if placement.contains(&NodeLabelPlacement::VTop) {
            new_placement.insert(NodeLabelPlacement::HLeft);
        } else if placement.contains(&NodeLabelPlacement::VCenter) {
            new_placement.insert(NodeLabelPlacement::HCenter);
        } else if placement.contains(&NodeLabelPlacement::VBottom) {
            new_placement.insert(NodeLabelPlacement::HRight);
        }

        shape.set_property(LayeredOptions::NODE_LABELS_PLACEMENT, Some(new_placement));
    }
}

impl ILayoutProcessor<LGraph> for GraphTransformer {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin(&format!("Graph transformation ({:?})", self.mode), 1.0);

        let mut nodes = layered_graph.layerless_nodes().clone();
        for layer in layered_graph.layers() {
            if let Ok(layer_guard) = layer.lock() {
                nodes.extend(layer_guard.nodes().clone());
            }
        }

        let congruency = layered_graph
            .get_property(LayeredOptions::DIRECTION_CONGRUENCY)
            .unwrap_or(DirectionCongruency::ReadingDirection);
        let direction = layered_graph
            .get_property(LayeredOptions::DIRECTION)
            .unwrap_or(org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction::Right);

        if congruency == DirectionCongruency::ReadingDirection {
            match direction {
                org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction::Left => {
                    self.mirror_all_x(layered_graph, &nodes);
                }
                org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction::Down => {
                    self.transpose_all(layered_graph, &nodes);
                }
                org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction::Up => {
                    match self.mode {
                        Mode::ToInternalLtr => {
                            self.transpose_all(layered_graph, &nodes);
                            self.mirror_all_y(layered_graph, &nodes);
                        }
                        Mode::ToInputDirection => {
                            self.mirror_all_y(layered_graph, &nodes);
                            self.transpose_all(layered_graph, &nodes);
                        }
                    }
                }
                _ => {}
            }
        } else {
            match self.mode {
                Mode::ToInternalLtr => match direction {
                    org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction::Left => {
                        self.mirror_all_x(layered_graph, &nodes);
                        self.mirror_all_y(layered_graph, &nodes);
                    }
                    org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction::Down => {
                        self.rotate90_clockwise(layered_graph, &nodes);
                    }
                    org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction::Up => {
                        self.rotate90_counter_clockwise(layered_graph, &nodes);
                    }
                    _ => {}
                },
                Mode::ToInputDirection => match direction {
                    org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction::Left => {
                        self.mirror_all_x(layered_graph, &nodes);
                        self.mirror_all_y(layered_graph, &nodes);
                    }
                    org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction::Down => {
                        self.rotate90_counter_clockwise(layered_graph, &nodes);
                    }
                    org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction::Up => {
                        self.rotate90_clockwise(layered_graph, &nodes);
                    }
                    _ => {}
                },
            }
        }

        monitor.done();
    }
}

fn mirror_vector_x(vector: &mut KVector, offset: f64) {
    vector.x = offset - vector.x;
}

fn mirror_vector_y(vector: &mut KVector, offset: f64) {
    vector.y = offset - vector.y;
}

fn transpose_vector(vector: &mut KVector) {
    std::mem::swap(&mut vector.x, &mut vector.y);
}

fn mirror_padding_x(padding: &mut LPadding) {
    let old_left = padding.left;
    let old_right = padding.right;
    padding.left = old_right;
    padding.right = old_left;
}

fn mirror_padding_y(padding: &mut LPadding) {
    let old_top = padding.top;
    let old_bottom = padding.bottom;
    padding.top = old_bottom;
    padding.bottom = old_top;
}

fn transpose_padding(padding: &mut LPadding) {
    let old_top = padding.top;
    let old_bottom = padding.bottom;
    let old_left = padding.left;
    let old_right = padding.right;

    padding.top = old_left;
    padding.bottom = old_right;
    padding.left = old_top;
    padding.right = old_bottom;
}

fn mirror_elk_padding_x(padding: &mut ElkPadding) {
    let old_left = padding.left;
    let old_right = padding.right;
    padding.left = old_right;
    padding.right = old_left;
}

fn mirror_elk_padding_y(padding: &mut ElkPadding) {
    let old_top = padding.top;
    let old_bottom = padding.bottom;
    padding.top = old_bottom;
    padding.bottom = old_top;
}

fn transpose_elk_padding(padding: &mut ElkPadding) {
    let old_top = padding.top;
    let old_bottom = padding.bottom;
    let old_left = padding.left;
    let old_right = padding.right;

    padding.top = old_left;
    padding.bottom = old_right;
    padding.left = old_top;
    padding.right = old_bottom;
}

fn shape_has_property<T: Clone + Send + Sync + 'static>(
    shape: &mut LShape,
    property: &Property<T>,
) -> bool {
    shape.graph_element().properties().has_property(property)
}

fn get_mirrored_port_side_x(side: PortSide) -> PortSide {
    match side {
        PortSide::East => PortSide::West,
        PortSide::West => PortSide::East,
        _ => side,
    }
}

fn get_mirrored_port_side_y(side: PortSide) -> PortSide {
    match side {
        PortSide::North => PortSide::South,
        PortSide::South => PortSide::North,
        _ => side,
    }
}

fn transpose_port_side(side: PortSide) -> PortSide {
    match side {
        PortSide::North => PortSide::West,
        PortSide::West => PortSide::North,
        PortSide::South => PortSide::East,
        PortSide::East => PortSide::South,
        _ => PortSide::Undefined,
    }
}

fn reverse_index(port: &mut crate::org::eclipse::elk::alg::layered::graph::LPort) {
    if let Some(index) = port.get_property(LayeredOptions::PORT_INDEX) {
        port.set_property(LayeredOptions::PORT_INDEX, Some(-index));
    }
}

fn mirror_label_x(label: &LLabelRef, base_offset: f64) {
    if let Ok(mut label_guard) = label.lock() {
        let shape = label_guard.shape();
        let size = *shape.size_ref();
        mirror_vector_x(shape.position(), base_offset - size.x);
    }
}

fn mirror_label_y(label: &LLabelRef, base_offset: f64) {
    if let Ok(mut label_guard) = label.lock() {
        let shape = label_guard.shape();
        let size = *shape.size_ref();
        mirror_vector_y(shape.position(), base_offset - size.y);
    }
}

fn transpose_label(label: &LLabelRef) {
    if let Ok(mut label_guard) = label.lock() {
        let shape = label_guard.shape();
        transpose_vector(shape.position());
        transpose_vector(shape.size());
    }
}
