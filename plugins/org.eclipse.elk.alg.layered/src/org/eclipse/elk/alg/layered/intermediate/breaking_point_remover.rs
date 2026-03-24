use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph, LNode, LNodeRef};
use crate::org::eclipse::elk::alg::layered::intermediate::breaking_point_info::BreakingPointInfoRef;
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};
use crate::org::eclipse::elk::alg::layered::p5edges::splines::SplineSegmentRef;

pub struct BreakingPointRemover;

impl ILayoutProcessor<LGraph> for BreakingPointRemover {
    fn process(&mut self, graph: &mut LGraph, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Breaking Point Removing", 1.0);

        let edge_routing = graph
            .get_property(LayeredOptions::EDGE_ROUTING)
            .unwrap_or(EdgeRouting::Orthogonal);

        for layer in graph.layers().clone() {
            let nodes = {
                let layer_guard = layer.lock();
                layer_guard.nodes().clone()
            };

            for node in nodes {
                if !is_end(&node) {
                    continue;
                }

                let Some(bp_info) = breaking_point_info(&node) else {
                    continue;
                };
                let has_next = {
                    let bp_info_guard = bp_info.lock();
                    bp_info_guard.next.is_some()
                };
                if !has_next {
                    Self::remove(&bp_info, edge_routing);
                }
            }
        }

        progress_monitor.done();
    }
}

