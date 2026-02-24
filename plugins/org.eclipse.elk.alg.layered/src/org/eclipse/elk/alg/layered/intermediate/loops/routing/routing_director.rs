use std::collections::HashSet;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::intermediate::loops::{
    SelfLoopHolderRef, SelfLoopPortRef, SelfLoopType,
};

pub struct RoutingDirector;

const UNCONNECTED_PORT_PENALTY: i32 = 1;
const CONNECTED_PORT_PENALTY: i32 = 3;

impl RoutingDirector {
    pub fn determine_loop_routes(&self, holder: &SelfLoopHolderRef) {
        assign_port_ids(holder);
        let port_penalties = compute_port_penalties(holder);

        let loops = holder
            .lock()
            .ok()
            .map(|holder_guard| holder_guard.sl_hyper_loops().clone())
            .unwrap_or_default();

        for sl_loop in loops {
            if let Ok(mut sl_loop_guard) = sl_loop.lock() {
                sl_loop_guard.sort_ports_by_id();
                sl_loop_guard.compute_ports_per_side();
                determine_loop_route(&mut sl_loop_guard, holder, &port_penalties);
                compute_occupied_port_sides(&mut sl_loop_guard);
            }
        }
    }
}

fn assign_port_ids(holder: &SelfLoopHolderRef) {
    // Java parity: assignPortIds assigns IDs by list position (0, 1, 2, ...).
    // Java's port list is sorted by side+position (from NodePortSorter earlier in the pipeline).
    // Rust MUST NOT sort by PORT_INDEX here - that changes which "column" each port occupies
    // in nextFreeRoutingSlotAtPort, causing incorrect slot assignments in shiftTowardsNode.
    let ports = holder
        .lock()
        .ok()
        .map(|holder_guard| holder_guard.l_node().clone())
        .and_then(|node| {
            node.lock()
                .ok()
                .map(|node_guard| node_guard.ports().clone())
        })
        .unwrap_or_default();

    for (index, port) in ports.into_iter().enumerate() {
        if let Ok(mut port_guard) = port.lock() {
            port_guard.shape().graph_element().id = index as i32;
        }
    }
}

fn determine_loop_route(
    sl_loop: &mut crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop,
    holder: &SelfLoopHolderRef,
    port_penalties: &[i32],
) {
    match sl_loop.self_loop_type() {
        Some(SelfLoopType::OneSide) => determine_one_side_route(sl_loop),
        Some(SelfLoopType::TwoSidesCorner) => determine_two_side_corner_route(sl_loop),
        Some(SelfLoopType::TwoSidesOpposing) => {
            determine_two_side_opposing_route(sl_loop, holder, port_penalties)
        }
        Some(SelfLoopType::ThreeSides) => determine_three_side_route(sl_loop),
        Some(SelfLoopType::FourSides) => determine_four_side_route(sl_loop, holder, port_penalties),
        None => determine_general_route(sl_loop),
    }
}

fn determine_one_side_route(
    sl_loop: &mut crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop,
) {
    let side = sl_loop
        .sl_ports()
        .first()
        .map(sl_port_side)
        .unwrap_or(PortSide::Undefined);
    if side == PortSide::Undefined {
        sl_loop.set_leftmost_port(None);
        sl_loop.set_rightmost_port(None);
        return;
    }

    let side_ports = sl_loop.ports_on_side(side);
    sl_loop.set_leftmost_port(
        side_ports
            .iter()
            .min_by_key(|sl_port| {
                crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id(
                    sl_port,
                )
            })
            .cloned(),
    );
    sl_loop.set_rightmost_port(
        side_ports
            .iter()
            .max_by_key(|sl_port| {
                crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id(
                    sl_port,
                )
            })
            .cloned(),
    );
}

fn determine_general_route(
    sl_loop: &mut crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop,
) {
    let ports = sl_loop.sl_ports().clone();
    sl_loop.set_leftmost_port(
        ports
            .iter()
            .min_by_key(|sl_port| {
                crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id(
                    sl_port,
                )
            })
            .cloned(),
    );
    sl_loop.set_rightmost_port(
        ports
            .iter()
            .max_by_key(|sl_port| {
                crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id(
                    sl_port,
                )
            })
            .cloned(),
    );
}

fn determine_two_side_corner_route(
    sl_loop: &mut crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop,
) {
    let sides = loop_sides(sl_loop);
    if sides.len() != 2 {
        determine_general_route(sl_loop);
        return;
    }

    let (left_side, right_side) = if sides[0].right() == sides[1] {
        (sides[0], sides[1])
    } else if sides[1].right() == sides[0] {
        (sides[1], sides[0])
    } else {
        (sides[0], sides[1])
    };

    assign_leftmost_rightmost_ports(sl_loop, left_side, right_side);
}

