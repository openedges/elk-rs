use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, NodeType as LNodeType};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InLayerConstraint, LayerConstraint, LayeredOptions,
};

/// Interactive layout processor that assigns reasonable positions to external port dummy nodes.
///
/// For interactive layout (using InteractiveCycleBreaker or InteractiveLayerer), dummy nodes
/// such as external port dummies need positions assigned up front. This processor positions
/// them appropriately - e.g., westward external ports are positioned left of all other nodes.
///
/// Based on Java's InteractiveExternalPortPositioner.
pub struct InteractiveExternalPortPositioner;

impl ILayoutProcessor<LGraph> for InteractiveExternalPortPositioner {
    fn process(&mut self, graph: &mut LGraph, _monitor: &mut dyn IElkProgressMonitor) {
        // If the graph does not contain any external ports, nothing to do
        if !graph
            .get_property(InternalProperties::GRAPH_PROPERTIES)
            .map(|gp| gp.contains(&GraphProperties::ExternalPorts))
            .unwrap_or(false)
        {
            return;
        }

        // Find the minimum and maximum x and y coordinates of normal nodes
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        let nodes = graph.layerless_nodes();
        for node_ref in nodes {
            {
                let mut node = node_ref.lock();
                if node.node_type() == LNodeType::Normal {
                    let pos = *node.shape().position();
                    let size = *node.shape().size();
                    let margins = node.get_property(CoreOptions::MARGINS).unwrap_or_default();

                    min_x = min_x.min(pos.x - margins.left);
                    max_x = max_x.max(pos.x + size.x + margins.right);
                    min_y = min_y.min(pos.y - margins.top);
                    max_y = max_y.max(pos.y + size.y + margins.bottom);
                }
            }
        }

        // Arbitrary spacing value to separate external port dummies from other nodes
        const ARBITRARY_SPACING: f64 = 10.0;

        // Assign reasonable coordinates to external port dummies
        //
        // Helpers may revisit this node through its edges. Release the node
        // guard before walking connections, then reacquire it to write coordinates.
        let nodes = graph.layerless_nodes().clone();
        for node_ref in &nodes {
            let (node_type, layer_constraint, in_layer_constraint) = {
                let node = node_ref.lock();
                (
                    node.node_type(),
                    node.get_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT),
                    node.get_property(InternalProperties::IN_LAYER_CONSTRAINT),
                )
            };
            if node_type != LNodeType::ExternalPort {
                continue;
            }

            // Computed with no guard held: each of these walks the node's own
            // edges and locks whatever it finds, including, legitimately, the
            // node itself.
            let mut x_from = None;
            let mut y_from = None;
            match layer_constraint {
                // WEST port — positioned left of all nodes.
                Some(LayerConstraint::FirstSeparate) => {
                    y_from = Self::find_y_from_targets(node_ref);
                }
                // EAST port — positioned right of all nodes.
                Some(LayerConstraint::LastSeparate) => {
                    y_from = Self::find_y_from_sources(node_ref);
                }
                _ => {}
            }
            if matches!(
                in_layer_constraint,
                Some(InLayerConstraint::Top) | Some(InLayerConstraint::Bottom)
            ) {
                x_from = Self::find_north_south_x(node_ref);
            }

            let mut node = node_ref.lock();
            match layer_constraint {
                Some(LayerConstraint::FirstSeparate) => {
                    node.shape().position().x = min_x - ARBITRARY_SPACING;
                    if let Some(y) = y_from {
                        node.shape().position().y = y;
                    }
                }
                Some(LayerConstraint::LastSeparate) => {
                    node.shape().position().x = max_x + ARBITRARY_SPACING;
                    if let Some(y) = y_from {
                        node.shape().position().y = y;
                    }
                }
                _ => {}
            }
            match in_layer_constraint {
                // NORTH port
                Some(InLayerConstraint::Top) => {
                    if let Some(x) = x_from {
                        node.shape().position().x = x + ARBITRARY_SPACING;
                    }
                    node.shape().position().y = min_y - ARBITRARY_SPACING;
                }
                // SOUTH port
                Some(InLayerConstraint::Bottom) => {
                    if let Some(x) = x_from {
                        node.shape().position().x = x + ARBITRARY_SPACING;
                    }
                    node.shape().position().y = max_y + ARBITRARY_SPACING;
                }
                _ => {}
            }
        }
    }
}

