use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LEdgeRef, LGraph, LNodeRef, LPort};
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;

pub struct HypernodeProcessor;

impl ILayoutProcessor<LGraph> for HypernodeProcessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Hypernodes processing", 1.0);

        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            for node in nodes {
                let (is_hypernode, ports) = match node.lock() {
                    Ok(mut node_guard) => (
                        node_guard
                            .get_property(LayeredOptions::HYPERNODE)
                            .unwrap_or(false),
                        node_guard.ports().clone(),
                    ),
                    Err(_) => (false, Vec::new()),
                };
                if !is_hypernode || ports.len() > 2 {
                    continue;
                }

                let mut top_edges = 0;
                let mut right_edges = 0;
                let mut bottom_edges = 0;
                let mut left_edges = 0;
                for port in ports {
                    let side = port
                        .lock()
                        .ok()
                        .map(|port_guard| port_guard.side())
                        .unwrap_or(PortSide::Undefined);
                    match side {
                        PortSide::North => top_edges += 1,
                        PortSide::East => right_edges += 1,
                        PortSide::South => bottom_edges += 1,
                        PortSide::West => left_edges += 1,
                        PortSide::Undefined => {}
                    }
                }

                // Java parity: only move if there are no top/bottom connections.
                if top_edges == 0 && bottom_edges == 0 {
                    let right = left_edges <= right_edges;
                    move_hypernode(layered_graph, &node, right);
                }
            }
        }

        monitor.done();
    }
}

fn move_hypernode(layered_graph: &mut LGraph, hypernode: &LNodeRef, right: bool) {
    let ports = hypernode
        .lock()
        .ok()
        .map(|node_guard| node_guard.ports().clone())
        .unwrap_or_default();

    let mut bend_edges: Vec<LEdgeRef> = Vec::new();
    let mut bend_x = i32::MAX as f64;
    let mut diff_x = i32::MAX as f64;
    let mut diff_y = i32::MAX as f64;

    if right {
        bend_x = layered_graph.size_ref().x;
        for port in ports {
            let outgoing = port
                .lock()
                .ok()
                .map(|port_guard| port_guard.outgoing_edges().clone())
                .unwrap_or_default();
            for edge in outgoing {
                let points = edge
                    .lock()
                    .ok()
                    .map(|edge_guard| edge_guard.bend_points_ref().to_array())
                    .unwrap_or_default();
                if points.is_empty() {
                    continue;
                }

                let first = points[0];
                if first.x < bend_x {
                    diff_x = bend_x - first.x;
                    diff_y = i32::MAX as f64;
                    bend_edges.clear();
                    bend_x = first.x;
                }
                if first.x <= bend_x {
                    bend_edges.push(edge);
                    if points.len() > 1 {
                        diff_y = diff_y.min((points[1].y - first.y).abs());
                    }
                }
            }
        }
    } else {
        // Keep Java behavior: bend_x starts at Integer.MAX_VALUE for this branch as well.
        for port in ports {
            let incoming = port
                .lock()
                .ok()
                .map(|port_guard| port_guard.incoming_edges().clone())
                .unwrap_or_default();
            for edge in incoming {
                let points = edge
                    .lock()
                    .ok()
                    .map(|edge_guard| edge_guard.bend_points_ref().to_array())
                    .unwrap_or_default();
                if points.is_empty() {
                    continue;
                }

                let last = points[points.len() - 1];
                if last.x > bend_x {
                    diff_x = last.x - bend_x;
                    diff_y = i32::MAX as f64;
                    bend_edges.clear();
                    bend_x = last.x;
                }
                if last.x >= bend_x {
                    bend_edges.push(edge);
                    if points.len() > 1 {
                        let penultimate = points[points.len() - 2];
                        diff_y = diff_y.min((penultimate.y - last.y).abs());
                    }
                }
            }
        }
    }

    let (node_width, node_height) = hypernode
        .lock()
        .ok()
        .map(|mut node_guard| {
            (
                node_guard.shape().size_ref().x,
                node_guard.shape().size_ref().y,
            )
        })
        .unwrap_or((0.0, 0.0));

    if bend_edges.is_empty() || diff_x <= node_width / 2.0 || diff_y <= node_height / 2.0 {
        return;
    }

    let north_port = LPort::new();
    LPort::set_node(&north_port, Some(hypernode.clone()));
    if let Ok(mut north_guard) = north_port.lock() {
        north_guard.set_side(PortSide::North);
        north_guard.shape().position().x = node_width / 2.0;
    }

    let south_port = LPort::new();
    LPort::set_node(&south_port, Some(hypernode.clone()));
    if let Ok(mut south_guard) = south_port.lock() {
        south_guard.set_side(PortSide::South);
        south_guard.shape().position().x = node_width / 2.0;
        south_guard.shape().position().y = node_height;
    }

    for edge in bend_edges {
        if right {
            process_right_edge(&edge, &north_port, &south_port);
        } else {
            process_left_edge(&edge, &north_port, &south_port);
        }
    }

    if let Ok(mut node_guard) = hypernode.lock() {
        node_guard.shape().position().x = bend_x - node_width / 2.0;
    }
}

