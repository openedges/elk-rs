use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::labels::LabelManagementOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LMargin, NodeType};
use crate::org::eclipse::elk::alg::layered::intermediate::loops::{
    Alignment, SelfHyperLoopRef, SelfLoopEdgeRef, SelfLoopHolderRef,
};
use crate::org::eclipse::elk::alg::layered::intermediate::loops::routing::{
    LabelPlacer, RoutingDirector, RoutingSlotAssigner,
};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};
use org_eclipse_elk_core::org::eclipse::elk::core::util::Random;

const EPSILON: f64 = 1e-3;

#[derive(Clone, Copy)]
struct RoutePoint {
    side: PortSide,
    outer_anchor: KVector,
}

pub struct SelfLoopRouter;

impl ILayoutProcessor<LGraph> for SelfLoopRouter {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Self-Loop routing", 1.0);

        let edge_routing = graph
            .get_property(LayeredOptions::EDGE_ROUTING)
            .unwrap_or(EdgeRouting::Orthogonal);
        let label_manager = graph.get_property(LabelManagementOptions::LABEL_MANAGER);
        let mut random = graph.get_property(InternalProperties::RANDOM).unwrap_or_default();
        let spacing_factor = if edge_routing == EdgeRouting::Splines {
            1.5
        } else {
            1.0
        };
        let edge_edge_distance = graph
            .get_property(LayeredOptions::SPACING_EDGE_EDGE)
            .unwrap_or(10.0)
            * spacing_factor;
        let edge_label_distance = graph
            .get_property(LayeredOptions::SPACING_EDGE_LABEL)
            .unwrap_or(2.0)
            * spacing_factor;
        let node_self_loop_distance = graph
            .get_property(LayeredOptions::SPACING_NODE_SELF_LOOP)
            .unwrap_or(10.0)
            * spacing_factor;