fn determine_two_side_opposing_route(
    sl_loop: &mut crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop,
    holder: &SelfLoopHolderRef,
    port_penalties: &[i32],
) {
    let sides = loop_sides(sl_loop);
    if sides.len() != 2 {
        determine_general_route(sl_loop);
        return;
    }

    let Some(option1_left) = lowest_port_on_side(sl_loop, sides[0]) else {
        determine_general_route(sl_loop);
        return;
    };
    let Some(option1_right) = highest_port_on_side(sl_loop, sides[1]) else {
        determine_general_route(sl_loop);
        return;
    };
    let option1_penalty =
        compute_edge_penalty(holder, port_penalties, &option1_left, &option1_right);

    let Some(option2_left) = lowest_port_on_side(sl_loop, sides[1]) else {
        determine_general_route(sl_loop);
        return;
    };
    let Some(option2_right) = highest_port_on_side(sl_loop, sides[0]) else {
        determine_general_route(sl_loop);
        return;
    };
    let option2_penalty =
        compute_edge_penalty(holder, port_penalties, &option2_left, &option2_right);

    if option1_penalty < option2_penalty {
        sl_loop.set_leftmost_port(Some(option1_left));
        sl_loop.set_rightmost_port(Some(option1_right));
        return;
    }
    if option2_penalty < option1_penalty {
        sl_loop.set_leftmost_port(Some(option2_left));
        sl_loop.set_rightmost_port(Some(option2_right));
        return;
    }

    // Java parity: for NORTH/SOUTH opposing loops, tie handling follows option order.
    // Keep option1 on tie to match RoutingDirector#determineTwoSideOpposingLoopRoutes.
    if is_north_south_opposing_pair(sides[0], sides[1]) {
        sl_loop.set_leftmost_port(Some(option1_left));
        sl_loop.set_rightmost_port(Some(option1_right));
        return;
    }

    // Java parity: opposing-side ties are effectively biased towards top/left routing.
    // Choose the option whose clockwise intermediate side is preferred.
    let option1_mid = sides[0].right();
    let option2_mid = sides[1].right();
    if opposing_tie_break_rank(option1_mid) <= opposing_tie_break_rank(option2_mid) {
        sl_loop.set_leftmost_port(Some(option1_left));
        sl_loop.set_rightmost_port(Some(option1_right));
    } else {
        sl_loop.set_leftmost_port(Some(option2_left));
        sl_loop.set_rightmost_port(Some(option2_right));
    }
}

fn opposing_tie_break_rank(side: PortSide) -> i32 {
    match side {
        PortSide::North => 0,
        PortSide::West => 1,
        PortSide::South => 2,
        PortSide::East => 3,
        PortSide::Undefined => 4,
    }
}

fn is_north_south_opposing_pair(a: PortSide, b: PortSide) -> bool {
    (a == PortSide::North && b == PortSide::South)
        || (a == PortSide::South && b == PortSide::North)
}


fn determine_three_side_route(
    sl_loop: &mut crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop,
) {
    let sides = loop_sides(sl_loop);
    if sides.len() != 3 {
        determine_general_route(sl_loop);
        return;
    }

    let missing = [
        PortSide::North,
        PortSide::East,
        PortSide::South,
        PortSide::West,
    ]
    .into_iter()
    .find(|side| !sides.contains(side));

    let Some((left_side, right_side)) = missing.map(|missing_side| match missing_side {
        PortSide::North => (PortSide::East, PortSide::West),
        PortSide::East => (PortSide::South, PortSide::North),
        PortSide::South => (PortSide::West, PortSide::East),
        PortSide::West => (PortSide::North, PortSide::South),
        PortSide::Undefined => (PortSide::North, PortSide::South),
    }) else {
        determine_general_route(sl_loop);
        return;
    };

    assign_leftmost_rightmost_ports(sl_loop, left_side, right_side);
}

fn determine_four_side_route(
    sl_loop: &mut crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop,
    holder: &SelfLoopHolderRef,
    port_penalties: &[i32],
) {
    let sorted_ports = sl_loop.sl_ports().clone();
    if sorted_ports.len() < 2 {
        determine_general_route(sl_loop);
        return;
    }

    let mut worst_left_port = sorted_ports
        .last()
        .cloned()
        .unwrap_or_else(|| sorted_ports[0].clone());
    let mut worst_right_port = sorted_ports[0].clone();
    let mut worst_penalty =
        compute_edge_penalty(holder, port_penalties, &worst_left_port, &worst_right_port);

    for right_index in 1..sorted_ports.len() {
        let curr_left_port = sorted_ports[right_index - 1].clone();
        let curr_right_port = sorted_ports[right_index].clone();
        let curr_penalty =
            compute_edge_penalty(holder, port_penalties, &curr_left_port, &curr_right_port);
        if curr_penalty > worst_penalty {
            worst_left_port = curr_left_port;
            worst_right_port = curr_right_port;
            worst_penalty = curr_penalty;
        }
    }

    // We do not route between the worst pair, so swap while assigning.
    sl_loop.set_leftmost_port(Some(worst_right_port));
    sl_loop.set_rightmost_port(Some(worst_left_port));
}

