use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use std::collections::HashSet;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkBendPoint, ElkConnectableShapeRef, ElkEdgeSection,
};
use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LGraphRef, LLabelRef, LNodeRef, LPortRef};
use crate::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, Origin, OriginId};

pub struct ElkGraphLayoutTransferrer<'a> {
    origin_store: &'a OriginStore,
}

impl<'a> ElkGraphLayoutTransferrer<'a> {
    pub fn new(origin_store: &'a OriginStore) -> Self {
        ElkGraphLayoutTransferrer { origin_store }
    }

    pub fn apply_layout(&self, lgraph: &LGraphRef) {
        let (origin, offset, nodes) = {
            let mut graph_guard = match lgraph.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let origin = graph_guard.get_property(InternalProperties::ORIGIN);
            let mut offset = *graph_guard.offset_ref();
            let padding = graph_guard.padding_ref().clone();
            offset.x += padding.left;
            offset.y += padding.top;
            let nodes = collect_nodes_from_graph(&graph_guard);
            (origin, offset, nodes)
        };

        let Some(Origin::ElkNode(graph_id)) = origin else {
            return;
        };
        let Some(_graph_node) = self.origin_store.get_node(graph_id) else {
            return;
        };

        self.apply_graph_layout(lgraph, graph_id);

        for node in &nodes {
            self.apply_node_layout(node, offset);
        }

        let edges = self.collect_edges(&nodes);
        for edge in edges {
            self.apply_edge_layout(&edge, offset);
        }

        for node in &nodes {
            let nested_graph = node.lock().ok().and_then(|node_guard| node_guard.nested_graph());
            if let Some(nested_graph) = nested_graph {
                self.apply_layout(&nested_graph);
            }
        }
    }

    fn apply_graph_layout(&self, lgraph: &LGraphRef, graph_id: OriginId) {
        let Some(elk_node) = self.origin_store.get_node(graph_id) else {
            return;
        };
        let actual_size = lgraph
            .lock()
            .ok()
            .map(|graph_guard| graph_guard.actual_size())
            .unwrap_or_else(KVector::new);

        let mut elk_node_mut = elk_node.borrow_mut();
        let shape = elk_node_mut.connectable().shape();
        shape.set_dimensions(actual_size.x, actual_size.y);
    }

    fn apply_node_layout(&self, lnode: &LNodeRef, offset: KVector) {
        let (origin, position, size, ports, labels) = {
            let mut node_guard = match lnode.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let origin = node_guard.get_property(InternalProperties::ORIGIN);
            let position = *node_guard.shape().position_ref();
            let size = *node_guard.shape().size_ref();
            let ports = node_guard.ports().clone();
            let labels = node_guard.labels().clone();
            (origin, position, size, ports, labels)
        };

        let Some(Origin::ElkNode(node_id)) = origin else {
            return;
        };
        let Some(elk_node) = self.origin_store.get_node(node_id) else {
            return;
        };

        {
            let mut elk_node_mut = elk_node.borrow_mut();
            let shape = elk_node_mut.connectable().shape();
            shape.set_x(position.x + offset.x);
            shape.set_y(position.y + offset.y);
            shape.set_dimensions(size.x, size.y);
        }

        for port in ports {
            self.apply_port_layout(&port);
        }

        for label in labels {
            self.apply_label_layout(&label);
        }
    }

    fn apply_port_layout(&self, lport: &LPortRef) {
        let (origin, position, size, labels, side) = {
            let mut port_guard = match lport.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let origin = port_guard.get_property(InternalProperties::ORIGIN);
            let position = *port_guard.shape().position_ref();
            let size = *port_guard.shape().size_ref();
            let labels = port_guard.labels().clone();
            let side = port_guard.side();
            (origin, position, size, labels, side)
        };

        let Some(Origin::ElkPort(port_id)) = origin else {
            return;
        };
        let Some(elk_port) = self.origin_store.get_port(port_id) else {
            return;
        };

        {
            let mut elk_port_mut = elk_port.borrow_mut();
            let shape = elk_port_mut.connectable().shape();
            shape.set_location(position.x, position.y);
            shape.set_dimensions(size.x, size.y);
            shape
                .graph_element()
                .properties_mut()
                .set_property(CoreOptions::PORT_SIDE, Some(side));
        }

        for label in labels {
            self.apply_label_layout(&label);
        }
    }

