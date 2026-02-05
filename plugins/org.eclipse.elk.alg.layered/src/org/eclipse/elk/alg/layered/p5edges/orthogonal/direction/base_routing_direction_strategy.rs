use std::collections::HashSet;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LPortRef};
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment::HyperEdgeSegment;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::orthogonal_routing_generator::OrthogonalRoutingGenerator;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::direction::north_to_south_routing_strategy::NorthToSouthRoutingStrategy;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::direction::routing_direction::RoutingDirection;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::direction::south_to_north_routing_strategy::SouthToNorthRoutingStrategy;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::direction::west_to_east_routing_strategy::WestToEastRoutingStrategy;

pub struct BaseRoutingDirectionStrategy {
    created_junction_points: HashSet<KVector>,
}

impl BaseRoutingDirectionStrategy {
    pub fn new() -> Self {
        BaseRoutingDirectionStrategy {
            created_junction_points: HashSet::new(),
        }
    }

    pub fn add_junction_point_if_necessary(
        &mut self,
        edge: &LEdgeRef,
        segment: &HyperEdgeSegment,
        pos: &KVector,
        vertical: bool,
    ) {
        let p = if vertical { pos.y } else { pos.x };

        if self.created_junction_points.contains(pos) {
            return;
        }

        let point_inside_edge_segment = p > segment.start_coordinate() && p < segment.end_coordinate();
        let mut point_at_segment_boundary = false;

        let incoming = segment.incoming_connection_coordinates();
        let outgoing = segment.outgoing_connection_coordinates();
        if !incoming.is_empty() && !outgoing.is_empty() {
            point_at_segment_boundary |=
                (p - incoming[0]).abs() < OrthogonalRoutingGenerator::TOLERANCE
                    && (p - outgoing[0]).abs() < OrthogonalRoutingGenerator::TOLERANCE;
            point_at_segment_boundary |=
                (p - incoming[incoming.len() - 1]).abs() < OrthogonalRoutingGenerator::TOLERANCE
                    && (p - outgoing[outgoing.len() - 1]).abs() < OrthogonalRoutingGenerator::TOLERANCE;
        }

        if point_inside_edge_segment || point_at_segment_boundary {
            if let Ok(mut edge_guard) = edge.lock() {
                let mut junction_points = edge_guard
                    .get_property(LayeredOptions::JUNCTION_POINTS)
                    .unwrap_or_else(KVectorChain::new);
                junction_points.add_vector(*pos);
                edge_guard.set_property(LayeredOptions::JUNCTION_POINTS, Some(junction_points));
            }
            self.created_junction_points.insert(*pos);
        }
    }

    pub fn clear_created_junction_points(&mut self) {
        self.created_junction_points.clear();
    }
}

impl Default for BaseRoutingDirectionStrategy {
    fn default() -> Self {
        Self::new()
    }
}

pub enum RoutingDirectionStrategy {
    WestToEast(WestToEastRoutingStrategy),
    NorthToSouth(NorthToSouthRoutingStrategy),
    SouthToNorth(SouthToNorthRoutingStrategy),
}

impl RoutingDirectionStrategy {
    pub fn for_routing_direction(direction: RoutingDirection) -> Self {
        match direction {
            RoutingDirection::WestToEast => RoutingDirectionStrategy::WestToEast(WestToEastRoutingStrategy::new()),
            RoutingDirection::NorthToSouth => {
                RoutingDirectionStrategy::NorthToSouth(NorthToSouthRoutingStrategy::new())
            }
            RoutingDirection::SouthToNorth => {
                RoutingDirectionStrategy::SouthToNorth(SouthToNorthRoutingStrategy::new())
            }
        }
    }

    pub fn direction(&self) -> RoutingDirection {
        match self {
            RoutingDirectionStrategy::WestToEast(_) => RoutingDirection::WestToEast,
            RoutingDirectionStrategy::NorthToSouth(_) => RoutingDirection::NorthToSouth,
            RoutingDirectionStrategy::SouthToNorth(_) => RoutingDirection::SouthToNorth,
        }
    }

    pub fn get_port_position_on_hyper_node(&self, port: &LPortRef) -> f64 {
        match self {
            RoutingDirectionStrategy::WestToEast(strategy) => strategy.get_port_position_on_hyper_node(port),
            RoutingDirectionStrategy::NorthToSouth(strategy) => strategy.get_port_position_on_hyper_node(port),
            RoutingDirectionStrategy::SouthToNorth(strategy) => strategy.get_port_position_on_hyper_node(port),
        }
    }

    pub fn get_source_port_side(&self) -> PortSide {
        match self {
            RoutingDirectionStrategy::WestToEast(strategy) => strategy.get_source_port_side(),
            RoutingDirectionStrategy::NorthToSouth(strategy) => strategy.get_source_port_side(),
            RoutingDirectionStrategy::SouthToNorth(strategy) => strategy.get_source_port_side(),
        }
    }

    pub fn get_target_port_side(&self) -> PortSide {
        match self {
            RoutingDirectionStrategy::WestToEast(strategy) => strategy.get_target_port_side(),
            RoutingDirectionStrategy::NorthToSouth(strategy) => strategy.get_target_port_side(),
            RoutingDirectionStrategy::SouthToNorth(strategy) => strategy.get_target_port_side(),
        }
    }

    pub fn calculate_bend_points(
        &mut self,
        segment: &HyperEdgeSegment,
        start_pos: f64,
        edge_spacing: f64,
    ) {
        match self {
            RoutingDirectionStrategy::WestToEast(strategy) => {
                strategy.calculate_bend_points(segment, start_pos, edge_spacing)
            }
            RoutingDirectionStrategy::NorthToSouth(strategy) => {
                strategy.calculate_bend_points(segment, start_pos, edge_spacing)
            }
            RoutingDirectionStrategy::SouthToNorth(strategy) => {
                strategy.calculate_bend_points(segment, start_pos, edge_spacing)
            }
        }
    }

    pub fn clear_created_junction_points(&mut self) {
        match self {
            RoutingDirectionStrategy::WestToEast(strategy) => strategy.clear_created_junction_points(),
            RoutingDirectionStrategy::NorthToSouth(strategy) => strategy.clear_created_junction_points(),
            RoutingDirectionStrategy::SouthToNorth(strategy) => strategy.clear_created_junction_points(),
        }
    }
}
