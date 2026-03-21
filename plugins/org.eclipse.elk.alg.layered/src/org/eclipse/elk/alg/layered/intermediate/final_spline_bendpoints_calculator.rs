use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::elk_math::ElkMath;
use org_eclipse_elk_core::org::eclipse::elk::core::math::elk_rectangle::ElkRectangle;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::{
    PortSide, SIDES_NORTH_SOUTH,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LGraph, LNodeRef};
use crate::org::eclipse::elk::alg::layered::options::{
    GraphCompactionStrategy, InternalProperties, LayeredOptions, SplineRoutingMode,
};
use crate::org::eclipse::elk::alg::layered::p5edges::splines::nub_spline::NubSpline;
use crate::org::eclipse::elk::alg::layered::p5edges::splines::spline_edge_router::SplineEdgeRouter;
use crate::org::eclipse::elk::alg::layered::p5edges::splines::spline_segment::{
    EdgeInformation, SplineSegmentRef,
};
use crate::org::eclipse::elk::alg::layered::p5edges::splines::splines_math::SplinesMath;

pub struct FinalSplineBendpointsCalculator {
    edge_edge_spacing: f64,
    edge_node_spacing: f64,
    spline_routing_mode: SplineRoutingMode,
    compaction_strategy: GraphCompactionStrategy,
}

impl Default for FinalSplineBendpointsCalculator {
    fn default() -> Self {
        FinalSplineBendpointsCalculator {
            edge_edge_spacing: 0.0,
            edge_node_spacing: 0.0,
            spline_routing_mode: SplineRoutingMode::Sloppy,
            compaction_strategy: GraphCompactionStrategy::None,
        }
    }
}

impl FinalSplineBendpointsCalculator {
    const ONE_HALF: f64 = 0.5;
    pub const NODE_TO_STRAIGHTENING_CP_GAP: f64 = 5.0;
    const SLOPPY_CENTER_CP_MULTIPLIER: f64 = 0.4;

