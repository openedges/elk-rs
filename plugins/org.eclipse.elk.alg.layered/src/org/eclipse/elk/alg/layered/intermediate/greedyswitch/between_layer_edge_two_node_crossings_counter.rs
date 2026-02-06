use std::collections::HashMap;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LNodeRef, LPortRef};
use crate::org::eclipse::elk::alg::layered::p3order::counting::in_north_south_east_west_order;

pub struct BetweenLayerEdgeTwoNodeCrossingsCounter {
    upper_lower_crossings: i32,
    lower_upper_crossings: i32,
    current_node_order: Vec<Vec<LNodeRef>>,
    free_layer_index: usize,
    port_positions: HashMap<usize, i32>,
    node_indices: HashMap<usize, usize>,
    eastern_adjacencies: Option<Vec<AdjacencyList>>,
    western_adjacencies: Option<Vec<AdjacencyList>>,
}

impl BetweenLayerEdgeTwoNodeCrossingsCounter {
    pub fn new(current_node_order: Vec<Vec<LNodeRef>>, free_layer_index: usize) -> Self {
        let mut counter = BetweenLayerEdgeTwoNodeCrossingsCounter {
            upper_lower_crossings: 0,
            lower_upper_crossings: 0,
            current_node_order,
            free_layer_index,
            port_positions: HashMap::new(),
            node_indices: HashMap::new(),
            eastern_adjacencies: None,
            western_adjacencies: None,
        };
        counter.set_port_positions_for_neighbouring_layers();
        counter.set_node_indices_for_free_layer();
        counter
    }

    pub fn count_eastern_edge_crossings(&mut self, upper_node: &LNodeRef, lower_node: &LNodeRef) {
        self.reset_crossing_count();
        if Arc::ptr_eq(upper_node, lower_node) {
            return;
        }
        self.add_crossings(upper_node, lower_node, PortSide::East);
    }

    pub fn count_western_edge_crossings(&mut self, upper_node: &LNodeRef, lower_node: &LNodeRef) {
        self.reset_crossing_count();
        if Arc::ptr_eq(upper_node, lower_node) {
            return;
        }
        self.add_crossings(upper_node, lower_node, PortSide::West);
    }

    pub fn count_both_side_crossings(&mut self, upper_node: &LNodeRef, lower_node: &LNodeRef) {
        self.reset_crossing_count();
        if Arc::ptr_eq(upper_node, lower_node) {
            return;
        }
        self.add_crossings(upper_node, lower_node, PortSide::West);
        self.add_crossings(upper_node, lower_node, PortSide::East);
    }

    pub fn upper_lower_crossings(&self) -> i32 {
        self.upper_lower_crossings
    }

    pub fn lower_upper_crossings(&self) -> i32 {
        self.lower_upper_crossings
    }

    fn add_crossings(&mut self, upper_node: &LNodeRef, lower_node: &LNodeRef, side: PortSide) {
        let (upper_index, lower_index) = match (
            self.node_indices.get(&node_ptr_id(upper_node)).copied(),
            self.node_indices.get(&node_ptr_id(lower_node)).copied(),
        ) {
            (Some(upper_index), Some(lower_index)) => (upper_index, lower_index),
            _ => return,
        };

        let adjacencies = self.ensure_adjacencies(side);
        if upper_index == lower_index {
            return;
        }

        let (upper_adjacencies, lower_adjacencies) = get_two_mut(adjacencies, upper_index, lower_index);
        upper_adjacencies.reset();
        lower_adjacencies.reset();
        if upper_adjacencies.current_size() == 0 || lower_adjacencies.current_size() == 0 {
            return;
        }

        let (upper_delta, lower_delta) =
            count_crossings_by_merging_adjacency_lists(upper_adjacencies, lower_adjacencies);
        self.upper_lower_crossings += upper_delta;
        self.lower_upper_crossings += lower_delta;
    }

    fn reset_crossing_count(&mut self) {
        self.upper_lower_crossings = 0;
        self.lower_upper_crossings = 0;
    }