fn compute_occupied_port_sides(
    sl_loop: &mut crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop,
) {
    let left_side = sl_loop
        .leftmost_port()
        .as_ref()
        .map(sl_port_side)
        .unwrap_or(PortSide::Undefined);
    let right_side = sl_loop
        .rightmost_port()
        .as_ref()
        .map(sl_port_side)
        .unwrap_or(PortSide::Undefined);

    let mut sides = HashSet::new();
    if left_side == PortSide::Undefined || right_side == PortSide::Undefined {
        sl_loop.set_occupied_port_sides(sides);
        return;
    }

    let mut current = left_side;
    for _ in 0..4 {
        sides.insert(current);
        if current == right_side {
            break;
        }
        current = current.right();
    }

    sl_loop.set_occupied_port_sides(sides);
}

fn assign_leftmost_rightmost_ports(
    sl_loop: &mut crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop,
    leftmost_side: PortSide,
    rightmost_side: PortSide,
) {
    sl_loop.set_leftmost_port(lowest_port_on_side(sl_loop, leftmost_side));
    sl_loop.set_rightmost_port(highest_port_on_side(sl_loop, rightmost_side));
}

fn lowest_port_on_side(
    sl_loop: &crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop,
    side: PortSide,
) -> Option<SelfLoopPortRef> {
    sl_loop.ports_on_side(side).into_iter().min_by_key(
        crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id,
    )
}

fn highest_port_on_side(
    sl_loop: &crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop,
    side: PortSide,
) -> Option<SelfLoopPortRef> {
    sl_loop.ports_on_side(side).into_iter().max_by_key(
        crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id,
    )
}

fn loop_sides(
    sl_loop: &crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop,
) -> Vec<PortSide> {
    let mut sides = Vec::new();
    for sl_port in sl_loop.sl_ports() {
        let side = sl_port_side(sl_port);
        if side != PortSide::Undefined && !sides.contains(&side) {
            sides.push(side);
        }
    }
    sides
}

fn compute_port_penalties(holder: &SelfLoopHolderRef) -> Vec<i32> {
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

    let mut by_id: Vec<(usize, i32)> = Vec::with_capacity(ports.len());
    let mut fallback_penalties = vec![0; ports.len()];
    let mut fallback_sum = 0;

    for (index, port) in ports.iter().enumerate() {
        let (port_id, penalty) = port
            .lock()
            .ok()
            .map(|mut port_guard| {
                let penalty =
                    if port_guard.incoming_edges().is_empty() && port_guard.outgoing_edges().is_empty() {
                        UNCONNECTED_PORT_PENALTY
                    } else {
                        CONNECTED_PORT_PENALTY
                    };
                (port_guard.shape().graph_element().id, penalty)
            })
            .unwrap_or((-1, CONNECTED_PORT_PENALTY));

        fallback_sum += penalty;
        fallback_penalties[index] = fallback_sum;

        if port_id >= 0 && (port_id as usize) < ports.len() {
            by_id.push((port_id as usize, penalty));
        }
    }

    if by_id.len() != ports.len() {
        return fallback_penalties;
    }

    by_id.sort_by_key(|(id, _)| *id);
    if by_id
        .iter()
        .enumerate()
        .any(|(expected_id, (actual_id, _))| expected_id != *actual_id)
    {
        return fallback_penalties;
    }

    let mut penalties = vec![0; ports.len()];
    let mut penalty_sum = 0;
    for (id, penalty) in by_id {
        penalty_sum += penalty;
        penalties[id] = penalty_sum;
    }
    penalties
}

fn compute_edge_penalty(
    holder: &SelfLoopHolderRef,
    port_penalties: &[i32],
    leftmost_port: &SelfLoopPortRef,
    rightmost_port: &SelfLoopPortRef,
) -> i32 {
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
        .unwrap_or_default();
    if port_count == 0 || port_penalties.is_empty() {
        return 0;
    }

    let leftmost_port_id =
        crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id(
            leftmost_port,
        );
    let rightmost_port_id =
        crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id(
            rightmost_port,
        );
    if leftmost_port_id < 0 || rightmost_port_id < 0 {
        return 0;
    }

    let left_id = leftmost_port_id as usize;
    let right_id = rightmost_port_id as usize;
    if left_id >= port_count || right_id >= port_count {
        return 0;
    }

    let left_of_right = if right_id == 0 {
        port_count.saturating_sub(1)
    } else {
        right_id - 1
    };
    if left_id <= left_of_right {
        port_penalties[left_of_right] - port_penalties[left_id]
    } else {
        port_penalties[port_count - 1] - port_penalties[left_id] + port_penalties[left_of_right]
    }
}

fn sl_port_side(sl_port: &SelfLoopPortRef) -> PortSide {
    sl_port
        .lock()
        .ok()
        .and_then(|port_guard| {
            port_guard
                .l_port()
                .lock()
                .ok()
                .map(|l_port_guard| l_port_guard.side())
        })
        .unwrap_or(PortSide::Undefined)
}
