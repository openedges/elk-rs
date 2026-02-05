use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::LPortRef;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment::HyperEdgeSegment;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::orthogonal_routing_generator::OrthogonalRoutingGenerator;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::direction::base_routing_direction_strategy::BaseRoutingDirectionStrategy;

pub struct SouthToNorthRoutingStrategy {
    base: BaseRoutingDirectionStrategy,
}

impl SouthToNorthRoutingStrategy {
    pub fn new() -> Self {
        SouthToNorthRoutingStrategy {
            base: BaseRoutingDirectionStrategy::new(),
        }
    }

    pub fn get_port_position_on_hyper_node(&self, port: &LPortRef) -> f64 {
        let Ok(mut port_guard) = port.lock() else {
            return 0.0;
        };
        let node_pos_x = port_guard
            .node()
            .and_then(|node| node.lock().ok().map(|mut node_guard| node_guard.shape().position_ref().x))
            .unwrap_or(0.0);
        let port_pos_x = port_guard.shape().position_ref().x;
        let anchor_x = port_guard.anchor_ref().x;
        node_pos_x + port_pos_x + anchor_x
    }

    pub fn get_source_port_side(&self) -> PortSide {
        PortSide::North
    }

    pub fn get_target_port_side(&self) -> PortSide {
        PortSide::South
    }

    pub fn calculate_bend_points(&mut self, segment: &HyperEdgeSegment, start_pos: f64, edge_spacing: f64) {
        if segment.is_dummy() {
            return;
        }

        let segment_y = start_pos - segment.routing_slot() as f64 * edge_spacing;
        for port in segment.ports() {
            let source_x = port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.absolute_anchor())
                .map(|anchor| anchor.x)
                .unwrap_or(0.0);

            let outgoing_edges = port
                .lock()
                .ok()
                .map(|port_guard| port_guard.outgoing_edges().clone())
                .unwrap_or_default();
            for edge in outgoing_edges {
                let is_self_loop = edge.lock().ok().map(|edge_guard| edge_guard.is_self_loop()).unwrap_or(false);
                if is_self_loop {
                    continue;
                }
                let target = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| edge_guard.target());
                let Some(target) = target else {
                    continue;
                };
                let target_x = target
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.absolute_anchor())
                    .map(|anchor| anchor.x)
                    .unwrap_or(0.0);

                if (source_x - target_x).abs() > OrthogonalRoutingGenerator::TOLERANCE {
                    let mut current_y = segment_y;

                    let mut bend = KVector::with_values(source_x, current_y);
                    if let Ok(mut edge_guard) = edge.lock() {
                        edge_guard.bend_points().add_vector(bend);
                    }
                    self.base
                        .add_junction_point_if_necessary(&edge, segment, &bend, false);

                    if let Some(split_partner_ref) = segment.split_partner() {
                        let (split_x, split_slot) = {
                            let split_partner = split_partner_ref.borrow();
                            let split_x = split_partner
                                .incoming_connection_coordinates()
                                .first()
                                .cloned()
                                .unwrap_or(source_x);
                            (split_x, split_partner.routing_slot())
                        };

                        bend = KVector::with_values(split_x, current_y);
                        if let Ok(mut edge_guard) = edge.lock() {
                            edge_guard.bend_points().add_vector(bend);
                        }
                        self.base
                            .add_junction_point_if_necessary(&edge, segment, &bend, false);

                        current_y = start_pos - split_slot as f64 * edge_spacing;

                        bend = KVector::with_values(split_x, current_y);
                        if let Ok(mut edge_guard) = edge.lock() {
                            edge_guard.bend_points().add_vector(bend);
                        }
                        if let Some(split_partner) = segment.split_partner() {
                            self.base.add_junction_point_if_necessary(
                                &edge,
                                &split_partner.borrow(),
                                &bend,
                                false,
                            );
                        }
                    }

                    bend = KVector::with_values(target_x, current_y);
                    if let Ok(mut edge_guard) = edge.lock() {
                        edge_guard.bend_points().add_vector(bend);
                    }
                    self.base
                        .add_junction_point_if_necessary(&edge, segment, &bend, false);
                }
            }
        }
    }

    pub fn clear_created_junction_points(&mut self) {
        self.base.clear_created_junction_points();
    }
}

impl Default for SouthToNorthRoutingStrategy {
    fn default() -> Self {
        Self::new()
    }
}
