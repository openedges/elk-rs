use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::LPortRef;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment::HyperEdgeSegment;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::orthogonal_routing_generator::OrthogonalRoutingGenerator;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::direction::base_routing_direction_strategy::BaseRoutingDirectionStrategy;

pub struct WestToEastRoutingStrategy {
    base: BaseRoutingDirectionStrategy,
}

impl WestToEastRoutingStrategy {
    pub fn new() -> Self {
        WestToEastRoutingStrategy {
            base: BaseRoutingDirectionStrategy::new(),
        }
    }

    pub fn get_port_position_on_hyper_node(&self, port: &LPortRef) -> f64 {
        let Some(mut port_guard) = port.lock_ok() else {
            return 0.0;
        };
        let node_pos_y = port_guard
            .node()
            .and_then(|node| {
                node.lock_ok()
                    .map(|mut node_guard| node_guard.shape().position_ref().y)
            })
            .unwrap_or(0.0);
        let port_pos_y = port_guard.shape().position_ref().y;
        let anchor_y = port_guard.anchor_ref().y;
        node_pos_y + port_pos_y + anchor_y
    }

    pub fn get_source_port_side(&self) -> PortSide {
        PortSide::East
    }

    pub fn get_target_port_side(&self) -> PortSide {
        PortSide::West
    }

    pub fn calculate_bend_points(
        &mut self,
        segment: &HyperEdgeSegment,
        start_pos: f64,
        edge_spacing: f64,
    ) {
        if segment.is_dummy() {
            return;
        }

        let segment_x = start_pos + segment.routing_slot() as f64 * edge_spacing;
        for port in segment.ports() {
            // Single lock to get both anchor and outgoing edges
            let (source_y, outgoing_edges) = {
                let Some(port_guard) = port.lock_ok() else {
                    continue;
                };
                let anchor_y = port_guard
                    .absolute_anchor()
                    .map(|a| a.y)
                    .unwrap_or(0.0);
                let edges = port_guard.outgoing_edges().clone();
                (anchor_y, edges)
            };

            for edge in outgoing_edges {
                // Single lock to get is_self_loop + target + absolute_anchor
                let (is_self_loop, target_y) = {
                    let Some(edge_guard) = edge.lock_ok() else {
                        continue;
                    };
                    if edge_guard.is_self_loop() {
                        (true, 0.0)
                    } else {
                        let ty = edge_guard
                            .target()
                            .and_then(|t| {
                                t.lock().absolute_anchor()
                                    .map(|anchor| anchor.y)
                            })
                            .unwrap_or(0.0);
                        (false, ty)
                    }
                };
                if is_self_loop {
                    continue;
                }

                if (source_y - target_y).abs() > OrthogonalRoutingGenerator::TOLERANCE {
                    // Single edge lock for ALL bend points + junction points
                    let Some(mut edge_guard) = edge.lock_ok() else {
                        continue;
                    };

                    let mut current_x = segment_x;
                    let mut current_segment = None;

                    let bend = KVector::with_values(current_x, source_y);
                    edge_guard.bend_points().add_vector(bend);
                    self.base
                        .add_junction_point_with_guard(&mut edge_guard, segment, &bend, true);

                    if let Some(split_partner_ref) = segment.split_partner() {
                        let (split_y, split_slot) = {
                            let split_partner = split_partner_ref.borrow();
                            let split_y = split_partner
                                .incoming_connection_coordinates()
                                .first()
                                .cloned()
                                .unwrap_or(source_y);
                            (split_y, split_partner.routing_slot())
                        };

                        let bend = KVector::with_values(current_x, split_y);
                        edge_guard.bend_points().add_vector(bend);
                        self.base
                            .add_junction_point_with_guard(&mut edge_guard, segment, &bend, true);

                        current_x = start_pos + split_slot as f64 * edge_spacing;
                        current_segment = Some(split_partner_ref.clone());

                        let bend = KVector::with_values(current_x, split_y);
                        edge_guard.bend_points().add_vector(bend);
                        if let Some(split_partner) = current_segment.as_ref() {
                            self.base.add_junction_point_with_guard(
                                &mut edge_guard,
                                &split_partner.borrow(),
                                &bend,
                                true,
                            );
                        }
                    }

                    let bend = KVector::with_values(current_x, target_y);
                    edge_guard.bend_points().add_vector(bend);
                    if let Some(split_partner) = current_segment.as_ref() {
                        self.base.add_junction_point_with_guard(
                            &mut edge_guard,
                            &split_partner.borrow(),
                            &bend,
                            true,
                        );
                    } else {
                        self.base
                            .add_junction_point_with_guard(&mut edge_guard, segment, &bend, true);
                    }
                }
            }
        }
    }

    pub fn clear_created_junction_points(&mut self) {
        self.base.clear_created_junction_points();
    }
}

impl Default for WestToEastRoutingStrategy {
    fn default() -> Self {
        Self::new()
    }
}
