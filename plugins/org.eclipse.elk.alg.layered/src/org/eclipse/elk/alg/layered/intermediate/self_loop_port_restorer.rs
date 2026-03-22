use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LPortRef, NodeType};
use crate::org::eclipse::elk::alg::layered::intermediate::loops::{
    SelfHyperLoopRef, SelfLoopHolderRef, SelfLoopPortRef, SelfLoopType,
};
use crate::org::eclipse::elk::alg::layered::options::internal_properties::Origin;
use crate::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions, SelfLoopDistributionStrategy, SelfLoopOrderingStrategy,
};

pub struct SelfLoopPortRestorer;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum PortSideArea {
    Start,
    Middle,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AddMode {
    Prepend,
    Append,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Target {
    first_side: PortSide,
    second_side: PortSide,
}

impl Target {
    const fn new(first_side: PortSide, second_side: PortSide) -> Self {
        Self {
            first_side,
            second_side,
        }
    }

    fn is_corner_target(self) -> bool {
        self.first_side != self.second_side
    }
}

const ASSIGNMENT_TARGETS: [Target; 8] = [
    Target::new(PortSide::North, PortSide::North),
    Target::new(PortSide::South, PortSide::South),
    Target::new(PortSide::East, PortSide::East),
    Target::new(PortSide::West, PortSide::West),
    Target::new(PortSide::West, PortSide::North),
    Target::new(PortSide::North, PortSide::East),
    Target::new(PortSide::South, PortSide::West),
    Target::new(PortSide::East, PortSide::South),
];

type TargetAreas = HashMap<(PortSide, PortSideArea), Vec<SelfLoopPortRef>>;
type LoopsByType = HashMap<SelfLoopType, Vec<SelfHyperLoopRef>>;

impl ILayoutProcessor<LGraph> for SelfLoopPortRestorer {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Self-Loop ordering", 1.0);

        let nodes = graph
            .layers()
            .iter()
            .flat_map(|layer| {
                layer
                    .lock().nodes().clone()
            })
            .collect::<Vec<_>>();

        for node in nodes {
            let holder = node.lock_ok().and_then(|mut node_guard| {
                if node_guard.node_type() != NodeType::Normal {
                    return None;
                }
                node_guard.get_property(InternalProperties::SELF_LOOP_HOLDER)
            });
            let Some(holder) = holder else {
                continue;
            };

            process_node(&holder);
        }

        monitor.done();
    }
}

fn process_node(holder: &SelfLoopHolderRef) {
    let ports_hidden = holder
        .lock_ok()
        .is_some_and(|holder_guard| holder_guard.are_ports_hidden());

    if !ports_hidden {
        compute_self_loop_types(holder);
        return;
    }

    let original_constraints = holder
        .lock_ok()
        .and_then(|holder_guard| {
            holder_guard
                .l_node()
                .lock_ok()
                .and_then(|mut node_guard| {
                    node_guard.get_property(InternalProperties::ORIGINAL_PORT_CONSTRAINTS)
                })
        })
        .unwrap_or(PortConstraints::Undefined);

    match original_constraints {
        PortConstraints::Undefined | PortConstraints::Free => {
            assign_hidden_port_sides(holder);
            compute_self_loop_types(holder);
            restore_ports(holder);
        }
        PortConstraints::FixedSide => {
            compute_self_loop_types(holder);
            restore_ports(holder);
        }
        PortConstraints::FixedOrder | PortConstraints::FixedRatio | PortConstraints::FixedPos => {
            debug_assert!(
                false,
                "hidden self-loop ports with fixed order constraints should not occur"
            );
            compute_self_loop_types(holder);
            restore_ports(holder);
        }
    }
}

fn compute_self_loop_types(holder: &SelfLoopHolderRef) {
    let loops = holder
        .lock().sl_hyper_loops().clone();
    for sl_loop in loops {
        {
            let mut sl_loop_guard = sl_loop.lock();
            sl_loop_guard.compute_ports_per_side();
        }
    }
}

fn assign_hidden_port_sides(holder: &SelfLoopHolderRef) {
    let (distribution, loops) = holder
        .lock_ok()
        .map(|holder_guard| {
            let l_node = holder_guard.l_node().clone();
            let loops = holder_guard.sl_hyper_loops().clone();
            let distribution = l_node
                .lock_ok()
                .and_then(|mut node_guard| {
                    node_guard.get_property(LayeredOptions::EDGE_ROUTING_SELF_LOOP_DISTRIBUTION)
                })
                .unwrap_or(SelfLoopDistributionStrategy::North);
            (distribution, loops)
        })
        .unwrap_or((SelfLoopDistributionStrategy::North, Vec::new()));

    match distribution {
        SelfLoopDistributionStrategy::North => assign_to_north_side(&loops),
        SelfLoopDistributionStrategy::NorthSouth => assign_to_north_or_south_side(&loops),
        SelfLoopDistributionStrategy::Equally => assign_to_all_sides(&loops),
    }
}

fn assign_to_north_side(loops: &[SelfHyperLoopRef]) {
    for sl_loop in loops {
        for sl_port in hidden_ports_of_loop(sl_loop) {
            set_hidden_port_side(&sl_port, PortSide::North);
        }
    }
}

fn assign_to_north_or_south_side(loops: &[SelfHyperLoopRef]) {
    let mut north_ports = 0usize;
    let mut south_ports = 0usize;

    for sl_loop in loops {
        let hidden_ports = hidden_ports_of_loop(sl_loop);
        if hidden_ports.is_empty() {
            continue;
        }

        let target_side = if north_ports <= south_ports {
            north_ports += hidden_ports.len();
            PortSide::North
        } else {
            south_ports += hidden_ports.len();
            PortSide::South
        };

        for sl_port in hidden_ports {
            set_hidden_port_side(&sl_port, target_side);
        }
    }
}

fn assign_to_all_sides(loops: &[SelfHyperLoopRef]) {
    let mut sorted_loops = loops.to_vec();
    sorted_loops.sort_by(|left, right| {
        let left_size = left
            .lock_ok()
            .map(|loop_guard| loop_guard.sl_ports().len())
            .unwrap_or_default();
        let right_size = right
            .lock_ok()
            .map(|loop_guard| loop_guard.sl_ports().len())
            .unwrap_or_default();
        right_size.cmp(&left_size)
    });

    for (index, sl_loop) in sorted_loops.iter().enumerate() {
        let target = ASSIGNMENT_TARGETS[index % ASSIGNMENT_TARGETS.len()];
        assign_to_target(sl_loop, target);
    }
}

fn assign_to_target(sl_loop: &SelfHyperLoopRef, target: Target) {
    let mut sl_ports = sl_loop
        .lock().sl_ports().clone();

    if target.is_corner_target() {
        sl_ports.sort_by_key(sl_port_net_flow);
    }

    let second_half_start_index = sl_ports.len() / 2;
    for sl_port in sl_ports.iter().take(second_half_start_index) {
        set_hidden_port_side(sl_port, target.first_side);
    }
    for sl_port in sl_ports.iter().skip(second_half_start_index) {
        set_hidden_port_side(sl_port, target.second_side);
    }
}

fn set_hidden_port_side(sl_port: &SelfLoopPortRef, side: PortSide) {
    let l_port = sl_port.lock_ok().and_then(|port_guard| {
        if port_guard.is_hidden() {
            Some(port_guard.l_port().clone())
        } else {
            None
        }
    });
    if let Some(l_port) = l_port {
        {
            let mut l_port_guard = l_port.lock();
            l_port_guard.set_side(side);
        }
    }
}

fn hidden_ports_of_loop(sl_loop: &SelfHyperLoopRef) -> Vec<SelfLoopPortRef> {
    sl_loop
        .lock().sl_ports().clone()
        .into_iter()
        .filter(sl_port_is_hidden)
        .collect()
}

fn restore_ports(holder: &SelfLoopHolderRef) {
    let mut target_areas = init_target_areas();
    let mut loops_by_type = gather_self_loops_by_type(holder);
    let ordering = holder
        .lock_ok()
        .and_then(|holder_guard| {
            holder_guard
                .l_node()
                .lock_ok()
                .and_then(|mut node_guard| {
                    node_guard.get_property(LayeredOptions::EDGE_ROUTING_SELF_LOOP_ORDERING)
                })
        })
        .unwrap_or(SelfLoopOrderingStrategy::Stacked);

    process_one_side_loops(ordering, &mut loops_by_type, &mut target_areas);
    process_two_side_corner_loops(&loops_by_type, &mut target_areas);
    process_three_side_loops(&loops_by_type, &mut target_areas);
    process_four_side_loops(&loops_by_type, &mut target_areas);
    process_two_side_opposing_loops(&loops_by_type, &mut target_areas);

    restore_ports_to_node(holder, &target_areas);

    for sl_ports in target_areas.values() {
        for sl_port in sl_ports {
            {
                let mut sl_port_guard = sl_port.lock();
                sl_port_guard.set_hidden(false);
            }
        }
    }
    {
        let mut holder_guard = holder.lock();
        holder_guard.set_ports_hidden(false);
    }
}

fn init_target_areas() -> TargetAreas {
    let mut target_areas = HashMap::new();
    let sides = [
        PortSide::Undefined,
        PortSide::North,
        PortSide::East,
        PortSide::South,
        PortSide::West,
    ];
    let areas = [PortSideArea::Start, PortSideArea::Middle, PortSideArea::End];

    for side in sides {
        for area in areas {
            target_areas.insert((side, area), Vec::new());
        }
    }

    target_areas
}

fn gather_self_loops_by_type(holder: &SelfLoopHolderRef) -> LoopsByType {
    let loops = holder
        .lock().sl_hyper_loops().clone();

    let mut loops_by_type = HashMap::new();
    for sl_loop in loops {
        let sl_type = sl_loop
            .lock().self_loop_type();
        if let Some(sl_type) = sl_type {
            loops_by_type
                .entry(sl_type)
                .or_insert_with(Vec::new)
                .push(sl_loop);
        }
    }

    loops_by_type
}

fn process_one_side_loops(
    ordering: SelfLoopOrderingStrategy,
    loops_by_type: &mut LoopsByType,
    target_areas: &mut TargetAreas,
) {
    let Some(one_side_loops) = loops_by_type.get_mut(&SelfLoopType::OneSide) else {
        return;
    };

    if ordering == SelfLoopOrderingStrategy::ReverseStacked {
        one_side_loops.reverse();
    }

    for sl_loop in one_side_loops {
        let side = sl_loop
            .lock_ok()
            .and_then(|loop_guard| loop_guard.sl_ports().first().cloned())
            .map(|sl_port| sl_port_side(&sl_port))
            .unwrap_or(PortSide::Undefined);
        if side == PortSide::Undefined {
            continue;
        }

        let mut sorted_ports = sl_loop
            .lock().sl_ports().clone();
        sorted_ports.sort_by_key(sl_port_net_flow);

        match ordering {
            SelfLoopOrderingStrategy::Sequenced => {
                add_to_target_area_from_ports(
                    &sorted_ports,
                    side,
                    PortSideArea::Middle,
                    AddMode::Append,
                    target_areas,
                );
            }
            SelfLoopOrderingStrategy::Stacked | SelfLoopOrderingStrategy::ReverseStacked => {
                let split_index = compute_port_list_split_index(&sorted_ports);
                add_to_target_area_from_ports(
                    &sorted_ports[..split_index],
                    side,
                    PortSideArea::Middle,
                    AddMode::Prepend,
                    target_areas,
                );
                add_to_target_area_from_ports(
                    &sorted_ports[split_index..],
                    side,
                    PortSideArea::Middle,
                    AddMode::Append,
                    target_areas,
                );
            }
        }
    }
}

fn compute_port_list_split_index(sorted_ports: &[SelfLoopPortRef]) -> usize {
    if sorted_ports.is_empty() {
        return 0;
    }

    let mut positive_net_flow_index = 0usize;
    while positive_net_flow_index < sorted_ports.len() {
        if sl_port_net_flow(&sorted_ports[positive_net_flow_index]) > 0 {
            break;
        }
        positive_net_flow_index += 1;
    }
    if positive_net_flow_index > 0 && positive_net_flow_index < sorted_ports.len() - 1 {
        return positive_net_flow_index;
    }

    let mut non_negative_net_flow_index = 0usize;
    while non_negative_net_flow_index < sorted_ports.len() {
        // Keep Java behavior: second pass checks > 0 again.
        if sl_port_net_flow(&sorted_ports[non_negative_net_flow_index]) > 0 {
            break;
        }
        non_negative_net_flow_index += 1;
    }
    if non_negative_net_flow_index > 0 && positive_net_flow_index < sorted_ports.len() - 1 {
        return non_negative_net_flow_index;
    }

    sorted_ports.len() / 2
}

fn process_two_side_corner_loops(loops_by_type: &LoopsByType, target_areas: &mut TargetAreas) {
    let Some(loops) = loops_by_type.get(&SelfLoopType::TwoSidesCorner) else {
        return;
    };

    for sl_loop in loops {
        let Some([start_side, target_side]) = sorted_two_side_loop_port_sides(sl_loop) else {
            continue;
        };
        add_to_target_area_loop(
            sl_loop,
            start_side,
            PortSideArea::End,
            AddMode::Prepend,
            target_areas,
        );
        add_to_target_area_loop(
            sl_loop,
            target_side,
            PortSideArea::Start,
            AddMode::Append,
            target_areas,
        );
    }
}

fn process_two_side_opposing_loops(loops_by_type: &LoopsByType, target_areas: &mut TargetAreas) {
    let Some(loops) = loops_by_type.get(&SelfLoopType::TwoSidesOpposing) else {
        return;
    };

    for sl_loop in loops {
        let Some([start_side, target_side]) = sorted_two_side_loop_port_sides(sl_loop) else {
            continue;
        };
        add_to_target_area_loop(
            sl_loop,
            start_side,
            PortSideArea::End,
            AddMode::Prepend,
            target_areas,
        );
        add_to_target_area_loop(
            sl_loop,
            target_side,
            PortSideArea::Start,
            AddMode::Append,
            target_areas,
        );
    }
}

fn sorted_two_side_loop_port_sides(sl_loop: &SelfHyperLoopRef) -> Option<[PortSide; 2]> {
    let mut sides = loop_port_sides(sl_loop);
    if sides.len() != 2 {
        return None;
    }
    sides.sort();
    if sides[0] == PortSide::North && sides[1] == PortSide::West {
        sides.swap(0, 1);
    }
    Some([sides[0], sides[1]])
}

fn process_three_side_loops(loops_by_type: &LoopsByType, target_areas: &mut TargetAreas) {
    let Some(loops) = loops_by_type.get(&SelfLoopType::ThreeSides) else {
        return;
    };

    for sl_loop in loops {
        let Some([start_side, middle_side, end_side]) = determine_loop_constellation(sl_loop)
        else {
            continue;
        };
        add_to_target_area_loop(
            sl_loop,
            start_side,
            PortSideArea::End,
            AddMode::Prepend,
            target_areas,
        );
        add_to_target_area_loop(
            sl_loop,
            middle_side,
            PortSideArea::Middle,
            AddMode::Append,
            target_areas,
        );
        add_to_target_area_loop(
            sl_loop,
            end_side,
            PortSideArea::Start,
            AddMode::Append,
            target_areas,
        );
    }
}

fn determine_loop_constellation(sl_loop: &SelfHyperLoopRef) -> Option<[PortSide; 3]> {
    let sides = loop_port_sides(sl_loop).into_iter().collect::<HashSet<_>>();

    if !sides.contains(&PortSide::North) {
        Some([PortSide::East, PortSide::South, PortSide::West])
    } else if !sides.contains(&PortSide::East) {
        Some([PortSide::South, PortSide::West, PortSide::North])
    } else if !sides.contains(&PortSide::South) {
        Some([PortSide::West, PortSide::North, PortSide::East])
    } else if !sides.contains(&PortSide::West) {
        Some([PortSide::North, PortSide::East, PortSide::South])
    } else {
        None
    }
}

fn process_four_side_loops(loops_by_type: &LoopsByType, target_areas: &mut TargetAreas) {
    let Some(loops) = loops_by_type.get(&SelfLoopType::FourSides) else {
        return;
    };

    for sl_loop in loops {
        for side in loop_port_sides(sl_loop) {
            add_to_target_area_loop(
                sl_loop,
                side,
                PortSideArea::Middle,
                AddMode::Append,
                target_areas,
            );
        }
    }
}

fn add_to_target_area_loop(
    sl_loop: &SelfHyperLoopRef,
    port_side: PortSide,
    area: PortSideArea,
    add_mode: AddMode,
    target_areas: &mut TargetAreas,
) {
    let ports = sl_loop
        .lock_ok()
        .map(|loop_guard| loop_guard.ports_on_side(port_side))
        .unwrap_or_default();
    add_to_target_area_from_ports(&ports, port_side, area, add_mode, target_areas);
}

fn add_to_target_area_from_ports(
    sl_ports: &[SelfLoopPortRef],
    port_side: PortSide,
    area: PortSideArea,
    add_mode: AddMode,
    target_areas: &mut TargetAreas,
) {
    let mut hidden_ports = sl_ports
        .iter()
        .filter(|sl_port| sl_port_is_hidden(sl_port))
        .cloned()
        .collect::<Vec<_>>();
    hidden_ports.reverse();

    let target_area = target_areas.entry((port_side, area)).or_default();
    if add_mode == AddMode::Prepend {
        target_area.splice(0..0, hidden_ports);
    } else {
        target_area.extend(hidden_ports);
    }
}

fn restore_ports_to_node(holder: &SelfLoopHolderRef, target_areas: &TargetAreas) {
    let l_node = holder
        .lock_ok()
        .map(|holder_guard| holder_guard.l_node().clone());
    let Some(l_node) = l_node else {
        return;
    };

    let old_port_list = l_node
        .lock().ports().clone();

    {
        let mut l_node_guard = l_node.lock();
        l_node_guard.ports_mut().clear();
    }

    let mut next_old_port_index = 0usize;

    add_all(target_areas, PortSide::North, PortSideArea::Start, &l_node);
    next_old_port_index = add_all_that(
        &old_port_list,
        next_old_port_index,
        |l_port| {
            l_port_side(l_port) == PortSide::North
                && is_north_south_port_with_west_or_west_east_connections(l_port)
        },
        &l_node,
    );
    add_all(target_areas, PortSide::North, PortSideArea::Middle, &l_node);
    next_old_port_index = add_all_that(
        &old_port_list,
        next_old_port_index,
        |l_port| l_port_side(l_port) == PortSide::North,
        &l_node,
    );
    add_all(target_areas, PortSide::North, PortSideArea::End, &l_node);

    add_all(target_areas, PortSide::East, PortSideArea::Start, &l_node);
    add_all(target_areas, PortSide::East, PortSideArea::Middle, &l_node);
    next_old_port_index = add_all_that(
        &old_port_list,
        next_old_port_index,
        |l_port| l_port_side(l_port) == PortSide::East,
        &l_node,
    );
    add_all(target_areas, PortSide::East, PortSideArea::End, &l_node);

    add_all(target_areas, PortSide::South, PortSideArea::Start, &l_node);
    next_old_port_index = add_all_that(
        &old_port_list,
        next_old_port_index,
        |l_port| {
            l_port_side(l_port) == PortSide::South
                && is_north_south_port_with_east_connections(l_port)
        },
        &l_node,
    );
    add_all(target_areas, PortSide::South, PortSideArea::Middle, &l_node);
    next_old_port_index = add_all_that(
        &old_port_list,
        next_old_port_index,
        |l_port| l_port_side(l_port) == PortSide::South,
        &l_node,
    );
    add_all(target_areas, PortSide::South, PortSideArea::End, &l_node);

    add_all(target_areas, PortSide::West, PortSideArea::Start, &l_node);
    next_old_port_index = add_all_that(
        &old_port_list,
        next_old_port_index,
        |l_port| l_port_side(l_port) == PortSide::West,
        &l_node,
    );
    add_all(target_areas, PortSide::West, PortSideArea::Middle, &l_node);
    add_all(target_areas, PortSide::West, PortSideArea::End, &l_node);

    if next_old_port_index < old_port_list.len() {
        {
            let mut l_node_guard = l_node.lock();
            l_node_guard
                .ports_mut()
                .extend(old_port_list[next_old_port_index..].iter().cloned());
        }
    }
}

fn add_all(
    target_areas: &TargetAreas,
    side: PortSide,
    area: PortSideArea,
    l_node: &crate::org::eclipse::elk::alg::layered::graph::LNodeRef,
) {
    let sl_ports = target_areas.get(&(side, area)).cloned().unwrap_or_default();
    for sl_port in sl_ports {
        let l_port = sl_port
            .lock_ok()
            .map(|sl_port_guard| sl_port_guard.l_port().clone());
        if let Some(l_port) = l_port {
            crate::org::eclipse::elk::alg::layered::graph::LPort::set_node(
                &l_port,
                Some(l_node.clone()),
            );
        }
    }
}

fn add_all_that<F>(
    l_ports: &[LPortRef],
    from_index: usize,
    condition: F,
    target_node: &crate::org::eclipse::elk::alg::layered::graph::LNodeRef,
) -> usize
where
    F: Fn(&LPortRef) -> bool,
{
    for (index, l_port) in l_ports.iter().enumerate().skip(from_index) {
        if condition(l_port) {
            {
                let mut target_guard = target_node.lock();
                target_guard.ports_mut().push(l_port.clone());
            }
        } else {
            return index;
        }
    }

    l_ports.len()
}

fn is_north_south_port_with_west_or_west_east_connections(l_port: &LPortRef) -> bool {
    let connection_sides = north_south_port_connection_sides(l_port);
    connection_sides.contains(&PortSide::West)
}

fn is_north_south_port_with_east_connections(l_port: &LPortRef) -> bool {
    let connection_sides = north_south_port_connection_sides(l_port);
    connection_sides.contains(&PortSide::East)
}

fn north_south_port_connection_sides(l_port: &LPortRef) -> HashSet<PortSide> {
    let mut connection_sides = HashSet::new();
    let port_dummy = l_port
        .lock_ok()
        .and_then(|mut l_port_guard| l_port_guard.get_property(InternalProperties::PORT_DUMMY));
    let Some(port_dummy) = port_dummy else {
        return connection_sides;
    };

    let dummy_ports = port_dummy
        .lock().ports().clone();
    for dummy_l_port in dummy_ports {
        let origin = dummy_l_port.lock_ok().and_then(|mut dummy_port_guard| {
            dummy_port_guard.get_property(InternalProperties::ORIGIN)
        });
        let Some(Origin::LPort(origin_port)) = origin else {
            continue;
        };
        if !Arc::ptr_eq(&origin_port, l_port) {
            continue;
        }

        let (side, has_edges) = dummy_l_port
            .lock_ok()
            .map(|dummy_port_guard| {
                (
                    dummy_port_guard.side(),
                    !dummy_port_guard.connected_edges().is_empty(),
                )
            })
            .unwrap_or((PortSide::Undefined, false));
        if has_edges {
            connection_sides.insert(side);
        }
    }

    connection_sides
}

fn sl_port_is_hidden(sl_port: &SelfLoopPortRef) -> bool {
    sl_port
        .lock().is_hidden()
}

fn l_port_side(l_port: &LPortRef) -> PortSide {
    l_port
        .lock().side()
}

fn sl_port_side(sl_port: &SelfLoopPortRef) -> PortSide {
    sl_port
        .lock_ok()
        .and_then(|port_guard| {
            port_guard
                .l_port()
                .lock_ok()
                .map(|l_port_guard| l_port_guard.side())
        })
        .unwrap_or(PortSide::Undefined)
}

fn sl_port_net_flow(sl_port: &SelfLoopPortRef) -> isize {
    sl_port
        .lock_ok()
        .and_then(|port_guard| {
            port_guard
                .l_port()
                .lock_ok()
                .map(|l_port_guard| l_port_guard.net_flow())
        })
        .unwrap_or_default()
}

fn loop_port_sides(sl_loop: &SelfHyperLoopRef) -> Vec<PortSide> {
    let mut sides = Vec::new();
    let sl_ports = sl_loop
        .lock().sl_ports().clone();
    for sl_port in sl_ports {
        let side = sl_port_side(&sl_port);
        if side != PortSide::Undefined && !sides.contains(&side) {
            sides.push(side);
        }
    }
    sides
}
