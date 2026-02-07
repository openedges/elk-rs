use std::any::Any;
use std::cmp::Ordering;

use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, PortConstraints, PortLabelPlacement, PortSide, SizeConstraint,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{
    ElkGraphAdapter, ElkLabelAdapter, ElkNodeAdapter, ElkPortAdapter, GraphAdapter,
    GraphElementAdapter, NodeAdapter, PortAdapter,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkEdgeRef, ElkNodeRef, ElkPortRef};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use std::rc::Rc;

use super::node_margin_calculator::NodeMarginCalculator;

pub struct NodeDimensionCalculation;

#[derive(Clone, Copy)]
struct HorizontalStackConfig {
    positive_direction: bool,
    gap_x: f64,
    gap_y: f64,
    start_coordinate: Option<f64>,
    clamp_x: Option<(f64, f64)>,
}

impl NodeDimensionCalculation {
    pub fn calculate_label_and_node_sizes<T, G>(adapter: &G)
    where
        T: 'static,
        G: GraphAdapter<T> + Any,
    {
        // Keep Java-style behavior for ELK adapters, and provide a best-effort generic fallback.
        if let Some(elk_adapter) = (adapter as &dyn Any).downcast_ref::<ElkGraphAdapter>() {
            Self::calculate_label_and_node_sizes_for_elk(elk_adapter);
            return;
        }

        Self::calculate_label_and_node_sizes_generic(adapter);
    }

    pub fn calculate_label_and_node_sizes_for_elk(adapter: &ElkGraphAdapter) {
        for node in adapter.get_nodes() {
            let placement = node
                .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                .unwrap_or_default();
            if PortLabelPlacement::is_fixed(&placement) {
                continue;
            }

            let constrained_placement = Self::has_constrained_port_label_placement(&node);
            let inside_label_placement = placement.contains(&PortLabelPlacement::Inside);
            let next_to_port_if_possible =
                placement.contains(&PortLabelPlacement::NextToPortIfPossible);
            let always_same_side = placement.contains(&PortLabelPlacement::AlwaysSameSide);
            let always_other_same_side = placement.contains(&PortLabelPlacement::AlwaysOtherSameSide);
            let space_efficient = placement.contains(&PortLabelPlacement::SpaceEfficient);
            let label_gap_horizontal = node
                .get_property(CoreOptions::SPACING_LABEL_PORT_HORIZONTAL)
                .unwrap_or(1.0);
            let label_gap_vertical = node
                .get_property(CoreOptions::SPACING_LABEL_PORT_VERTICAL)
                .unwrap_or(1.0);
            let node_width = node.get_size().x;

            let ports = node.get_ports();
            let any_incident_edges = ports.iter().any(|port| {
                !port.get_incoming_edges().is_empty() || !port.get_outgoing_edges().is_empty()
            });
            let port_count = ports.len();
            let mut north_entries = Vec::new();
            let mut south_entries = Vec::new();

            for (index, port) in ports.iter().enumerate() {
                let relation = Self::label_placement_relation(
                    index,
                    port_count,
                    any_incident_edges,
                    inside_label_placement,
                    next_to_port_if_possible,
                    always_same_side,
                    always_other_same_side,
                    space_efficient,
                    Self::should_label_be_placed_next_to_port(port, &node, inside_label_placement),
                );

                let port_size = port.get_size();
                for label in port.get_labels() {
                    let mut label_pos = label.get_position();
                    let label_size = label.get_size();
                    match port.get_side() {
                        PortSide::North | PortSide::South => {
                            label_pos.x = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.x - label_size.x) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.x + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.x - 1.0,
                            };
                        }
                        _ => {
                            label_pos.y = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.y - label_size.y) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.y + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.y - 1.0,
                            };
                        }
                    }
                    label.set_position(label_pos);

