use std::cmp::Ordering;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LPortRef};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions, PortSortingStrategy};

pub struct PortListSorter;

impl Default for PortListSorter {
    fn default() -> Self {
        PortListSorter
    }
}

impl ILayoutProcessor<LGraph> for PortListSorter {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Port order processing", 1.0);

        let sorting_strategy = graph
            .get_property(LayeredOptions::PORT_SORTING_STRATEGY)
            .unwrap_or(PortSortingStrategy::InputOrder);

        let layers = graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            for node in nodes {
                let constraints = node
                    .lock()
                    .ok()
                    .and_then(|mut node_guard| node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS))
                    .unwrap_or(PortConstraints::Undefined);

                if let Ok(mut node_guard) = node.lock() {
                    if constraints.is_order_fixed() {
                        stable_sort_by(node_guard.ports_mut(), |p1, p2| {
                            compare_ports_combined(p1, p2, constraints)
                        });
                    } else if constraints.is_side_fixed() {
                        stable_sort_by(node_guard.ports_mut(), compare_port_side);
                        reverse_west_and_south_side(node_guard.ports_mut());

                        if sorting_strategy == PortSortingStrategy::PortDegree {
                            stable_sort_by(
                                node_guard.ports_mut(),
                                compare_port_degree_east_west,
                            );
                        }
                    }
                    node_guard.cache_port_sides();
                }
            }
        }

        monitor.done();
    }
}

fn compare_port_side(p1: &LPortRef, p2: &LPortRef) -> Ordering {
    let side1 = port_side(p1);
    let side2 = port_side(p2);
    side1.cmp(&side2)
}

fn compare_port_degree_east_west(p1: &LPortRef, p2: &LPortRef) -> Ordering {
    let side1 = port_side(p1);
    let side2 = port_side(p2);
    if side1 != side2 {
        return Ordering::Equal;
    }

    match side1 {
        PortSide::East => real_degree(p2, true).cmp(&real_degree(p1, true)),
        PortSide::West => real_degree(p1, false).cmp(&real_degree(p2, false)),
        _ => Ordering::Equal,
    }
}

fn compare_ports_combined(p1: &LPortRef, p2: &LPortRef, constraints: PortConstraints) -> Ordering {
    let side_cmp = compare_port_side(p1, p2);
    if side_cmp != Ordering::Equal {
        return side_cmp;
    }

    compare_fixed_order_and_pos(p1, p2, constraints)
}

fn compare_fixed_order_and_pos(
    p1: &LPortRef,
    p2: &LPortRef,
    constraints: PortConstraints,
) -> Ordering {
    let side1 = port_side(p1);
    let side2 = port_side(p2);
    if side1 != side2 || !constraints.is_order_fixed() {
        return Ordering::Equal;
    }

    if constraints == PortConstraints::FixedOrder {
        let idx1 = p1
            .lock()
            .ok()
            .and_then(|mut port_guard| port_guard.get_property(LayeredOptions::PORT_INDEX));
        let idx2 = p2
            .lock()
            .ok()
            .and_then(|mut port_guard| port_guard.get_property(LayeredOptions::PORT_INDEX));
        if let (Some(i1), Some(i2)) = (idx1, idx2) {
            if i1 != i2 {
                return i1.cmp(&i2);
            }
        }
    }

    let pos1 = p1
        .lock()
        .ok()
        .map(|mut port_guard| *port_guard.shape().position_ref())
        .unwrap_or_default();
    let pos2 = p2
        .lock()
        .ok()
        .map(|mut port_guard| *port_guard.shape().position_ref())
        .unwrap_or_default();
    match side1 {
        PortSide::North => pos1.x.partial_cmp(&pos2.x).unwrap_or(Ordering::Equal),
        PortSide::East => pos1.y.partial_cmp(&pos2.y).unwrap_or(Ordering::Equal),
        PortSide::South => pos2.x.partial_cmp(&pos1.x).unwrap_or(Ordering::Equal),
        PortSide::West => pos2.y.partial_cmp(&pos1.y).unwrap_or(Ordering::Equal),
        PortSide::Undefined => Ordering::Equal,
    }
}

fn reverse_west_and_south_side(ports: &mut [LPortRef]) {
    if ports.len() <= 1 {
        return;
    }

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

fn stable_sort_by<F>(ports: &mut Vec<LPortRef>, mut compare: F)
where
    F: FnMut(&LPortRef, &LPortRef) -> Ordering,
{
    if ports.len() <= 1 {
        return;
    }

    let mut indexed: Vec<(usize, LPortRef)> = ports.drain(..).enumerate().collect();
    indexed.sort_by(|(idx_a, port_a), (idx_b, port_b)| {
        let ord = compare(port_a, port_b);
        if ord == Ordering::Equal {
            idx_a.cmp(idx_b)
        } else {
            ord
        }
    });
    ports.extend(indexed.into_iter().map(|(_, port)| port));
}

fn real_degree(port: &LPortRef, outgoing: bool) -> i32 {
    let edges = port
        .lock()
        .ok()
        .map(|port_guard| {
            if outgoing {
                port_guard.outgoing_edges().clone()
            } else {
                port_guard.incoming_edges().clone()
            }
        })
        .unwrap_or_default();

    let mut degree = 0;
    for edge in edges {
        let reversed = edge
            .lock()
            .ok()
            .and_then(|mut edge_guard| edge_guard.get_property(InternalProperties::REVERSED))
            .unwrap_or(false);
        if !reversed {
            degree += 1;
        }
    }

    degree
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