impl InteractiveExternalPortPositioner {
    /// Find Y coordinate for WEST ports by looking at target nodes
    fn find_y_from_targets(
        node_ref: &crate::org::eclipse::elk::alg::layered::graph::LNodeRef,
    ) -> Option<f64> {
        let edges = node_ref.lock().outgoing_edges();
        for edge_ref in edges {
            let target_port = edge_ref.lock().target();
            let Some(target_port) = target_port else {
                continue;
            };
            let target_node_ref = target_port.lock().node();
            let Some(target_node_ref) = target_node_ref else {
                continue;
            };
            let mut target_node = target_node_ref.lock();
            let pos = *target_node.shape().position();
            let size = *target_node.shape().size();
            return Some(pos.y + size.y / 2.0);
        }
        None
    }

    /// Find Y coordinate for EAST ports by looking at source nodes
    fn find_y_from_sources(
        node_ref: &crate::org::eclipse::elk::alg::layered::graph::LNodeRef,
    ) -> Option<f64> {
        let edges = node_ref.lock().incoming_edges();
        for edge_ref in edges {
            let source_port = edge_ref.lock().source();
            let Some(source_port) = source_port else {
                continue;
            };
            let source_node_ref = source_port.lock().node();
            let Some(source_node_ref) = source_node_ref else {
                continue;
            };
            let mut source_node = source_node_ref.lock();
            let pos = *source_node.shape().position();
            let size = *source_node.shape().size();
            return Some(pos.y + size.y / 2.0);
        }
        None
    }