    fn index_nodes_per_layer(graph: &mut LGraph) {
        let layers = graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock_ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for (index, node) in nodes.iter().enumerate() {
                if let Some(mut node_guard) = node.lock_ok() {
                    node_guard.shape().graph_element().id = index as i32;
                }
            }
        }
    }

    fn calculate_control_points(&mut self, segment: &SplineSegmentRef) {
        let snapshot = {
            let mut segment_guard = match segment.lock_ok() {
            Some(guard) => guard,
            None => return,
            };
            if segment_guard.handled {
                return;
            }
            segment_guard.handled = true;
            SegmentSnapshot::from_segment(&segment_guard)
        };

        for edge in &snapshot.edges {
            if snapshot.is_straight && !snapshot.is_hyper_edge {
                self.calculate_control_points_straight(&snapshot);
                continue;
            }

            let edge_info = {
                let segment_guard = segment.lock();
                segment_guard.edge_information.get(&edge_key(edge)).copied()
            };
            let Some(edge_info) = edge_info else {
                continue;
            };

            if edge_info.inverted_left || edge_info.inverted_right {
                self.calculate_control_points_inverted_edge(edge, &snapshot, &edge_info);
                continue;
            }

            let sloppy = self.spline_routing_mode == SplineRoutingMode::Sloppy
                && (edge_info.normal_source_node || edge_info.normal_target_node)
                && self.segment_allows_sloppy_routing(&snapshot)
                && !snapshot.is_hyper_edge;

            if sloppy {
                self.calculate_control_points_sloppy(edge, &snapshot, &edge_info);
            } else {
                self.calculate_control_points_conservative(edge, &snapshot, &edge_info);
            }
        }

        if snapshot.inverse_order {
            for edge in &snapshot.edges {
                if let Some(mut edge_guard) = edge.lock_ok() {
                    let reversed = KVectorChain::reverse(edge_guard.bend_points_ref());
                    edge_guard.bend_points().clear();
                    edge_guard.bend_points().add_all(&reversed.to_array());
                }
            }
        }
    }

    fn calculate_control_points_straight(&self, segment: &SegmentSnapshot) {
        let x_start_pos = segment.bounding_box.x;
        let x_end_pos = segment.bounding_box.x + segment.bounding_box.width;
        let halfway = KVector::with_values(
            x_start_pos + (x_end_pos - x_start_pos) / 2.0,
            segment.center_control_point_y,
        );
        if let Some(edge) = segment.edges.first() {
            if let Some(mut edge_guard) = edge.lock_ok() {
                edge_guard.bend_points().add_vector(halfway);
            }
        }
    }

    fn calculate_control_points_inverted_edge(
        &self,
        edge: &LEdgeRef,
        segment: &SegmentSnapshot,
        edge_info: &EdgeInformation,
    ) {
        let start_x_pos = segment.bounding_box.x;
        let end_x_pos = segment.bounding_box.x + segment.bounding_box.width;
        let y_source_anchor = edge_info.start_y;
        let y_target_anchor = edge_info.end_y;

        let source_straight_cp = if edge_info.inverted_left {
            KVector::with_values(end_x_pos, y_source_anchor)
        } else {
            KVector::with_values(start_x_pos, y_source_anchor)
        };
        let target_straight_cp = if edge_info.inverted_right {
            KVector::with_values(start_x_pos, y_target_anchor)
        } else {
            KVector::with_values(end_x_pos, y_target_anchor)
        };

        let mut center_x_pos = start_x_pos;
        if !segment.is_west_of_initial_layer {
            center_x_pos += self.edge_node_spacing;
        }
        center_x_pos += segment.x_delta + (segment.rank as f64) * self.edge_edge_spacing;

        let source_vertical_cp = KVector::with_values(center_x_pos, y_source_anchor);
        let target_vertical_cp = KVector::with_values(center_x_pos, y_target_anchor);

        if let Some(mut edge_guard) = edge.lock_ok() {
            edge_guard.bend_points().add_vector(source_straight_cp);
            edge_guard.bend_points().add_vector(source_vertical_cp);
            if segment.is_hyper_edge {
                let center = KVector::with_values(center_x_pos, segment.center_control_point_y);
                edge_guard.bend_points().add_vector(center);
            }
            edge_guard.bend_points().add_vector(target_vertical_cp);
            edge_guard.bend_points().add_vector(target_straight_cp);
        }
    }

    fn calculate_control_points_conservative(
        &self,
        edge: &LEdgeRef,
        segment: &SegmentSnapshot,
        edge_info: &EdgeInformation,
    ) {
        let start_x_pos = segment.bounding_box.x;
        let end_x_pos = segment.bounding_box.x + segment.bounding_box.width;
        let y_source_anchor = edge_info.start_y;
        let y_target_anchor = edge_info.end_y;

        let source_straight_cp = KVector::with_values(start_x_pos, y_source_anchor);
        let target_straight_cp = KVector::with_values(end_x_pos, y_target_anchor);

        let mut center_x_pos = start_x_pos;
        if !segment.is_west_of_initial_layer {
            center_x_pos += self.edge_node_spacing;
        }
        center_x_pos += segment.x_delta + (segment.rank as f64) * self.edge_edge_spacing;

        let source_vertical_cp = KVector::with_values(center_x_pos, y_source_anchor);
        let target_vertical_cp = KVector::with_values(center_x_pos, y_target_anchor);

        if let Some(mut edge_guard) = edge.lock_ok() {
            edge_guard.bend_points().add_vector(source_straight_cp);
            edge_guard.bend_points().add_vector(source_vertical_cp);
            if segment.is_hyper_edge {
                let center = KVector::with_values(center_x_pos, segment.center_control_point_y);
                edge_guard.bend_points().add_vector(center);
            }
            edge_guard.bend_points().add_vector(target_vertical_cp);
            edge_guard.bend_points().add_vector(target_straight_cp);
        }
    }

    fn calculate_control_points_sloppy(
        &self,
        edge: &LEdgeRef,
        segment: &SegmentSnapshot,
        edge_info: &EdgeInformation,
    ) {
        let start_x_pos = segment.bounding_box.x;
        let end_x_pos = segment.bounding_box.x + segment.bounding_box.width;
        let y_source_anchor = edge_info.start_y;
        let y_target_anchor = edge_info.end_y;
        let edge_points_downwards = y_source_anchor < y_target_anchor;

        let source_straight_cp = KVector::with_values(start_x_pos, y_source_anchor);
        let target_straight_cp = KVector::with_values(end_x_pos, y_target_anchor);
        let center_x_pos = (start_x_pos + end_x_pos) / 2.0;
        let source_vertical_cp = KVector::with_values(center_x_pos, y_source_anchor);
        let target_vertical_cp = KVector::with_values(center_x_pos, y_target_anchor);

        let center_y_pos = self.compute_sloppy_center_y(edge, y_source_anchor, y_target_anchor);
        let v1 = segment
            .source_port
            .as_ref()
            .and_then(|port| {
                port.lock_ok()
                    .and_then(|port_guard| port_guard.absolute_anchor())
            })
            .unwrap_or_default();
        let v2 = KVector::with_values(center_x_pos, center_y_pos);
        let v3 = segment
            .target_port
            .as_ref()
            .and_then(|port| {
                port.lock_ok()
                    .and_then(|port_guard| port_guard.absolute_anchor())
            })
            .unwrap_or_default();
        let approx = ElkMath::approximate_bezier_segment(2, &[v1, v2, v3]);
        let approx_center = approx.first().copied().unwrap_or(v2);

        let mut short_cut_source = false;
        if let Some(source_port) = segment.source_port.as_ref() {
            let src_node = source_port
                .lock_ok()
                .and_then(|port_guard| port_guard.node());
            if let Some(src_node) = src_node {
                let src_layer = src_node
                    .lock_ok()
                    .and_then(|node_guard| node_guard.layer());
                if src_layer.is_some() && edge_info.normal_source_node {
                    let node_id = src_node
                        .lock_ok()
                        .map(|mut node_guard| node_guard.shape().graph_element().id)
                        .unwrap_or(0) as isize;
                    let layer_nodes = src_layer
                        .as_ref()
                        .and_then(|layer| {
                            layer
                                .lock_ok()
                                .map(|layer_guard| layer_guard.nodes().clone())
                        })
                        .unwrap_or_default();
                    let need_to_check_src = (edge_points_downwards
                        && node_id < (layer_nodes.len() as isize - 1))
                        || (!edge_points_downwards && node_id > 0);

                    if !need_to_check_src {
                        short_cut_source = true;
                    } else {
                        let neighbor_index = if edge_points_downwards {
                            (node_id + 1) as usize
                        } else {
                            (node_id - 1) as usize
                        };
                        if let Some(neighbor) = layer_nodes.get(neighbor_index) {
                            let box_rect = node_to_bounding_box(neighbor);
                            short_cut_source =
                                !(ElkMath::intersects((&box_rect, &v1, &approx_center))
                                    || ElkMath::contains((&box_rect, &v1, &approx_center)));
                        }
                    }
                }
            }
        }

        let mut short_cut_target = false;
        if let Some(target_port) = segment.target_port.as_ref() {
            let tgt_node = target_port
                .lock_ok()
                .and_then(|port_guard| port_guard.node());
            if let Some(tgt_node) = tgt_node {
                let tgt_layer = tgt_node
                    .lock_ok()
                    .and_then(|node_guard| node_guard.layer());
                if tgt_layer.is_some() && edge_info.normal_target_node {
                    let node_id = tgt_node
                        .lock_ok()
                        .map(|mut node_guard| node_guard.shape().graph_element().id)
                        .unwrap_or(0) as isize;
                    let layer_nodes = tgt_layer
                        .as_ref()
                        .and_then(|layer| {
                            layer
                                .lock_ok()
                                .map(|layer_guard| layer_guard.nodes().clone())
                        })
                        .unwrap_or_default();
                    let need_to_check_tgt = (edge_points_downwards && node_id > 0)
                        || (!edge_points_downwards && node_id < (layer_nodes.len() as isize - 1));

                    if !need_to_check_tgt {
                        short_cut_target = true;
                    } else {
                        let neighbor_index = if edge_points_downwards {
                            (node_id - 1) as usize
                        } else {
                            (node_id + 1) as usize
                        };
                        if let Some(neighbor) = layer_nodes.get(neighbor_index) {
                            let box_rect = node_to_bounding_box(neighbor);
                            short_cut_target =
                                !(ElkMath::intersects((&box_rect, &approx_center, &v3))
                                    || ElkMath::contains((&box_rect, &approx_center, &v3)));
                        }
                    }
                }
            }
        }

        if let Some(mut edge_guard) = edge.lock_ok() {
            if short_cut_source && short_cut_target {
                edge_guard.bend_points().add_vector(v2);
            }
            if !short_cut_source {
                edge_guard.bend_points().add_vector(source_straight_cp);
                edge_guard.bend_points().add_vector(source_vertical_cp);
            }
            if !short_cut_target {
                edge_guard.bend_points().add_vector(target_vertical_cp);
                edge_guard.bend_points().add_vector(target_straight_cp);
            }
        }
    }

    fn compute_sloppy_center_y(
        &self,
        edge: &LEdgeRef,
        y_source_anchor: f64,
        y_target_anchor: f64,
    ) -> f64 {
        let mut indegree = 0i32;
        let mut outdegree = 0i32;

        let source_port = edge.lock_ok().and_then(|edge_guard| edge_guard.source());
        let target_port = edge.lock_ok().and_then(|edge_guard| edge_guard.target());

        if let Some(target_port) = target_port {
            if let Some(node) = target_port
                .lock_ok()
                .and_then(|port_guard| port_guard.node())
            {
                let ports = node
                    .lock_ok()
                    .map(|node_guard| node_guard.ports().clone())
                    .unwrap_or_default();
                for port in ports {
                    indegree += port
                        .lock_ok()
                        .map(|port_guard| port_guard.incoming_edges().len() as i32)
                        .unwrap_or(0);
                }
            }
        } else {
            indegree = 1;
        }

        if let Some(source_port) = source_port {
            if let Some(node) = source_port
                .lock_ok()
                .and_then(|port_guard| port_guard.node())
            {
                let ports = node
                    .lock_ok()
                    .map(|node_guard| node_guard.ports().clone())
                    .unwrap_or_default();
                for port in ports {
                    outdegree += port
                        .lock_ok()
                        .map(|port_guard| port_guard.outgoing_edges().len() as i32)
                        .unwrap_or(0);
                }
            }
        } else {
            outdegree = 1;
        }

        let degree_diff = (outdegree - indegree).signum() as f64;
        (y_target_anchor + y_source_anchor) / 2.0
            + (y_target_anchor - y_source_anchor)
                * (Self::SLOPPY_CENTER_CP_MULTIPLIER * degree_diff)
    }

    fn calculate_bezier_bend_points(
        &mut self,
        edge_chain: Vec<LEdgeRef>,
        surviving_edge: Option<LEdgeRef>,
    ) {
        if edge_chain.is_empty() {
            return;
        }

        let mut all_cp = KVectorChain::new();
        let edge = surviving_edge.unwrap_or_else(|| edge_chain[0].clone());
        let source_port = edge.lock_ok().and_then(|edge_guard| edge_guard.source());
        let Some(source_port) = source_port else {
            return;
        };

        let source_node = source_port
            .lock_ok()
            .and_then(|port_guard| port_guard.node());
        let qualified = source_node
            .as_ref()
            .and_then(|node| {
                node.lock_ok()
                    .map(|node_guard| SplineEdgeRouter::is_qualified_as_starting_node(&node_guard))
            })
            .unwrap_or(false);
        if !qualified {
            panic!("The target node of the edge must be a normal node or a northSouthPort.");
        }

        let source_anchor = source_port
            .lock_ok()
            .and_then(|port_guard| port_guard.absolute_anchor())
            .unwrap_or_else(KVector::new);
        all_cp.add_vector(source_anchor);

        if let Some(mut port_guard) = source_port.lock_ok() {
            if SIDES_NORTH_SOUTH.contains(&port_guard.side()) {
                let y = port_guard
                    .get_property(InternalProperties::SPLINE_NS_PORT_Y_COORD)
                    .unwrap_or(source_anchor.y);
                let north_south_cp = KVector::with_values(source_anchor.x, y);
                all_cp.add_vector(north_south_cp);
            }
        }

        let mut last_cp: Option<KVector> = None;
        let mut add_mid_point = false;
        for current_edge in edge_chain {
            let mut edge_guard = match current_edge.lock_ok() {
            Some(guard) => guard,
            None => continue,
            };
            let current_bend_points = edge_guard.bend_points();
            if !current_bend_points.is_empty() {
                if add_mid_point {
                    if let Some(last_cp_value) = last_cp {
                        let mut halfway = last_cp_value;
                        halfway.add(&current_bend_points.get_first());
                        halfway.scale(Self::ONE_HALF);
                        all_cp.add_vector(halfway);
                    }
                    add_mid_point = false;
                } else {
                    add_mid_point = true;
                }
                last_cp = Some(current_bend_points.get_last());
                all_cp.add_all(&current_bend_points.to_array());
                current_bend_points.clear();
            }
        }

        let target_port = edge.lock_ok().and_then(|edge_guard| edge_guard.target());
        let Some(target_port) = target_port else {
            return;
        };
        let target_anchor = target_port
            .lock_ok()
            .and_then(|port_guard| port_guard.absolute_anchor())
            .unwrap_or_else(KVector::new);

        if let Some(mut port_guard) = target_port.lock_ok() {
            if SIDES_NORTH_SOUTH.contains(&port_guard.side()) {
                let y = port_guard
                    .get_property(InternalProperties::SPLINE_NS_PORT_Y_COORD)
                    .unwrap_or(target_anchor.y);
                let north_south_cp = KVector::with_values(target_anchor.x, y);
                all_cp.add_vector(north_south_cp);
            }
        }

        all_cp.add_vector(target_anchor);

        if self.spline_routing_mode == SplineRoutingMode::Conservative {
            self.insert_straightening_control_points(&mut all_cp, &source_port, &target_port);
        }

        let mut nub_spline = NubSpline::new(true, SplineEdgeRouter::SPLINE_DIMENSION, all_cp);
        let bezier = nub_spline.get_bezier_cp_default();
        if let Some(mut edge_guard) = edge.lock_ok() {
            edge_guard.bend_points().add_all(&bezier.to_array());
        };
    }

    fn insert_straightening_control_points(
        &self,
        all_cps: &mut KVectorChain,
        src_port: &crate::org::eclipse::elk::alg::layered::graph::LPortRef,
        tgt_port: &crate::org::eclipse::elk::alg::layered::graph::LPortRef,
    ) {
        let first = all_cps.get_first();
        let second = all_cps.get(1);

        let src_side = src_port
            .lock_ok()
            .map(|port_guard| port_guard.side())
            .unwrap_or(PortSide::Undefined);
        let mut v = KVector::from_angle(SplinesMath::port_side_to_direction(src_side));
        v.scale(Self::NODE_TO_STRAIGHTENING_CP_GAP);
        let mut v2 = second;
        v2.sub(&first);
        let mut straighten_beginning = KVector::with_values(abs_min(v.x, v2.x), abs_min(v.y, v2.y));
        straighten_beginning.add(&first);

        all_cps.insert(1, straighten_beginning);

        let last = all_cps.get_last();
        let second_last = all_cps.get(all_cps.size() - 2);

        let tgt_side = tgt_port
            .lock_ok()
            .map(|port_guard| port_guard.side())
            .unwrap_or(PortSide::Undefined);
        let mut v = KVector::from_angle(SplinesMath::port_side_to_direction(tgt_side));
        v.scale(Self::NODE_TO_STRAIGHTENING_CP_GAP);
        let mut v2 = second_last;
        v2.sub(&last);
        let mut straighten_ending = KVector::with_values(abs_min(v.x, v2.x), abs_min(v.y, v2.y));
        straighten_ending.add(&last);

        all_cps.insert(all_cps.size() - 1, straighten_ending);
    }

    fn segment_allows_sloppy_routing(&self, segment: &SegmentSnapshot) -> bool {
        if self.compaction_strategy == GraphCompactionStrategy::None {
            return true;
        }

        let start_x_pos = segment.bounding_box.x;
        let end_x_pos = segment.bounding_box.x + segment.bounding_box.width;

        if segment.initial_segment {
            if let Some(node) = segment.source_node.as_ref() {
                let threshold = segment_node_distance_threshold(node);
                let node_pos_x = node
                    .lock_ok()
                    .map(|mut node_guard| node_guard.shape().position_ref().x)
                    .unwrap_or(0.0);
                let node_size_x = node
                    .lock_ok()
                    .map(|mut node_guard| node_guard.shape().size_ref().x)
                    .unwrap_or(0.0);
                let node_segment_distance = start_x_pos - (node_pos_x + node_size_x);
                if node_segment_distance > threshold {
                    return false;
                }
            }
        }
        if segment.last_segment {
            if let Some(node) = segment.target_node.as_ref() {
                let threshold = segment_node_distance_threshold(node);
                let node_pos_x = node
                    .lock_ok()
                    .map(|mut node_guard| node_guard.shape().position_ref().x)
                    .unwrap_or(0.0);
                let node_segment_distance = node_pos_x - end_x_pos;
                if node_segment_distance > threshold {
                    return false;
                }
            }
        }

        true
    }
}

