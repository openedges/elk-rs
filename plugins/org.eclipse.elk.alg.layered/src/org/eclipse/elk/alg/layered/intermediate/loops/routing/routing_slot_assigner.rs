use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::Random;

use crate::org::eclipse::elk::alg::layered::graph::LPortRef;
use crate::org::eclipse::elk::alg::layered::intermediate::loops::{SelfHyperLoopRef, SelfLoopHolderRef};
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::direction::routing_direction::RoutingDirection;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment::{HyperEdgeSegment, HyperEdgeSegmentRef};
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment_dependency::HyperEdgeSegmentDependency;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::orthogonal_routing_generator::OrthogonalRoutingGenerator;

pub struct RoutingSlotAssigner;

type LoopKey = usize;
type LabelCrossingMatrix = Vec<Vec<bool>>;
type LabelIdByLoop = HashMap<LoopKey, usize>;
type SegmentMap = HashMap<LoopKey, HyperEdgeSegmentRef>;
type LoopActivity = HashMap<LoopKey, Vec<bool>>;

impl RoutingSlotAssigner {
    pub fn assign_routing_slots(&self, holder: &SelfLoopHolderRef, random: &mut Random) {
        reset_routing_slots(holder);

        let (label_crossing_matrix, label_id_by_loop) = compute_label_crossing_matrix(holder);
        let (hyper_edge_segments, sl_loop_to_segment_map, sl_loop_activity_over_ports) =
            create_crossing_graph(holder, &label_crossing_matrix, &label_id_by_loop);

        OrthogonalRoutingGenerator::break_non_critical_cycles(&hyper_edge_segments, random);

        assign_raw_routing_slots_to_segments(&hyper_edge_segments);
        assign_raw_routing_slots_to_loops(holder, &sl_loop_to_segment_map);
        shift_towards_node(
            holder,
            &label_crossing_matrix,
            &label_id_by_loop,
            &sl_loop_activity_over_ports,
        );
        update_holder_routing_slot_count(holder);
    }
}

fn reset_routing_slots(holder: &SelfLoopHolderRef) {
    let loops = holder
        .lock()
        .ok()
        .map(|holder_guard| holder_guard.sl_hyper_loops().clone())
        .unwrap_or_default();

    for sl_loop in loops {
        if let Ok(mut sl_loop_guard) = sl_loop.lock() {
            sl_loop_guard.clear_routing_slots();
        }
    }
}

fn compute_label_crossing_matrix(holder: &SelfLoopHolderRef) -> (LabelCrossingMatrix, LabelIdByLoop) {
    let loops = holder
        .lock()
        .ok()
        .map(|holder_guard| holder_guard.sl_hyper_loops().clone())
        .unwrap_or_default();

    let mut label_id_by_loop = HashMap::new();
    let mut label_id = 0usize;
    for sl_loop in &loops {
        if loop_has_labels(sl_loop) {
            label_id_by_loop.insert(loop_key(sl_loop), label_id);
            label_id += 1;
        }
    }

    let mut crossing_matrix = vec![vec![false; label_id]; label_id];
    for first_idx in 0..loops.len() {
        let sl_loop1 = &loops[first_idx];
        let Some(&label1_id) = label_id_by_loop.get(&loop_key(sl_loop1)) else {
            continue;
        };
        for sl_loop2 in loops.iter().skip(first_idx + 1) {
            let Some(&label2_id) = label_id_by_loop.get(&loop_key(sl_loop2)) else {
                continue;
            };
            let overlap = labels_overlap(sl_loop1, sl_loop2);
            crossing_matrix[label1_id][label2_id] = overlap;
            crossing_matrix[label2_id][label1_id] = overlap;
        }
    }

    (crossing_matrix, label_id_by_loop)
}

fn labels_overlap(sl_loop1: &SelfHyperLoopRef, sl_loop2: &SelfHyperLoopRef) -> bool {
    let Some((side1, start1, end1)) = loop_label_span(sl_loop1) else {
        return false;
    };
    let Some((side2, start2, end2)) = loop_label_span(sl_loop2) else {
        return false;
    };

    if side1 != side2 || side1 == PortSide::East || side1 == PortSide::West {
        return false;
    }

    start1 <= end2 && end1 >= start2
}