fn process_right_edge(
    edge: &LEdgeRef,
    north_port: &crate::org::eclipse::elk::alg::layered::graph::LPortRef,
    south_port: &crate::org::eclipse::elk::alg::layered::graph::LPortRef,
) {
    let mut removed = None;
    let mut second = None;

    if let Ok(mut edge_guard) = edge.lock() {
        let mut points = edge_guard.bend_points_ref().to_array();
        if points.is_empty() {
            return;
        }
        let first = points.remove(0);
        rewrite_bend_points(&mut edge_guard, points.clone());

        if let Some(next) = points.first().copied() {
            second = Some(next);
        } else if let Some(target) = edge_guard.target() {
            second = target
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.absolute_anchor());
        }
        removed = Some(first);
    }

    let Some(first) = removed else {
        return;
    };
    let Some(second_point) = second else {
        return;
    };

    if second_point.y >= first.y {
        LEdge::set_source(edge, Some(south_port.clone()));
    } else {
        LEdge::set_source(edge, Some(north_port.clone()));
    }
    remove_junction_point(edge, first);
}

fn process_left_edge(
    edge: &LEdgeRef,
    north_port: &crate::org::eclipse::elk::alg::layered::graph::LPortRef,
    south_port: &crate::org::eclipse::elk::alg::layered::graph::LPortRef,
) {
    let mut removed = None;
    let mut second = None;

    if let Ok(mut edge_guard) = edge.lock() {
        let mut points = edge_guard.bend_points_ref().to_array();
        let Some(last) = points.pop() else {
            return;
        };
        rewrite_bend_points(&mut edge_guard, points.clone());

        if let Some(prev) = points.last().copied() {
            second = Some(prev);
        } else if let Some(source) = edge_guard.source() {
            second = source
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.absolute_anchor());
        }
        removed = Some(last);
    }

    let Some(last_point) = removed else {
        return;
    };
    let Some(second_point) = second else {
        return;
    };

    if second_point.y >= last_point.y {
        LEdge::set_target(edge, Some(south_port.clone()));
    } else {
        LEdge::set_target(edge, Some(north_port.clone()));
    }
    remove_junction_point(edge, last_point);
}

fn rewrite_bend_points(
    edge_guard: &mut crate::org::eclipse::elk::alg::layered::graph::LEdge,
    points: Vec<KVector>,
) {
    edge_guard.bend_points().clear();
    edge_guard.bend_points().add_all(&points);
}

fn remove_junction_point(edge: &LEdgeRef, removed_point: KVector) {
    if let Ok(mut edge_guard) = edge.lock() {
        let Some(mut junction_points) = edge_guard.get_property(LayeredOptions::JUNCTION_POINTS)
        else {
            return;
        };

        let filtered: Vec<KVector> = junction_points
            .iter()
            .copied()
            .filter(|point| *point != removed_point)
            .collect();
        junction_points.clear();
        junction_points.add_all(&filtered);
        edge_guard.set_property(LayeredOptions::JUNCTION_POINTS, Some(junction_points));
    }
}