impl ILayoutProcessor<LGraph> for FinalSplineBendpointsCalculator {
    fn process(&mut self, graph: &mut LGraph, _progress_monitor: &mut dyn IElkProgressMonitor) {
        self.edge_edge_spacing = graph
            .get_property(LayeredOptions::SPACING_EDGE_EDGE_BETWEEN_LAYERS)
            .unwrap_or(0.0);
        self.edge_node_spacing = graph
            .get_property(LayeredOptions::SPACING_EDGE_NODE_BETWEEN_LAYERS)
            .unwrap_or(0.0);
        self.spline_routing_mode = graph
            .get_property(LayeredOptions::EDGE_ROUTING_SPLINES_MODE)
            .unwrap_or(SplineRoutingMode::Sloppy);
        self.compaction_strategy = graph
            .get_property(LayeredOptions::COMPACTION_POST_COMPACTION_STRATEGY)
            .unwrap_or(GraphCompactionStrategy::None);

        Self::index_nodes_per_layer(graph);

        let mut start_edges: Vec<LEdgeRef> = Vec::new();
        let layers = graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock_ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let outgoing_edges = node
                    .lock_ok()
                    .map(|node_guard| node_guard.outgoing_edges())
                    .unwrap_or_default();
                for edge in outgoing_edges {
                    if edge
                        .lock_ok()
                        .map(|edge_guard| edge_guard.is_self_loop())
                        .unwrap_or(false)
                    {
                        continue;
                    }
                    let has_spline = edge
                        .lock_ok()
                        .and_then(|mut edge_guard| {
                            edge_guard.get_property(InternalProperties::SPLINE_ROUTE_START)
                        })
                        .is_some();
                    if has_spline {
                        start_edges.push(edge.clone());
                    }
                }
            }
        }

        for edge in &start_edges {
            let spline_segments = edge.lock_ok().and_then(|mut edge_guard| {
                edge_guard.get_property(InternalProperties::SPLINE_ROUTE_START)
            });
            if let Some(spline) = spline_segments {
                for segment in &spline {
                    self.calculate_control_points(segment);
                }
                if let Some(mut edge_guard) = edge.lock_ok() {
                    edge_guard.set_property::<Vec<SplineSegmentRef>>(
                        InternalProperties::SPLINE_ROUTE_START,
                        None,
                    );
                }
            }
        }

        for edge in &start_edges {
            let surviving_edge = edge.lock_ok().and_then(|mut edge_guard| {
                edge_guard.get_property(InternalProperties::SPLINE_SURVIVING_EDGE)
            });
            let edge_chain = edge
                .lock_ok()
                .and_then(|mut edge_guard| {
                    edge_guard.get_property(InternalProperties::SPLINE_EDGE_CHAIN)
                })
                .unwrap_or_default();
            self.calculate_bezier_bend_points(edge_chain, surviving_edge);
            if let Some(mut edge_guard) = edge.lock_ok() {
                edge_guard
                    .set_property::<Vec<LEdgeRef>>(InternalProperties::SPLINE_EDGE_CHAIN, None);
            }
        }
    }
}