    fn set_port_positions_for_neighbouring_layers(&mut self) {
        if self.free_layer_index > 0 {
            self.set_port_positions_for_layer(self.free_layer_index - 1, PortSide::East);
        }
        if self.free_layer_index + 1 < self.current_node_order.len() {
            self.set_port_positions_for_layer(self.free_layer_index + 1, PortSide::West);
        }
    }

    fn set_port_positions_for_layer(&mut self, layer_index: usize, port_side: PortSide) {
        if let Some(layer) = self.current_node_order.get(layer_index) {
            let mut port_id = 0;
            for node in layer {
                for port in in_north_south_east_west_order(node, port_side) {
                    self.port_positions.insert(port_ptr_id(&port), port_id);
                    port_id += 1;
                }
            }
        }
    }

    fn set_node_indices_for_free_layer(&mut self) {
        if let Some(layer) = self.current_node_order.get(self.free_layer_index) {
            for (index, node) in layer.iter().enumerate() {
                self.node_indices.insert(node_ptr_id(node), index);
            }
        }
    }

    fn ensure_adjacencies(&mut self, side: PortSide) -> &mut Vec<AdjacencyList> {
        let target = match side {
            PortSide::East => &mut self.eastern_adjacencies,
            PortSide::West => &mut self.western_adjacencies,
            _ => &mut self.eastern_adjacencies,
        };

        if target.is_none() {
            let lists = if let Some(layer) = self.current_node_order.get(self.free_layer_index) {
                layer
                    .iter()
                    .map(|node| AdjacencyList::new(node, side, &self.port_positions))
                    .collect()
            } else {
                Vec::new()
            };
            *target = Some(lists);
        }

        target.as_mut().expect("adjacencies not initialized")
    }
}

fn get_two_mut<T>(items: &mut [T], first: usize, second: usize) -> (&mut T, &mut T) {
    if first < second {
        let (left, right) = items.split_at_mut(second);
        (&mut left[first], &mut right[0])
    } else {
        let (left, right) = items.split_at_mut(first);
        (&mut right[0], &mut left[second])
    }
}

fn count_crossings_by_merging_adjacency_lists(
    upper_adjacencies: &mut AdjacencyList,
    lower_adjacencies: &mut AdjacencyList,
) -> (i32, i32) {
    let mut upper_lower_crossings = 0;
    let mut lower_upper_crossings = 0;
    while !upper_adjacencies.is_empty() && !lower_adjacencies.is_empty() {
        let upper_first = upper_adjacencies.first();
        let lower_first = lower_adjacencies.first();
        if upper_first > lower_first {
            upper_lower_crossings += upper_adjacencies.current_size() as i32;
            lower_adjacencies.remove_first();
        } else if lower_first > upper_first {
            lower_upper_crossings += lower_adjacencies.current_size() as i32;
            upper_adjacencies.remove_first();
        } else {
            upper_lower_crossings +=
                upper_adjacencies.count_adjacencies_below_node_of_first_port() as i32;
            lower_upper_crossings +=
                lower_adjacencies.count_adjacencies_below_node_of_first_port() as i32;
            upper_adjacencies.remove_first();
            lower_adjacencies.remove_first();
        }
    }
    (upper_lower_crossings, lower_upper_crossings)
}

struct AdjacencyList {
    adjacency_list: Vec<Adjacency>,
    side: PortSide,
    size: usize,
    current_size: usize,
    current_index: usize,
}

impl AdjacencyList {
    fn new(node: &LNodeRef, side: PortSide, port_positions: &HashMap<usize, i32>) -> Self {
        let mut list = AdjacencyList {
            adjacency_list: Vec::new(),
            side,
            size: 0,
            current_size: 0,
            current_index: 0,
        };
        list.collect_adjacencies(node, port_positions);
        list.adjacency_list
            .sort_by(|left, right| left.position.cmp(&right.position));
        list.reset();
        list
    }

