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
        for edge in &edges {
            self.apply_edge_layout(edge, offset);
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
            .unwrap_or_default();

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

        let (incoming_shape, outgoing_shape) = {
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
            section_mut.set_incoming_shape(incoming_shape);
            section_mut.set_outgoing_shape(outgoing_shape);

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

    fn normalize_graph_bounds(&self, graph_id: OriginId, nodes: &[LNodeRef], edges: &[LEdgeRef]) {
        let Some(graph_node) = self.origin_store.get_node(graph_id) else {
            return;
        };

        let (mut min_x, mut min_y, mut max_x, mut max_y): (f64, f64, f64, f64) = {
            let mut graph_mut = graph_node.borrow_mut();
            let shape = graph_mut.connectable().shape();
            (0.0, 0.0, shape.width(), shape.height())
        };

        for node in nodes {
            let origin = node
                .lock()
                .ok()
                .and_then(|mut guard| guard.get_property(InternalProperties::ORIGIN));
            let Some(Origin::ElkNode(node_id)) = origin else {
                continue;
            };
            let Some(elk_node) = self.origin_store.get_node(node_id) else {
                continue;
            };

            let mut elk_node_mut = elk_node.borrow_mut();
            let shape = elk_node_mut.connectable().shape();
            min_x = min_x.min(shape.x());
            min_y = min_y.min(shape.y());
            max_x = max_x.max(shape.x() + shape.width());
            max_y = max_y.max(shape.y() + shape.height());
        }

        for edge in edges {
            let origin = edge
                .lock()
                .ok()
                .and_then(|mut guard| guard.get_property(InternalProperties::ORIGIN));
            let Some(Origin::ElkEdge(edge_id)) = origin else {
                continue;
            };
            let Some(elk_edge) = self.origin_store.get_edge(edge_id) else {
                continue;
            };

            let (sections, labels) = {
                let mut edge_mut = elk_edge.borrow_mut();
                let sections = edge_mut.sections().iter().cloned().collect::<Vec<_>>();
                let labels = edge_mut.element().labels().iter().cloned().collect::<Vec<_>>();
                (sections, labels)
            };

            for section_ref in sections {
                let mut section = section_ref.borrow_mut();
                min_x = min_x.min(section.start_x()).min(section.end_x());
                min_y = min_y.min(section.start_y()).min(section.end_y());
                max_x = max_x.max(section.start_x()).max(section.end_x());
                max_y = max_y.max(section.start_y()).max(section.end_y());
                let bends = section.bend_points().to_vec();
                drop(section);
                for bend_ref in bends {
                    let bend = bend_ref.borrow();
                    min_x = min_x.min(bend.x());
                    min_y = min_y.min(bend.y());
                    max_x = max_x.max(bend.x());
                    max_y = max_y.max(bend.y());
                }
            }

            for label_ref in labels {
                let mut label_mut = label_ref.borrow_mut();
                let shape = label_mut.shape();
                min_x = min_x.min(shape.x());
                min_y = min_y.min(shape.y());
                max_x = max_x.max(shape.x() + shape.width());
                max_y = max_y.max(shape.y() + shape.height());
            }
        }

        let dx = if min_x < -1e-6 { -min_x } else { 0.0 };
        let dy = if min_y < -1e-6 { -min_y } else { 0.0 };

        if dx > 0.0 || dy > 0.0 {
            for node in nodes {
                let origin = node
                    .lock()
                    .ok()
                    .and_then(|mut guard| guard.get_property(InternalProperties::ORIGIN));
                let Some(Origin::ElkNode(node_id)) = origin else {
                    continue;
                };
                let Some(elk_node) = self.origin_store.get_node(node_id) else {
                    continue;
                };
                let mut elk_node_mut = elk_node.borrow_mut();
                let shape = elk_node_mut.connectable().shape();
                shape.set_location(shape.x() + dx, shape.y() + dy);
            }

            for edge in edges {
                let origin = edge
                    .lock()
                    .ok()
                    .and_then(|mut guard| guard.get_property(InternalProperties::ORIGIN));
                let Some(Origin::ElkEdge(edge_id)) = origin else {
                    continue;
                };
                let Some(elk_edge) = self.origin_store.get_edge(edge_id) else {
                    continue;
                };

                let (sections, labels) = {
                    let mut edge_mut = elk_edge.borrow_mut();
                    let sections = edge_mut.sections().iter().cloned().collect::<Vec<_>>();
                    let labels = edge_mut.element().labels().iter().cloned().collect::<Vec<_>>();
                    (sections, labels)
                };

                for section_ref in sections {
                    let mut section = section_ref.borrow_mut();
                    let start_x = section.start_x();
                    let start_y = section.start_y();
                    let end_x = section.end_x();
                    let end_y = section.end_y();
                    section.set_start_x(start_x + dx);
                    section.set_start_y(start_y + dy);
                    section.set_end_x(end_x + dx);
                    section.set_end_y(end_y + dy);
                    let bends = section.bend_points().to_vec();
                    for bend_ref in bends {
                        let mut bend = bend_ref.borrow_mut();
                        let bend_x = bend.x();
                        let bend_y = bend.y();
                        bend.set_x(bend_x + dx);
                        bend.set_y(bend_y + dy);
                    }
                }

                for label_ref in labels {
                    let mut label_mut = label_ref.borrow_mut();
                    let shape = label_mut.shape();
                    shape.set_location(shape.x() + dx, shape.y() + dy);
                }
            }

            max_x += dx;
            max_y += dy;
        }

        let mut graph_mut = graph_node.borrow_mut();
        let shape = graph_mut.connectable().shape();
        shape.set_dimensions(shape.width().max(max_x), shape.height().max(max_y));
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
    list.iter().next().cloned()
}