fn loop_label_span(sl_loop: &SelfHyperLoopRef) -> Option<(PortSide, f64, f64)> {
    sl_loop.lock().ok().and_then(|sl_loop_guard| {
        let sl_labels = sl_loop_guard.sl_labels()?;
        let side = sl_labels.side();
        let start = sl_labels.position().x;
        let end = start + sl_labels.size().x;
        Some((side, start, end))
    })
}

fn loop_has_labels(sl_loop: &SelfHyperLoopRef) -> bool {
    sl_loop
        .lock()
        .ok()
        .and_then(|sl_loop_guard| sl_loop_guard.sl_labels().map(|labels| !labels.l_labels().is_empty()))
        .unwrap_or(false)
}

fn create_crossing_graph(
    holder: &SelfLoopHolderRef,
    label_crossing_matrix: &LabelCrossingMatrix,
    label_id_by_loop: &LabelIdByLoop,
) -> (Vec<HyperEdgeSegmentRef>, SegmentMap, LoopActivity) {
    let loops = holder
        .lock()
        .ok()
        .map(|holder_guard| holder_guard.sl_hyper_loops().clone())
        .unwrap_or_default();

    let mut hyper_edge_segments = Vec::with_capacity(loops.len());
    let mut sl_loop_to_segment_map = HashMap::new();
    for sl_loop in &loops {
        let segment = HyperEdgeSegment::new(RoutingDirection::WestToEast);
        hyper_edge_segments.push(segment.clone());
        sl_loop_to_segment_map.insert(loop_key(sl_loop), segment);
    }

    let sl_loop_activity_over_ports = compute_loop_activity(holder, &loops);

    for first_idx in 0..loops.len().saturating_sub(1) {
        let sl_loop1 = &loops[first_idx];
        for sl_loop2 in loops.iter().skip(first_idx + 1) {
            create_dependencies(
                sl_loop1,
                sl_loop2,
                label_crossing_matrix,
                label_id_by_loop,
                &sl_loop_activity_over_ports,
                &sl_loop_to_segment_map,
            );
        }
    }

    (
        hyper_edge_segments,
        sl_loop_to_segment_map,
        sl_loop_activity_over_ports,
    )
}

fn compute_loop_activity(holder: &SelfLoopHolderRef, loops: &[SelfHyperLoopRef]) -> LoopActivity {
    let port_count = holder
        .lock()
        .ok()
        .and_then(|holder_guard| {
            holder_guard
                .l_node()
                .lock()
                .ok()
                .map(|node_guard| node_guard.ports().len())
        })
        .unwrap_or(0);

    let mut activity = HashMap::new();
    for sl_loop in loops {
        let mut loop_activity = vec![false; port_count];

        let (Some(leftmost_port), Some(rightmost_port)) = sl_loop
            .lock()
            .ok()
            .map(|sl_loop_guard| (sl_loop_guard.leftmost_port(), sl_loop_guard.rightmost_port()))
            .unwrap_or((None, None))
        else {
            activity.insert(loop_key(sl_loop), loop_activity);
            continue;
        };

        let leftmost_port_id =
            crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id(&leftmost_port);
        let rightmost_port_id =
            crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id(&rightmost_port);

        if port_count > 0 && leftmost_port_id >= 0 && rightmost_port_id >= 0 {
            let mut port_idx = leftmost_port_id as usize;
            if port_idx == 0 {
                port_idx = port_count - 1;
            } else {
                port_idx -= 1;
            }
            let target_idx = rightmost_port_id as usize;

            while port_idx != target_idx {
                port_idx = (port_idx + 1) % port_count;
                loop_activity[port_idx] = true;
            }
        }

        activity.insert(loop_key(sl_loop), loop_activity);
    }

    activity
}