    fn apply_label_layout(&self, llabel: &LLabelRef) {
        let (origin, position, size) = {
            let mut label_guard = match llabel.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let origin = label_guard.get_property(InternalProperties::ORIGIN);
            let position = *label_guard.shape().position_ref();
            let size = *label_guard.shape().size_ref();
            (origin, position, size)
        };

        let Some(Origin::ElkLabel(label_id)) = origin else {
            return;
        };
        let Some(elk_label) = self.origin_store.get_label(label_id) else {
            return;
        };

        let mut elk_label_mut = elk_label.borrow_mut();
        let shape = elk_label_mut.shape();
        shape.set_location(position.x, position.y);
        shape.set_dimensions(size.x, size.y);
    }

    fn apply_edge_layout(&self, ledge: &LEdgeRef, offset: KVector) {
        let (origin, bend_points, source, target, labels) = {
            let mut edge_guard = match ledge.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let origin = edge_guard.get_property(InternalProperties::ORIGIN);
            let bend_points = edge_guard.bend_points_ref().to_array();
            let source = edge_guard.source();
            let target = edge_guard.target();
            let labels = edge_guard.labels().clone();
            (origin, bend_points, source, target, labels)
        };

        let Some(Origin::ElkEdge(edge_id)) = origin else {
            return;
        };
        let Some(elk_edge) = self.origin_store.get_edge(edge_id) else {
            return;
        };

        let start = source
            .as_ref()
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.absolute_anchor()))
            .unwrap_or_else(KVector::new);
        let end = target
            .as_ref()
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.absolute_anchor()))
            .unwrap_or_else(KVector::new);

        {
            let mut edge_mut = elk_edge.borrow_mut();
            edge_mut.sections().clear();
        }

        let (outgoing_shape, incoming_shape) = {
            let edge_ref = elk_edge.borrow();
            (
                first_shape(edge_ref.sources_ro()),
                first_shape(edge_ref.targets_ro()),
            )
        };

        let section = ElkEdgeSection::new();
        {
            let mut section_mut = section.borrow_mut();
            section_mut.set_start_x(start.x + offset.x);
            section_mut.set_start_y(start.y + offset.y);
            section_mut.set_end_x(end.x + offset.x);
            section_mut.set_end_y(end.y + offset.y);
            section_mut.set_outgoing_shape(outgoing_shape);
            section_mut.set_incoming_shape(incoming_shape);

            for point in bend_points {
                let bend = ElkBendPoint::new();
                {
                    let mut bend_mut = bend.borrow_mut();
                    bend_mut.set_x(point.x + offset.x);
                    bend_mut.set_y(point.y + offset.y);
                }
                section_mut.bend_points().push(bend);
            }
        }

        {
            let mut edge_mut = elk_edge.borrow_mut();
            edge_mut.sections().add(section);
        }

        for label in labels {
            self.apply_label_layout(&label);
        }
    }

    fn collect_edges(&self, nodes: &[LNodeRef]) -> Vec<LEdgeRef> {
        let mut seen: HashSet<usize> = HashSet::new();
        let mut edges = Vec::new();
        for node in nodes {
            let ports = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.ports().clone())
                .unwrap_or_default();
            for port in ports {
                let port_edges = port
                    .lock()
                    .ok()
                    .map(|port_guard| port_guard.connected_edges())
                    .unwrap_or_default();
                for edge in port_edges {
                    let key = Arc::as_ptr(&edge) as usize;
                    if seen.insert(key) {
                        edges.push(edge);
                    }
                }
            }
        }
        edges
    }
}

fn collect_nodes_from_graph(graph: &crate::org::eclipse::elk::alg::layered::graph::LGraph) -> Vec<LNodeRef> {
    let mut seen: HashSet<usize> = HashSet::new();
    let mut nodes = Vec::new();

    for node in graph.layerless_nodes() {
        let key = Arc::as_ptr(node) as usize;
        if seen.insert(key) {
            nodes.push(node.clone());
        }
    }

    for layer in graph.layers() {
        if let Ok(layer_guard) = layer.lock() {
            for node in layer_guard.nodes() {
                let key = Arc::as_ptr(node) as usize;
                if seen.insert(key) {
                    nodes.push(node.clone());
                }
            }
        }
    }

    nodes
}

fn first_shape(list: &org_eclipse_elk_graph::org::eclipse::elk::graph::EdgeEndpointList) -> Option<ElkConnectableShapeRef> {
    list.get(0)
}
