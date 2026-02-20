use std::collections::HashSet;
use std::sync::{Arc, LazyLock};

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::elk_margin::ElkMargin;
use org_eclipse_elk_core::org::eclipse::elk::core::math::elk_rectangle::ElkRectangle;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_alignment::PortAlignment;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_label_placement::PortLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, IElkProgressMonitor};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::nodespacing::node_dimension_calculation::NodeDimensionCalculation;

use crate::org::eclipse::elk::alg::layered::graph::transform::LGraphAdapters;
use crate::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphUtil, LNodeRef, LPortRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::graph_properties::GraphProperties;
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};

pub struct LabelAndNodeSizeProcessor;

static TRACE_NODE_SIZE: LazyLock<bool> =
    LazyLock::new(|| std::env::var("ELK_TRACE_NODE_SIZE").is_ok());
static ENABLE_PHASE1_PORT_PLACEMENT: LazyLock<bool> = LazyLock::new(|| {
    std::env::var("ELK_LAYERED_ENABLE_LABEL_NODE_PHASE1")
        .map(|value| !(value == "0" || value.eq_ignore_ascii_case("false")))
        .unwrap_or(true)
});

impl Default for LabelAndNodeSizeProcessor {
    fn default() -> Self {
        LabelAndNodeSizeProcessor
    }
}

impl ILayoutProcessor<LGraph> for LabelAndNodeSizeProcessor {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Node and Port Label Placement and Node Sizing", 1.0);

        let graph_port_spacing = graph
            .get_property(LayeredOptions::SPACING_PORT_PORT)
            .unwrap_or(10.0);
        let graph_ports_surrounding = graph
            .get_property(LayeredOptions::SPACING_PORTS_SURROUNDING)
            .unwrap_or_default();
        let graph_topdown_layout = graph
            .get_property(CoreOptions::TOPDOWN_LAYOUT)
            .unwrap_or(false);
        let graph_node_size_fixed_graph_size = graph
            .get_property(CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE)
            .unwrap_or(false);

        // Step 1: Calculate node sizes and labels via NodeDimensionCalculation (matching Java)
        // Java's LabelAndNodeSizeProcessor calls this FIRST, then port positions are set by
        // the internal PortPlacementCalculator within process_node. Since Rust's process_node
        // doesn't have PortPlacementCalculator, we do node sizing first, then port placement.
        if *TRACE_NODE_SIZE {
            eprintln!("label-node-size: step1 (node sizing) begin");
        }
        let adapter = LGraphAdapters::adapt(graph, true, true, |node| {
            node.node_type() == NodeType::Normal
        });
        NodeDimensionCalculation::calculate_label_and_node_sizes(&adapter);
        if *TRACE_NODE_SIZE {
            eprintln!("label-node-size: step1 (node sizing) done");
        }

        // Step 2: Port placement workaround (not present in Java).
        // Keep enabled by default for current behavior; allow disabling for parity experiments.
        if *ENABLE_PHASE1_PORT_PLACEMENT {
            if *TRACE_NODE_SIZE {
                eprintln!("label-node-size: step2 (port placement) begin");
            }
            let mut seen = HashSet::new();
            for node in graph.layerless_nodes().clone() {
                let key = Arc::as_ptr(&node) as usize;
                if seen.insert(key) && should_apply_phase1_port_placement(&node) {
                    place_ports_on_node(
                        &node,
                        graph_port_spacing,
                        &graph_ports_surrounding,
                        graph_topdown_layout,
                        graph_node_size_fixed_graph_size,
                    );
                }
            }

            for layer in graph.layers().clone() {
                let nodes = layer
                    .lock()
                    .ok()
                    .map(|layer_guard| LGraphUtil::to_node_array(layer_guard.nodes()))
                    .unwrap_or_default();
                for node in nodes {
                    let key = Arc::as_ptr(&node) as usize;
                    if seen.insert(key) && should_apply_phase1_port_placement(&node) {
                        place_ports_on_node(
                            &node,
                            graph_port_spacing,
                            &graph_ports_surrounding,
                            graph_topdown_layout,
                            graph_node_size_fixed_graph_size,
                        );
                    }
                }
            }
            if *TRACE_NODE_SIZE {
                eprintln!("label-node-size: step2 (port placement) done");
            }
            let should_run_phase2b = graph_needs_phase2b_inside_port_label_restack(graph);
            if should_run_phase2b {
                if *TRACE_NODE_SIZE {
                    eprintln!("label-node-size: phase2b (inside port label restack) begin");
                }
                NodeDimensionCalculation::calculate_label_and_node_sizes(&adapter);
                if *TRACE_NODE_SIZE {
                    eprintln!("label-node-size: phase2b (inside port label restack) done");
                }
            } else if *TRACE_NODE_SIZE {
                eprintln!("label-node-size: phase2b skipped (no inside port labels)");
            }

            // Java parity guard: phase2b can shrink nodes again after port placement.
            // Re-apply port-driven sizing for affected nodes only:
            // 1) self-loop helper-port holders (existing guard)
            // 2) nodes whose ports no longer fit on their side axis after phase2b
            let mut phase2c_reapplied_nodes = 0usize;
            let mut seen = HashSet::new();
            for node in graph.layerless_nodes().clone() {
                let key = Arc::as_ptr(&node) as usize;
                if seen.insert(key)
                    && (should_reapply_phase2_self_loop_port_sizing(&node)
                        || should_reapply_phase2_port_axis_overflow_sizing(&node))
                {
                    place_ports_on_node(
                        &node,
                        graph_port_spacing,
                        &graph_ports_surrounding,
                        graph_topdown_layout,
                        graph_node_size_fixed_graph_size,
                    );
                    phase2c_reapplied_nodes += 1;
                }
            }

            for layer in graph.layers().clone() {
                let nodes = layer
                    .lock()
                    .ok()
                    .map(|layer_guard| LGraphUtil::to_node_array(layer_guard.nodes()))
                .unwrap_or_default();
                for node in nodes {
                    let key = Arc::as_ptr(&node) as usize;
                    if seen.insert(key)
                        && (should_reapply_phase2_self_loop_port_sizing(&node)
                            || should_reapply_phase2_port_axis_overflow_sizing(&node))
                    {
                        place_ports_on_node(
                            &node,
                            graph_port_spacing,
                            &graph_ports_surrounding,
                            graph_topdown_layout,
                            graph_node_size_fixed_graph_size,
                        );
                        phase2c_reapplied_nodes += 1;
                    }
                }
            }
            if *TRACE_NODE_SIZE {
                eprintln!(
                    "label-node-size: phase2c (self-loop port sizing reapply) nodes={}",
                    phase2c_reapplied_nodes
                );
            }
        } else if *TRACE_NODE_SIZE {
            eprintln!("label-node-size: step2 skipped (experiment)");
        }

        // Phase 3: If the graph has external ports, handle labels of external port dummies
        if *TRACE_NODE_SIZE {
            eprintln!("label-node-size: phase3 begin");
        }
        let has_external_ports = graph
            .get_property(InternalProperties::GRAPH_PROPERTIES)
            .map(|props| props.contains(&GraphProperties::ExternalPorts))
            .unwrap_or(false);

