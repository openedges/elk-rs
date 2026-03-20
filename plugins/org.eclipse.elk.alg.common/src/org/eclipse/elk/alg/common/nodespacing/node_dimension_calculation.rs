use std::any::Any;
use std::cmp::Ordering;
use std::sync::LazyLock;

static NODE_DIM_FULL_FOR_FIXED: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_NODE_DIM_FULL_FOR_FIXED").is_some());
static NODE_DIM_USE_FULL_PROCESS: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_NODE_DIM_SKIP_FULL_PROCESS").is_none());

use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, Direction, NodeLabelPlacement, PortLabelPlacement, PortSide, SizeConstraint,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{
    ElkGraphAdapter, ElkLabelAdapter, ElkNodeAdapter, ElkPortAdapter, GraphAdapter,
    GraphElementAdapter, NodeAdapter, PortAdapter,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkNodeRef, ElkPortRef,
};
use std::rc::Rc;

use super::node_label_and_size_calculator::NodeLabelAndSizeCalculator;
use super::node_margin_calculator::NodeMarginCalculator;

pub struct NodeDimensionCalculation;

static COMPOUND_NODE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("compoundNode", false));

#[derive(Clone, Copy, Default)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Rect {
    fn union(self, other: Rect) -> Rect {
        let left = self.x.min(other.x);
        let top = self.y.min(other.y);
        let right = (self.x + self.width).max(other.x + other.width);
        let bottom = (self.y + self.height).max(other.y + other.height);
        Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }

    fn overlaps(self, other: Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }
}

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
        <<G as GraphAdapter<T>>::NodeAdapter as NodeAdapter<<G as GraphAdapter<T>>::Node>>::Graph: GraphElementAdapter<<G as GraphAdapter<T>>::Node>,
        <<G as GraphAdapter<T>>::NodeAdapter as NodeAdapter<<G as GraphAdapter<T>>::Node>>::Label: 'static,
        <<G as GraphAdapter<T>>::NodeAdapter as NodeAdapter<<G as GraphAdapter<T>>::Node>>::LabelAdapter: 'static,
    {
        // Java: single path — NodeLabelAndSizeCalculator.process(adapter)
        // which iterates nodes and calls process_node() for each.
        // We match this by calling process_node for each node, then handling port labels.
        //
        // For ELK adapters we use the ELK-specific path that has full endpoint analysis.
        // For other adapters (e.g. LGraphAdapter), we use process_node + generic port label path.
        if let Some(elk_adapter) = (adapter as &dyn Any).downcast_ref::<ElkGraphAdapter>() {
            Self::calculate_label_and_node_sizes_for_elk(elk_adapter);
            return;
        }

        // Java: NodeLabelAndSizeCalculator.process(adapter) handles node labels first,
        // then port label placement. For non-ELK adapters (e.g. LGraph), we mimic this
        // by running process_node before placing port labels.
        Self::calculate_label_and_node_sizes_with_process_node(adapter);
    }

    pub fn calculate_label_and_node_sizes_for_elk(adapter: &ElkGraphAdapter) {
        let layout_direction = adapter
            .get_property(CoreOptions::DIRECTION)
            .unwrap_or(Direction::Undefined);
        let use_full_process_for_fixed = *NODE_DIM_FULL_FOR_FIXED;
        for node in adapter.get_nodes() {
            let placement = node
                .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                .unwrap_or_default();
            if use_full_process_for_fixed && PortLabelPlacement::is_fixed(&placement) {
                NodeLabelAndSizeCalculator::process(&node, layout_direction);
                continue;
            }

            NodeLabelAndSizeCalculator::process_node(&node, layout_direction);

            if PortLabelPlacement::is_fixed(&placement) {
                continue;
            }

            let size_constraints = node
                .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
                .unwrap_or_default();
            let constrained_placement = placement.contains(&PortLabelPlacement::Inside)
                || placement.contains(&PortLabelPlacement::Outside);
            let inside_label_placement = placement.contains(&PortLabelPlacement::Inside);
            let next_to_port_if_possible =
                placement.contains(&PortLabelPlacement::NextToPortIfPossible);
            let always_same_side = placement.contains(&PortLabelPlacement::AlwaysSameSide);
            let always_other_same_side =
                placement.contains(&PortLabelPlacement::AlwaysOtherSameSide);
            let space_efficient = placement.contains(&PortLabelPlacement::SpaceEfficient);
            let label_gap_horizontal = node
                .get_property(CoreOptions::SPACING_LABEL_PORT_HORIZONTAL)
                .unwrap_or(1.0);
            let label_gap_vertical = node
                .get_property(CoreOptions::SPACING_LABEL_PORT_VERTICAL)
                .unwrap_or(1.0);
            let node_width = node.get_size().x;

            let node_label_bounds = Self::inside_node_label_bounds(&node);
            let ports = node.get_ports();
            let node_is_compound = node.element().borrow().is_hierarchical();
            let any_incident_edges = ports.iter().any(|port| {
                !port.get_incoming_edges().is_empty() || !port.get_outgoing_edges().is_empty()
            }) || node_is_compound;
            let stack_label_overlaps = size_constraints.contains(&SizeConstraint::PortLabels);
            let stack_inside_labels = inside_label_placement && stack_label_overlaps;
            // Compute per-side port counts for label_placement_relation
            let mut side_counts: [usize; 5] = [0; 5];
            for port in ports.iter() {
                let idx = match port.get_side() {
                    PortSide::North => 0,
                    PortSide::East => 1,
                    PortSide::South => 2,
                    PortSide::West => 3,
                    PortSide::Undefined => 4,
                };
                side_counts[idx] += 1;
            }
            let mut side_indices: [usize; 5] = [0; 5];
            let mut north_entries = Vec::new();
            let mut south_entries = Vec::new();
            let mut east_entries = Vec::new();
            let mut west_entries = Vec::new();

            for port in ports.iter() {
                let side_idx = match port.get_side() {
                    PortSide::North => 0,
                    PortSide::East => 1,
                    PortSide::South => 2,
                    PortSide::West => 3,
                    PortSide::Undefined => 4,
                };
                let per_side_index = side_indices[side_idx];
                let per_side_count = side_counts[side_idx];
                side_indices[side_idx] += 1;

                // Java's NodeContext.comparePortContexts reverses WEST/SOUTH ports in TreeMultimap
                let effective_index =
                    if port.get_side() == PortSide::South || port.get_side() == PortSide::West {
                        per_side_count
                            .saturating_sub(1)
                            .saturating_sub(per_side_index)
                    } else {
                        per_side_index
                    };

                let relation = Self::label_placement_relation(
                    effective_index,
                    per_side_count,
                    any_incident_edges,
                    inside_label_placement,
                    next_to_port_if_possible,
                    always_same_side,
                    always_other_same_side,
                    space_efficient,
                    Self::should_label_be_placed_next_to_port(
                        port,
                        &node,
                        inside_label_placement,
                        next_to_port_if_possible,
                    ),
                );

                let port_size = port.get_size();
                let port_border_offset = port
                    .get_property(CoreOptions::PORT_BORDER_OFFSET)
                    .unwrap_or(0.0);
                let label_border_offset = Self::port_label_border_offset_for_port_side(
                    &node,
                    port.get_side(),
                    label_gap_horizontal,
                    label_gap_vertical,
                );
                let labels = port.get_labels();
                let label_count = labels.len();
                let total_label_height: f64 = if label_count > 1
                    && matches!(
                        port.get_side(),
                        PortSide::East | PortSide::West | PortSide::Undefined
                    ) {
                    labels.iter().map(|l| l.get_size().y).sum()
                } else {
                    0.0
                };
                let mut y_cursor = if label_count > 1
                    && matches!(
                        port.get_side(),
                        PortSide::East | PortSide::West | PortSide::Undefined
                    ) {
                    match relation {
                        LabelPlacementRelation::BelowOrRight => {
                            port_size.y + label_gap_vertical
                        }
                        LabelPlacementRelation::AboveOrLeft => {
                            -total_label_height - label_gap_vertical
                        }
                        LabelPlacementRelation::Centered => {
                            (port_size.y - total_label_height) / 2.0
                        }
                    }
                } else {
                    0.0
                };
                for label in labels {
                    let mut label_pos = label.get_position();
                    let label_size = label.get_size();
                    match port.get_side() {
                        PortSide::North => {
                            label_pos.x = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.x - label_size.x) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.x + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.x - 1.0,
                            };
                            if constrained_placement {
                                label_pos.y = if inside_label_placement {
                                    port_size.y + port_border_offset + label_border_offset
                                } else {
                                    -label_size.y - label_gap_vertical
                                };
                            }
                        }
                        PortSide::South => {
                            label_pos.x = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.x - label_size.x) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.x + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.x - 1.0,
                            };
                            if constrained_placement {
                                label_pos.y = if inside_label_placement {
                                    -port_border_offset - label_border_offset - label_size.y
                                } else {
                                    port_size.y + label_gap_vertical
                                };
                            }
                        }
                        PortSide::East => {
                            if constrained_placement {
                                label_pos.x = if inside_label_placement {
                                    -port_border_offset - label_border_offset - label_size.x
                                } else {
                                    port_size.x + label_gap_horizontal
                                };
                            }
                            if label_count > 1 {
                                label_pos.y = y_cursor;
                                y_cursor += label_size.y;
                            } else {
                                label_pos.y = match relation {
                                    LabelPlacementRelation::Centered => {
                                        (port_size.y - label_size.y) / 2.0
                                    }
                                    LabelPlacementRelation::BelowOrRight => {
                                        port_size.y + label_gap_vertical
                                    }
                                    LabelPlacementRelation::AboveOrLeft => {
                                        -label_size.y - label_gap_vertical
                                    }
                                };
                            }
                        }
                        PortSide::West | PortSide::Undefined => {
                            if constrained_placement {
                                label_pos.x = if inside_label_placement {
                                    port_size.x + port_border_offset + label_border_offset
                                } else {
                                    -label_size.x - label_gap_horizontal
                                };
                            }
                            if label_count > 1 {
                                label_pos.y = y_cursor;
                                y_cursor += label_size.y;
                            } else {
                                label_pos.y = match relation {
                                    LabelPlacementRelation::Centered => {
                                        (port_size.y - label_size.y) / 2.0
                                    }
                                    LabelPlacementRelation::BelowOrRight => {
                                        port_size.y + label_gap_vertical
                                    }
                                    LabelPlacementRelation::AboveOrLeft => {
                                        -label_size.y - label_gap_vertical
                                    }
                                };
                            }
                        }
                    }
                    if stack_label_overlaps {
                        if let Some(bounds) = node_label_bounds {
                            label_pos = Self::avoid_node_label_overlap(
                                &node,
                                port,
                                label_pos,
                                label_size,
                                bounds,
                                label_gap_horizontal,
                                label_gap_vertical,
                            );
                        }
                    }
                    label.set_position(label_pos);

                    if constrained_placement {
                        match port.get_side() {
                            PortSide::North => north_entries.push((port.clone(), label.clone())),
                            PortSide::South => south_entries.push((port.clone(), label.clone())),
                            PortSide::East => east_entries.push((port.clone(), label.clone())),
                            PortSide::West | PortSide::Undefined => {
                                west_entries.push((port.clone(), label.clone()))
                            }
                        }
                    }
                }
            }

            if constrained_placement {
                let stack_constrained_inside =
                    inside_label_placement && (always_same_side || always_other_same_side);
                if stack_label_overlaps || stack_constrained_inside {
                    let north_positive = inside_label_placement;
                    let south_positive = !inside_label_placement;
                    let clamp_x = inside_label_placement.then_some(
                        Self::inside_horizontal_label_clamp_bounds(&node, node_width),
                    );

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

                if stack_inside_labels {
                    let mut avoid_rects = Vec::new();
                    if let Some(bounds) = node_label_bounds {
                        avoid_rects.push(bounds);
                    }
                    Self::append_label_rects(&mut avoid_rects, &north_entries);
                    Self::append_label_rects(&mut avoid_rects, &south_entries);
                    Self::stack_vertical_side_labels(
                        &west_entries,
                        label_gap_horizontal,
                        label_gap_vertical,
                        &avoid_rects,
                        next_to_port_if_possible,
                    );
                    Self::stack_vertical_side_labels(
                        &east_entries,
                        label_gap_horizontal,
                        label_gap_vertical,
                        &avoid_rects,
                        next_to_port_if_possible,
                    );
                }
            }
        }
    }

    /// Java-matching path: calls process_node (for node label sizing) then port label placement.
    /// This is what Java's NodeDimensionCalculation.calculateLabelAndNodeSizes does via
    /// NodeLabelAndSizeCalculator.process(adapter).
    fn calculate_label_and_node_sizes_with_process_node<T, G>(adapter: &G)
    where
        T: 'static,
        G: GraphAdapter<T>,
        <<G as GraphAdapter<T>>::NodeAdapter as NodeAdapter<<G as GraphAdapter<T>>::Node>>::Graph: GraphElementAdapter<<G as GraphAdapter<T>>::Node>,
        <<G as GraphAdapter<T>>::NodeAdapter as NodeAdapter<<G as GraphAdapter<T>>::Node>>::Label: 'static,
        <<G as GraphAdapter<T>>::NodeAdapter as NodeAdapter<<G as GraphAdapter<T>>::Node>>::LabelAdapter: 'static,
        <<G as GraphAdapter<T>>::NodeAdapter as NodeAdapter<<G as GraphAdapter<T>>::Node>>::Port: 'static,
        <<G as GraphAdapter<T>>::NodeAdapter as NodeAdapter<<G as GraphAdapter<T>>::Node>>::PortAdapter: 'static,
        <G as GraphAdapter<T>>::Node: 'static,
    {
        if *NODE_DIM_USE_FULL_PROCESS {
            let layout_direction = adapter
                .get_property(CoreOptions::DIRECTION)
                .unwrap_or(Direction::Undefined);
            for node in adapter.get_nodes() {
                // process() with are_size_constraints_fixed keeps the current node size,
                // which is correct for most nodes. However, compound nodes whose inner
                // layout has already run may have a size that doesn't account for fixed
                // port labels extending into the node (the inner layout doesn't know about
                // parent-level port labels). In that case, process_node() is needed because
                // it always computes the full size including port label space.
                //
                // Detect this case: size constraints are effectively fixed (empty or just
                // PortLabels) AND ports have labels at non-zero positions (indicating they
                // were placed by a prior layout pass and extend into the node).
                if Self::needs_process_node_for_fixed_port_labels(&node) {
                    // First compute the correct size (including port label space),
                    // then run process() which will keep this size (fixed constraints)
                    // but also place ports at the correct positions.
                    NodeLabelAndSizeCalculator::process_node(&node, layout_direction);
                    NodeLabelAndSizeCalculator::process(&node, layout_direction);
                } else {
                    NodeLabelAndSizeCalculator::process(&node, layout_direction);
                }
            }
            return;
        }

        let layout_direction = adapter
            .get_property(CoreOptions::DIRECTION)
            .unwrap_or(Direction::Undefined);
        for node in adapter.get_nodes() {
            NodeLabelAndSizeCalculator::process_node(&node, layout_direction);

            let placement = node
                .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                .unwrap_or_default();
            if PortLabelPlacement::is_fixed(&placement) {
                continue;
            }

            let constrained_placement = placement.contains(&PortLabelPlacement::Inside)
                || placement.contains(&PortLabelPlacement::Outside);
            let inside_label_placement = placement.contains(&PortLabelPlacement::Inside);
            let next_to_port_if_possible =
                placement.contains(&PortLabelPlacement::NextToPortIfPossible);
            let always_same_side = placement.contains(&PortLabelPlacement::AlwaysSameSide);
            let always_other_same_side =
                placement.contains(&PortLabelPlacement::AlwaysOtherSameSide);
            let space_efficient = placement.contains(&PortLabelPlacement::SpaceEfficient);
            let label_gap_horizontal_raw = node
                .get_property(CoreOptions::SPACING_LABEL_PORT_HORIZONTAL);
            let label_gap_horizontal = label_gap_horizontal_raw.unwrap_or(1.0);
            let label_gap_vertical = node
                .get_property(CoreOptions::SPACING_LABEL_PORT_VERTICAL)
                .unwrap_or(1.0);
            let node_width = node.get_size().x;
            let size_constraints = node
                .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
                .unwrap_or_default();
            let stack_label_overlaps = size_constraints.contains(&SizeConstraint::PortLabels);
            let stack_inside_labels = inside_label_placement && stack_label_overlaps;
            let node_label_bounds = Self::inside_node_label_bounds_generic(&node);

            let ports = node.get_ports();
            let node_is_compound = node.get_property(&COMPOUND_NODE_PROPERTY).unwrap_or(false);
            let any_incident_edges = ports.iter().any(|port| {
                !port.get_incoming_edges().is_empty()
                    || !port.get_outgoing_edges().is_empty()
                    || port.has_compound_connections()
            }) || node_is_compound;
            let mut side_counts: [usize; 5] = [0; 5];
            for port in ports.iter() {
                let idx = match port.get_side() {
                    PortSide::North => 0,
                    PortSide::East => 1,
                    PortSide::South => 2,
                    PortSide::West => 3,
                    PortSide::Undefined => 4,
                };
                side_counts[idx] += 1;
            }
            let mut side_indices: [usize; 5] = [0; 5];
            let mut north_entries: Vec<(usize, usize)> = Vec::new();
            let mut south_entries: Vec<(usize, usize)> = Vec::new();
            let mut east_entries: Vec<(usize, usize)> = Vec::new();
            let mut west_entries: Vec<(usize, usize)> = Vec::new();

            for (port_idx, port) in ports.iter().enumerate() {
                let side_idx = match port.get_side() {
                    PortSide::North => 0,
                    PortSide::East => 1,
                    PortSide::South => 2,
                    PortSide::West => 3,
                    PortSide::Undefined => 4,
                };
                let per_side_index = side_indices[side_idx];
                let per_side_count = side_counts[side_idx];
                side_indices[side_idx] += 1;

                // Java's NodeContext.comparePortContexts reverses WEST/SOUTH ports in TreeMultimap
                let effective_index =
                    if port.get_side() == PortSide::South || port.get_side() == PortSide::West {
                        per_side_count
                            .saturating_sub(1)
                            .saturating_sub(per_side_index)
                    } else {
                        per_side_index
                    };

                let relation = Self::label_placement_relation(
                    effective_index,
                    per_side_count,
                    any_incident_edges,
                    inside_label_placement,
                    next_to_port_if_possible,
                    always_same_side,
                    always_other_same_side,
                    space_efficient,
                    Self::should_label_be_placed_next_to_port_generic(
                        port,
                        node_is_compound,
                        inside_label_placement,
                        next_to_port_if_possible,
                    ),
                );

                let port_size = port.get_size();
                let port_border_offset = port
                    .get_property(CoreOptions::PORT_BORDER_OFFSET)
                    .unwrap_or(0.0);
                let label_border_offset = {
                    let padding = node.get_padding();
                    match port.get_side() {
                        PortSide::North => padding.top + label_gap_vertical,
                        PortSide::East => padding.right + label_gap_horizontal,
                        PortSide::South => padding.bottom + label_gap_vertical,
                        PortSide::West | PortSide::Undefined => padding.left + label_gap_horizontal,
                    }
                };
                let labels = port.get_labels();
                let label_count = labels.len();
                let total_label_height: f64 = if label_count > 1
                    && matches!(
                        port.get_side(),
                        PortSide::East | PortSide::West | PortSide::Undefined
                    ) {
                    labels.iter().map(|l| l.get_size().y).sum()
                } else {
                    0.0
                };

                let mut y_cursor = if label_count > 1
                    && matches!(
                        port.get_side(),
                        PortSide::East | PortSide::West | PortSide::Undefined
                    ) {
                    match relation {
                        LabelPlacementRelation::BelowOrRight => {
                            port_size.y + label_gap_vertical
                        }
                        LabelPlacementRelation::AboveOrLeft => {
                            -total_label_height - label_gap_vertical
                        }
                        LabelPlacementRelation::Centered => {
                            (port_size.y - total_label_height) / 2.0
                        }
                    }
                } else {
                    0.0
                };

                for (label_idx, label) in labels.iter().enumerate() {
                    let mut label_pos = label.get_position();
                    let label_size = label.get_size();
                    // Java parity: for INSIDE labels, don't include port_border_offset
                    // in the position formula.  Java's cell-based
                    // simpleInsidePortLabelPlacement effectively uses 0 for the
                    // computed border offset set by LGraphUtil.initializePort.
                    let inside_pbo = if inside_label_placement {
                        0.0
                    } else {
                        port_border_offset
                    };
                    match port.get_side() {
                        PortSide::North => {
                            label_pos.x = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.x - label_size.x) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.x + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.x - 1.0,
                            };
                            if constrained_placement {
                                label_pos.y = if inside_label_placement {
                                    port_size.y + inside_pbo + label_border_offset
                                } else {
                                    -label_size.y - label_gap_vertical
                                };
                            }
                        }
                        PortSide::South => {
                            label_pos.x = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.x - label_size.x) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.x + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.x - 1.0,
                            };
                            if constrained_placement {
                                label_pos.y = if inside_label_placement {
                                    -inside_pbo - label_border_offset - label_size.y
                                } else {
                                    port_size.y + label_gap_vertical
                                };
                            }
                        }
                        PortSide::East => {
                            if constrained_placement {
                                label_pos.x = if inside_label_placement {
                                    -inside_pbo - label_border_offset - label_size.x
                                } else {
                                    port_size.x + label_gap_horizontal
                                };
                            }
                            if label_count > 1 {
                                label_pos.y = y_cursor;
                                y_cursor += label_size.y;
                            } else {
                                label_pos.y = match relation {
                                    LabelPlacementRelation::Centered => {
                                        (port_size.y - label_size.y) / 2.0
                                    }
                                    LabelPlacementRelation::BelowOrRight => {
                                        port_size.y + label_gap_vertical
                                    }
                                    LabelPlacementRelation::AboveOrLeft => {
                                        -label_size.y - label_gap_vertical
                                    }
                                };
                            }
                        }
                        PortSide::West | PortSide::Undefined => {
                            if constrained_placement {
                                label_pos.x = if inside_label_placement {
                                    port_size.x + inside_pbo + label_border_offset
                                } else {
                                    -label_size.x - label_gap_horizontal
                                };
                            }
                            if label_count > 1 {
                                label_pos.y = y_cursor;
                                y_cursor += label_size.y;
                            } else {
                                label_pos.y = match relation {
                                    LabelPlacementRelation::Centered => {
                                        (port_size.y - label_size.y) / 2.0
                                    }
                                    LabelPlacementRelation::BelowOrRight => {
                                        port_size.y + label_gap_vertical
                                    }
                                    LabelPlacementRelation::AboveOrLeft => {
                                        -label_size.y - label_gap_vertical
                                    }
                                };
                            }
                        }
                    }
                    if stack_label_overlaps {
                        if let Some(bounds) = node_label_bounds {
                            label_pos = Self::avoid_node_label_overlap_generic(
                                &node,
                                port,
                                label_pos,
                                label_size,
                                bounds,
                                label_gap_horizontal,
                                label_gap_vertical,
                            );
                        }
                    }
                    label.set_position(label_pos);

                    if constrained_placement {
                        match port.get_side() {
                            PortSide::North => north_entries.push((port_idx, label_idx)),
                            PortSide::South => south_entries.push((port_idx, label_idx)),
                            PortSide::East => east_entries.push((port_idx, label_idx)),
                            PortSide::West | PortSide::Undefined => {
                                west_entries.push((port_idx, label_idx))
                            }
                        }
                    }
                }
            }

            if constrained_placement {
                let stack_constrained_inside =
                    inside_label_placement && (always_same_side || always_other_same_side);
                if stack_label_overlaps || stack_constrained_inside {
                    let north_positive = inside_label_placement;
                    let south_positive = !inside_label_placement;
                    let clamp_x = inside_label_placement.then(|| {
                        Self::inside_horizontal_label_clamp_bounds_generic(&node, node_width)
                    });

                    Self::stack_horizontal_side_labels_generic(
                        &node,
                        &north_entries,
                        HorizontalStackConfig {
                            positive_direction: north_positive,
                            gap_x: label_gap_horizontal,
                            gap_y: label_gap_vertical,
                            start_coordinate: Self::compute_stack_start_coordinate_generic(
                                &node,
                                &north_entries,
                                north_positive,
                                label_gap_vertical,
                            ),
                            clamp_x,
                        },
                    );
                    Self::stack_horizontal_side_labels_generic(
                        &node,
                        &south_entries,
                        HorizontalStackConfig {
                            positive_direction: south_positive,
                            gap_x: label_gap_horizontal,
                            gap_y: label_gap_vertical,
                            start_coordinate: Self::compute_stack_start_coordinate_generic(
                                &node,
                                &south_entries,
                                south_positive,
                                label_gap_vertical,
                            ),
                            clamp_x,
                        },
                    );
                }

                if stack_inside_labels {
                    let mut avoid_rects = Vec::new();
                    if let Some(bounds) = node_label_bounds {
                        avoid_rects.push(bounds);
                    }
                    Self::append_label_rects_generic(&node, &mut avoid_rects, &north_entries);
                    Self::append_label_rects_generic(&node, &mut avoid_rects, &south_entries);
                    Self::stack_vertical_side_labels_generic(
                        &node,
                        &west_entries,
                        label_gap_horizontal,
                        label_gap_vertical,
                        &avoid_rects,
                        next_to_port_if_possible,
                    );
                    Self::stack_vertical_side_labels_generic(
                        &node,
                        &east_entries,
                        label_gap_horizontal,
                        label_gap_vertical,
                        &avoid_rects,
                        next_to_port_if_possible,
                    );
                }
            }
        }
    }

    #[allow(dead_code)]
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
            let always_other_same_side =
                placement.contains(&PortLabelPlacement::AlwaysOtherSameSide);
            let space_efficient = placement.contains(&PortLabelPlacement::SpaceEfficient);

            let ports = node.get_ports();
            let node_is_compound = node.get_property(&COMPOUND_NODE_PROPERTY).unwrap_or(false);
            let any_incident_edges = ports.iter().any(|port| {
                !port.get_incoming_edges().is_empty()
                    || !port.get_outgoing_edges().is_empty()
                    || port.has_compound_connections()
            }) || node_is_compound;

            // Compute per-side port counts (Java uses per-side index/count for label_placement_relation)
            let mut side_counts: [usize; 5] = [0; 5]; // N=0, E=1, S=2, W=3, Undef=4
            for port in ports.iter() {
                let idx = match port.get_side() {
                    PortSide::North => 0,
                    PortSide::East => 1,
                    PortSide::South => 2,
                    PortSide::West => 3,
                    PortSide::Undefined => 4,
                };
                side_counts[idx] += 1;
            }
            let mut side_indices: [usize; 5] = [0; 5];

            let constrained_placement = placement.contains(&PortLabelPlacement::Inside)
                || placement.contains(&PortLabelPlacement::Outside);
            let label_gap_horizontal = node
                .get_property(CoreOptions::SPACING_LABEL_PORT_HORIZONTAL)
                .unwrap_or(1.0);
            let label_gap_vertical = node
                .get_property(CoreOptions::SPACING_LABEL_PORT_VERTICAL)
                .unwrap_or(1.0);
            let node_width = node.get_size().x;
            let size_constraints = node
                .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
                .unwrap_or_default();
            let stack_label_overlaps = size_constraints.contains(&SizeConstraint::PortLabels);
            let stack_inside_labels = inside_label_placement && stack_label_overlaps;
            let node_label_bounds = Self::inside_node_label_bounds_generic(&node);

            // Collect north/south entries for stacking (generic version stores tuples)
            let mut north_entries: Vec<(usize, usize)> = Vec::new(); // (port_idx, label_idx)
            let mut south_entries: Vec<(usize, usize)> = Vec::new();
            let mut east_entries: Vec<(usize, usize)> = Vec::new();
            let mut west_entries: Vec<(usize, usize)> = Vec::new();

            for (port_idx, port) in ports.iter().enumerate() {
                let side_idx = match port.get_side() {
                    PortSide::North => 0,
                    PortSide::East => 1,
                    PortSide::South => 2,
                    PortSide::West => 3,
                    PortSide::Undefined => 4,
                };
                let per_side_index = side_indices[side_idx];
                let per_side_count = side_counts[side_idx];
                side_indices[side_idx] += 1;

                // Java's NodeContext.comparePortContexts reverses WEST/SOUTH ports in TreeMultimap
                let effective_index =
                    if port.get_side() == PortSide::South || port.get_side() == PortSide::West {
                        per_side_count
                            .saturating_sub(1)
                            .saturating_sub(per_side_index)
                    } else {
                        per_side_index
                    };

                let relation = Self::label_placement_relation(
                    effective_index,
                    per_side_count,
                    any_incident_edges,
                    inside_label_placement,
                    next_to_port_if_possible,
                    always_same_side,
                    always_other_same_side,
                    space_efficient,
                    Self::should_label_be_placed_next_to_port_generic(
                        port,
                        node_is_compound,
                        inside_label_placement,
                        next_to_port_if_possible,
                    ),
                );

                let port_size = port.get_size();
                let port_border_offset = port
                    .get_property(CoreOptions::PORT_BORDER_OFFSET)
                    .unwrap_or(0.0);
                let label_border_offset = {
                    let padding = node.get_padding();
                    match port.get_side() {
                        PortSide::North => padding.top + label_gap_vertical,
                        PortSide::East => padding.right + label_gap_horizontal,
                        PortSide::South => padding.bottom + label_gap_vertical,
                        PortSide::West | PortSide::Undefined => padding.left + label_gap_horizontal,
                    }
                };
                let labels = port.get_labels();
                let label_count = labels.len();
                // For EAST/WEST stacking: compute total label height for centering
                let total_label_height: f64 = if label_count > 1
                    && matches!(
                        port.get_side(),
                        PortSide::East | PortSide::West | PortSide::Undefined
                    ) {
                    labels.iter().map(|l| l.get_size().y).sum()
                } else {
                    0.0
                };

                let mut y_cursor = if label_count > 1
                    && matches!(
                        port.get_side(),
                        PortSide::East | PortSide::West | PortSide::Undefined
                    ) {
                    match relation {
                        LabelPlacementRelation::BelowOrRight => {
                            port_size.y + label_gap_vertical
                        }
                        LabelPlacementRelation::AboveOrLeft => {
                            -total_label_height - label_gap_vertical
                        }
                        LabelPlacementRelation::Centered => {
                            (port_size.y - total_label_height) / 2.0
                        }
                    }
                } else {
                    0.0
                };

                for (label_idx, label) in labels.iter().enumerate() {
                    let mut label_pos = label.get_position();
                    let label_size = label.get_size();
                    match port.get_side() {
                        PortSide::North => {
                            label_pos.x = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.x - label_size.x) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.x + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.x - 1.0,
                            };
                            if constrained_placement {
                                label_pos.y = if inside_label_placement {
                                    port_size.y + port_border_offset + label_border_offset
                                } else {
                                    -label_size.y - label_gap_vertical
                                };
                            }
                        }
                        PortSide::South => {
                            label_pos.x = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.x - label_size.x) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.x + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.x - 1.0,
                            };
                            if constrained_placement {
                                label_pos.y = if inside_label_placement {
                                    -port_border_offset - label_border_offset - label_size.y
                                } else {
                                    port_size.y + label_gap_vertical
                                };
                            }
                        }
                        PortSide::East => {
                            if constrained_placement {
                                label_pos.x = if inside_label_placement {
                                    -port_border_offset - label_border_offset - label_size.x
                                } else {
                                    port_size.x + label_gap_horizontal
                                };
                            }
                            if label_count > 1 {
                                label_pos.y = y_cursor;
                                y_cursor += label_size.y;
                            } else {
                                label_pos.y = match relation {
                                    LabelPlacementRelation::Centered => {
                                        (port_size.y - label_size.y) / 2.0
                                    }
                                    LabelPlacementRelation::BelowOrRight => {
                                        port_size.y + label_gap_vertical
                                    }
                                    LabelPlacementRelation::AboveOrLeft => {
                                        -label_size.y - label_gap_vertical
                                    }
                                };
                            }
                        }
                        PortSide::West | PortSide::Undefined => {
                            if constrained_placement {
                                label_pos.x = if inside_label_placement {
                                    port_size.x + port_border_offset + label_border_offset
                                } else {
                                    -label_size.x - label_gap_horizontal
                                };
                            }
                            if label_count > 1 {
                                label_pos.y = y_cursor;
                                y_cursor += label_size.y;
                            } else {
                                label_pos.y = match relation {
                                    LabelPlacementRelation::Centered => {
                                        (port_size.y - label_size.y) / 2.0
                                    }
                                    LabelPlacementRelation::BelowOrRight => {
                                        port_size.y + label_gap_vertical
                                    }
                                    LabelPlacementRelation::AboveOrLeft => {
                                        -label_size.y - label_gap_vertical
                                    }
                                };
                            }
                        }
                    }
                    if stack_label_overlaps {
                        if let Some(bounds) = node_label_bounds {
                            label_pos = Self::avoid_node_label_overlap_generic(
                                &node,
                                port,
                                label_pos,
                                label_size,
                                bounds,
                                label_gap_horizontal,
                                label_gap_vertical,
                            );
                        }
                    }
                    label.set_position(label_pos);

                    // Collect north/south entries for stacking
                    if constrained_placement {
                        match port.get_side() {
                            PortSide::North => north_entries.push((port_idx, label_idx)),
                            PortSide::South => south_entries.push((port_idx, label_idx)),
                            PortSide::East => east_entries.push((port_idx, label_idx)),
                            PortSide::West | PortSide::Undefined => {
                                west_entries.push((port_idx, label_idx))
                            }
                        }
                    }
                }
            }

            // Apply stacking to north/south labels (generic version using stored indices)
            if constrained_placement {
                let stack_constrained_inside =
                    inside_label_placement && (always_same_side || always_other_same_side);
                if stack_label_overlaps || stack_constrained_inside {
                    let north_positive = inside_label_placement;
                    let south_positive = !inside_label_placement;
                    let clamp_x = inside_label_placement.then(|| {
                        Self::inside_horizontal_label_clamp_bounds_generic(&node, node_width)
                    });

                    Self::stack_horizontal_side_labels_generic(
                        &node,
                        &north_entries,
                        HorizontalStackConfig {
                            positive_direction: north_positive,
                            gap_x: label_gap_horizontal,
                            gap_y: label_gap_vertical,
                            start_coordinate: Self::compute_stack_start_coordinate_generic(
                                &node,
                                &north_entries,
                                north_positive,
                                label_gap_vertical,
                            ),
                            clamp_x,
                        },
                    );
                    Self::stack_horizontal_side_labels_generic(
                        &node,
                        &south_entries,
                        HorizontalStackConfig {
                            positive_direction: south_positive,
                            gap_x: label_gap_horizontal,
                            gap_y: label_gap_vertical,
                            start_coordinate: Self::compute_stack_start_coordinate_generic(
                                &node,
                                &south_entries,
                                south_positive,
                                label_gap_vertical,
                            ),
                            clamp_x,
                        },
                    );
                }

                if stack_inside_labels {
                    let mut avoid_rects = Vec::new();
                    if let Some(bounds) = node_label_bounds {
                        avoid_rects.push(bounds);
                    }
                    Self::append_label_rects_generic(&node, &mut avoid_rects, &north_entries);
                    Self::append_label_rects_generic(&node, &mut avoid_rects, &south_entries);
                    Self::stack_vertical_side_labels_generic(
                        &node,
                        &west_entries,
                        label_gap_horizontal,
                        label_gap_vertical,
                        &avoid_rects,
                        next_to_port_if_possible,
                    );
                    Self::stack_vertical_side_labels_generic(
                        &node,
                        &east_entries,
                        label_gap_horizontal,
                        label_gap_vertical,
                        &avoid_rects,
                        next_to_port_if_possible,
                    );
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
        next_to_port_if_possible: bool,
    ) -> bool {
        let parent_node = parent.element();
        let port_node = port.element();
        let parent_is_compound = parent.element().borrow().is_hierarchical();
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
            Self::classify_incident_edges_from_containment(
                &port_node,
                &parent_node,
                &mut edges_to_insides,
                &mut edges_to_somewhere_else,
            );
        }

        if inside_label_placement {
            if parent_is_compound {
                next_to_port_if_possible && !edges_to_insides
            } else {
                true
            }
        } else {
            next_to_port_if_possible && !edges_to_somewhere_else
        }
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
        parent_is_compound: bool,
        inside_label_placement: bool,
        next_to_port_if_possible: bool,
    ) -> bool
    where
        P: PortAdapter<T>,
    {
        let has_incident_edge =
            !port.get_incoming_edges().is_empty() || !port.get_outgoing_edges().is_empty();
        let has_inside_connections = port.has_compound_connections();
        if inside_label_placement {
            if parent_is_compound {
                next_to_port_if_possible && !has_inside_connections
            } else {
                true
            }
        } else {
            next_to_port_if_possible && !has_incident_edge
        }
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
        let labels_next_to_port = if inside_label_placement {
            should_place_next_to_port
        } else {
            next_to_port_if_possible && should_place_next_to_port
        };

        if labels_next_to_port {
            LabelPlacementRelation::Centered
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

    fn port_label_border_offset_for_port_side(
        node: &ElkNodeAdapter,
        side: PortSide,
        horizontal_spacing: f64,
        vertical_spacing: f64,
    ) -> f64 {
        let padding = node.get_padding();
        match side {
            PortSide::North => padding.top + vertical_spacing,
            PortSide::East => padding.right + horizontal_spacing,
            PortSide::South => padding.bottom + vertical_spacing,
            PortSide::West | PortSide::Undefined => padding.left + horizontal_spacing,
        }
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

    fn stack_vertical_side_labels(
        entries: &[(ElkPortAdapter, ElkLabelAdapter)],
        gap_x: f64,
        gap_y: f64,
        avoid_rects: &[Rect],
        restrict_to_port_band: bool,
    ) {
        if entries.is_empty() {
            return;
        }
        if entries.len() <= 1 && avoid_rects.is_empty() {
            return;
        }

        let mut sorted: Vec<(ElkPortAdapter, ElkLabelAdapter)> = if restrict_to_port_band {
            entries
                .iter()
                .filter(|(port, label)| {
                    let port_height = port.get_size().y;
                    let label_pos = label.get_position();
                    let label_size = label.get_size();
                    label_pos.y + label_size.y > 0.0 && label_pos.y < port_height
                })
                .cloned()
                .collect()
        } else {
            entries.to_vec()
        };
        if sorted.is_empty() {
            return;
        }
        if sorted.len() <= 1 && avoid_rects.is_empty() {
            return;
        }

        sorted.sort_by(|(left_port, left_label), (right_port, right_label)| {
            let left_y = left_port.get_position().y + left_label.get_position().y;
            let right_y = right_port.get_position().y + right_label.get_position().y;
            left_y.partial_cmp(&right_y).unwrap_or(Ordering::Equal)
        });

        let mut placed_rectangles: Vec<(i32, f64, f64, f64, f64)> =
            Vec::with_capacity(avoid_rects.len());
        for rect in avoid_rects {
            placed_rectangles.push((i32::MIN, rect.x, rect.y, rect.width, rect.height));
        }

        for (port, label) in sorted {
            let port_position = port.get_position();
            let label_size = label.get_size();
            let mut label_position = label.get_position();
            let port_height = port.get_size().y;
            let port_id = port.get_volatile_id();

            let absolute_x = port_position.x + label_position.x;
            let mut absolute_y = port_position.y + label_position.y;
            let min_abs_y = port_position.y - label_size.y;
            let max_abs_y = port_position.y + port_height;

            loop {
                let mut adjusted = false;
                for (placed_port_id, placed_x, placed_y, placed_w, placed_h) in &placed_rectangles {
                    // Keep intra-port label spacing as defined by the label cell itself.
                    if *placed_port_id == port_id {
                        continue;
                    }
                    let horizontal_overlap = absolute_x < *placed_x + *placed_w + gap_x
                        && absolute_x + label_size.x > *placed_x - gap_x;
                    let vertical_overlap = absolute_y < *placed_y + *placed_h + gap_y
                        && absolute_y + label_size.y > *placed_y - gap_y;

                    if horizontal_overlap && vertical_overlap {
                        absolute_y = *placed_y + *placed_h + gap_y;
                        adjusted = true;
                    }
                }

                if !adjusted {
                    break;
                }
            }

            if restrict_to_port_band {
                if absolute_y < min_abs_y {
                    absolute_y = min_abs_y;
                } else if absolute_y > max_abs_y {
                    absolute_y = max_abs_y;
                }
            }
            label_position.y = absolute_y - port_position.y;
            label.set_position(label_position);
            placed_rectangles.push((port_id, absolute_x, absolute_y, label_size.x, label_size.y));
        }
    }

    fn append_label_rects(
        avoid_rects: &mut Vec<Rect>,
        entries: &[(ElkPortAdapter, ElkLabelAdapter)],
    ) {
        for (port, label) in entries {
            let port_pos = port.get_position();
            let label_pos = label.get_position();
            let label_size = label.get_size();
            avoid_rects.push(Rect {
                x: port_pos.x + label_pos.x,
                y: port_pos.y + label_pos.y,
                width: label_size.x,
                height: label_size.y,
            });
        }
    }

    fn append_label_rects_generic<T, N>(
        node: &N,
        avoid_rects: &mut Vec<Rect>,
        entries: &[(usize, usize)],
    ) where
        N: NodeAdapter<T>,
    {
        let ports = node.get_ports();
        for (port_idx, label_idx) in entries {
            let Some(port) = ports.get(*port_idx) else {
                continue;
            };
            let labels = port.get_labels();
            let Some(label) = labels.get(*label_idx) else {
                continue;
            };
            let port_pos = port.get_position();
            let label_pos = label.get_position();
            let label_size = label.get_size();
            avoid_rects.push(Rect {
                x: port_pos.x + label_pos.x,
                y: port_pos.y + label_pos.y,
                width: label_size.x,
                height: label_size.y,
            });
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

    fn inside_node_label_bounds(node: &ElkNodeAdapter) -> Option<Rect> {
        let node_placement = node
            .get_property(CoreOptions::NODE_LABELS_PLACEMENT)
            .unwrap_or_else(NodeLabelPlacement::fixed);
        let mut bounds: Option<Rect> = None;
        for label in node.get_labels() {
            let placement = if label.has_property(CoreOptions::NODE_LABELS_PLACEMENT) {
                label
                    .get_property(CoreOptions::NODE_LABELS_PLACEMENT)
                    .unwrap_or_else(|| node_placement.clone())
            } else {
                node_placement.clone()
            };
            if !placement.contains(&NodeLabelPlacement::Inside) {
                continue;
            }
            let pos = label.get_position();
            let size = label.get_size();
            let rect = Rect {
                x: pos.x,
                y: pos.y,
                width: size.x,
                height: size.y,
            };
            bounds = Some(bounds.map_or(rect, |existing| existing.union(rect)));
        }
        bounds
    }

    fn avoid_node_label_overlap(
        node: &ElkNodeAdapter,
        port: &ElkPortAdapter,
        mut label_pos: KVector,
        label_size: KVector,
        node_label_bounds: Rect,
        label_gap_horizontal: f64,
        label_gap_vertical: f64,
    ) -> KVector {
        let port_pos = port.get_position();
        let label_rect = Rect {
            x: port_pos.x + label_pos.x,
            y: port_pos.y + label_pos.y,
            width: label_size.x,
            height: label_size.y,
        };
        if !label_rect.overlaps(node_label_bounds) {
            return label_pos;
        }

        match port.get_side() {
            PortSide::East | PortSide::West | PortSide::Undefined => {
                let label_center = label_rect.y + label_rect.height / 2.0;
                let node_center = node_label_bounds.y + node_label_bounds.height / 2.0;
                let new_abs_y = if label_center <= node_center {
                    node_label_bounds.y - label_size.y - label_gap_vertical
                } else {
                    node_label_bounds.y + node_label_bounds.height + label_gap_vertical
                };
                label_pos.y = new_abs_y - port_pos.y;
            }
            PortSide::North | PortSide::South => {
                let label_center = label_rect.x + label_rect.width / 2.0;
                let node_center = node_label_bounds.x + node_label_bounds.width / 2.0;
                let new_abs_x = if label_center <= node_center {
                    node_label_bounds.x - label_size.x - label_gap_horizontal
                } else {
                    node_label_bounds.x + node_label_bounds.width + label_gap_horizontal
                };
                label_pos.x = new_abs_x - port_pos.x;
            }
        }

        let node_size = node.get_size();
        if label_pos.x.is_nan()
            || label_pos.y.is_nan()
            || label_pos.x.is_infinite()
            || label_pos.y.is_infinite()
        {
            return label_pos;
        }
        if label_pos.x + label_size.x < -node_size.x {
            label_pos.x = -node_size.x;
        }
        if label_pos.y + label_size.y < -node_size.y {
            label_pos.y = -node_size.y;
        }

        label_pos
    }

    fn inside_horizontal_label_clamp_bounds_generic<T, N>(node: &N, node_width: f64) -> (f64, f64)
    where
        N: NodeAdapter<T>,
    {
        let width = node_width.max(0.0);
        let mut min_x: f64 = 0.0;
        let mut max_x: f64 = width;

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

    fn inside_node_label_bounds_generic<T, N>(node: &N) -> Option<Rect>
    where
        N: NodeAdapter<T>,
    {
        let node_placement = node
            .get_property(CoreOptions::NODE_LABELS_PLACEMENT)
            .unwrap_or_else(NodeLabelPlacement::fixed);
        let mut bounds: Option<Rect> = None;
        for label in node.get_labels() {
            let placement = if label.has_property(CoreOptions::NODE_LABELS_PLACEMENT) {
                label
                    .get_property(CoreOptions::NODE_LABELS_PLACEMENT)
                    .unwrap_or_else(|| node_placement.clone())
            } else {
                node_placement.clone()
            };
            if !placement.contains(&NodeLabelPlacement::Inside) {
                continue;
            }
            let pos = label.get_position();
            let size = label.get_size();
            let rect = Rect {
                x: pos.x,
                y: pos.y,
                width: size.x,
                height: size.y,
            };
            bounds = Some(bounds.map_or(rect, |existing| existing.union(rect)));
        }
        bounds
    }

    fn avoid_node_label_overlap_generic<T, N>(
        node: &N,
        port: &<N as NodeAdapter<T>>::PortAdapter,
        mut label_pos: KVector,
        label_size: KVector,
        node_label_bounds: Rect,
        label_gap_horizontal: f64,
        label_gap_vertical: f64,
    ) -> KVector
    where
        N: NodeAdapter<T>,
    {
        let port_pos = port.get_position();
        let label_rect = Rect {
            x: port_pos.x + label_pos.x,
            y: port_pos.y + label_pos.y,
            width: label_size.x,
            height: label_size.y,
        };
        if !label_rect.overlaps(node_label_bounds) {
            return label_pos;
        }

        match port.get_side() {
            PortSide::East | PortSide::West | PortSide::Undefined => {
                let label_center = label_rect.y + label_rect.height / 2.0;
                let node_center = node_label_bounds.y + node_label_bounds.height / 2.0;
                let new_abs_y = if label_center <= node_center {
                    node_label_bounds.y - label_size.y - label_gap_vertical
                } else {
                    node_label_bounds.y + node_label_bounds.height + label_gap_vertical
                };
                label_pos.y = new_abs_y - port_pos.y;
            }
            PortSide::North | PortSide::South => {
                let label_center = label_rect.x + label_rect.width / 2.0;
                let node_center = node_label_bounds.x + node_label_bounds.width / 2.0;
                let new_abs_x = if label_center <= node_center {
                    node_label_bounds.x - label_size.x - label_gap_horizontal
                } else {
                    node_label_bounds.x + node_label_bounds.width + label_gap_horizontal
                };
                label_pos.x = new_abs_x - port_pos.x;
            }
        }

        let node_size = node.get_size();
        if label_pos.x.is_nan()
            || label_pos.y.is_nan()
            || label_pos.x.is_infinite()
            || label_pos.y.is_infinite()
        {
            return label_pos;
        }
        if label_pos.x + label_size.x < -node_size.x {
            label_pos.x = -node_size.x;
        }
        if label_pos.y + label_size.y < -node_size.y {
            label_pos.y = -node_size.y;
        }

        label_pos
    }

    fn stack_horizontal_side_labels_generic<T, N>(
        node: &N,
        entries: &[(usize, usize)],
        config: HorizontalStackConfig,
    ) where
        N: NodeAdapter<T>,
    {
        if entries.is_empty() {
            return;
        }

        if entries.len() <= 2 && config.clamp_x.is_none() {
            return;
        }

        let ports = node.get_ports();

        // Build (port, label, absolute_x) tuples for sorting
        let mut sortable_entries: Vec<(usize, usize, f64)> = Vec::new();
        for &(port_idx, label_idx) in entries {
            let port = &ports[port_idx];
            let labels = port.get_labels();
            let label = &labels[label_idx];
            let port_position = port.get_position();
            let label_position = label.get_position();
            let absolute_x = port_position.x + label_position.x;
            sortable_entries.push((port_idx, label_idx, absolute_x));
        }

        sortable_entries.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal));

        let mut placed_rectangles: Vec<(f64, f64, f64, f64)> = Vec::new();
        for (port_idx, label_idx, _) in sortable_entries {
            let port = &ports[port_idx];
            let labels = port.get_labels();
            let label = &labels[label_idx];

            let port_position = port.get_position();
            let port_size = port.get_size();
            let label_size = label.get_size();
            let mut label_position = label.get_position();

            let mut absolute_x = port_position.x + label_position.x;
            if let Some((min_x, max_x)) = config.clamp_x {
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

    fn stack_vertical_side_labels_generic<T, N>(
        node: &N,
        entries: &[(usize, usize)],
        gap_x: f64,
        gap_y: f64,
        avoid_rects: &[Rect],
        restrict_to_port_band: bool,
    ) where
        N: NodeAdapter<T>,
    {
        if entries.is_empty() {
            return;
        }
        if entries.len() <= 1 && avoid_rects.is_empty() {
            return;
        }

        let ports = node.get_ports();
        let mut sortable_entries: Vec<(usize, usize, f64)> = Vec::new();
        for &(port_idx, label_idx) in entries {
            let port = &ports[port_idx];
            let labels = port.get_labels();
            let label = &labels[label_idx];
            if restrict_to_port_band {
                let port_height = port.get_size().y;
                let label_position = label.get_position();
                let label_size = label.get_size();
                if label_position.y + label_size.y <= 0.0 || label_position.y >= port_height {
                    continue;
                }
            }
            let port_position = port.get_position();
            let label_position = label.get_position();
            let absolute_y = port_position.y + label_position.y;
            sortable_entries.push((port_idx, label_idx, absolute_y));
        }

        if sortable_entries.is_empty() {
            return;
        }
        if sortable_entries.len() <= 1 && avoid_rects.is_empty() {
            return;
        }

        sortable_entries.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal));

        let mut placed_rectangles: Vec<(usize, f64, f64, f64, f64)> =
            Vec::with_capacity(avoid_rects.len());
        for rect in avoid_rects {
            placed_rectangles.push((usize::MAX, rect.x, rect.y, rect.width, rect.height));
        }

        for (port_idx, label_idx, _) in sortable_entries {
            let port = &ports[port_idx];
            let labels = port.get_labels();
            let label = &labels[label_idx];

            let port_position = port.get_position();
            let port_height = port.get_size().y;
            let label_size = label.get_size();
            let mut label_position = label.get_position();

            let absolute_x = port_position.x + label_position.x;
            let mut absolute_y = port_position.y + label_position.y;
            let min_abs_y = port_position.y - label_size.y;
            let max_abs_y = port_position.y + port_height;

            loop {
                let mut adjusted = false;
                for (placed_port_idx, placed_x, placed_y, placed_w, placed_h) in &placed_rectangles {
                    // Keep intra-port label spacing as defined by the label cell itself.
                    if *placed_port_idx == port_idx {
                        continue;
                    }
                    let horizontal_overlap = absolute_x < *placed_x + *placed_w + gap_x
                        && absolute_x + label_size.x > *placed_x - gap_x;
                    let vertical_overlap = absolute_y < *placed_y + *placed_h + gap_y
                        && absolute_y + label_size.y > *placed_y - gap_y;

                    if horizontal_overlap && vertical_overlap {
                        absolute_y = *placed_y + *placed_h + gap_y;
                        adjusted = true;
                    }
                }

                if !adjusted {
                    break;
                }
            }

            if restrict_to_port_band {
                if absolute_y < min_abs_y {
                    absolute_y = min_abs_y;
                } else if absolute_y > max_abs_y {
                    absolute_y = max_abs_y;
                }
            }
            label_position.y = absolute_y - port_position.y;
            label.set_position(label_position);
            placed_rectangles.push((port_idx, absolute_x, absolute_y, label_size.x, label_size.y));
        }
    }

    fn compute_stack_start_coordinate_generic<T, N>(
        node: &N,
        entries: &[(usize, usize)],
        positive_direction: bool,
        gap_y: f64,
    ) -> Option<f64>
    where
        N: NodeAdapter<T>,
    {
        if entries.is_empty() {
            return None;
        }

        let ports = node.get_ports();

        if positive_direction {
            let baseline = entries
                .iter()
                .map(|(port_idx, _)| {
                    let port = &ports[*port_idx];
                    let position = port.get_position();
                    let size = port.get_size();
                    position.y + size.y
                })
                .fold(f64::NEG_INFINITY, f64::max);
            Some(baseline + gap_y)
        } else {
            let baseline = entries
                .iter()
                .map(|(port_idx, _)| {
                    let port = &ports[*port_idx];
                    port.get_position().y
                })
                .fold(f64::INFINITY, f64::min);
            Some(baseline - gap_y)
        }
    }

    /// Returns true when a node needs `process_node()` instead of `process()` because
    /// it has fixed size constraints AND ports with labels at non-zero positions that
    /// extend into the node interior.
    ///
    /// This handles compound nodes whose inner layout has already set their size but
    /// didn't account for fixed port labels placed by a prior ELK-level layout pass.
    /// `process()` respects `are_size_constraints_fixed` and keeps the current size,
    /// but `process_node()` always computes the full size including port label space.
    ///
    /// Nodes with port labels at default (0,0) positions (e.g., freshly parsed models)
    /// don't need this fallback — `process()` handles them correctly.
    fn needs_process_node_for_fixed_port_labels<N, T>(node: &N) -> bool
    where
        T: 'static,
        N: NodeAdapter<T>,
        N::Port: 'static,
        N::PortAdapter: 'static,
    {
        let size_constraints = node
            .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
            .unwrap_or_default();
        let is_fixed = size_constraints.is_empty()
            || size_constraints == EnumSet::of(&[SizeConstraint::PortLabels]);
        if !is_fixed {
            return false;
        }

        let port_labels_placement = node
            .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
            .unwrap_or_default();
        if !PortLabelPlacement::is_fixed(&port_labels_placement) {
            return false;
        }

        // Check if any port has labels at non-zero positions (indicating they were
        // placed by a prior layout pass and may extend into the node).
        for port in node.get_ports() {
            for label in port.get_labels() {
                let pos = label.get_position();
                if pos.x.abs() > 0.01 || pos.y.abs() > 0.01 {
                    return true;
                }
            }
        }

        false
    }
}

#[derive(Clone, Copy)]
enum LabelPlacementRelation {
    Centered,
    BelowOrRight,
    AboveOrLeft,
}