impl BreakingPointRemover {
    fn remove(bp_info: &BreakingPointInfoRef, edge_routing: EdgeRouting) {
        let (start, end, node_start_edge, start_end_edge, original_edge, prev) = {
            let bp_info_guard = bp_info.lock();
            (
                bp_info_guard.start.clone(),
                bp_info_guard.end.clone(),
                bp_info_guard.node_start_edge.clone(),
                bp_info_guard.start_end_edge.clone(),
                bp_info_guard.original_edge.clone(),
                bp_info_guard.prev.clone(),
            )
        };

        let mut new_bends = KVectorChain::new();

        match edge_routing {
            EdgeRouting::Splines => {
                let s1 = {
                    let edge_guard = node_start_edge.lock();
                    edge_guard
                        .get_property(InternalProperties::SPLINE_ROUTE_START)
                        .unwrap_or_default()
                };
                let mut s2 = {
                    let edge_guard = start_end_edge.lock();
                    edge_guard
                        .get_property(InternalProperties::SPLINE_ROUTE_START)
                        .unwrap_or_default()
                };
                let s3 = {
                    let edge_guard = original_edge.lock();
                    edge_guard
                        .get_property(InternalProperties::SPLINE_ROUTE_START)
                        .unwrap_or_default()
                };

                let e1 = {
                    let edge_guard = node_start_edge.lock();
                    edge_guard
                        .get_property(InternalProperties::SPLINE_EDGE_CHAIN)
                        .unwrap_or_default()
                };
                let e2 = {
                    let edge_guard = start_end_edge.lock();
                    edge_guard
                        .get_property(InternalProperties::SPLINE_EDGE_CHAIN)
                        .unwrap_or_default()
                };
                let e3 = {
                    let edge_guard = original_edge.lock();
                    edge_guard
                        .get_property(InternalProperties::SPLINE_EDGE_CHAIN)
                        .unwrap_or_default()
                };

                for segment in &mut s2 {
                    let mut segment_guard = segment.lock();
                    segment_guard.inverse_order = true;
                }

                let mut joined_segments: Vec<SplineSegmentRef> = Vec::new();
                joined_segments.extend(s1);
                joined_segments.extend(s2.into_iter().rev());
                joined_segments.extend(s3);

                let mut joined_edges = Vec::new();
                joined_edges.extend(e1);
                joined_edges.extend(e2.into_iter().rev());
                joined_edges.extend(e3);

                {
                    let mut original_edge_guard = original_edge.lock();
                    original_edge_guard.set_property(
                        InternalProperties::SPLINE_ROUTE_START,
                        Some(joined_segments),
                    );
                    original_edge_guard
                        .set_property(InternalProperties::SPLINE_EDGE_CHAIN, Some(joined_edges));
                    original_edge_guard.set_property(
                        InternalProperties::SPLINE_SURVIVING_EDGE,
                        Some(original_edge.clone()),
                    );
                }

                {
                    let mut node_start_edge_guard = node_start_edge.lock();
                    node_start_edge_guard.set_property(
                        InternalProperties::SPLINE_ROUTE_START,
                        None::<Vec<SplineSegmentRef>>,
                    );
                    node_start_edge_guard
                        .set_property(InternalProperties::SPLINE_EDGE_CHAIN, None::<Vec<_>>);
                }
                {
                    let mut start_end_edge_guard = start_end_edge.lock();
                    start_end_edge_guard.set_property(
                        InternalProperties::SPLINE_ROUTE_START,
                        None::<Vec<SplineSegmentRef>>,
                    );
                    start_end_edge_guard
                        .set_property(InternalProperties::SPLINE_EDGE_CHAIN, None::<Vec<_>>);
                }
            }
            EdgeRouting::Polyline => {
                let node_start_bends = {
                    let edge_guard = node_start_edge.lock();
                    edge_guard.bend_points_ref().to_array()
                };
                let start_end_bends = {
                    let edge_guard = start_end_edge.lock();
                    edge_guard.bend_points_ref().to_array()
                };
                let original_bends = {
                    let edge_guard = original_edge.lock();
                    edge_guard.bend_points_ref().to_array()
                };

                new_bends.add_all(&node_start_bends);
                {
                    let mut start_guard = start.lock();
                    new_bends.add_vector(*start_guard.shape().position_ref());
                }
                let reversed = KVectorChain::reverse(&KVectorChain::from_vectors(&start_end_bends));
                new_bends.add_all(&reversed.to_array());
                {
                    let mut end_guard = end.lock();
                    new_bends.add_vector(*end_guard.shape().position_ref());
                }
                new_bends.add_all(&original_bends);
            }
            _ => {
                let node_start_bends = {
                    let edge_guard = node_start_edge.lock();
                    edge_guard.bend_points_ref().to_array()
                };
                let start_end_bends = {
                    let edge_guard = start_end_edge.lock();
                    edge_guard.bend_points_ref().to_array()
                };
                let original_bends = {
                    let edge_guard = original_edge.lock();
                    edge_guard.bend_points_ref().to_array()
                };

                new_bends.add_all(&node_start_bends);
                let reversed = KVectorChain::reverse(&KVectorChain::from_vectors(&start_end_bends));
                new_bends.add_all(&reversed.to_array());
                new_bends.add_all(&original_bends);
            }
        }

        if edge_routing != EdgeRouting::Splines {
            let mut original_edge_guard = original_edge.lock();
            original_edge_guard.bend_points().clear();
            original_edge_guard
                .bend_points()
                .add_all(&new_bends.to_array());
        }

        let restored_source = {
            let edge_guard = node_start_edge.lock();
            edge_guard.source()
        };
        LEdge::set_source(&original_edge, restored_source);

        let junction_points_one = {
            let edge_guard = node_start_edge.lock();
            edge_guard.get_property(LayeredOptions::JUNCTION_POINTS)
        };
        let junction_points_two = {
            let edge_guard = start_end_edge.lock();
            edge_guard.get_property(LayeredOptions::JUNCTION_POINTS)
        };
        let junction_points_three = {
            let edge_guard = original_edge.lock();
            edge_guard.get_property(LayeredOptions::JUNCTION_POINTS)
        };

        if junction_points_one.is_some()
            || junction_points_two.is_some()
            || junction_points_three.is_some()
        {
            let mut new_junction_points = KVectorChain::new();
            add_null_safe(&mut new_junction_points, junction_points_three);
            add_null_safe(&mut new_junction_points, junction_points_two);
            add_null_safe(&mut new_junction_points, junction_points_one);
            let mut original_edge_guard = original_edge.lock();
            original_edge_guard
                .set_property(LayeredOptions::JUNCTION_POINTS, Some(new_junction_points));
        }

        LEdge::set_source(&start_end_edge, None);
        LEdge::set_target(&start_end_edge, None);
        LEdge::set_source(&node_start_edge, None);
        LEdge::set_target(&node_start_edge, None);
        LNode::set_layer(&end, None);
        LNode::set_layer(&start, None);

        if let Some(prev) = prev {
            Self::remove(&prev, edge_routing);
        }
    }
}

fn breaking_point_info(node: &LNodeRef) -> Option<BreakingPointInfoRef> {
    let node_guard = node.lock();
    node_guard.get_property(InternalProperties::BREAKING_POINT_INFO)
}

fn is_end(node: &LNodeRef) -> bool {
    let Some(bp_info) = breaking_point_info(node) else {
        return false;
    };
    let bp_info_guard = bp_info.lock();
    Arc::ptr_eq(&bp_info_guard.end, node)
}

fn add_null_safe(container: &mut KVectorChain, to_add: Option<KVectorChain>) -> bool {
    let Some(to_add) = to_add else {
        return false;
    };
    container.add_all(&to_add.to_array());
    true
}