        let nodes = graph
            .layers()
            .iter()
            .flat_map(|layer| {
                layer
                    .lock()
                    .ok()
                    .map(|layer_guard| layer_guard.nodes().clone())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();

        for node in nodes {
            let holder = node.lock().ok().and_then(|mut node_guard| {
                if node_guard.node_type() != NodeType::Normal {
                    return None;
                }
                node_guard.get_property(InternalProperties::SELF_LOOP_HOLDER)
            });
            let Some(holder) = holder else {
                continue;
            };

            let routing_director = RoutingDirector;
            routing_director.determine_loop_routes(&holder);

            // Java parity: side/alignment + label management must happen before slot assignment.
            let label_placer = LabelPlacer;
            label_placer.prepare_labels(&holder, label_manager.as_ref());

            let routing_slot_assigner = RoutingSlotAssigner;
            routing_slot_assigner.assign_routing_slots(&holder, &mut random);

            route_node(
                &holder,
                edge_edge_distance,
                edge_label_distance,
                node_self_loop_distance,
            );
        }

        monitor.done();
    }
}

fn route_node(
    holder: &SelfLoopHolderRef,
    edge_edge_distance: f64,
    edge_label_distance: f64,
    node_self_loop_distance: f64,
) {
    let (l_node, routing_slot_count, hyper_loops) = if let Ok(holder_guard) = holder.lock() {
        (
            holder_guard.l_node().clone(),
            holder_guard.routing_slot_count().to_vec(),
            holder_guard.sl_hyper_loops().clone(),
        )
    } else {
        return;
    };

    let (node_size, mut new_margins) = if let Ok(mut node_guard) = l_node.lock() {
        (*node_guard.shape().size_ref(), node_guard.margin().clone())
    } else {
        return;
    };

    let routing_slot_positions = compute_routing_slot_positions(
        &routing_slot_count,
        &hyper_loops,
        node_size,
        &new_margins,
        edge_edge_distance,
        edge_label_distance,
        node_self_loop_distance,
    );

    for sl_loop in &hyper_loops {
        let sl_edges = sl_loop
            .lock()
            .ok()
            .map(|loop_guard| loop_guard.sl_edges().clone())
            .unwrap_or_default();

        for sl_edge in sl_edges {
            let (l_edge, sl_source, sl_target) = sl_edge
                .lock()
                .ok()
                .map(|sl_edge_guard| {
                    (
                        sl_edge_guard.l_edge().clone(),
                        sl_edge_guard.sl_source().clone(),
                        sl_edge_guard.sl_target().clone(),
                    )
                })
                .unwrap_or_else(|| panic!("self loop edge lock poisoned"));

            let source_port = sl_source
                .lock()
                .ok()
                .map(|port_guard| port_guard.l_port().clone())
                .unwrap_or_else(|| panic!("self loop source lock poisoned"));
            let target_port = sl_target
                .lock()
                .ok()
                .map(|port_guard| port_guard.l_port().clone())
                .unwrap_or_else(|| panic!("self loop target lock poisoned"));

            let inside_self_loop = l_edge
                .lock()
                .ok()
                .and_then(|mut edge_guard| edge_guard.get_property(CoreOptions::INSIDE_SELF_LOOPS_YO))
                .unwrap_or(false);
            if inside_self_loop {
                if let Ok(mut edge_guard) = l_edge.lock() {
                    edge_guard.bend_points().clear();
                }
                continue;
            }

            let source_point = route_point_for_port(&source_port, &routing_slot_positions, sl_loop, node_size);
            let target_point = route_point_for_port(&target_port, &routing_slot_positions, sl_loop, node_size);
            let (Some(source_point), Some(target_point)) = (source_point, target_point) else {
                continue;
            };

            let path = compute_orthogonal_bend_points(
                &sl_edge,
                sl_loop,
                &source_port,
                &target_port,
                &source_point,
                &target_point,
                &routing_slot_positions,
            );

            for bend_point in &path {
                update_margins_with_point(node_size, &mut new_margins, bend_point);
            }

            if let Ok(mut edge_guard) = l_edge.lock() {
                edge_guard.bend_points().clear();
                edge_guard.bend_points().add_all(&path);
            };
        }

        place_loop_labels(
            sl_loop,
            node_size,
            edge_label_distance,
            &routing_slot_positions,
            &mut new_margins,
        );
    }

    if let Ok(mut node_guard) = l_node.lock() {
        *node_guard.margin() = new_margins;
    };
}

fn compute_routing_slot_positions(
    routing_slot_count: &[i32],
    hyper_loops: &[SelfHyperLoopRef],
    node_size: KVector,
    node_margin: &LMargin,
    edge_edge_distance: f64,
    edge_label_distance: f64,
    node_self_loop_distance: f64,
) -> Vec<Vec<f64>> {
    let mut positions = vec![Vec::new(); 5];
    for side in [PortSide::North, PortSide::East, PortSide::South, PortSide::West] {
        let count = routing_slot_count
            .get(side_index(side))
            .copied()
            .unwrap_or_default()
            .max(0) as usize;
        positions[side_index(side)] = vec![0.0; count];
    }

    initialize_max_label_height(&mut positions, hyper_loops, PortSide::North);
    initialize_max_label_height(&mut positions, hyper_loops, PortSide::South);

    for side in [PortSide::North, PortSide::East, PortSide::South, PortSide::West] {
        let side_positions = &mut positions[side_index(side)];
        let mut curr_pos = baseline_position(side, node_size, node_margin, node_self_loop_distance);
        let factor = if side == PortSide::North || side == PortSide::West {
            -1.0
        } else {
            1.0
        };

        for slot_pos in side_positions.iter_mut() {
            let mut largest_label_size = *slot_pos;
            if largest_label_size > 0.0 {
                largest_label_size += edge_label_distance;
            }

            *slot_pos = curr_pos;
            curr_pos += factor * (largest_label_size + edge_edge_distance);
        }
    }

    positions
}

fn initialize_max_label_height(
    positions: &mut [Vec<f64>],
    hyper_loops: &[SelfHyperLoopRef],
    side: PortSide,
) {
    for sl_loop in hyper_loops {
        let (slot, label_height) = sl_loop
            .lock()
            .ok()
            .and_then(|loop_guard| {
                let labels = loop_guard.sl_labels()?;
                if labels.side() != side {
                    return None;
                }
                Some((loop_guard.routing_slot(side).max(0) as usize, labels.size().y))
            })
            .unwrap_or((usize::MAX, 0.0));
        if slot == usize::MAX {
            continue;
        }

        if let Some(slot_entry) = positions[side_index(side)].get_mut(slot) {
            *slot_entry = slot_entry.max(label_height);
        }
    }
}

fn baseline_position(
    side: PortSide,
    node_size: KVector,
    node_margin: &LMargin,
    node_self_loop_distance: f64,
) -> f64 {
    match side {
        PortSide::North => -node_margin.top - node_self_loop_distance,
        PortSide::East => node_size.x + node_margin.right + node_self_loop_distance,
        PortSide::South => node_size.y + node_margin.bottom + node_self_loop_distance,
        PortSide::West => -node_margin.left - node_self_loop_distance,
        PortSide::Undefined => 0.0,
    }
}

fn compute_orthogonal_bend_points(
    sl_edge: &SelfLoopEdgeRef,
    sl_loop: &SelfHyperLoopRef,
    source_port: &crate::org::eclipse::elk::alg::layered::graph::LPortRef,
    target_port: &crate::org::eclipse::elk::alg::layered::graph::LPortRef,
    source: &RoutePoint,
    target: &RoutePoint,
    routing_slot_positions: &[Vec<f64>],
) -> Vec<KVector> {
    let mut points = Vec::new();
    push_unique(&mut points, source.outer_anchor);

    if source.side != target.side {
        let clockwise = compute_edge_routing_direction(sl_loop, source_port, target_port, source.side, target.side);
        let mut curr_side = source.side;
        while curr_side != target.side {
            let next_side = if clockwise {
                curr_side.right()
            } else {
                curr_side.left()
            };

            let curr_slot = sl_loop
                .lock()
                .ok()
                .map(|loop_guard| loop_guard.routing_slot(curr_side).max(0) as usize)
                .unwrap_or_default();
            let next_slot = sl_loop
                .lock()
                .ok()
                .map(|loop_guard| loop_guard.routing_slot(next_side).max(0) as usize)
                .unwrap_or_default();

            let mut curr_component = base_vector(curr_side, curr_slot, routing_slot_positions);
            let mut next_component = base_vector(next_side, next_slot, routing_slot_positions);

            if let Some((label_side, label_size)) = inline_label_side_and_size(sl_edge, sl_loop) {
                if curr_side == label_side {
                    adjust_vector_for_label_side(&mut curr_component, label_side, label_size);
                } else if next_side == label_side {
                    adjust_vector_for_label_side(&mut next_component, label_side, label_size);
                }
            }

            let mut corner = curr_component;
            corner.add(&next_component);
            push_unique(&mut points, corner);

            curr_side = next_side;
        }
    }

    push_unique(&mut points, target.outer_anchor);
    points
}

fn inline_label_side_and_size(
    sl_edge: &SelfLoopEdgeRef,
    sl_loop: &SelfHyperLoopRef,
) -> Option<(PortSide, KVector)> {
    let has_inline_label = sl_edge
        .lock()
        .ok()
        .map(|sl_edge_guard| sl_edge_guard.l_edge().clone())
        .is_some_and(|l_edge| {
            l_edge.lock().ok().is_some_and(|edge_guard| {
                edge_guard.labels().iter().any(|label| {
                    label
                        .lock()
                        .ok()
                        .and_then(|mut label_guard| {
                            label_guard.get_property(LayeredOptions::EDGE_LABELS_INLINE)
                        })
                        .unwrap_or(false)
                })
            })
        });
    if !has_inline_label {
        return None;
    }

    sl_loop.lock().ok().and_then(|loop_guard| {
        let labels = loop_guard.sl_labels()?;
        if labels.side() == PortSide::Undefined {
            return None;
        }
        Some((labels.side(), *labels.size()))
    })
}

fn adjust_vector_for_label_side(vector: &mut KVector, label_side: PortSide, label_size: KVector) {
    match label_side {
        PortSide::North => vector.y -= label_size.y / 2.0,
        PortSide::South => vector.y += label_size.y / 2.0,
        PortSide::West => vector.x -= label_size.x / 2.0,
        PortSide::East => vector.x += label_size.x / 2.0,
        PortSide::Undefined => {}
    }
}

fn compute_edge_routing_direction(
    sl_loop: &SelfHyperLoopRef,
    source_port: &crate::org::eclipse::elk::alg::layered::graph::LPortRef,
    target_port: &crate::org::eclipse::elk::alg::layered::graph::LPortRef,
    source_side: PortSide,
    target_side: PortSide,
) -> bool {
    if source_side == target_side {
        let source_id = source_port
            .lock()
            .ok()
            .map(|mut source_guard| source_guard.shape().graph_element().id)
            .unwrap_or_default();
        let target_id = target_port
            .lock()
            .ok()
            .map(|mut target_guard| target_guard.shape().graph_element().id)
            .unwrap_or_default();
        return source_id < target_id;
    }

    if source_side.right() == target_side {
        return true;
    }
    if source_side.left() == target_side {
        return false;
    }

    sl_loop
        .lock()
        .ok()
        .map(|loop_guard| {
            loop_guard
                .occupied_port_sides()
                .contains(&source_side.right())
        })
        .unwrap_or(true)
}

fn place_loop_labels(
    sl_loop: &SelfHyperLoopRef,
    node_size: KVector,
    edge_label_distance: f64,
    routing_slot_positions: &[Vec<f64>],
    margins: &mut LMargin,
) {
    let (side, alignment, align_ref, label_size, slot, inline_labels) = {
        let mut sl_loop_guard = match sl_loop.lock().ok() {
            Some(guard) => guard,
            None => return,
        };
        let labels = match sl_loop_guard.sl_labels_mut() {
            Some(labels) => labels,
            None => return,
        };
        let side = labels.side();
        if side == PortSide::Undefined {
            return;
        }

        let inline = labels.l_labels().iter().any(|label| {
            label
                .lock()
                .ok()
                .and_then(|mut label_guard| label_guard.get_property(LayeredOptions::EDGE_LABELS_INLINE))
                .unwrap_or(false)
        });

        (
            side,
            labels.alignment(),
            labels.alignment_reference_sl_port(),
            *labels.size(),
            sl_loop_guard.routing_slot(side).max(0) as usize,
            inline,
        )
    };

    let lane_position = slot_position(side, slot, routing_slot_positions).unwrap_or_default();
    let local = local_position(node_size, label_size, alignment, align_ref);
    let label_distance = if inline_labels { 0.0 } else { edge_label_distance };

    let mut relative = KVector::new();
    match side {
        PortSide::North => {
            relative.x = local.x;
            relative.y = lane_position - label_distance - label_size.y;
        }
        PortSide::South => {
            relative.x = local.x;
            relative.y = lane_position + label_distance;
        }
        PortSide::East => {
            relative.x = lane_position + label_distance;
            relative.y = local.y;
        }
        PortSide::West => {
            relative.x = lane_position - label_distance - label_size.x;
            relative.y = local.y;
        }
        PortSide::Undefined => return,
    }

    if let Ok(mut sl_loop_guard) = sl_loop.lock() {
        if let Some(labels) = sl_loop_guard.sl_labels_mut() {
            labels.apply_vertical_stack(relative, 2.0);
            *labels.position_mut() = relative;
            let local_top_left = relative;
            let local_bottom_right = KVector::with_values(
                relative.x + labels.size().x,
                relative.y + labels.size().y,
            );
            update_margins_with_point(node_size, margins, &local_top_left);
            update_margins_with_point(node_size, margins, &local_bottom_right);
        }
    }
}

fn route_point_for_port(
    port: &crate::org::eclipse::elk::alg::layered::graph::LPortRef,
    routing_slot_positions: &[Vec<f64>],
    sl_loop: &SelfHyperLoopRef,
    node_size: KVector,
) -> Option<RoutePoint> {
    let (mut side, local_anchor) = port.lock().ok().map(|mut port_guard| {
        let side = port_guard.side();
        let (pos_x, pos_y) = {
            let pos = port_guard.shape().position_ref();
            (pos.x, pos.y)
        };
        let anchor = *port_guard.anchor_ref();
        (side, KVector::with_values(pos_x + anchor.x, pos_y + anchor.y))
    })?;

    if side == PortSide::Undefined {
        side = infer_side_from_position(&local_anchor, node_size);
    }

    let slot = sl_loop
        .lock()
        .ok()
        .map(|loop_guard| loop_guard.routing_slot(side).max(0) as usize)
        .unwrap_or_default();
    let base = base_vector(side, slot, routing_slot_positions);
    let outer_anchor = match side {
        PortSide::North | PortSide::South => KVector::with_values(local_anchor.x, base.y),
        PortSide::East | PortSide::West => KVector::with_values(base.x, local_anchor.y),
        PortSide::Undefined => KVector::with_values(local_anchor.x, base.y),
    };

    Some(RoutePoint { side, outer_anchor })
}

fn local_position(
    node_size: KVector,
    label_size: KVector,
    alignment: Alignment,
    align_ref: Option<crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfLoopPortRef>,
) -> KVector {
    let mut local = KVector::new();
    match alignment {
        Alignment::Center => {
            local.x = (node_size.x - label_size.x) / 2.0;
            local.y = (node_size.y - label_size.y) / 2.0;
        }
        Alignment::Left => {
            local.x = align_ref
                .as_ref()
                .and_then(alignment_reference_xy)
                .map(|(x, _)| x)
                .unwrap_or(0.0);
            local.y = (node_size.y - label_size.y) / 2.0;
        }
        Alignment::Right => {
            local.x = align_ref
                .as_ref()
                .and_then(alignment_reference_xy)
                .map(|(x, _)| x - label_size.x)
                .unwrap_or(node_size.x - label_size.x);
            local.y = (node_size.y - label_size.y) / 2.0;
        }
        Alignment::Top => {
            local.x = (node_size.x - label_size.x) / 2.0;
            local.y = align_ref
                .as_ref()
                .and_then(alignment_reference_xy)
                .map(|(_, y)| y)
                .unwrap_or(0.0);
        }
    }
    local
}

fn alignment_reference_xy(
    sl_port: &crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfLoopPortRef,
) -> Option<(f64, f64)> {
    sl_port.lock().ok().and_then(|port_guard| {
        port_guard.l_port().lock().ok().map(|mut port_guard| {
            (
                port_guard.shape().position_ref().x + port_guard.anchor_ref().x,
                port_guard.shape().position_ref().y + port_guard.anchor_ref().y,
            )
        })
    })
}

fn update_margins_with_point(node_size: KVector, margins: &mut LMargin, point: &KVector) {
    margins.left = margins.left.max(-point.x);
    margins.right = margins.right.max(point.x - node_size.x);
    margins.top = margins.top.max(-point.y);
    margins.bottom = margins.bottom.max(point.y - node_size.y);
}

fn infer_side_from_position(point: &KVector, node_size: KVector) -> PortSide {
    let left_distance = point.x.abs();
    let right_distance = (node_size.x - point.x).abs();
    let north_distance = point.y.abs();
    let south_distance = (node_size.y - point.y).abs();

    let min = left_distance
        .min(right_distance)
        .min(north_distance)
        .min(south_distance);
    if near(min, north_distance) {
        PortSide::North
    } else if near(min, right_distance) {
        PortSide::East
    } else if near(min, south_distance) {
        PortSide::South
    } else {
        PortSide::West
    }
}

fn slot_position(side: PortSide, slot: usize, routing_slot_positions: &[Vec<f64>]) -> Option<f64> {
    routing_slot_positions
        .get(side_index(side))
        .and_then(|slots| slots.get(slot))
        .copied()
}

fn base_vector(side: PortSide, slot: usize, routing_slot_positions: &[Vec<f64>]) -> KVector {
    let position = slot_position(side, slot, routing_slot_positions).unwrap_or_default();
    match side {
        PortSide::North | PortSide::South => KVector::with_values(0.0, position),
        PortSide::East | PortSide::West => KVector::with_values(position, 0.0),
        PortSide::Undefined => KVector::new(),
    }
}

fn side_index(side: PortSide) -> usize {
    match side {
        PortSide::Undefined => 0,
        PortSide::North => 1,
        PortSide::East => 2,
        PortSide::South => 3,
        PortSide::West => 4,
    }
}

fn push_unique(points: &mut Vec<KVector>, point: KVector) {
    let should_push = match points.last() {
        Some(last) => !near_point(last, &point),
        None => true,
    };
    if should_push {
        points.push(point);
    }
}

fn near_point(a: &KVector, b: &KVector) -> bool {
    near(a.x, b.x) && near(a.y, b.y)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= EPSILON
}