fn create_dependencies(
    sl_loop1: &SelfHyperLoopRef,
    sl_loop2: &SelfHyperLoopRef,
    label_crossing_matrix: &LabelCrossingMatrix,
    label_id_by_loop: &LabelIdByLoop,
    sl_loop_activity_over_ports: &LoopActivity,
    sl_loop_to_segment_map: &SegmentMap,
) {
    let first_above_second_crossings =
        count_crossings(sl_loop1, sl_loop2, sl_loop_activity_over_ports);
    let second_above_first_crossings =
        count_crossings(sl_loop2, sl_loop1, sl_loop_activity_over_ports);

    let Some(segment1) = sl_loop_to_segment_map.get(&loop_key(sl_loop1)) else {
        return;
    };
    let Some(segment2) = sl_loop_to_segment_map.get(&loop_key(sl_loop2)) else {
        return;
    };

    if first_above_second_crossings < second_above_first_crossings {
        HyperEdgeSegmentDependency::create_and_add_regular(
            segment1,
            segment2,
            second_above_first_crossings - first_above_second_crossings,
        );
    } else if second_above_first_crossings < first_above_second_crossings {
        HyperEdgeSegmentDependency::create_and_add_regular(
            segment2,
            segment1,
            first_above_second_crossings - second_above_first_crossings,
        );
    } else if first_above_second_crossings != 0
        || labels_overlap_by_ids(sl_loop1, sl_loop2, label_crossing_matrix, label_id_by_loop)
    {
        HyperEdgeSegmentDependency::create_and_add_regular(segment1, segment2, 0);
        HyperEdgeSegmentDependency::create_and_add_regular(segment2, segment1, 0);
    }
}

fn count_crossings(
    sl_upper_loop: &SelfHyperLoopRef,
    sl_lower_loop: &SelfHyperLoopRef,
    sl_loop_activity_over_ports: &LoopActivity,
) -> i32 {
    let Some(lower_loop_activity) = sl_loop_activity_over_ports.get(&loop_key(sl_lower_loop)) else {
        return 0;
    };

    let sl_upper_ports = sl_upper_loop
        .lock()
        .ok()
        .map(|sl_loop_guard| sl_loop_guard.sl_ports().clone())
        .unwrap_or_default();

    let mut crossings = 0i32;
    for sl_port in sl_upper_ports {
        let port_id =
            crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id(&sl_port);
        if port_id >= 0
            && (port_id as usize) < lower_loop_activity.len()
            && lower_loop_activity[port_id as usize]
        {
            crossings += 1;
        }
    }

    crossings
}

fn labels_overlap_by_ids(
    sl_loop1: &SelfHyperLoopRef,
    sl_loop2: &SelfHyperLoopRef,
    label_crossing_matrix: &LabelCrossingMatrix,
    label_id_by_loop: &LabelIdByLoop,
) -> bool {
    let Some(&label1_id) = label_id_by_loop.get(&loop_key(sl_loop1)) else {
        return false;
    };
    let Some(&label2_id) = label_id_by_loop.get(&loop_key(sl_loop2)) else {
        return false;
    };

    label_crossing_matrix
        .get(label1_id)
        .and_then(|row| row.get(label2_id))
        .copied()
        .unwrap_or(false)
}

fn assign_raw_routing_slots_to_segments(hyper_edge_segments: &[HyperEdgeSegmentRef]) {
    let mut sinks = VecDeque::new();

    for segment in hyper_edge_segments {
        let (in_weight, out_weight) = {
            let segment_guard = segment.borrow();
            (
                segment_guard.incoming_segment_dependencies().len() as i32,
                segment_guard.outgoing_segment_dependencies().len() as i32,
            )
        };
        {
            let mut segment_guard = segment.borrow_mut();
            segment_guard.set_in_weight(in_weight);
            segment_guard.set_out_weight(out_weight);
            if out_weight == 0 {
                segment_guard.set_routing_slot(0);
                sinks.push_back(segment.clone());
            }
        }
    }

    while let Some(segment) = sinks.pop_front() {
        let next_routing_slot = segment.borrow().routing_slot() + 1;
        let incoming_dependencies = segment.borrow().incoming_segment_dependencies().clone();

        for dependency in incoming_dependencies {
            let Some(source_segment) = dependency.borrow().source() else {
                continue;
            };

            let next_out_weight = {
                let mut source_segment_guard = source_segment.borrow_mut();
                let current_slot = source_segment_guard.routing_slot();
                source_segment_guard.set_routing_slot(current_slot.max(next_routing_slot));
                let w = source_segment_guard.out_weight() - 1;
                source_segment_guard.set_out_weight(w);
                w
            };
            if next_out_weight == 0 {
                sinks.push_back(source_segment);
            }
        }
    }
}