    /// Find X coordinate for NORTH/SOUTH ports
    fn find_north_south_x(
        node_ref: &crate::org::eclipse::elk::alg::layered::graph::LNodeRef,
    ) -> Option<f64> {
        let port_ref = node_ref.lock().ports().first().cloned()?;
        let (outgoing, incoming) = {
            let port = port_ref.lock();
            (port.outgoing_edges().clone(), port.incoming_edges().clone())
        };
        let mut min = f64::INFINITY;
        for edge_ref in outgoing {
            let target_port = edge_ref.lock().target();
            let Some(target_port) = target_port else {
                continue;
            };
            let target_node = target_port.lock().node();
            let Some(target_node) = target_node else {
                continue;
            };
            let mut node = target_node.lock();
            let x = node.shape().position().x;
            let margins = node.get_property(CoreOptions::MARGINS).unwrap_or_default();
            min = min.min(x - margins.left);
        }
        if min != f64::INFINITY {
            return Some(min);
        }

        let mut max = f64::NEG_INFINITY;
        for edge_ref in incoming {
            let source_port = edge_ref.lock().source();
            let Some(source_port) = source_port else {
                continue;
            };
            let source_node = source_port.lock().node();
            let Some(source_node) = source_node else {
                continue;
            };
            let mut node = source_node.lock();
            let x = node.shape().position().x;
            let width = node.shape().size().x;
            let margins = node.get_property(CoreOptions::MARGINS).unwrap_or_default();
            max = max.max(x + width + margins.right);
        }
        (max != f64::NEG_INFINITY).then_some(max)
    }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraphRef, LNode, LNodeRef, LPort};
    use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, NullElkProgressMonitor};
    use std::sync::mpsc;
    use std::time::Duration;

    fn bounded(run: impl FnOnce() + Send + 'static) {
        let (send, receive) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            run();
            send.send(()).unwrap();
        });
        receive
            .recv_timeout(Duration::from_secs(5))
            .expect("external port positioning must not deadlock");
        worker.join().unwrap();
    }

    fn node(graph: &LGraphRef, external: bool) -> LNodeRef {
        let node = LNode::new(graph);
        {
            let mut n = node.lock();
            n.set_node_type(if external {
                LNodeType::ExternalPort
            } else {
                LNodeType::Normal
            });
            n.shape().position().x = 100.0;
            n.shape().position().y = 200.0;
            n.shape().size().x = 40.0;
            n.shape().size().y = 20.0;
        }
        graph.lock().layerless_nodes_mut().push(node.clone());
        node
    }

    fn graph() -> LGraphRef {
        crate::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout();
        let graph = LGraph::new();
        graph.lock().set_property(
            InternalProperties::GRAPH_PROPERTIES,
            Some(EnumSet::of(&[GraphProperties::ExternalPorts])),
        );
        graph
    }

    fn connect(source: &LNodeRef, target: &LNodeRef) {
        let sp = LPort::new();
        let tp = LPort::new();
        LPort::set_node(&sp, Some(source.clone()));
        LPort::set_node(&tp, Some(target.clone()));
        let edge = LEdge::new();
        LEdge::set_source(&edge, Some(sp));
        LEdge::set_target(&edge, Some(tp));
    }

    #[test]
    fn interactive_external_ports_preserve_all_four_side_coordinates() {
        bounded(|| {
            for side in 0..4 {
                let graph = graph();
                let normal = node(&graph, false);
                let external = node(&graph, true);
                let expected = match side {
                    0 => {
                        external.lock().set_property(
                            LayeredOptions::LAYERING_LAYER_CONSTRAINT,
                            Some(LayerConstraint::FirstSeparate),
                        );
                        connect(&external, &normal);
                        (90.0, 210.0)
                    }
                    1 => {
                        external.lock().set_property(
                            LayeredOptions::LAYERING_LAYER_CONSTRAINT,
                            Some(LayerConstraint::LastSeparate),
                        );
                        connect(&normal, &external);
                        (150.0, 210.0)
                    }
                    2 => {
                        external.lock().set_property(
                            InternalProperties::IN_LAYER_CONSTRAINT,
                            Some(InLayerConstraint::Top),
                        );
                        connect(&external, &normal);
                        (110.0, 190.0)
                    }
                    _ => {
                        external.lock().set_property(
                            InternalProperties::IN_LAYER_CONSTRAINT,
                            Some(InLayerConstraint::Bottom),
                        );
                        connect(&normal, &external);
                        (150.0, 230.0)
                    }
                };
                InteractiveExternalPortPositioner
                    .process(&mut graph.lock(), &mut NullElkProgressMonitor);
                let mut external = external.lock();
                let pos = external.shape().position();
                assert_eq!((pos.x, pos.y), expected, "side={side}");
            }
        });
    }

    #[test]
    fn external_port_helpers_allow_edges_back_to_the_same_node() {
        bounded(|| {
            let graph = graph();
            let external = node(&graph, true);
            connect(&external, &external);
            assert_eq!(
                InteractiveExternalPortPositioner::find_y_from_targets(&external),
                Some(210.0)
            );
            assert_eq!(
                InteractiveExternalPortPositioner::find_y_from_sources(&external),
                Some(210.0)
            );
            assert_eq!(
                InteractiveExternalPortPositioner::find_north_south_x(&external),
                Some(100.0)
            );
            let disconnected = node(&graph, true);
            assert_eq!(
                InteractiveExternalPortPositioner::find_y_from_targets(&disconnected),
                None
            );
            assert_eq!(
                InteractiveExternalPortPositioner::find_y_from_sources(&disconnected),
                None
            );
            assert_eq!(
                InteractiveExternalPortPositioner::find_north_south_x(&disconnected),
                None
            );
        });
    }
}