                    if constrained_placement {
                        match port.get_side() {
                            PortSide::North => north_entries.push((port.clone(), label.clone())),
                            PortSide::South => south_entries.push((port.clone(), label.clone())),
                            _ => {}
                        }
                    }
                }
            }

            if constrained_placement {
                let north_positive = inside_label_placement;
                let south_positive = !inside_label_placement;
                let clamp_x = inside_label_placement
                    .then_some(Self::inside_horizontal_label_clamp_bounds(&node, node_width));

                // Java uses a strip overlap remover with side-dependent start coordinates.
                // This simplified variant keeps the same directional stacking behavior.
                Self::stack_horizontal_side_labels(
                    &north_entries,
                    HorizontalStackConfig {
                        positive_direction: north_positive,
                        gap_x: label_gap_horizontal,
                        gap_y: label_gap_vertical,
                        start_coordinate: Self::compute_stack_start_coordinate(
                            &north_entries,
                            north_positive,
                            label_gap_vertical,
                        ),
                        clamp_x,
                    },
                );
                Self::stack_horizontal_side_labels(
                    &south_entries,
                    HorizontalStackConfig {
                        positive_direction: south_positive,
                        gap_x: label_gap_horizontal,
                        gap_y: label_gap_vertical,
                        start_coordinate: Self::compute_stack_start_coordinate(
                            &south_entries,
                            south_positive,
                            label_gap_vertical,
                        ),
                        clamp_x,
                    },
                );
            }
        }
    }

    fn calculate_label_and_node_sizes_generic<T, G>(adapter: &G)
    where
        G: GraphAdapter<T>,
    {
        for node in adapter.get_nodes() {
            let placement = node
                .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                .unwrap_or_default();
            if PortLabelPlacement::is_fixed(&placement) {
                continue;
            }

            let inside_label_placement = placement.contains(&PortLabelPlacement::Inside);
            let next_to_port_if_possible =
                placement.contains(&PortLabelPlacement::NextToPortIfPossible);
            let always_same_side = placement.contains(&PortLabelPlacement::AlwaysSameSide);
            let always_other_same_side = placement.contains(&PortLabelPlacement::AlwaysOtherSameSide);
            let space_efficient = placement.contains(&PortLabelPlacement::SpaceEfficient);

            let ports = node.get_ports();
            let any_incident_edges = ports.iter().any(|port| {
                !port.get_incoming_edges().is_empty() || !port.get_outgoing_edges().is_empty()
            });
            let port_count = ports.len();

            for (index, port) in ports.iter().enumerate() {
                let relation = Self::label_placement_relation(
                    index,
                    port_count,
                    any_incident_edges,
                    inside_label_placement,
                    next_to_port_if_possible,
                    always_same_side,
                    always_other_same_side,
                    space_efficient,
                    Self::should_label_be_placed_next_to_port_generic(port, inside_label_placement),
                );

                let port_size = port.get_size();
                for label in port.get_labels() {
                    let mut label_pos = label.get_position();
                    let label_size = label.get_size();
                    match port.get_side() {
                        PortSide::North | PortSide::South => {
                            label_pos.x = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.x - label_size.x) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.x + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.x - 1.0,
                            };
                        }
                        _ => {
                            label_pos.y = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.y - label_size.y) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.y + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.y - 1.0,
                            };
                        }
                    }
                    label.set_position(label_pos);
                }
            }
        }
    }

    pub fn calculate_node_margins<T, G>(adapter: &G)
    where
        G: GraphAdapter<T>,
    {
        let mut calculator = NodeMarginCalculator::new(adapter);
        calculator.process();
    }

    pub fn get_node_margin_calculator<T, G>(adapter: &G) -> NodeMarginCalculator<'_, G, T>
    where
        G: GraphAdapter<T>,
    {
        NodeMarginCalculator::new(adapter)
    }

    pub fn sort_port_lists<T, G>(adapter: &G)
    where
        G: GraphAdapter<T>,
    {
        for node in adapter.get_nodes() {
            node.sort_port_list();
        }
    }

    fn should_label_be_placed_next_to_port(
        port: &ElkPortAdapter,
        parent: &ElkNodeAdapter,
        inside_label_placement: bool,
    ) -> bool {
        let parent_node = parent.element();
        let port_node = port.element();
        let mut edges_to_insides = false;
        let mut edges_to_somewhere_else = false;
        let mut found_incident_edge = false;

        let incoming_edges = port.get_incoming_edges();
        let outgoing_edges = port.get_outgoing_edges();

        for out_edge in outgoing_edges {
            found_incident_edge = true;
            let target = {
                let edge_ref = out_edge.element();
                let edge = edge_ref.borrow();
                edge.targets_ro().get(0)
            };
            if let Some(target_shape) = target {
                Self::classify_endpoint_against_parent(
                    &target_shape,
                    &parent_node,
                    &mut edges_to_insides,
                    &mut edges_to_somewhere_else,
                );
            }
        }

        for in_edge in incoming_edges {
            found_incident_edge = true;
            let source = {
                let edge_ref = in_edge.element();
                let edge = edge_ref.borrow();
                edge.sources_ro().get(0)
            };
            if let Some(source_shape) = source {
                Self::classify_endpoint_against_parent(
                    &source_shape,
                    &parent_node,
                    &mut edges_to_insides,
                    &mut edges_to_somewhere_else,
                );
            }
        }

        if !found_incident_edge {
            found_incident_edge = Self::classify_incident_edges_from_containment(
                &port_node,
                &parent_node,
                &mut edges_to_insides,
                &mut edges_to_somewhere_else,
            );
        }

        if !found_incident_edge {
            return true;
        }

        (inside_label_placement && !edges_to_insides)
            || (!inside_label_placement && !edges_to_somewhere_else)
    }

    fn classify_endpoint_against_parent(
        endpoint: &ElkConnectableShapeRef,
        parent_node: &ElkNodeRef,
        edges_to_insides: &mut bool,
        edges_to_somewhere_else: &mut bool,
    ) {
        if let Some(endpoint_node) = ElkGraphUtil::connectable_shape_to_node(endpoint) {
            let inside_edge = Rc::ptr_eq(&endpoint_node, parent_node)
                || ElkGraphUtil::is_descendant(&endpoint_node, parent_node);
            *edges_to_insides |= inside_edge;
            *edges_to_somewhere_else |= !inside_edge;
        }
    }

    fn classify_incident_edges_from_containment(
        port: &ElkPortRef,
        parent_node: &ElkNodeRef,
        edges_to_insides: &mut bool,
        edges_to_somewhere_else: &mut bool,
    ) -> bool {
        let contained_edges: Vec<ElkEdgeRef> = parent_node
            .borrow_mut()
            .contained_edges()
            .iter()
            .cloned()
            .collect();

        let mut found_incident_edge = false;
        for edge_ref in contained_edges {
            let edge = edge_ref.borrow();
            let source_matches = edge.sources_ro().iter().any(|source| {
                matches!(source, ElkConnectableShapeRef::Port(source_port) if Rc::ptr_eq(source_port, port))
            });
            if source_matches {
                found_incident_edge = true;
                for target in edge.targets_ro().iter() {
                    Self::classify_endpoint_against_parent(
                        target,
                        parent_node,
                        edges_to_insides,
                        edges_to_somewhere_else,
                    );
                }
            }

            let target_matches = edge.targets_ro().iter().any(|target| {
                matches!(target, ElkConnectableShapeRef::Port(target_port) if Rc::ptr_eq(target_port, port))
            });
            if target_matches {
                found_incident_edge = true;
                for source in edge.sources_ro().iter() {
                    Self::classify_endpoint_against_parent(
                        source,
                        parent_node,
                        edges_to_insides,
                        edges_to_somewhere_else,
                    );
                }
            }
        }

        found_incident_edge
    }

    fn should_label_be_placed_next_to_port_generic<T, P>(
        port: &P,
        inside_label_placement: bool,
    ) -> bool
    where
        P: PortAdapter<T>,
    {
        let has_incident_edge =
            !port.get_incoming_edges().is_empty() || !port.get_outgoing_edges().is_empty();
        if !has_incident_edge {
            return true;
        }

        // Generic adapters do not expose endpoint ownership, so use a conservative fallback.
        !inside_label_placement
    }

    #[allow(clippy::too_many_arguments)]
    fn label_placement_relation(
        index: usize,
        port_count: usize,
        any_incident_edges: bool,
        inside_label_placement: bool,
        next_to_port_if_possible: bool,
        always_same_side: bool,
        always_other_same_side: bool,
        space_efficient: bool,
        should_place_next_to_port: bool,
    ) -> LabelPlacementRelation {
        let labels_next_to_port = next_to_port_if_possible && should_place_next_to_port;

        if next_to_port_if_possible {
            if labels_next_to_port {
                LabelPlacementRelation::Centered
            } else {
                LabelPlacementRelation::BelowOrRight
            }
        } else if inside_label_placement {
            if any_incident_edges {
                LabelPlacementRelation::BelowOrRight
            } else {
                LabelPlacementRelation::Centered
            }
        } else if always_same_side {
            LabelPlacementRelation::BelowOrRight
        } else if always_other_same_side
            || (!labels_next_to_port && (port_count == 2 || space_efficient) && index == 0)
        {
            LabelPlacementRelation::AboveOrLeft
        } else {
            LabelPlacementRelation::BelowOrRight
        }
    }

    fn has_constrained_port_label_placement(node: &ElkNodeAdapter) -> bool {
        let size_constraints = node
            .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
            .unwrap_or_default();
        let port_constraints = node
            .get_property(CoreOptions::PORT_CONSTRAINTS)
            .unwrap_or(PortConstraints::Undefined);

        !size_constraints.contains(&SizeConstraint::PortLabels)
            || port_constraints == PortConstraints::FixedPos
    }

    fn inside_horizontal_label_clamp_bounds(node: &ElkNodeAdapter, node_width: f64) -> (f64, f64) {
        let width = node_width.max(0.0);
        let mut min_x: f64 = 0.0;
        let mut max_x: f64 = width;

        // Keep defaults stable; only explicit properties influence clamp bounds.
        if node.has_property(CoreOptions::PADDING) {
            let padding = node.get_padding();
            min_x = min_x.max(padding.left.max(0.0));
            max_x = (width - padding.right.max(0.0)).max(min_x);
        }
        if node.has_property(CoreOptions::SPACING_LABEL_NODE) {
            let spacing = node
                .get_property(CoreOptions::SPACING_LABEL_NODE)
                .unwrap_or(0.0)
                .max(0.0);
            min_x = min_x.max(spacing);
            max_x = (width - spacing).max(min_x);
        }
        if node.has_property(CoreOptions::NODE_LABELS_PADDING) {
            let padding = node
                .get_property(CoreOptions::NODE_LABELS_PADDING)
                .unwrap_or_default();
            min_x = min_x.max(padding.left.max(0.0));
            max_x = (width - padding.right.max(0.0)).max(min_x);
        }

        (min_x, max_x)
    }

    fn stack_horizontal_side_labels(
        entries: &[(ElkPortAdapter, ElkLabelAdapter)],
        config: HorizontalStackConfig,
    ) {
        if entries.is_empty() {
            return;
        }

        // Java constrainedOutside falls back to simple placement for <=2 ports.
        // Keep that behavior, but allow constrainedInside clamping for small counts.
        if entries.len() <= 2 && config.clamp_x.is_none() {
            return;
        }

        let mut sorted = entries.to_vec();
        sorted.sort_by(|(left_port, left_label), (right_port, right_label)| {
            let left_x = left_port.get_position().x + left_label.get_position().x;
            let right_x = right_port.get_position().x + right_label.get_position().x;
            left_x.partial_cmp(&right_x).unwrap_or(Ordering::Equal)
        });

        let mut placed_rectangles: Vec<(f64, f64, f64, f64)> = Vec::new();
        for (port, label) in sorted {
            let port_position = port.get_position();
            let port_size = port.get_size();
            let label_size = label.get_size();
            let mut label_position = label.get_position();

            let mut absolute_x = port_position.x + label_position.x;
            if let Some((min_x, max_x)) = config.clamp_x {
                // Keep labels from drifting past the port extent at node boundaries.
                let actual_min_x = min_x.min(port_position.x);
                let actual_max_x = max_x.max(port_position.x + port_size.x);
                if absolute_x < actual_min_x {
                    absolute_x = actual_min_x;
                } else if absolute_x + label_size.x > actual_max_x {
                    absolute_x = actual_max_x - label_size.x;
                }
            }
            let mut absolute_y = match config.start_coordinate {
                Some(start) => {
                    if config.positive_direction {
                        start
                    } else {
                        start - label_size.y
                    }
                }
                None => port_position.y + label_position.y,
            };

            loop {
                let mut adjusted = false;
                for (placed_x, placed_y, placed_w, placed_h) in &placed_rectangles {
                    let horizontal_overlap = absolute_x < *placed_x + *placed_w + config.gap_x
                        && absolute_x + label_size.x > *placed_x - config.gap_x;
                    let vertical_overlap = absolute_y < *placed_y + *placed_h + config.gap_y
                        && absolute_y + label_size.y > *placed_y - config.gap_y;

                    if horizontal_overlap && vertical_overlap {
                        absolute_y = if config.positive_direction {
                            *placed_y + *placed_h + config.gap_y
                        } else {
                            *placed_y - label_size.y - config.gap_y
                        };
                        adjusted = true;
                    }
                }

                if !adjusted {
                    break;
                }
            }

            label_position.x = absolute_x - port_position.x;
            label_position.y = absolute_y - port_position.y;
            label.set_position(label_position);
            placed_rectangles.push((absolute_x, absolute_y, label_size.x, label_size.y));
        }
    }

    fn compute_stack_start_coordinate(
        entries: &[(ElkPortAdapter, ElkLabelAdapter)],
        positive_direction: bool,
        gap_y: f64,
    ) -> Option<f64> {
        if entries.is_empty() {
            return None;
        }

        if positive_direction {
            let baseline = entries
                .iter()
                .map(|(port, _)| {
                    let position = port.get_position();
                    let size = port.get_size();
                    position.y + size.y
                })
                .fold(f64::NEG_INFINITY, f64::max);
            Some(baseline + gap_y)
        } else {
            let baseline = entries
                .iter()
                .map(|(port, _)| port.get_position().y)
                .fold(f64::INFINITY, f64::min);
            Some(baseline - gap_y)
        }
    }
}

enum LabelPlacementRelation {
    Centered,
    BelowOrRight,
    AboveOrLeft,
}