fn node_to_bounding_box(node: &LNodeRef) -> ElkRectangle {
    let mut node_guard = node.lock();
    let pos = *node_guard.shape().position_ref();
    let size = *node_guard.shape().size_ref();
    let margin = node_guard.margin();
    ElkRectangle::with_values(
        pos.x - margin.left,
        pos.y - margin.top,
        size.x + margin.left + margin.right,
        size.y + margin.top + margin.bottom,
    )
}

fn abs_min(d1: f64, d2: f64) -> f64 {
    if d1.abs() < d2.abs() {
        d1
    } else {
        d2
    }
}

fn edge_key(edge: &LEdgeRef) -> usize {
    std::sync::Arc::as_ptr(edge) as usize
}

fn segment_node_distance_threshold(node: &LNodeRef) -> f64 {
    let layer_size = node
        .lock_ok()
        .and_then(|node_guard| node_guard.layer())
        .and_then(|layer| {
            layer
                .lock_ok()
                .map(|layer_guard| layer_guard.size_ref().x)
        })
        .unwrap_or(0.0);
    let node_size = node
        .lock_ok()
        .map(|mut node_guard| node_guard.shape().size_ref().x)
        .unwrap_or(0.0);
    layer_size - node_size / 2.0
}

struct SegmentSnapshot {
    edges: Vec<LEdgeRef>,
    is_straight: bool,
    is_hyper_edge: bool,
    bounding_box: ElkRectangle,
    rank: i32,
    x_delta: f64,
    is_west_of_initial_layer: bool,
    center_control_point_y: f64,
    source_port: Option<crate::org::eclipse::elk::alg::layered::graph::LPortRef>,
    target_port: Option<crate::org::eclipse::elk::alg::layered::graph::LPortRef>,
    initial_segment: bool,
    last_segment: bool,
    source_node: Option<LNodeRef>,
    target_node: Option<LNodeRef>,
    inverse_order: bool,
}

impl SegmentSnapshot {
    fn from_segment(
        segment: &crate::org::eclipse::elk::alg::layered::p5edges::splines::spline_segment::SplineSegment,
    ) -> Self {
        SegmentSnapshot {
            edges: segment.edges.clone(),
            is_straight: segment.is_straight,
            is_hyper_edge: segment.edges.len() > 1,
            bounding_box: segment.bounding_box,
            rank: segment.rank,
            x_delta: segment.x_delta,
            is_west_of_initial_layer: segment.is_west_of_initial_layer,
            center_control_point_y: segment.center_control_point_y,
            source_port: segment.source_port.clone(),
            target_port: segment.target_port.clone(),
            initial_segment: segment.initial_segment,
            last_segment: segment.last_segment,
            source_node: segment.source_node.clone(),
            target_node: segment.target_node.clone(),
            inverse_order: segment.inverse_order,
        }
    }
}