fn assign_raw_routing_slots_to_loops(holder: &SelfLoopHolderRef, sl_loop_to_segment_map: &SegmentMap) {
    let loops = holder
        .lock()
        .ok()
        .map(|holder_guard| holder_guard.sl_hyper_loops().clone())
        .unwrap_or_default();

    for sl_loop in loops {
        let key = loop_key(&sl_loop);
        let slot = sl_loop_to_segment_map
            .get(&key)
            .map(|segment| segment.borrow().routing_slot())
            .unwrap_or(0);

        let occupied_port_sides = sl_loop
            .lock()
            .ok()
            .map(|sl_loop_guard| sl_loop_guard.occupied_port_sides().clone())
            .unwrap_or_default();

        if let Ok(mut sl_loop_guard) = sl_loop.lock() {
            for port_side in occupied_port_sides {
                sl_loop_guard.set_routing_slot(port_side, slot);
            }
        }
    }
}

fn shift_towards_node(
    holder: &SelfLoopHolderRef,
    label_crossing_matrix: &LabelCrossingMatrix,
    label_id_by_loop: &LabelIdByLoop,
    sl_loop_activity_over_ports: &LoopActivity,
) {
    let port_count = holder
        .lock()
        .ok()
        .and_then(|holder_guard| {
            holder_guard
                .l_node()
                .lock()
                .ok()
                .map(|node_guard| node_guard.ports().len())
        })
        .unwrap_or(0);
    let mut next_free_routing_slot_at_port = vec![0; port_count];

    shift_towards_node_on_side(
        holder,
        PortSide::North,
        &mut next_free_routing_slot_at_port,
        label_crossing_matrix,
        label_id_by_loop,
        sl_loop_activity_over_ports,
    );
    shift_towards_node_on_side(
        holder,
        PortSide::East,
        &mut next_free_routing_slot_at_port,
        label_crossing_matrix,
        label_id_by_loop,
        sl_loop_activity_over_ports,
    );
    shift_towards_node_on_side(
        holder,
        PortSide::South,
        &mut next_free_routing_slot_at_port,
        label_crossing_matrix,
        label_id_by_loop,
        sl_loop_activity_over_ports,
    );
    shift_towards_node_on_side(
        holder,
        PortSide::West,
        &mut next_free_routing_slot_at_port,
        label_crossing_matrix,
        label_id_by_loop,
        sl_loop_activity_over_ports,
    );
}

