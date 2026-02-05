use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use crate::org::eclipse::elk::alg::layered::graph::{LNodeRef, LPortRef, NodeType};
use crate::org::eclipse::elk::alg::layered::intermediate::greedyswitch::crossing_matrix_filler::CrossingMatrixFiller;
use crate::org::eclipse::elk::alg::layered::intermediate::greedyswitch::north_south_edge_neighbouring_node_crossings_counter::NorthSouthEdgeNeighbouringNodeCrossingsCounter;
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, Origin};
use crate::org::eclipse::elk::alg::layered::p3order::counting::CrossingsCounter;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossingCountSide {
    West,
    East,
}

pub struct ParentCrossingContext {
    parent_node_order: Vec<Vec<LNodeRef>>,
    parent_port_positions: Vec<i32>,
    parent_layer_index: usize,
    right_most_layer: bool,
}

impl ParentCrossingContext {
    pub fn new(
        parent_node_order: Vec<Vec<LNodeRef>>,
        parent_port_positions: Vec<i32>,
        parent_layer_index: usize,
        right_most_layer: bool,
    ) -> Self {
        ParentCrossingContext {
            parent_node_order,
            parent_port_positions,
            parent_layer_index,
            right_most_layer,
        }
    }
}

pub struct SwitchDecider {
    left_in_layer_counter: CrossingsCounter,
    right_in_layer_counter: CrossingsCounter,
    north_south_counter: NorthSouthEdgeNeighbouringNodeCrossingsCounter,
    crossing_matrix_filler: CrossingMatrixFiller,
    parent_cross_counter: Option<CrossingsCounter>,
    count_crossings_caused_by_port_switch: bool,
}

