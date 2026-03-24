use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::Random;

use crate::org::eclipse::elk::alg::layered::intermediate::loops::{
    SelfHyperLoopRef, SelfLoopHolderRef,
};
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::direction::routing_direction::RoutingDirection;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment::{
    HyperEdgeSegment, HyperEdgeSegmentRef,
};
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment_dependency::HyperEdgeSegmentDependency;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::orthogonal_routing_generator::OrthogonalRoutingGenerator;

pub struct RoutingSlotAssigner;

type LoopKey = usize;
type LabelCrossingMatrix = Vec<Vec<bool>>;
type LabelIdByLoop = HashMap<LoopKey, usize>;
type SegmentMap = HashMap<LoopKey, HyperEdgeSegmentRef>;
type LoopActivity = HashMap<LoopKey, Vec<bool>>;
type SidePortRanges = [Option<(usize, usize)>; 5];

#[derive(Clone)]
struct LoopRoutingState {
    occupied_sides: [bool; 5],
    routing_slots: [i32; 5],
}

impl LoopRoutingState {
    fn new() -> Self {
        Self {
            occupied_sides: [false; 5],
            routing_slots: [0; 5],
        }
    }
}

struct ShiftContext<'a> {
    label_crossing_matrix: &'a LabelCrossingMatrix,
    label_id_by_loop: &'a LabelIdByLoop,
    sl_loop_activity_over_ports: &'a LoopActivity,
}

impl RoutingSlotAssigner {
    pub fn assign_routing_slots(&self, holder: &SelfLoopHolderRef, random: &mut Random) {
        let loops = holder
            .lock().sl_hyper_loops().clone();

        reset_routing_slots(&loops);
        let (port_count, side_port_ranges) = collect_port_side_ranges(holder);

        let (label_crossing_matrix, label_id_by_loop) = compute_label_crossing_matrix(&loops);
        let (hyper_edge_segments, sl_loop_to_segment_map, sl_loop_activity_over_ports) =
            create_crossing_graph(
                &loops,
                port_count,
                &label_crossing_matrix,
                &label_id_by_loop,
            );

        OrthogonalRoutingGenerator::break_non_critical_cycles(&hyper_edge_segments, random);

        assign_raw_routing_slots_to_segments(&hyper_edge_segments);
        let mut loop_routing_state =
            assign_raw_routing_slots_to_loops(&loops, &sl_loop_to_segment_map);
        // Java parity: update holder counts AFTER shift phase.
        // Java's setRoutingSlot() updates routingSlotCount inline on every call (both raw and shift phases).
        // The shift phase can INCREASE slots (e.g., when a loop's activity range spans ports used by
        // other sides, nextFreeRoutingSlotAtPort gets incremented and forces a higher slot on the current
        // side). Calling update_holder_routing_slot_count before shift captures pre-shift slots that may
        // be lower than the final slots, causing routing_slot_positions to have fewer entries than needed
        // and base_vector() to return 0.0 for out-of-bounds slot indices.
        shift_towards_node(
            &loops,
            &mut loop_routing_state,
            &side_port_ranges,
            port_count,
            &label_crossing_matrix,
            &label_id_by_loop,
            &sl_loop_activity_over_ports,
        );
        // Must be called AFTER shift_towards_node to capture final (potentially increased) slot values.
        update_holder_routing_slot_count(holder, &loop_routing_state);
    }
}

fn reset_routing_slots(loops: &[SelfHyperLoopRef]) {
    for sl_loop in loops {
        {
            let mut sl_loop_guard = sl_loop.lock();
            sl_loop_guard.clear_routing_slots();
        }
    }
}