fn shift_towards_node_on_side(
    holder: &SelfLoopHolderRef,
    side: PortSide,
    next_free_routing_slot_at_port: &mut [i32],
    label_crossing_matrix: &LabelCrossingMatrix,
    label_id_by_loop: &LabelIdByLoop,
    sl_loop_activity_over_ports: &LoopActivity,
) {
    let mut sl_loops = holder
        .lock()
        .ok()
        .map(|holder_guard| {
            holder_guard
                .sl_hyper_loops()
                .iter()
                .filter(|sl_loop| {
                    sl_loop
                        .lock()
                        .ok()
                        .map(|sl_loop_guard| sl_loop_guard.occupied_port_sides().contains(&side))
                        .unwrap_or(false)
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    sl_loops.sort_by_key(|sl_loop| {
        sl_loop
            .lock()
            .ok()
            .map(|sl_loop_guard| sl_loop_guard.routing_slot(side))
            .unwrap_or(i32::MAX)
    });

    let ports = holder
        .lock()
        .ok()
        .and_then(|holder_guard| {
            holder_guard
                .l_node()
                .lock()
                .ok()
                .map(|node_guard| node_guard.ports().clone())
        })
        .unwrap_or_default();
    let (min_l_port_index, max_l_port_index) = min_max_port_id_on_side(&ports, side);

    let Some((min_l_port_index, max_l_port_index)) = min_l_port_index.zip(max_l_port_index) else {
        for (slot, sl_loop) in sl_loops.into_iter().enumerate() {
            if let Ok(mut sl_loop_guard) = sl_loop.lock() {
                sl_loop_guard.set_routing_slot(side, slot as i32);
            }
        }
        return;
    };

    let mut slot_assigned_to_label = vec![-1; label_crossing_matrix.len()];
    for sl_loop in sl_loops {
        let loop_key = loop_key(&sl_loop);
        let active_at_port = sl_loop_activity_over_ports.get(&loop_key);
        let mut lowest_available_slot = 0i32;

        for port_index in min_l_port_index..=max_l_port_index {
            if active_at_port
                .and_then(|active| active.get(port_index))
                .copied()
                .unwrap_or(false)
            {
                lowest_available_slot =
                    lowest_available_slot.max(next_free_routing_slot_at_port[port_index]);
            }
        }

        if let Some(&our_label_idx) = label_id_by_loop.get(&loop_key) {
            let mut slots_with_label_conflicts = HashSet::new();
            for other_label_idx in 0..label_crossing_matrix.len() {
                if label_crossing_matrix[our_label_idx][other_label_idx] {
                    let assigned_slot = slot_assigned_to_label[other_label_idx];
                    if assigned_slot >= 0 {
                        slots_with_label_conflicts.insert(assigned_slot);
                    }
                }
            }

            while slots_with_label_conflicts.contains(&lowest_available_slot) {
                lowest_available_slot += 1;
            }
        }

        if let Ok(mut sl_loop_guard) = sl_loop.lock() {
            sl_loop_guard.set_routing_slot(side, lowest_available_slot);
        }

        for port_index in min_l_port_index..=max_l_port_index {
            if active_at_port
                .and_then(|active| active.get(port_index))
                .copied()
                .unwrap_or(false)
            {
                next_free_routing_slot_at_port[port_index] = lowest_available_slot + 1;
            }
        }

        if let Some(&label_idx) = label_id_by_loop.get(&loop_key) {
            slot_assigned_to_label[label_idx] = lowest_available_slot;
        }
    }
}

fn min_max_port_id_on_side(ports: &[LPortRef], side: PortSide) -> (Option<usize>, Option<usize>) {
    let mut min_idx: Option<usize> = None;
    let mut max_idx: Option<usize> = None;

    for l_port in ports {
        let Some((port_side, port_id)) = l_port.lock().ok().map(|mut port_guard| {
            (
                port_guard.side(),
                port_guard.shape().graph_element().id,
            )
        }) else {
            continue;
        };
        if port_side != side || port_id < 0 {
            continue;
        }

        let port_id = port_id as usize;
        min_idx = Some(min_idx.map_or(port_id, |curr| curr.min(port_id)));
        max_idx = Some(max_idx.map_or(port_id, |curr| curr.max(port_id)));
    }

    (min_idx, max_idx)
}

fn update_holder_routing_slot_count(holder: &SelfLoopHolderRef) {
    let loops = holder
        .lock()
        .ok()
        .map(|holder_guard| holder_guard.sl_hyper_loops().clone())
        .unwrap_or_default();

    let mut routing_slot_count = vec![0; 5];
    for sl_loop in loops {
        if let Ok(sl_loop_guard) = sl_loop.lock() {
            for side in [PortSide::North, PortSide::East, PortSide::South, PortSide::West] {
                let slot = sl_loop_guard.routing_slot(side).max(0);
                let side_idx = side_index(side);
                routing_slot_count[side_idx] = routing_slot_count[side_idx].max(slot + 1);
            }
        }
    }

    if let Ok(mut holder_guard) = holder.lock() {
        *holder_guard.routing_slot_count_mut() = routing_slot_count;
    }
}

fn loop_key(sl_loop: &SelfHyperLoopRef) -> LoopKey {
    Arc::as_ptr(sl_loop) as LoopKey
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