    fn collect_adjacencies(&mut self, node: &LNodeRef, port_positions: &HashMap<usize, i32>) {
        let ports = in_north_south_east_west_order(node, self.side);
        for port in ports {
            let edges = edges_connected_to(&port, self.side);
            for edge in edges {
                let (is_self_loop, is_in_layer) = edge
                    .lock()
                    .ok()
                    .map(|edge_guard| (edge_guard.is_self_loop(), edge_guard.is_in_layer_edge()))
                    .unwrap_or((false, false));
                if is_self_loop || is_in_layer {
                    continue;
                }
                self.add_adjacency_of(&edge, port_positions);
            }
        }
    }

    fn add_adjacency_of(&mut self, edge: &LEdgeRef, port_positions: &HashMap<usize, i32>) {
        let adjacent_port = adjacent_port_of(edge, self.side);
        let Some(adjacent_port) = adjacent_port else {
            return;
        };
        let Some(adjacent_position) = port_positions.get(&port_ptr_id(&adjacent_port)).copied() else {
            return;
        };

        if let Some(last) = self.adjacency_list.last_mut() {
            if last.position == adjacent_position {
                last.cardinality += 1;
                last.current_cardinality += 1;
            } else {
                self.adjacency_list.push(Adjacency::new(adjacent_position));
            }
        } else {
            self.adjacency_list.push(Adjacency::new(adjacent_position));
        }
        self.size += 1;
        self.current_size += 1;
    }

    fn reset(&mut self) {
        self.current_index = 0;
        self.current_size = self.size;
        if let Some(adjacency) = self.adjacency_list.get_mut(self.current_index) {
            adjacency.reset();
        }
    }

    fn count_adjacencies_below_node_of_first_port(&self) -> usize {
        if self.current_size == 0 {
            return 0;
        }
        let current = &self.adjacency_list[self.current_index];
        self.current_size.saturating_sub(current.current_cardinality)
    }

    fn remove_first(&mut self) {
        if self.is_empty() {
            return;
        }

        if let Some(adjacency) = self.adjacency_list.get_mut(self.current_index) {
            if adjacency.current_cardinality == 1 {
                self.increment_current_index();
            } else {
                adjacency.current_cardinality -= 1;
            }
        }
        self.current_size = self.current_size.saturating_sub(1);
    }

    fn increment_current_index(&mut self) {
        self.current_index += 1;
        if self.current_index < self.adjacency_list.len() {
            self.adjacency_list[self.current_index].reset();
        }
    }

    fn is_empty(&self) -> bool {
        self.current_size == 0
    }

    fn first(&self) -> i32 {
        self.adjacency_list
            .get(self.current_index)
            .map(|adjacency| adjacency.position)
            .unwrap_or(0)
    }

    fn current_size(&self) -> usize {
        self.current_size
    }
}

struct Adjacency {
    position: i32,
    cardinality: usize,
    current_cardinality: usize,
}

impl Adjacency {
    fn new(position: i32) -> Self {
        Adjacency {
            position,
            cardinality: 1,
            current_cardinality: 1,
        }
    }

    fn reset(&mut self) {
        self.current_cardinality = self.cardinality;
    }
}

fn edges_connected_to(port: &LPortRef, side: PortSide) -> Vec<LEdgeRef> {
    port.lock()
        .ok()
        .map(|port_guard| {
            if side == PortSide::West {
                port_guard.incoming_edges().clone()
            } else {
                port_guard.outgoing_edges().clone()
            }
        })
        .unwrap_or_default()
}

fn adjacent_port_of(edge: &LEdgeRef, side: PortSide) -> Option<LPortRef> {
    if let Ok(edge_guard) = edge.lock() {
        if side == PortSide::West {
            return edge_guard.source();
        }
        return edge_guard.target();
    }
    None
}

fn port_ptr_id(port: &LPortRef) -> usize {
    Arc::as_ptr(port) as usize
}

fn node_ptr_id(node: &LNodeRef) -> usize {
    Arc::as_ptr(node) as usize
}
