use std::f64::consts::PI;

use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::intermediate::rotation::IRadialRotator;
use crate::org::eclipse::elk::alg::radial::options::RadialOptions;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;

#[derive(Default)]
pub struct AngleRotation;

impl IRadialRotator for AngleRotation {
    fn rotate(&mut self, graph: &ElkNodeRef) {
        let mut target_angle = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RadialOptions::ROTATION_TARGET_ANGLE)
        }
        .unwrap_or(0.0);

        let compute_additional = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RadialOptions::ROTATION_COMPUTE_ADDITIONAL_WEDGE_SPACE)
        }
        .unwrap_or(false);

        if compute_additional {
            if let Some(root) = RadialUtil::root_from_graph(graph) {
                let outgoing_edges = {
                    let mut root_mut = root.borrow_mut();
                    root_mut
                        .connectable()
                        .outgoing_edges()
                        .iter()
                        .collect::<Vec<_>>()
                };
                if !outgoing_edges.is_empty() {
                    let last_edge = outgoing_edges.last().unwrap().clone();
                    let first_edge = outgoing_edges.first().unwrap().clone();

                    let last_node = {
                        let edge_borrow = last_edge.borrow();
                        edge_borrow.targets_ro().get(0)
                    }
                    .and_then(|shape| match shape {
                        org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Node(node) => {
                            Some(node)
                        }
                        org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Port(port) => {
                            port.borrow().parent()
                        }
                    });
                    let first_node = {
                        let edge_borrow = first_edge.borrow();
                        edge_borrow.targets_ro().get(0)
                    }
                    .and_then(|shape| match shape {
                        org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Node(node) => {
                            Some(node)
                        }
                        org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Port(port) => {
                            port.borrow().parent()
                        }
                    });

                    if let (Some(last_node), Some(first_node)) = (last_node, first_node) {
                        let last_vector = {
                            let mut node_mut = last_node.borrow_mut();
                            let shape = node_mut.connectable().shape();
                            KVector::with_values(
                                shape.x() + shape.width() / 2.0,
                                shape.y() + shape.height() / 2.0,
                            )
                        };
                        let first_vector = {
                            let mut node_mut = first_node.borrow_mut();
                            let shape = node_mut.connectable().shape();
                            KVector::with_values(
                                shape.x() + shape.width() / 2.0,
                                shape.y() + shape.height() / 2.0,
                            )
                        };

                        let mut alpha = target_angle;
                        if alpha <= 0.0 {
                            alpha += 2.0 * PI;
                        }

                        let mut wedge_angle = last_vector.angle(&first_vector);
                        if wedge_angle <= 0.0 {
                            wedge_angle += 2.0 * PI;
                        }

                        let mut alignment_angle = last_vector.y.atan2(last_vector.x);
                        if alignment_angle <= 0.0 {
                            alignment_angle += 2.0 * PI;
                        }

                        target_angle = PI - (alignment_angle - alpha + wedge_angle / 2.0);
                    }
                }
            }
        }

        let children: Vec<ElkNodeRef> = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.children().iter().cloned().collect()
        };

        for node in children {
            let (width, height, center_x, center_y) = {
                let mut node_mut = node.borrow_mut();
                let shape = node_mut.connectable().shape();
                (
                    shape.width(),
                    shape.height(),
                    shape.x() + shape.width() / 2.0,
                    shape.y() + shape.height() / 2.0,
                )
            };
            let mut pos = KVector::with_values(center_x, center_y);
            pos.rotate(target_angle);
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            shape.set_location(pos.x - width / 2.0, pos.y - height / 2.0);
        }
    }
}