impl SwitchDecider {
    pub fn new(
        free_layer: &[LNodeRef],
        crossing_matrix_filler: CrossingMatrixFiller,
        port_positions: &[i32],
        parent_context: Option<ParentCrossingContext>,
        count_crossings_caused_by_port_switch: bool,
    ) -> Self {
        let mut left_in_layer_counter = CrossingsCounter::new(port_positions.to_vec());
        left_in_layer_counter.init_port_positions_for_in_layer_crossings(free_layer, PortSide::West);
        let mut right_in_layer_counter = CrossingsCounter::new(port_positions.to_vec());
        right_in_layer_counter.init_port_positions_for_in_layer_crossings(free_layer, PortSide::East);
        let north_south_counter = NorthSouthEdgeNeighbouringNodeCrossingsCounter::new(free_layer);

        let parent_cross_counter = if count_crossings_caused_by_port_switch {
            parent_context.map(|context| {
                let mut counter = CrossingsCounter::new(context.parent_port_positions);
                let parent_layers = context.parent_node_order;
                let left_layer = if context.parent_layer_index > 0 {
                    parent_layers
                        .get(context.parent_layer_index - 1)
                        .cloned()
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                let middle_layer = parent_layers
                    .get(context.parent_layer_index)
                    .cloned()
                    .unwrap_or_default();
                let right_layer = if context.parent_layer_index + 1 < parent_layers.len() {
                    parent_layers
                        .get(context.parent_layer_index + 1)
                        .cloned()
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                if context.right_most_layer {
                    counter.init_for_counting_between(&middle_layer, &right_layer);
                } else {
                    counter.init_for_counting_between(&left_layer, &middle_layer);
                }
                counter
            })
        } else {
            None
        };

        SwitchDecider {
            left_in_layer_counter,
            right_in_layer_counter,
            north_south_counter,
            crossing_matrix_filler,
            parent_cross_counter,
            count_crossings_caused_by_port_switch,
        }
    }

    pub fn notify_of_switch(&mut self, upper_node: &LNodeRef, lower_node: &LNodeRef) {
        self.left_in_layer_counter
            .switch_nodes(upper_node, lower_node, PortSide::West);
        self.right_in_layer_counter
            .switch_nodes(upper_node, lower_node, PortSide::East);
        if self.count_crossings_caused_by_port_switch {
            if let (Some(upper_port), Some(lower_port)) =
                (origin_port_of(upper_node), origin_port_of(lower_node))
            {
                if let Some(counter) = self.parent_cross_counter.as_mut() {
                    counter.switch_ports(&upper_port, &lower_port);
                }
            }
        }
    }

    pub fn does_switch_reduce_crossings(
        &mut self,
        upper_node: &LNodeRef,
        lower_node: &LNodeRef,
    ) -> bool {
        if self.constraints_prevent_switch(upper_node, lower_node) {
            return false;
        }

        let left_in_layer = self.left_in_layer_counter.count_in_layer_crossings_between_nodes_in_both_orders(
            upper_node,
            lower_node,
            PortSide::West,
        );
        let right_in_layer = self.right_in_layer_counter.count_in_layer_crossings_between_nodes_in_both_orders(
            upper_node,
            lower_node,
            PortSide::East,
        );
        self.north_south_counter
            .count_crossings(upper_node, lower_node);

        let upper_lower_crossings = self.crossing_matrix_filler.crossing_matrix_entry(upper_node, lower_node)
            + *left_in_layer.first()
            + *right_in_layer.first()
            + self.north_south_counter.upper_lower_crossings();
        let lower_upper_crossings = self.crossing_matrix_filler.crossing_matrix_entry(lower_node, upper_node)
            + *left_in_layer.second()
            + *right_in_layer.second()
            + self.north_south_counter.lower_upper_crossings();

        let (mut upper_lower_crossings, mut lower_upper_crossings) =
            (upper_lower_crossings, lower_upper_crossings);

        if self.count_crossings_caused_by_port_switch {
            if let (Some(upper_port), Some(lower_port)) =
                (origin_port_of(upper_node), origin_port_of(lower_node))
            {
                if let Some(counter) = self.parent_cross_counter.as_mut() {
                    let crossing_numbers =
                        counter.count_crossings_between_ports_in_both_orders(&upper_port, &lower_port);
                    upper_lower_crossings += *crossing_numbers.first();
                    lower_upper_crossings += *crossing_numbers.second();
                }
            }
        }

        upper_lower_crossings > lower_upper_crossings
    }

    fn constraints_prevent_switch(&self, upper_node: &LNodeRef, lower_node: &LNodeRef) -> bool {
        self.have_successor_constraints(upper_node, lower_node)
            || self.have_layout_unit_constraints(upper_node, lower_node)
            || self.are_normal_and_north_south_port_dummy(upper_node, lower_node)
    }

    fn have_successor_constraints(&self, upper_node: &LNodeRef, lower_node: &LNodeRef) -> bool {
        let constraints = upper_node
            .lock()
            .ok()
            .and_then(|mut node_guard| node_guard.get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS))
            .unwrap_or_default();
        !constraints.is_empty()
            && constraints
                .iter()
                .any(|candidate| std::sync::Arc::ptr_eq(candidate, lower_node))
    }

    fn have_layout_unit_constraints(&self, upper_node: &LNodeRef, lower_node: &LNodeRef) -> bool {
        let upper_type = upper_node
            .lock()
            .ok()
            .map(|node_guard| node_guard.node_type())
            .unwrap_or(NodeType::Normal);
        let lower_type = lower_node
            .lock()
            .ok()
            .map(|node_guard| node_guard.node_type())
            .unwrap_or(NodeType::Normal);
        let neither_long_edge = upper_type != NodeType::LongEdge && lower_type != NodeType::LongEdge;

        let upper_layout_unit = upper_node
            .lock()
            .ok()
            .and_then(|mut node_guard| node_guard.get_property(InternalProperties::IN_LAYER_LAYOUT_UNIT));
        let lower_layout_unit = lower_node
            .lock()
            .ok()
            .and_then(|mut node_guard| node_guard.get_property(InternalProperties::IN_LAYER_LAYOUT_UNIT));

        let are_in_different_layout_units = match (&upper_layout_unit, &lower_layout_unit) {
            (Some(upper), Some(lower)) => !std::sync::Arc::ptr_eq(upper, lower),
            (None, None) => false,
            _ => true,
        };

        let nodes_have_layout_units =
            self.part_of_multi_node_layout_unit(upper_node, upper_layout_unit.as_ref())
                || self.part_of_multi_node_layout_unit(lower_node, lower_layout_unit.as_ref());

        let upper_has_northern_edges = self.has_edges_on_side(upper_node, PortSide::North);
        let lower_has_southern_edges = self.has_edges_on_side(lower_node, PortSide::South);

        let nodes_have_layout_units = nodes_have_layout_units
            || self.has_edges_on_side(upper_node, PortSide::South)
            || self.has_edges_on_side(lower_node, PortSide::North);

        let has_layout_unit_constraint =
            (nodes_have_layout_units && are_in_different_layout_units) || upper_has_northern_edges
                || lower_has_southern_edges;

        neither_long_edge && has_layout_unit_constraint
    }

    fn has_edges_on_side(&self, node: &LNodeRef, side: PortSide) -> bool {
        let ports = node
            .lock()
            .ok()
            .map(|mut node_guard| node_guard.port_side_view(side))
            .unwrap_or_default();
        for port in ports {
            let has_dummy = port
                .lock()
                .ok()
                .and_then(|mut port_guard| port_guard.get_property(InternalProperties::PORT_DUMMY))
                .is_some();
            let has_edges = port
                .lock()
                .ok()
                .map(|port_guard| !port_guard.connected_edges().is_empty())
                .unwrap_or(false);
            if has_dummy || has_edges {
                return true;
            }
        }
        false
    }

    fn part_of_multi_node_layout_unit(
        &self,
        node: &LNodeRef,
        layout_unit: Option<&LNodeRef>,
    ) -> bool {
        layout_unit
            .map(|unit| !std::sync::Arc::ptr_eq(unit, node))
            .unwrap_or(false)
    }

    fn are_normal_and_north_south_port_dummy(
        &self,
        upper_node: &LNodeRef,
        lower_node: &LNodeRef,
    ) -> bool {
        (self.is_north_south_port_node(upper_node) && self.is_normal_node(lower_node))
            || (self.is_north_south_port_node(lower_node) && self.is_normal_node(upper_node))
    }

    fn is_normal_node(&self, node: &LNodeRef) -> bool {
        node.lock()
            .ok()
            .map(|node_guard| node_guard.node_type() == NodeType::Normal)
            .unwrap_or(false)
    }

    fn is_north_south_port_node(&self, node: &LNodeRef) -> bool {
        node.lock()
            .ok()
            .map(|node_guard| node_guard.node_type() == NodeType::NorthSouthPort)
            .unwrap_or(false)
    }
}

fn origin_port_of(node: &LNodeRef) -> Option<LPortRef> {
    node.lock()
        .ok()
        .and_then(|mut node_guard| node_guard.get_property(InternalProperties::ORIGIN))
        .and_then(|origin| match origin {
            Origin::LPort(port) => Some(port),
            _ => None,
        })
}