        if has_external_ports {
            let port_label_placement = graph
                .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                .unwrap_or_default();
            let place_next_to_port =
                port_label_placement.contains(&PortLabelPlacement::NextToPortIfPossible);
            let treat_as_group = graph
                .get_property(CoreOptions::PORT_LABELS_TREAT_AS_GROUP)
                .unwrap_or(true);

            for layer in graph.layers().clone() {
                let nodes: Vec<LNodeRef> = layer
                    .lock()
                    .ok()
                    .map(|layer_guard| {
                        layer_guard
                            .nodes()
                            .iter()
                            .filter(|node| {
                                node.lock()
                                    .ok()
                                    .map(|g| g.node_type() == NodeType::ExternalPort)
                                    .unwrap_or(false)
                            })
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();

                for dummy in nodes {
                    place_external_port_dummy_labels(
                        &dummy,
                        &port_label_placement,
                        place_next_to_port,
                        treat_as_group,
                    );
                }
            }
        }
        if *TRACE_NODE_SIZE {
            eprintln!("label-node-size: phase3 done");
        }

        monitor.done();
    }
}

// ============================================================
// Phase 1: Port placement
// ============================================================

fn should_apply_phase1_port_placement(node: &LNodeRef) -> bool {
    !node
        .lock()
        .ok()
        .and_then(|mut node_guard| node_guard.get_property(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE))
        .unwrap_or(false)
}

fn should_reapply_phase2_self_loop_port_sizing(node: &LNodeRef) -> bool {
    if !should_apply_phase1_port_placement(node) {
        return false;
    }
    node.lock().ok().is_some_and(|mut node_guard| {
        let has_self_loop_holder = node_guard
            .get_property(InternalProperties::SELF_LOOP_HOLDER)
            .is_some();
        let size_constraints = node_guard
            .get_property(LayeredOptions::NODE_SIZE_CONSTRAINTS)
            .unwrap_or_default();
        has_self_loop_holder && size_constraints.contains(&SizeConstraint::Ports)
    })
}

fn should_reapply_phase2_port_axis_overflow_sizing(node: &LNodeRef) -> bool {
    if !should_apply_phase1_port_placement(node) {
        return false;
    }

    let (node_size, size_constraints, port_constraints, ports) = match node.lock() {
        Ok(mut node_guard) => (
            *node_guard.shape().size_ref(),
            node_guard
                .get_property(LayeredOptions::NODE_SIZE_CONSTRAINTS)
                .unwrap_or_default(),
            node_guard
                .get_property(LayeredOptions::PORT_CONSTRAINTS)
                .unwrap_or(PortConstraints::Undefined),
            node_guard.ports().clone(),
        ),
        Err(_) => return false,
    };

    if !size_constraints.contains(&SizeConstraint::Ports)
        && !size_constraints.contains(&SizeConstraint::PortLabels)
    {
        return false;
    }

    if port_constraints.is_pos_fixed() || port_constraints.is_ratio_fixed() {
        return false;
    }

    const EPS: f64 = 1e-6;
    for port in ports {
        let (side, pos, size) = match port.lock() {
            Ok(mut port_guard) => (
                port_guard.side(),
                *port_guard.shape().position_ref(),
                *port_guard.shape().size_ref(),
            ),
            Err(_) => continue,
        };

        let overflows = match side {
            PortSide::North | PortSide::South => {
                pos.x < -EPS || pos.x + size.x > node_size.x + EPS
            }
            PortSide::East | PortSide::West => {
                pos.y < -EPS || pos.y + size.y > node_size.y + EPS
            }
            PortSide::Undefined => false,
        };
        if overflows {
            return true;
        }
    }

    false
}

fn place_ports_on_node(
    node: &LNodeRef,
    graph_port_spacing: f64,
    graph_ports_surrounding: &ElkMargin,
    graph_topdown_layout: bool,
    graph_node_size_fixed_graph_size: bool,
) {
    let (
        node_type,
        mut node_size,
        port_constraints,
        inside_self_loops_active,
        size_constraints,
        port_labels_are_fixed,
        topdown_layout,
        node_size_fixed_graph_size,
    ) = match node.lock() {
        Ok(mut node_guard) => {
            let topdown_layout = node_property_or_graph_default(
                &mut node_guard,
                CoreOptions::TOPDOWN_LAYOUT,
                graph_topdown_layout,
            );
            let node_size_fixed_graph_size = node_property_or_graph_default(
                &mut node_guard,
                CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE,
                graph_node_size_fixed_graph_size,
            );
            (
                node_guard.node_type(),
                *node_guard.shape().size_ref(),
                node_guard
                    .get_property(LayeredOptions::PORT_CONSTRAINTS)
                    .unwrap_or(PortConstraints::Undefined),
                node_guard
                    .get_property(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE)
                    .unwrap_or(false),
                node_guard
                    .get_property(LayeredOptions::NODE_SIZE_CONSTRAINTS)
                    .unwrap_or_default(),
                PortLabelPlacement::is_fixed(
                    &node_guard
                        .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                        .unwrap_or_default(),
                ),
                topdown_layout,
                node_size_fixed_graph_size,
            )
        }
        Err(_) => return,
    };
    if *TRACE_NODE_SIZE {
        let id = node
            .lock()
            .ok()
            .map(|mut node_guard| node_guard.shape().graph_element().id)
            .unwrap_or(-1);
        let (n_count, e_count, s_count, w_count, u_count) = node
            .lock()
            .ok()
            .map(|mut node_guard| {
                (
                    node_guard.port_side_view(PortSide::North).len(),
                    node_guard.port_side_view(PortSide::East).len(),
                    node_guard.port_side_view(PortSide::South).len(),
                    node_guard.port_side_view(PortSide::West).len(),
                    node_guard.port_side_view(PortSide::Undefined).len(),
                )
            })
            .unwrap_or((0, 0, 0, 0, 0));
        eprintln!(
            "label-node-size: node id={} size=({}, {}) constraints={:?} port_constraints={:?} sides(N/E/S/W/U)=({}/{}/{}/{}/{})",
            id,
            node_size.x,
            node_size.y,
            size_constraints,
            port_constraints,
            n_count,
            e_count,
            s_count,
            w_count,
            u_count
        );
    }

    if node_type != NodeType::Normal {
        return;
    }

    if std::env::var("ELK_DISABLE_CLOCKWISE_SIDE_ORDER").is_err() {
        ensure_clockwise_port_order(node, port_constraints);
    }

    let allow_shrink = !topdown_layout
        && !node_size_fixed_graph_size
        && !(port_labels_are_fixed && size_constraints.contains(&SizeConstraint::PortLabels));

    if size_constraints.contains(&SizeConstraint::Ports) {
        node_size = enforce_port_driven_minimum_size(
            node,
            node_size,
            graph_port_spacing,
            graph_ports_surrounding,
            size_constraints.contains(&SizeConstraint::PortLabels),
            allow_shrink,
        );
    }
    if size_constraints.contains(&SizeConstraint::PortLabels) {
        node_size = enforce_inside_port_label_minimum_size(
            node,
            node_size,
            graph_port_spacing,
            graph_ports_surrounding,
            allow_shrink,
        );
    }

    if inside_self_loops_active {
        place_inside_self_loop_ports(node, node_size.x, node_size.y);
        update_node_margin(node);
        return;
    }

    if port_constraints.is_pos_fixed() {
        // Java's FIXED_POS path keeps the axis-aligned coordinate and snaps only the
        // border coordinate according to side/border-offset.
        adjust_ports_on_side(node, PortSide::North, node_size.x, node_size.y);
        adjust_ports_on_side(node, PortSide::South, node_size.x, node_size.y);
        adjust_ports_on_side(node, PortSide::East, node_size.x, node_size.y);
        adjust_ports_on_side(node, PortSide::West, node_size.x, node_size.y);
        update_node_margin(node);
        return;
    }

    if port_constraints.is_ratio_fixed() {
        place_ports_fixed_ratio_on_side(node, PortSide::North, node_size.x, node_size.y);
        place_ports_fixed_ratio_on_side(node, PortSide::South, node_size.x, node_size.y);
        place_ports_fixed_ratio_on_side(node, PortSide::East, node_size.x, node_size.y);
        place_ports_fixed_ratio_on_side(node, PortSide::West, node_size.x, node_size.y);
        update_node_margin(node);
        return;
    }

    place_ports_on_side(
        node,
        PortSide::North,
        node_size.x,
        node_size.y,
        graph_port_spacing,
        graph_ports_surrounding,
    );
    place_ports_on_side(
        node,
        PortSide::South,
        node_size.x,
        node_size.y,
        graph_port_spacing,
        graph_ports_surrounding,
    );
    place_ports_on_side(
        node,
        PortSide::East,
        node_size.x,
        node_size.y,
        graph_port_spacing,
        graph_ports_surrounding,
    );
    place_ports_on_side(
        node,
        PortSide::West,
        node_size.x,
        node_size.y,
        graph_port_spacing,
        graph_ports_surrounding,
    );
    update_node_margin(node);
}

fn place_inside_self_loop_ports(node: &LNodeRef, width: f64, height: f64) {
    adjust_ports_on_side(node, PortSide::North, width, height);
    adjust_ports_on_side(node, PortSide::South, width, height);

    let center_y = height / 2.0;
    let west_ports = node
        .lock()
        .ok()
        .map(|mut node_guard| node_guard.port_side_view(PortSide::West))
        .unwrap_or_default();
    for port in west_ports {
        if let Ok(mut port_guard) = port.lock() {
            let port_size = *port_guard.shape().size_ref();
            let pos = port_guard.shape().position();
            pos.x = -port_size.x;
            pos.y = center_y - port_size.y / 2.0;
        }
    }

    let east_ports = node
        .lock()
        .ok()
        .map(|mut node_guard| node_guard.port_side_view(PortSide::East))
        .unwrap_or_default();
    for port in east_ports {
        if let Ok(mut port_guard) = port.lock() {
            let port_size = *port_guard.shape().size_ref();
            let pos = port_guard.shape().position();
            pos.x = width;
            pos.y = center_y - port_size.y / 2.0;
        }
    }
}

fn graph_needs_phase2b_inside_port_label_restack(graph: &LGraph) -> bool {
    let mut seen = HashSet::new();
    for node in graph.layerless_nodes().clone() {
        let key = Arc::as_ptr(&node) as usize;
        if seen.insert(key) && node_has_inside_port_label_constraints(&node) {
            return true;
        }
    }

    for layer in graph.layers().clone() {
        let nodes = layer
            .lock()
            .ok()
            .map(|layer_guard| LGraphUtil::to_node_array(layer_guard.nodes()))
            .unwrap_or_default();
        for node in nodes {
            let key = Arc::as_ptr(&node) as usize;
            if seen.insert(key) && node_has_inside_port_label_constraints(&node) {
                return true;
            }
        }
    }

    false
}

fn node_has_inside_port_label_constraints(node: &LNodeRef) -> bool {
    node.lock().ok().is_some_and(|mut node_guard| {
        let placement = node_guard
            .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
            .unwrap_or_default();
        let size_constraints = node_guard
            .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
            .unwrap_or_default();
        placement.contains(&PortLabelPlacement::Inside)
            && size_constraints.contains(&SizeConstraint::PortLabels)
    })
}

fn update_node_margin(node: &LNodeRef) {
    let (node_width, node_height, labels, ports) = match node.lock() {
        Ok(mut node_guard) => (
            node_guard.shape().size_ref().x,
            node_guard.shape().size_ref().y,
            node_guard.labels().clone(),
            node_guard.ports().clone(),
        ),
        Err(_) => return,
    };

    let mut margin_top = 0.0_f64;
    let mut margin_right = 0.0_f64;
    let mut margin_bottom = 0.0_f64;
    let mut margin_left = 0.0_f64;

    let mut include_rect = |x: f64, y: f64, width: f64, height: f64| {
        if x < 0.0 {
            margin_left = margin_left.max(-x);
        }
        if y < 0.0 {
            margin_top = margin_top.max(-y);
        }
        let right = x + width;
        if right > node_width {
            margin_right = margin_right.max(right - node_width);
        }
        let bottom = y + height;
        if bottom > node_height {
            margin_bottom = margin_bottom.max(bottom - node_height);
        }
    };

    for label in labels {
        if let Ok(mut label_guard) = label.lock() {
            let pos = *label_guard.shape().position_ref();
            let size = *label_guard.shape().size_ref();
            include_rect(pos.x, pos.y, size.x, size.y);
        }
    }

    for port in ports {
        if let Ok(mut port_guard) = port.lock() {
            let port_pos = *port_guard.shape().position_ref();
            let port_size = *port_guard.shape().size_ref();
            include_rect(port_pos.x, port_pos.y, port_size.x, port_size.y);

            for label in port_guard.labels().clone() {
                if let Ok(mut label_guard) = label.lock() {
                    let label_pos = *label_guard.shape().position_ref();
                    let label_size = *label_guard.shape().size_ref();
                    include_rect(
                        port_pos.x + label_pos.x,
                        port_pos.y + label_pos.y,
                        label_size.x,
                        label_size.y,
                    );
                }
            }
        }
    }

    if let Ok(mut node_guard) = node.lock() {
        let margin = node_guard.margin();
        margin.top = margin_top;
        margin.right = margin_right;
        margin.bottom = margin_bottom;
        margin.left = margin_left;
    }
}

#[derive(Clone)]
struct PortPlacementContext {
    port: LPortRef,
    margin: ElkMargin,
    labels_next_to_port: bool,
    label_size: KVector,
    label_count: usize,
}

fn port_label_min_size(
    port: &LPortRef,
    label_label_spacing: f64,
    port_labels_fixed: bool,
) -> (KVector, usize) {
    if port_labels_fixed {
        return (KVector::new(), 0);
    }

    let labels = port
        .lock()
        .ok()
        .map(|port_guard| port_guard.labels().clone())
        .unwrap_or_default();
    if labels.is_empty() {
        return (KVector::new(), 0);
    }

    let mut max_width = 0.0_f64;
    let mut total_height = 0.0_f64;
    let mut count = 0_usize;
    for label in labels {
        if let Ok(mut label_guard) = label.lock() {
            let size = *label_guard.shape().size_ref();
            max_width = max_width.max(size.x);
            total_height += size.y;
            count += 1;
        }
    }
    if count > 1 {
        total_height += label_label_spacing * (count.saturating_sub(1) as f64);
    }

    (KVector::with_values(max_width, total_height), count)
}

fn labels_bounds_for_port(port: &LPortRef) -> Option<ElkRectangle> {
    let labels = port
        .lock()
        .ok()
        .map(|port_guard| port_guard.labels().clone())
        .unwrap_or_default();
    if labels.is_empty() {
        return None;
    }

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for label in labels {
        if let Ok(mut label_guard) = label.lock() {
            let pos = *label_guard.shape().position_ref();
            let size = *label_guard.shape().size_ref();
            min_x = min_x.min(pos.x);
            min_y = min_y.min(pos.y);
            max_x = max_x.max(pos.x + size.x);
            max_y = max_y.max(pos.y + size.y);
        }
    }

    if !min_x.is_finite() || !min_y.is_finite() {
        return None;
    }

    Some(ElkRectangle::with_values(
        min_x,
        min_y,
        max_x - min_x,
        max_y - min_y,
    ))
}

fn labels_next_to_port(
    placement: &EnumSet<PortLabelPlacement>,
    port_labels_next_to_port: bool,
    node_is_compound: bool,
    has_compound_connections: bool,
    has_edges: bool,
) -> bool {
    if placement.contains(&PortLabelPlacement::Inside) {
        if node_is_compound {
            port_labels_next_to_port && !has_compound_connections
        } else {
            true
        }
    } else if placement.contains(&PortLabelPlacement::Outside) {
        if port_labels_next_to_port {
            !has_edges
        } else {
            false
        }
    } else {
        false
    }
}

#[allow(clippy::too_many_arguments)]
fn setup_horizontal_port_margins(
    contexts: &mut [PortPlacementContext],
    port_labels_outside: bool,
    always_same_side: bool,
    always_other_same_side: bool,
    space_efficient: bool,
    uniform_port_spacing: bool,
    port_label_spacing_horizontal: f64,
    port_labels_fixed: bool,
) {
    let space_efficient_labels =
        !always_same_side && !always_other_same_side && (space_efficient || contexts.len() == 2);

    for ctx in contexts.iter_mut() {
        let label_width = ctx.label_size.x;
        if label_width > 0.0 {
            if ctx.labels_next_to_port {
                if let Ok(mut port_guard) = ctx.port.lock() {
                    let port_width = port_guard.shape().size_ref().x;
                    if label_width > port_width {
                        let overhang = (label_width - port_width) / 2.0;
                        ctx.margin.left = overhang;
                        ctx.margin.right = overhang;
                    }
                }
            } else {
                ctx.margin.right = port_label_spacing_horizontal + label_width;
            }
        } else if port_labels_fixed {
            if let Some(bounds) = labels_bounds_for_port(&ctx.port) {
                if let Ok(mut port_guard) = ctx.port.lock() {
                    let port_width = port_guard.shape().size_ref().x;
                    if bounds.x < 0.0 {
                        ctx.margin.left = -bounds.x;
                    }
                    if bounds.x + bounds.width > port_width {
                        ctx.margin.right = bounds.x + bounds.width - port_width;
                    }
                }
            }
        }
    }

    if port_labels_outside && !contexts.is_empty() {
        let leftmost = 0;
        let rightmost = contexts.len() - 1;
        contexts[leftmost].margin.left = 0.0;
        contexts[rightmost].margin.right = 0.0;

        if space_efficient_labels && !contexts[leftmost].labels_next_to_port {
            contexts[leftmost].margin.right = 0.0;
        }
    }

    if uniform_port_spacing && !contexts.is_empty() {
        let mut max_left = 0.0_f64;
        let mut max_right = 0.0_f64;
        for ctx in contexts.iter() {
            max_left = max_left.max(ctx.margin.left);
            max_right = max_right.max(ctx.margin.right);
        }
        for ctx in contexts.iter_mut() {
            ctx.margin.left = max_left;
            ctx.margin.right = max_right;
        }

        if port_labels_outside {
            let leftmost = 0;
            let rightmost = contexts.len() - 1;
            contexts[leftmost].margin.left = 0.0;
            contexts[rightmost].margin.right = 0.0;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn setup_vertical_port_margins(
    contexts: &mut [PortPlacementContext],
    port_labels_outside: bool,
    always_same_side: bool,
    always_other_same_side: bool,
    space_efficient: bool,
    uniform_port_spacing: bool,
    port_label_spacing_vertical: f64,
    port_labels_fixed: bool,
    port_labels_treat_as_group: bool,
) {
    let space_efficient_labels =
        !always_same_side && !always_other_same_side && (space_efficient || contexts.len() == 2);

    for ctx in contexts.iter_mut() {
        let label_height = ctx.label_size.y;
        if label_height > 0.0 {
            if ctx.labels_next_to_port {
                if let Ok(mut port_guard) = ctx.port.lock() {
                    let port_height = port_guard.shape().size_ref().y;
                    if label_height > port_height {
                        if port_labels_treat_as_group || ctx.label_count <= 1 {
                            let overhang = (label_height - port_height) / 2.0;
                            ctx.margin.top = overhang;
                            ctx.margin.bottom = overhang;
                        } else {
                            let first_label_height =
                                ctx.port
                                    .lock()
                                    .ok()
                                    .and_then(|port_guard| {
                                        port_guard.labels().first().and_then(|label| {
                                            label.lock().ok().map(|mut label_guard| {
                                                label_guard.shape().size_ref().y
                                            })
                                        })
                                    })
                                    .unwrap_or(0.0);
                            let first_overhang = (first_label_height - port_height) / 2.0;
                            ctx.margin.top = first_overhang.max(0.0);
                            ctx.margin.bottom = label_height - first_overhang - port_height;
                        }
                    }
                }
            } else {
                ctx.margin.bottom = port_label_spacing_vertical + label_height;
            }
        } else if port_labels_fixed {
            if let Some(bounds) = labels_bounds_for_port(&ctx.port) {
                if let Ok(mut port_guard) = ctx.port.lock() {
                    let port_height = port_guard.shape().size_ref().y;
                    if bounds.y < 0.0 {
                        ctx.margin.top = -bounds.y;
                    }
                    if bounds.y + bounds.height > port_height {
                        ctx.margin.bottom = bounds.y + bounds.height - port_height;
                    }
                }
            }
        }
    }

    if port_labels_outside && !contexts.is_empty() {
        let topmost = 0;
        let bottommost = contexts.len() - 1;
        contexts[topmost].margin.top = 0.0;
        contexts[bottommost].margin.bottom = 0.0;

        if space_efficient_labels && !contexts[topmost].labels_next_to_port {
            contexts[topmost].margin.bottom = 0.0;
        }
    }

    if uniform_port_spacing && !contexts.is_empty() {
        let mut max_top = 0.0_f64;
        let mut max_bottom = 0.0_f64;
        for ctx in contexts.iter() {
            max_top = max_top.max(ctx.margin.top);
            max_bottom = max_bottom.max(ctx.margin.bottom);
        }
        for ctx in contexts.iter_mut() {
            ctx.margin.top = max_top;
            ctx.margin.bottom = max_bottom;
        }

        if port_labels_outside {
            let topmost = 0;
            let bottommost = contexts.len() - 1;
            contexts[topmost].margin.top = 0.0;
            contexts[bottommost].margin.bottom = 0.0;
        }
    }
}

fn place_ports_on_side(
    node: &LNodeRef,
    side: PortSide,
    width: f64,
    height: f64,
    graph_port_spacing: f64,
    graph_ports_surrounding: &ElkMargin,
) {
    let (
        ports,
        mut alignment,
        port_spacing,
        surrounding_spacing,
        constraints,
        size_constraints,
        size_options,
        port_label_placement,
        port_labels_treat_as_group,
        label_label_spacing,
        label_port_spacing_horizontal,
        label_port_spacing_vertical,
        node_is_compound,
    ) = node
        .lock()
        .ok()
        .map(|mut node_guard| {
            (
                node_guard.port_side_view(side),
                port_alignment_for_side(&mut node_guard, side),
                property_with_graph_default(
                    &mut node_guard,
                    CoreOptions::SPACING_PORT_PORT,
                    graph_port_spacing,
                ),
                property_with_graph_default(
                    &mut node_guard,
                    CoreOptions::SPACING_PORTS_SURROUNDING,
                    graph_ports_surrounding.clone(),
                ),
                node_guard
                    .get_property(CoreOptions::PORT_CONSTRAINTS)
                    .unwrap_or(PortConstraints::Undefined),
                node_guard
                    .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
                    .unwrap_or_default(),
                node_guard
                    .get_property(CoreOptions::NODE_SIZE_OPTIONS)
                    .unwrap_or_default(),
                node_guard
                    .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                    .unwrap_or_default(),
                node_guard
                    .get_property(CoreOptions::PORT_LABELS_TREAT_AS_GROUP)
                    .unwrap_or(true),
                property_with_graph_default(&mut node_guard, CoreOptions::SPACING_LABEL_LABEL, 0.0),
                property_with_graph_default(
                    &mut node_guard,
                    CoreOptions::SPACING_LABEL_PORT_HORIZONTAL,
                    0.0,
                ),
                property_with_graph_default(
                    &mut node_guard,
                    CoreOptions::SPACING_LABEL_PORT_VERTICAL,
                    0.0,
                ),
                node_guard
                    .get_property(InternalProperties::COMPOUND_NODE)
                    .unwrap_or(false),
            )
        })
        .unwrap_or_else(|| {
            (
                Vec::new(),
                PortAlignment::Distributed,
                graph_port_spacing,
                graph_ports_surrounding.clone(),
                PortConstraints::Undefined,
                EnumSet::none_of(),
                EnumSet::none_of(),
                EnumSet::none_of(),
                true,
                0.0,
                0.0,
                0.0,
                false,
            )
        });
    let count = ports.len();
    if count == 0 {
        return;
    }

    let length = match side {
        PortSide::North | PortSide::South => width,
        PortSide::East | PortSide::West => height,
        _ => return,
    };

    let (surrounding_start, surrounding_end) = match side {
        PortSide::North | PortSide::South => (surrounding_spacing.left, surrounding_spacing.right),
        PortSide::East | PortSide::West => (surrounding_spacing.top, surrounding_spacing.bottom),
        PortSide::Undefined => (0.0, 0.0),
    };
    let ordered_ports: Vec<_> = match side {
        PortSide::West if alignment == PortAlignment::Center && constraints.is_order_fixed() => {
            let mut rotated: Vec<_> = ports.iter().collect();
            if rotated.len() > 1 {
                rotated.rotate_right(1);
            }
            rotated
        }
        // Only reverse WEST if ensure_clockwise_port_order didn't already reverse
        // (ensure_clockwise_port_order runs for FIXED_SIDE but NOT FIXED_ORDER)
        PortSide::West if constraints.is_order_fixed() => ports.iter().rev().collect(),
        PortSide::South if constraints.is_order_fixed() => ports.iter().rev().collect(),
        _ => ports.iter().collect(),
    };

    let port_labels_next_to_port =
        port_label_placement.contains(&PortLabelPlacement::NextToPortIfPossible);
    let port_labels_outside = port_label_placement.contains(&PortLabelPlacement::Outside);
    let always_same_side = port_label_placement.contains(&PortLabelPlacement::AlwaysSameSide);
    let always_other_same_side =
        port_label_placement.contains(&PortLabelPlacement::AlwaysOtherSameSide);
    let space_efficient = port_label_placement.contains(&PortLabelPlacement::SpaceEfficient);
    let port_labels_fixed = PortLabelPlacement::is_fixed(&port_label_placement);
    let include_label_margins = size_constraints.contains(&SizeConstraint::PortLabels);

    let mut contexts: Vec<PortPlacementContext> = ordered_ports
        .iter()
        .map(|port| {
            let has_edges = port
                .lock()
                .map(|port_guard| {
                    !(port_guard.incoming_edges().is_empty()
                        && port_guard.outgoing_edges().is_empty())
                })
                .unwrap_or(false);
            let has_compound_connections = port
                .lock()
                .ok()
                .and_then(|mut port_guard| {
                    port_guard.get_property(InternalProperties::INSIDE_CONNECTIONS)
                })
                .unwrap_or(false);
            let labels_next = labels_next_to_port(
                &port_label_placement,
                port_labels_next_to_port,
                node_is_compound,
                has_compound_connections,
                has_edges,
            );
            let (label_size, label_count) =
                port_label_min_size(port, label_label_spacing, port_labels_fixed);
            PortPlacementContext {
                port: (*port).clone(),
                margin: ElkMargin::new(),
                labels_next_to_port: labels_next,
                label_size,
                label_count,
            }
        })
        .collect();

    if include_label_margins {
        match side {
            PortSide::North | PortSide::South => {
                setup_horizontal_port_margins(
                    &mut contexts,
                    port_labels_outside,
                    always_same_side,
                    always_other_same_side,
                    space_efficient,
                    size_options.contains(&SizeOptions::UniformPortSpacing),
                    label_port_spacing_horizontal,
                    port_labels_fixed,
                );
            }
            PortSide::East | PortSide::West => {
                setup_vertical_port_margins(
                    &mut contexts,
                    port_labels_outside,
                    always_same_side,
                    always_other_same_side,
                    space_efficient,
                    size_options.contains(&SizeOptions::UniformPortSpacing),
                    label_port_spacing_vertical,
                    port_labels_fixed,
                    port_labels_treat_as_group,
                );
            }
            _ => {}
        }
    }

    let ports_overhang = size_options.contains(&SizeOptions::PortsOverhang);
    let mut placement_size = 0.0_f64;
    for (index, ctx) in contexts.iter().enumerate() {
        let port_size = ctx
            .port
            .lock()
            .ok()
            .map(|mut port_guard| *port_guard.shape().size_ref())
            .unwrap_or_else(KVector::new);
        let (margin_start, margin_end) = match side {
            PortSide::North | PortSide::South => (ctx.margin.left, ctx.margin.right),
            PortSide::East | PortSide::West => (ctx.margin.top, ctx.margin.bottom),
            _ => (0.0, 0.0),
        };
        let axis_size = match side {
            PortSide::North | PortSide::South => port_size.x,
            PortSide::East | PortSide::West => port_size.y,
            _ => 0.0,
        };
        placement_size += margin_start + axis_size + margin_end;
        if index + 1 < contexts.len() {
            placement_size += port_spacing;
        }
    }
    if alignment == PortAlignment::Distributed {
        placement_size += 2.0 * port_spacing;
    }

    if (alignment == PortAlignment::Distributed || alignment == PortAlignment::Justified)
        && count == 1
    {
        if alignment == PortAlignment::Distributed {
            placement_size -= 2.0 * port_spacing;
        }
        alignment = PortAlignment::Center;
    }

    let available_space = length - surrounding_start - surrounding_end;
    let mut current_pos = surrounding_start;
    let mut space_between_ports = port_spacing;

    if available_space < placement_size && !ports_overhang {
        if alignment == PortAlignment::Distributed {
            space_between_ports += (available_space - placement_size) / (count as f64 + 1.0);
            current_pos += space_between_ports;
        } else if count > 1 {
            space_between_ports += (available_space - placement_size) / (count as f64 - 1.0);
        }
    } else {
        if available_space < placement_size
            && (alignment == PortAlignment::Distributed || alignment == PortAlignment::Justified)
        {
            if alignment == PortAlignment::Distributed {
                placement_size -= 2.0 * port_spacing;
            }
            alignment = PortAlignment::Center;
        }

        match alignment {
            PortAlignment::Begin => {}
            PortAlignment::Center => {
                current_pos += (available_space - placement_size) / 2.0;
            }
            PortAlignment::End => {
                current_pos += available_space - placement_size;
            }
            PortAlignment::Distributed => {
                let additional = (available_space - placement_size) / (count as f64 + 1.0);
                space_between_ports += additional.max(0.0);
                current_pos += space_between_ports;
            }
            PortAlignment::Justified => {
                if count > 1 {
                    let additional = (available_space - placement_size) / (count as f64 - 1.0);
                    space_between_ports += additional.max(0.0);
                }
            }
        }
    }

    for ctx in contexts.iter() {
        if let Ok(mut port_guard) = ctx.port.lock() {
            let port_size = *port_guard.shape().size_ref();
            let border_offset = port_guard
                .get_property(CoreOptions::PORT_BORDER_OFFSET)
                .unwrap_or(0.0);
            let pos = port_guard.shape().position();
            let (margin_start, margin_end) = match side {
                PortSide::North | PortSide::South => (ctx.margin.left, ctx.margin.right),
                PortSide::East | PortSide::West => (ctx.margin.top, ctx.margin.bottom),
                _ => (0.0, 0.0),
            };
            let axis_size = match side {
                PortSide::North | PortSide::South => port_size.x,
                PortSide::East | PortSide::West => port_size.y,
                _ => 0.0,
            };
            let axis_position = current_pos + margin_start;
            match side {
                PortSide::North => {
                    pos.x = axis_position;
                    pos.y = -port_size.y - border_offset;
                }
                PortSide::South => {
                    pos.x = axis_position;
                    pos.y = height + border_offset;
                }
                PortSide::East => {
                    pos.x = width + border_offset;
                    pos.y = axis_position;
                }
                PortSide::West => {
                    pos.x = -port_size.x - border_offset;
                    pos.y = axis_position;
                }
                _ => {}
            }
            current_pos += margin_start + axis_size + margin_end + space_between_ports;
        }
    }
}

fn ensure_clockwise_port_order(node: &LNodeRef, port_constraints: PortConstraints) {
    if !port_constraints.is_side_fixed() || port_constraints.is_order_fixed() {
        return;
    }

    let mut ports = if let Ok(node_guard) = node.lock() {
        node_guard.ports().clone()
    } else {
        Vec::new()
    };
    if ports.len() <= 1 {
        return;
    }

    let has_dummy_edges = ports.iter().any(|port| {
        if let Ok(port_guard) = port.lock() {
            return port_guard.connected_edges().iter().any(|edge| {
                edge.lock()
                    .ok()
                    .and_then(|mut edge_guard| {
                        edge_guard
                            .graph_element()
                            .get_property(InternalProperties::ORIGIN)
                    })
                    .is_none()
            });
        }
        false
    });
    if has_dummy_edges {
        return;
    }

    let mut indexed: Vec<(usize, LPortRef)> = ports.drain(..).enumerate().collect();
    indexed.sort_by(|(idx_a, port_a), (idx_b, port_b)| {
        let side_a = port_a
            .lock()
            .ok()
            .map(|port_guard| port_guard.side())
            .unwrap_or(PortSide::Undefined);
        let side_b = port_b
            .lock()
            .ok()
            .map(|port_guard| port_guard.side())
            .unwrap_or(PortSide::Undefined);
        let ord = side_a.cmp(&side_b);
        if ord == std::cmp::Ordering::Equal {
            idx_a.cmp(idx_b)
        } else {
            ord
        }
    });

    let mut sorted_ports: Vec<LPortRef> = indexed.into_iter().map(|(_, port)| port).collect();
    reverse_west_and_south_side(&mut sorted_ports);

    if let Ok(mut node_guard) = node.lock() {
        *node_guard.ports_mut() = sorted_ports;
        node_guard.cache_port_sides();
    }
}

fn reverse_west_and_south_side(ports: &mut [LPortRef]) {
    if ports.len() <= 1 {
        return;
    }

    // Java's NodeContext.comparePortContexts sorts SOUTH and WEST ports in descending
    // volatile_id order (right-to-left / bottom-to-top) so that the placement loop can
    // iterate left-to-right / top-to-bottom.  We achieve the same by reversing after
    // the ascending sort.  PortListSorter runs BEFORE this processor but
    // ensure_clockwise_port_order re-sorts ports, so BOTH sides must be reversed here.
    let (south_low, south_high) = find_port_side_range(ports, PortSide::South);
    reverse_range(ports, south_low, south_high);

    let (west_low, west_high) = find_port_side_range(ports, PortSide::West);
    reverse_range(ports, west_low, west_high);
}

fn find_port_side_range(ports: &[LPortRef], side: PortSide) -> (usize, usize) {
    if ports.is_empty() {
        return (0, 0);
    }

    let lb = side_ordinal(side);
    let hb = lb + 1;
    let mut low_idx = 0;

    while low_idx < ports.len() && side_ordinal(port_side(&ports[low_idx])) < lb {
        low_idx += 1;
    }

    let mut high_idx = low_idx;
    while high_idx < ports.len() && side_ordinal(port_side(&ports[high_idx])) < hb {
        high_idx += 1;
    }

    (low_idx, high_idx)
}

fn reverse_range(ports: &mut [LPortRef], low_idx: usize, high_idx: usize) {
    if high_idx <= low_idx + 1 {
        return;
    }

    ports[low_idx..high_idx].reverse();
}

fn port_side(port: &LPortRef) -> PortSide {
    port.lock()
        .ok()
        .map(|port_guard| port_guard.side())
        .unwrap_or(PortSide::Undefined)
}

fn side_ordinal(side: PortSide) -> i32 {
    match side {
        PortSide::Undefined => 0,
        PortSide::North => 1,
        PortSide::East => 2,
        PortSide::South => 3,
        PortSide::West => 4,
    }
}

fn port_alignment_for_side(
    node_guard: &mut crate::org::eclipse::elk::alg::layered::graph::LNode,
    side: PortSide,
) -> PortAlignment {
    match side {
        PortSide::North => node_guard.get_property(CoreOptions::PORT_ALIGNMENT_NORTH),
        PortSide::South => node_guard.get_property(CoreOptions::PORT_ALIGNMENT_SOUTH),
        PortSide::East => node_guard.get_property(CoreOptions::PORT_ALIGNMENT_EAST),
        PortSide::West => node_guard.get_property(CoreOptions::PORT_ALIGNMENT_WEST),
        PortSide::Undefined => None,
    }
    .or_else(|| node_guard.get_property(CoreOptions::PORT_ALIGNMENT_DEFAULT))
    .unwrap_or(PortAlignment::Distributed)
}

fn enforce_port_driven_minimum_size(
    node: &LNodeRef,
    node_size: org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector,
    graph_port_spacing: f64,
    graph_ports_surrounding: &ElkMargin,
    include_port_labels: bool,
    allow_shrink: bool,
) -> org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector {
    let (required_width, required_height) = if let Ok(mut node_guard) = node.lock() {
        let fixed_port_labels = include_port_labels
            && PortLabelPlacement::is_fixed(
                &node_guard
                    .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                    .unwrap_or_default(),
            );
        (
            required_axis_length_for_side(
                &mut node_guard,
                PortSide::North,
                graph_port_spacing,
                graph_ports_surrounding,
                fixed_port_labels,
            )
            .max(required_axis_length_for_side(
                &mut node_guard,
                PortSide::South,
                graph_port_spacing,
                graph_ports_surrounding,
                fixed_port_labels,
            )),
            required_axis_length_for_side(
                &mut node_guard,
                PortSide::East,
                graph_port_spacing,
                graph_ports_surrounding,
                fixed_port_labels,
            )
            .max(required_axis_length_for_side(
                &mut node_guard,
                PortSide::West,
                graph_port_spacing,
                graph_ports_surrounding,
                fixed_port_labels,
            )),
        )
    } else {
        (0.0, 0.0)
    };

    let mut adjusted = node_size;
    if allow_shrink {
        if required_width > 0.0 {
            adjusted.x = required_width;
        }
        if required_height > 0.0 {
            adjusted.y = required_height;
        }
    } else {
        adjusted.x = adjusted.x.max(required_width);
        adjusted.y = adjusted.y.max(required_height);
    }

    if (adjusted.x - node_size.x).abs() > f64::EPSILON
        || (adjusted.y - node_size.y).abs() > f64::EPSILON
    {
        if let Ok(mut node_guard) = node.lock() {
            node_guard.shape().size().x = adjusted.x;
            node_guard.shape().size().y = adjusted.y;
        }
    }

    adjusted
}

fn enforce_inside_port_label_minimum_size(
    node: &LNodeRef,
    node_size: org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector,
    graph_port_spacing: f64,
    graph_ports_surrounding: &ElkMargin,
    allow_shrink: bool,
) -> org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector {
    let (padding, label_gap_horizontal, label_gap_vertical, ports, keep_current_size_on_shrink) =
        match node.lock() {
            Ok(mut node_guard) => {
                let placement = node_guard
                    .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                    .unwrap_or_default();
                if !placement.contains(&PortLabelPlacement::Inside) {
                    return node_size;
                }
                let size_constraints = node_guard
                    .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
                    .unwrap_or_default();
                let padding = node_guard.padding().clone();
                let label_gap_horizontal = node_guard
                    .get_property(CoreOptions::SPACING_LABEL_PORT_HORIZONTAL)
                    .unwrap_or(1.0);
                let label_gap_vertical = node_guard
                    .get_property(CoreOptions::SPACING_LABEL_PORT_VERTICAL)
                    .unwrap_or(1.0);
                let ports = node_guard.ports().clone();
                let keep_current_size_on_shrink =
                    size_constraints.contains(&SizeConstraint::NodeLabels);
                (
                    padding,
                    label_gap_horizontal,
                    label_gap_vertical,
                    ports,
                    keep_current_size_on_shrink,
                )
            }
            Err(_) => return node_size,
        };

    let (required_width_from_port_layout, required_height_from_port_layout) = match node.lock() {
        Ok(mut node_guard) => (
            required_inside_port_axis_for_side(
                &mut node_guard,
                PortSide::North,
                graph_port_spacing,
                graph_ports_surrounding,
            )
            .max(required_inside_port_axis_for_side(
                &mut node_guard,
                PortSide::South,
                graph_port_spacing,
                graph_ports_surrounding,
            )),
            required_inside_port_axis_for_side(
                &mut node_guard,
                PortSide::East,
                graph_port_spacing,
                graph_ports_surrounding,
            )
            .max(required_inside_port_axis_for_side(
                &mut node_guard,
                PortSide::West,
                graph_port_spacing,
                graph_ports_surrounding,
            )),
        ),
        Err(_) => (0.0, 0.0),
    };

    let mut max_west = 0.0_f64;
    let mut max_east = 0.0_f64;
    let mut max_north = 0.0_f64;
    let mut max_south = 0.0_f64;
    let mut has_east_west_labels = false;
    let mut has_north_south_labels = false;
    let mut max_label_height_west = 0.0_f64;
    let mut max_label_height_east = 0.0_f64;
    let mut max_port_height_west = 0.0_f64;
    let mut max_port_height_east = 0.0_f64;
    let mut west_port_total_height = 0.0_f64;
    let mut east_port_total_height = 0.0_f64;
    let mut west_port_count: usize = 0;
    let mut east_port_count: usize = 0;

    for port in ports {
        let (side, port_size, labels) = match port.lock() {
            Ok(mut port_guard) => (
                port_guard.side(),
                *port_guard.shape().size_ref(),
                port_guard.labels().clone(),
            ),
            Err(_) => continue,
        };

        match side {
            PortSide::West | PortSide::Undefined => {
                west_port_count += 1;
                west_port_total_height += port_size.y;
                max_port_height_west = max_port_height_west.max(port_size.y);
            }
            PortSide::East => {
                east_port_count += 1;
                east_port_total_height += port_size.y;
                max_port_height_east = max_port_height_east.max(port_size.y);
            }
            _ => {}
        }

        for label in labels {
            let label_size = match label.lock() {
                Ok(mut label_guard) => *label_guard.shape().size_ref(),
                Err(_) => continue,
            };
            match side {
                PortSide::West | PortSide::Undefined => {
                    max_west = max_west.max(label_size.x);
                    max_label_height_west = max_label_height_west.max(label_size.y);
                    has_east_west_labels = true;
                }
                PortSide::East => {
                    max_east = max_east.max(label_size.x);
                    max_label_height_east = max_label_height_east.max(label_size.y);
                    has_east_west_labels = true;
                }
                PortSide::North => {
                    max_north = max_north.max(label_size.y);
                    has_north_south_labels = true;
                }
                PortSide::South => {
                    max_south = max_south.max(label_size.y);
                    has_north_south_labels = true;
                }
            }
        }
    }

    if max_west == 0.0 && max_east == 0.0 && max_north == 0.0 && max_south == 0.0 {
        return node_size;
    }

    let required_width =
        padding.left + padding.right + 2.0 * label_gap_horizontal + max_west + max_east;
    let north_south_height = 2.0 * label_gap_vertical + max_north + max_south;
    let west_required_height = if west_port_count > 0 {
        let required_gap =
            (max_label_height_west + label_gap_vertical - max_port_height_west).max(0.0);
        west_port_total_height + (west_port_count as f64 + 1.0) * required_gap
    } else {
        0.0
    };
    let east_required_height = if east_port_count > 0 {
        let required_gap =
            (max_label_height_east + label_gap_vertical - max_port_height_east).max(0.0);
        east_port_total_height + (east_port_count as f64 + 1.0) * required_gap
    } else {
        0.0
    };
    let required_width =
        required_width.max(padding.left + padding.right + required_width_from_port_layout);
    let required_height = (padding.top
        + padding.bottom
        + north_south_height
            .max(west_required_height)
            .max(east_required_height))
    .max(padding.top + padding.bottom + required_height_from_port_layout);
    // Avoid shrinking when labels were already sized by step 1 (inside node labels present).
    // Phase 1 sizes nodes for node labels and port labels together; shrinking here can break those
    // larger guarantees (for example when both large node labels and inside port labels coexist).
    let prevent_shrink_for_mixed_inside_sides = has_east_west_labels && has_north_south_labels;
    let prevent_shrink = keep_current_size_on_shrink || prevent_shrink_for_mixed_inside_sides;

    let mut adjusted = node_size;
    if allow_shrink && !prevent_shrink {
        if required_width > 0.0 {
            adjusted.x = required_width;
        }
        if required_height > 0.0 {
            adjusted.y = required_height;
        }
    } else {
        adjusted.x = adjusted.x.max(required_width);
        adjusted.y = adjusted.y.max(required_height);
    }

    if (adjusted.x - node_size.x).abs() > f64::EPSILON
        || (adjusted.y - node_size.y).abs() > f64::EPSILON
    {
        if let Ok(mut node_guard) = node.lock() {
            node_guard.shape().size().x = adjusted.x;
            node_guard.shape().size().y = adjusted.y;
        }
    }

    adjusted
}

fn required_axis_length_for_side(
    node_guard: &mut crate::org::eclipse::elk::alg::layered::graph::LNode,
    side: PortSide,
    graph_port_spacing: f64,
    graph_ports_surrounding: &ElkMargin,
    include_fixed_label_margins: bool,
) -> f64 {
    let ports = node_guard.port_side_view(side);
    if ports.is_empty() {
        return 0.0;
    }
    let count = ports.len();

    let alignment = port_alignment_for_side(node_guard, side);
    let port_spacing = property_with_graph_default(
        node_guard,
        CoreOptions::SPACING_PORT_PORT,
        graph_port_spacing,
    );
    let surrounding_spacing = property_with_graph_default(
        node_guard,
        CoreOptions::SPACING_PORTS_SURROUNDING,
        graph_ports_surrounding.clone(),
    );
    let (surrounding_start, surrounding_end) = match side {
        PortSide::North | PortSide::South => (surrounding_spacing.left, surrounding_spacing.right),
        PortSide::East | PortSide::West => (surrounding_spacing.top, surrounding_spacing.bottom),
        PortSide::Undefined => (0.0, 0.0),
    };

    let mut base = surrounding_start.max(0.0) + surrounding_end.max(0.0);
    for port in &ports {
        let extent = port
            .lock()
            .ok()
            .map(|mut port_guard| {
                let port_size = *port_guard.shape().size_ref();
                let axis_size = match side {
                    PortSide::North | PortSide::South => port_size.x,
                    PortSide::East | PortSide::West => port_size.y,
                    PortSide::Undefined => 0.0,
                };
                if !include_fixed_label_margins {
                    return axis_size;
                }

                let mut min_x = 0.0_f64;
                let mut min_y = 0.0_f64;
                let mut max_x = 0.0_f64;
                let mut max_y = 0.0_f64;
                let mut has_label = false;
                for label in port_guard.labels().iter() {
                    let Some(label_bounds) = label
                        .lock()
                        .ok()
                        .map(|mut label_guard| {
                            let pos = *label_guard.shape().position_ref();
                            let size = *label_guard.shape().size_ref();
                            (pos.x, pos.y, pos.x + size.x, pos.y + size.y)
                        })
                    else {
                        continue;
                    };

                    if !has_label {
                        min_x = label_bounds.0;
                        min_y = label_bounds.1;
                        max_x = label_bounds.2;
                        max_y = label_bounds.3;
                        has_label = true;
                    } else {
                        min_x = min_x.min(label_bounds.0);
                        min_y = min_y.min(label_bounds.1);
                        max_x = max_x.max(label_bounds.2);
                        max_y = max_y.max(label_bounds.3);
                    }
                }
                if !has_label {
                    return axis_size;
                }

                match side {
                    PortSide::North | PortSide::South => {
                        let left = (-min_x).max(0.0);
                        let right = (max_x - port_size.x).max(0.0);
                        axis_size + left + right
                    }
                    PortSide::East | PortSide::West => {
                        let top = (-min_y).max(0.0);
                        let bottom = (max_y - port_size.y).max(0.0);
                        axis_size + top + bottom
                    }
                    PortSide::Undefined => axis_size,
                }
            })
            .unwrap_or(0.0);
        base += extent;
    }
    if count > 1 {
        base += port_spacing.max(0.0) * (count as f64 - 1.0);
    }

    match alignment {
        PortAlignment::Distributed => base + 2.0 * port_spacing.max(0.0),
        PortAlignment::Justified
        | PortAlignment::Begin
        | PortAlignment::End
        | PortAlignment::Center => base,
    }
}

fn required_inside_port_axis_for_side(
    node_guard: &mut crate::org::eclipse::elk::alg::layered::graph::LNode,
    side: PortSide,
    graph_port_spacing: f64,
    graph_ports_surrounding: &ElkMargin,
) -> f64 {
    let ports = node_guard.port_side_view(side);
    if ports.is_empty() {
        return 0.0;
    }

    let port_spacing = property_with_graph_default(
        node_guard,
        CoreOptions::SPACING_PORT_PORT,
        graph_port_spacing,
    )
    .max(0.0);
    let surrounding_spacing = property_with_graph_default(
        node_guard,
        CoreOptions::SPACING_PORTS_SURROUNDING,
        graph_ports_surrounding.clone(),
    );
    let (surrounding_start, surrounding_end) = match side {
        PortSide::North | PortSide::South => (surrounding_spacing.left, surrounding_spacing.right),
        PortSide::East | PortSide::West => (surrounding_spacing.top, surrounding_spacing.bottom),
        PortSide::Undefined => (0.0, 0.0),
    };

    let mut required = surrounding_start.max(0.0) + surrounding_end.max(0.0);
    for port in &ports {
        let port_extent = port
            .lock()
            .ok()
            .map(|mut port_guard| {
                let port_size = *port_guard.shape().size_ref();
                let label_extent = port_guard
                    .labels()
                    .iter()
                    .filter_map(|label| {
                        label
                            .lock()
                            .ok()
                            .map(|mut label_guard| *label_guard.shape().size_ref())
                    })
                    .map(|label_size| match side {
                        PortSide::North | PortSide::South => label_size.x,
                        PortSide::East | PortSide::West => label_size.y,
                        PortSide::Undefined => 0.0,
                    })
                    .fold(0.0, f64::max);
                let axis_size = match side {
                    PortSide::North | PortSide::South => port_size.x,
                    PortSide::East | PortSide::West => port_size.y,
                    PortSide::Undefined => 0.0,
                };
                axis_size.max(label_extent)
            })
            .unwrap_or(0.0);
        required += port_extent;
    }

    if ports.len() > 1 {
        required += port_spacing * (ports.len() as f64 - 1.0);
        if port_alignment_for_side(node_guard, side) == PortAlignment::Distributed {
            required += 2.0 * port_spacing;
        }
    }

    required
}

fn property_with_graph_default<T: Clone + Send + Sync + 'static>(
    node_guard: &mut crate::org::eclipse::elk::alg::layered::graph::LNode,
    property: &'static Property<T>,
    graph_default: T,
) -> T {
    let has_individual = node_guard
        .shape()
        .graph_element()
        .properties()
        .has_property(CoreOptions::SPACING_INDIVIDUAL);
    if has_individual {
        if let Some(mut individual) = node_guard.get_property(CoreOptions::SPACING_INDIVIDUAL) {
            if individual.properties().has_property(property) {
                if let Some(value) = individual.properties_mut().get_property(property) {
                    return value;
                }
            }
        }
    }

    if node_guard
        .shape()
        .graph_element()
        .properties()
        .has_property(property)
    {
        if let Some(value) = node_guard.get_property(property) {
            return value;
        }
    }

    graph_default
}

fn node_property_or_graph_default<T: Clone + Send + Sync + 'static>(
    node_guard: &mut crate::org::eclipse::elk::alg::layered::graph::LNode,
    property: &'static Property<T>,
    graph_default: T,
) -> T {
    if node_guard
        .shape()
        .graph_element()
        .properties()
        .has_property(property)
    {
        if let Some(value) = node_guard.get_property(property) {
            return value;
        }
    }
    graph_default
}

fn adjust_ports_on_side(node: &LNodeRef, side: PortSide, width: f64, height: f64) {
    let ports = node
        .lock()
        .ok()
        .map(|mut node_guard| node_guard.port_side_view(side))
        .unwrap_or_default();
    if ports.is_empty() {
        return;
    }

    for port in ports {
        if let Ok(mut port_guard) = port.lock() {
            let port_size = *port_guard.shape().size_ref();
            let border_offset = port_guard
                .get_property(CoreOptions::PORT_BORDER_OFFSET)
                .unwrap_or(0.0);
            let pos = port_guard.shape().position();
            match side {
                PortSide::North => {
                    pos.y = -port_size.y - border_offset;
                }
                PortSide::South => {
                    pos.y = height + border_offset;
                }
                PortSide::East => {
                    pos.x = width + border_offset;
                }
                PortSide::West => {
                    pos.x = -port_size.x - border_offset;
                }
                _ => {}
            }
        }
    }
}

fn place_ports_fixed_ratio_on_side(node: &LNodeRef, side: PortSide, width: f64, height: f64) {
    let ports = node
        .lock()
        .ok()
        .map(|mut node_guard| node_guard.port_side_view(side))
        .unwrap_or_default();
    if ports.is_empty() {
        return;
    }

    for port in ports {
        if let Ok(mut port_guard) = port.lock() {
            let port_size = *port_guard.shape().size_ref();
            let border_offset = port_guard
                .get_property(CoreOptions::PORT_BORDER_OFFSET)
                .unwrap_or(0.0);
            let ratio = port_guard
                .get_property(InternalProperties::PORT_RATIO_OR_POSITION)
                .unwrap_or(0.0);
            let pos = port_guard.shape().position();
            match side {
                PortSide::North => {
                    pos.x = width * ratio;
                    pos.y = -port_size.y - border_offset;
                }
                PortSide::South => {
                    pos.x = width * ratio;
                    pos.y = height + border_offset;
                }
                PortSide::East => {
                    pos.x = width + border_offset;
                    pos.y = height * ratio;
                }
                PortSide::West => {
                    pos.x = -port_size.x - border_offset;
                    pos.y = height * ratio;
                }
                _ => {}
            }
        }
    }
}

// ============================================================
// Phase 3: External port dummy label placement
// ============================================================

/// Places labels of an external port dummy node. Java: placeExternalPortDummyLabels()
fn place_external_port_dummy_labels(
    dummy: &LNodeRef,
    graph_port_label_placement: &org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet<
        PortLabelPlacement,
    >,
    place_next_to_port_if_possible: bool,
    treat_as_group: bool,
) {
    let (
        label_port_spacing_horizontal,
        label_port_spacing_vertical,
        label_label_spacing,
        dummy_size,
    ) = match dummy.lock() {
        Ok(mut guard) => (
            guard
                .get_property(LayeredOptions::SPACING_LABEL_PORT_HORIZONTAL)
                .unwrap_or(0.0),
            guard
                .get_property(LayeredOptions::SPACING_LABEL_PORT_VERTICAL)
                .unwrap_or(0.0),
            guard
                .get_property(LayeredOptions::SPACING_LABEL_LABEL)
                .unwrap_or(0.0),
            *guard.shape().size_ref(),
        ),
        Err(_) => return,
    };

    // External port dummies have exactly one port
    let dummy_port = match dummy.lock() {
        Ok(guard) => match guard.ports().first() {
            Some(p) => p.clone(),
            None => return,
        },
        Err(_) => return,
    };

    let dummy_port_pos = dummy_port
        .lock()
        .ok()
        .map(|mut g| *g.shape().position_ref())
        .unwrap_or_default();

    // Compute port label box
    let port_labels: Vec<_> = dummy_port
        .lock()
        .ok()
        .map(|g| g.labels().clone())
        .unwrap_or_default();
    if port_labels.is_empty() {
        return;
    }

    let mut label_box = ElkRectangle::default();
    for label in &port_labels {
        if let Ok(mut g) = label.lock() {
            let sz = g.shape().size_ref();
            label_box.width = label_box.width.max(sz.x);
            label_box.height += sz.y;
        }
    }
    label_box.height += (port_labels.len() as f64 - 1.0) * label_label_spacing;

    let ext_port_side = dummy
        .lock()
        .ok()
        .and_then(|mut g| g.get_property(InternalProperties::EXT_PORT_SIDE))
        .unwrap_or(PortSide::Undefined);

    // Determine the position of the label box
    if graph_port_label_placement.contains(&PortLabelPlacement::Inside) {
        match ext_port_side {
            PortSide::North => {
                label_box.x = (dummy_size.x - label_box.width) / 2.0 - dummy_port_pos.x;
                label_box.y = label_port_spacing_vertical;
            }
            PortSide::South => {
                label_box.x = (dummy_size.x - label_box.width) / 2.0 - dummy_port_pos.x;
                label_box.y = -label_port_spacing_vertical - label_box.height;
            }
            PortSide::East => {
                if label_next_to_port(&dummy_port, true, place_next_to_port_if_possible) {
                    let label_height = if treat_as_group {
                        label_box.height
                    } else {
                        port_labels
                            .first()
                            .and_then(|l| l.lock().ok().map(|mut g| g.shape().size_ref().y))
                            .unwrap_or(0.0)
                    };
                    label_box.y = (dummy_size.y - label_height) / 2.0 - dummy_port_pos.y;
                } else {
                    label_box.y = dummy_size.y + label_port_spacing_vertical - dummy_port_pos.y;
                }
                label_box.x = -label_port_spacing_horizontal - label_box.width;
            }
            PortSide::West => {
                if label_next_to_port(&dummy_port, true, place_next_to_port_if_possible) {
                    let label_height = if treat_as_group {
                        label_box.height
                    } else {
                        port_labels
                            .first()
                            .and_then(|l| l.lock().ok().map(|mut g| g.shape().size_ref().y))
                            .unwrap_or(0.0)
                    };
                    label_box.y = (dummy_size.y - label_height) / 2.0 - dummy_port_pos.y;
                } else {
                    label_box.y = dummy_size.y + label_port_spacing_vertical - dummy_port_pos.y;
                }
                label_box.x = label_port_spacing_horizontal;
            }
            _ => {}
        }
    } else if graph_port_label_placement.contains(&PortLabelPlacement::Outside) {
        match ext_port_side {
            PortSide::North | PortSide::South => {
                label_box.x = dummy_port_pos.x + label_port_spacing_horizontal;
            }
            PortSide::East | PortSide::West => {
                if label_next_to_port(&dummy_port, false, place_next_to_port_if_possible) {
                    let label_height = if treat_as_group {
                        label_box.height
                    } else {
                        port_labels
                            .first()
                            .and_then(|l| l.lock().ok().map(|mut g| g.shape().size_ref().y))
                            .unwrap_or(0.0)
                    };
                    label_box.y = (dummy_size.y - label_height) / 2.0 - dummy_port_pos.y;
                } else {
                    label_box.y = dummy_port_pos.y + label_port_spacing_vertical;
                }
            }
            _ => {}
        }
    }

    // Place the labels
    let mut current_y = label_box.y;
    for label in &port_labels {
        if let Ok(mut g) = label.lock() {
            let label_size_y = g.shape().size_ref().y;
            let pos = g.shape().position();
            pos.x = label_box.x;
            pos.y = current_y;
            current_y += label_size_y + label_label_spacing;
        }
    }
}

/// Checks whether labels of the given port should be placed next to the port or below it.
fn label_next_to_port(
    dummy_port: &crate::org::eclipse::elk::alg::layered::graph::LPortRef,
    inside_labels: bool,
    place_next_to_port_if_possible: bool,
) -> bool {
    if !place_next_to_port_if_possible {
        return false;
    }

    if inside_labels {
        return dummy_port
            .lock()
            .ok()
            .map(|port_guard| {
                port_guard.incoming_edges().is_empty() && port_guard.outgoing_edges().is_empty()
            })
            .unwrap_or(false);
    }

    // Java: !dummyPort.isConnectedToExternalNodes()
    !dummy_port
        .lock()
        .ok()
        .map(|port_guard| port_guard.is_connected_to_external_nodes())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::label_next_to_port;
    use crate::org::eclipse::elk::alg::layered::graph::LPort;

    #[test]
    fn label_next_to_port_outside_path_does_not_deadlock() {
        let port = LPort::new();
        assert!(!label_next_to_port(&port, false, true));
    }
}
