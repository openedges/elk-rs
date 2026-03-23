use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::{ArenaSync, LPortRef};
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
        let mut port_guard = port.lock();
        let node_pos_x = port_guard
            .node()
            .map(|node| node.lock().shape().position_ref().x)
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

    pub fn calculate_bend_points(
        &mut self,
        segment: &HyperEdgeSegment,
        start_pos: f64,
        edge_spacing: f64,
        sync: &ArenaSync,
    ) {
        if segment.is_dummy() {
            return;
        }

        let arena = sync.arena();
        let segment_y = start_pos - segment.routing_slot() as f64 * edge_spacing;
        for port in segment.ports() {
            let (source_x, outgoing_edges) = if let Some(pid) = sync.port_id(port) {
                let anchor_x = arena.port_absolute_anchor(pid).x;
                let edge_ids = arena.port_outgoing_edges(pid);
                let edges: Vec<_> = edge_ids.iter().map(|&eid| sync.edge_ref(eid).clone()).collect();
                (anchor_x, edges)
            } else {
                let port_guard = port.lock();
                let anchor_x = port_guard
                    .absolute_anchor()
                    .map(|a| a.x)
                    .unwrap_or(0.0);
                let edges = port_guard.outgoing_edges().clone();
                (anchor_x, edges)
            };

            for edge in outgoing_edges {
                let (is_self_loop, target_x) = if let Some(eid) = sync.edge_id(&edge) {
                    let src_pid = arena.edge_source(eid);
                    let tgt_pid = arena.edge_target(eid);
                    if src_pid == tgt_pid {
                        (true, 0.0)
                    } else {
                        (false, arena.port_absolute_anchor(tgt_pid).x)
                    }
                } else {
                    let edge_guard = edge.lock();
                    if edge_guard.is_self_loop() {
                        (true, 0.0)
                    } else {
                        let tx = edge_guard
                            .target()
                            .and_then(|t| {
                                t.lock().absolute_anchor()
                                    .map(|anchor| anchor.x)
                            })
                            .unwrap_or(0.0);
                        (false, tx)
                    }
                };
                if is_self_loop {
                    continue;
                }

                if (source_x - target_x).abs() > OrthogonalRoutingGenerator::TOLERANCE {
                    let mut edge_guard = edge.lock();

                    let mut current_y = segment_y;
                    let mut current_segment = None;

                    let bend = KVector::with_values(source_x, current_y);
                    edge_guard.bend_points().add_vector(bend);
                    self.base
                        .add_junction_point_with_guard(&mut edge_guard, segment, &bend, false);

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

                        let bend = KVector::with_values(split_x, current_y);
                        edge_guard.bend_points().add_vector(bend);
                        self.base
                            .add_junction_point_with_guard(&mut edge_guard, segment, &bend, false);

                        current_y = start_pos - split_slot as f64 * edge_spacing;
                        current_segment = Some(split_partner_ref.clone());

                        let bend = KVector::with_values(split_x, current_y);
                        edge_guard.bend_points().add_vector(bend);
                        if let Some(split_partner) = current_segment.as_ref() {
                            self.base.add_junction_point_with_guard(
                                &mut edge_guard,
                                &split_partner.borrow(),
                                &bend,
                                false,
                            );
                        }
                    }

                    let bend = KVector::with_values(target_x, current_y);
                    edge_guard.bend_points().add_vector(bend);
                    if let Some(split_partner) = current_segment.as_ref() {
                        self.base.add_junction_point_with_guard(
                            &mut edge_guard,
                            &split_partner.borrow(),
                            &bend,
                            false,
                        );
                    } else {
                        self.base
                            .add_junction_point_with_guard(&mut edge_guard, segment, &bend, false);
                    }
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