fn compute_label_crossing_matrix(
    loops: &[SelfHyperLoopRef],
) -> (LabelCrossingMatrix, LabelIdByLoop) {
    let mut label_id_by_loop = HashMap::new();
    let mut label_id = 0usize;
    for sl_loop in loops {
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
    let sl_loop_guard = sl_loop.lock();
    let sl_labels = sl_loop_guard.sl_labels()?;
    let side = sl_labels.side();
    let start = sl_labels.position().x;
    let end = start + sl_labels.size().x;
    Some((side, start, end))
}

fn loop_has_labels(sl_loop: &SelfHyperLoopRef) -> bool {
    // Java parity: match `getSLLabels() != null` — don't check if labels list is empty
    sl_loop.lock().sl_labels().is_some()
}

fn create_crossing_graph(
    loops: &[SelfHyperLoopRef],
    port_count: usize,
    label_crossing_matrix: &LabelCrossingMatrix,
    label_id_by_loop: &LabelIdByLoop,
) -> (Vec<HyperEdgeSegmentRef>, SegmentMap, LoopActivity) {
    let mut hyper_edge_segments = Vec::with_capacity(loops.len());
    let mut sl_loop_to_segment_map = HashMap::new();
    for sl_loop in loops {
        let segment = HyperEdgeSegment::new(RoutingDirection::WestToEast);
        hyper_edge_segments.push(segment.clone());
        sl_loop_to_segment_map.insert(loop_key(sl_loop), segment);
    }

    let sl_loop_activity_over_ports = compute_loop_activity(port_count, loops);

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

fn compute_loop_activity(port_count: usize, loops: &[SelfHyperLoopRef]) -> LoopActivity {
    let mut activity = HashMap::new();
    for sl_loop in loops {
        let mut loop_activity = vec![false; port_count];

        let (leftmost_port, rightmost_port) = {
            let sl_loop_guard = sl_loop.lock();
            (
                sl_loop_guard.leftmost_port(),
                sl_loop_guard.rightmost_port(),
            )
        };
        let (Some(leftmost_port), Some(rightmost_port)) = (leftmost_port, rightmost_port) else {
            activity.insert(loop_key(sl_loop), loop_activity);
            continue;
        };

        let leftmost_port_id =
            crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id(
                &leftmost_port,
            );
        let rightmost_port_id =
            crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id(
                &rightmost_port,
            );

        if port_count > 0 && leftmost_port_id >= 0 && rightmost_port_id >= 0 {
            let mut port_idx = leftmost_port_id - 1;
            let target_idx = rightmost_port_id;
            let port_count_i32 = port_count as i32;

            while port_idx != target_idx {
                port_idx = (port_idx + 1).rem_euclid(port_count_i32);
                loop_activity[port_idx as usize] = true;
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
    let Some(lower_loop_activity) = sl_loop_activity_over_ports.get(&loop_key(sl_lower_loop))
    else {
        return 0;
    };

    let sl_upper_ports = sl_upper_loop
        .lock().sl_ports().clone();

    let mut crossings = 0i32;
    for sl_port in sl_upper_ports {
        let port_id =
            crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id(
                &sl_port,
            );
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

fn assign_raw_routing_slots_to_loops(
    loops: &[SelfHyperLoopRef],
    sl_loop_to_segment_map: &SegmentMap,
) -> HashMap<LoopKey, LoopRoutingState> {
    let mut loop_routing_state = HashMap::with_capacity(loops.len());
    for sl_loop in loops {
        let key = loop_key(sl_loop);
        let slot = sl_loop_to_segment_map
            .get(&key)
            .map(|segment| segment.borrow().routing_slot())
            .unwrap_or(0);
        let mut state = LoopRoutingState::new();

        {
            let mut sl_loop_guard = sl_loop.lock();
            let occupied_port_sides = sl_loop_guard
                .occupied_port_sides()
                .iter()
                .copied()
                .collect::<Vec<_>>();
            for port_side in occupied_port_sides {
                let side_idx = side_index(port_side);
                state.occupied_sides[side_idx] = true;
                state.routing_slots[side_idx] = slot;
                sl_loop_guard.set_routing_slot(port_side, slot);
            }
        }

        loop_routing_state.insert(key, state);
    }

    loop_routing_state
}

fn shift_towards_node(
    loops: &[SelfHyperLoopRef],
    loop_routing_state: &mut HashMap<LoopKey, LoopRoutingState>,
    side_port_ranges: &SidePortRanges,
    port_count: usize,
    label_crossing_matrix: &LabelCrossingMatrix,
    label_id_by_loop: &LabelIdByLoop,
    sl_loop_activity_over_ports: &LoopActivity,
) {
    let shift_context = ShiftContext {
        label_crossing_matrix,
        label_id_by_loop,
        sl_loop_activity_over_ports,
    };
    let mut next_free_routing_slot_at_port = vec![0; port_count];

    shift_towards_node_on_side(
        loops,
        loop_routing_state,
        PortSide::North,
        side_port_ranges[side_index(PortSide::North)],
        &mut next_free_routing_slot_at_port,
        &shift_context,
    );
    shift_towards_node_on_side(
        loops,
        loop_routing_state,
        PortSide::East,
        side_port_ranges[side_index(PortSide::East)],
        &mut next_free_routing_slot_at_port,
        &shift_context,
    );
    shift_towards_node_on_side(
        loops,
        loop_routing_state,
        PortSide::South,
        side_port_ranges[side_index(PortSide::South)],
        &mut next_free_routing_slot_at_port,
        &shift_context,
    );
    shift_towards_node_on_side(
        loops,
        loop_routing_state,
        PortSide::West,
        side_port_ranges[side_index(PortSide::West)],
        &mut next_free_routing_slot_at_port,
        &shift_context,
    );
}

fn shift_towards_node_on_side(
    loops: &[SelfHyperLoopRef],
    loop_routing_state: &mut HashMap<LoopKey, LoopRoutingState>,
    side: PortSide,
    side_port_range: Option<(usize, usize)>,
    next_free_routing_slot_at_port: &mut [i32],
    shift_context: &ShiftContext<'_>,
) {
    let side_idx = side_index(side);
    let mut sl_loops = loops
        .iter()
        .filter(|sl_loop| {
            loop_routing_state
                .get(&loop_key(sl_loop))
                .map(|state| state.occupied_sides[side_idx])
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    sl_loops.sort_by_key(|sl_loop| {
        loop_routing_state
            .get(&loop_key(sl_loop))
            .map(|state| state.routing_slots[side_idx])
            .unwrap_or(i32::MAX)
    });

    let Some((min_l_port_index, max_l_port_index)) = side_port_range else {
        for (slot, sl_loop) in sl_loops.into_iter().enumerate() {
            let loop_key = loop_key(&sl_loop);
            if let Some(state) = loop_routing_state.get_mut(&loop_key) {
                state.routing_slots[side_idx] = slot as i32;
            }
            {
                let mut sl_loop_guard = sl_loop.lock();
                sl_loop_guard.set_routing_slot(side, slot as i32);
            }
        }
        return;
    };

    let mut slot_assigned_to_label = vec![-1; shift_context.label_crossing_matrix.len()];
    for sl_loop in sl_loops {
        let loop_key = loop_key(&sl_loop);
        let active_at_port = shift_context.sl_loop_activity_over_ports.get(&loop_key);
        let mut lowest_available_slot = 0i32;

        for (port_index, next_free_slot) in next_free_routing_slot_at_port
            .iter()
            .enumerate()
            .take(max_l_port_index + 1)
            .skip(min_l_port_index)
        {
            if active_at_port
                .and_then(|active| active.get(port_index))
                .copied()
                .unwrap_or(false)
            {
                lowest_available_slot = lowest_available_slot.max(*next_free_slot);
            }
        }

        if let Some(&our_label_idx) = shift_context.label_id_by_loop.get(&loop_key) {
            let mut slots_with_label_conflicts = HashSet::new();
            for (other_label_idx, crosses) in
                shift_context.label_crossing_matrix[our_label_idx]
                    .iter()
                    .enumerate()
            {
                if *crosses {
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

        if let Some(state) = loop_routing_state.get_mut(&loop_key) {
            state.routing_slots[side_idx] = lowest_available_slot;
        }
        {
            let mut sl_loop_guard = sl_loop.lock();
            sl_loop_guard.set_routing_slot(side, lowest_available_slot);
        }

        for (port_index, next_free_slot) in next_free_routing_slot_at_port
            .iter_mut()
            .enumerate()
            .take(max_l_port_index + 1)
            .skip(min_l_port_index)
        {
            if active_at_port
                .and_then(|active| active.get(port_index))
                .copied()
                .unwrap_or(false)
            {
                *next_free_slot = lowest_available_slot + 1;
            }
        }

        if let Some(&label_idx) = shift_context.label_id_by_loop.get(&loop_key) {
            slot_assigned_to_label[label_idx] = lowest_available_slot;
        }
    }
}

fn collect_port_side_ranges(holder: &SelfLoopHolderRef) -> (usize, SidePortRanges) {
    let ports = holder.lock().l_node().lock().ports().clone();

    let mut side_port_ranges: SidePortRanges = [None; 5];
    for l_port in &ports {
        let (port_side, port_id) = {
            let mut port_guard = l_port.lock();
            (port_guard.side(), port_guard.shape().graph_element().id)
        };
        if port_id < 0 {
            continue;
        }

        let port_id = port_id as usize;
        let side_idx = side_index(port_side);
        if let Some((min_idx, max_idx)) = side_port_ranges[side_idx].as_mut() {
            *min_idx = (*min_idx).min(port_id);
            *max_idx = (*max_idx).max(port_id);
        } else {
            side_port_ranges[side_idx] = Some((port_id, port_id));
        }
    }

    (ports.len(), side_port_ranges)
}

fn update_holder_routing_slot_count(
    holder: &SelfLoopHolderRef,
    loop_routing_state: &HashMap<LoopKey, LoopRoutingState>,
) {
    let mut routing_slot_count = vec![0; 5];
    for state in loop_routing_state.values() {
        for side in [PortSide::North, PortSide::East, PortSide::South, PortSide::West] {
            let side_idx = side_index(side);
            let slot = state.routing_slots[side_idx].max(0);
            routing_slot_count[side_idx] = routing_slot_count[side_idx].max(slot + 1);
        }
    }

    {
        let mut holder_guard = holder.lock();
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
