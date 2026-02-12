use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_util::ElkUtil;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeSection,
};
use std::collections::HashSet;
use std::sync::Arc;

use crate::org::eclipse::elk::alg::layered::graph::l_graph_util::LGraphUtil;
use crate::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use crate::org::eclipse::elk::alg::layered::graph::NodeType;
use crate::org::eclipse::elk::alg::layered::graph::{
    LEdgeRef, LGraphRef, LLabelRef, LNodeRef, LPortRef,
};
use crate::org::eclipse::elk::alg::layered::intermediate::INCLUDE_LABEL;
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions, Origin,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;

pub struct ElkGraphLayoutTransferrer<'a> {
    origin_store: &'a OriginStore,
}

impl<'a> ElkGraphLayoutTransferrer<'a> {
    pub fn new(origin_store: &'a OriginStore) -> Self {
        ElkGraphLayoutTransferrer { origin_store }
    }

    pub fn apply_layout(&self, lgraph: &LGraphRef) {
        // 1. Extract parent ElkNode from LGraph's ORIGIN property
        // 2. Get offset from LGraph (offset + padding.left/top)
        // 3. Get padding
        let (origin, offset, padding) = {
            let mut graph_guard = match lgraph.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let origin = graph_guard.get_property(InternalProperties::ORIGIN);
            let mut offset = *graph_guard.offset_ref();
            let padding = graph_guard.padding_ref().clone();
            offset.x += padding.left;
            offset.y += padding.top;
            (origin, offset, padding)
        };

        let Some(Origin::ElkNode(graph_id)) = origin else {
            return;
        };
        let Some(elk_node) = self.origin_store.get_node(graph_id) else {
            return;
        };

        // 4. Set computed padding if NODE_SIZE_OPTIONS.COMPUTE_PADDING on the ElkNode
        {
            let mut elk_node_mut = elk_node.borrow_mut();
            let props = elk_node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            let size_options = props
                .get_property(CoreOptions::NODE_SIZE_OPTIONS)
                .unwrap_or_else(EnumSet::none_of);
            if size_options.contains(&SizeOptions::ComputePadding) {
                let elk_padding =
                    org_eclipse_elk_core::org::eclipse::elk::core::math::elk_padding::ElkPadding::with_values(
                        padding.top, padding.right, padding.bottom, padding.left,
                    );
                props.set_property(CoreOptions::PADDING, Some(elk_padding));
            }
        }

        // 5. Process layerless nodes and collect from layers
        let parent_node = lgraph.lock().ok().and_then(|g| g.parent_node());
        let all_nodes = {
            let graph_guard = match lgraph.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            collect_nodes_from_graph(&graph_guard)
        };

        let mut edges: Vec<LEdgeRef> = Vec::new();
        let mut edge_seen: HashSet<usize> = HashSet::new();

        for lnode in &all_nodes {
            let (node_origin, node_type) = {
                let mut node_guard = match lnode.lock() {
                    Ok(guard) => guard,
                    Err(_) => continue,
                };
                (
                    node_guard.get_property(InternalProperties::ORIGIN),
                    node_guard.node_type(),
                )
            };

            match &node_origin {
                Some(Origin::ElkNode(_)) => {
                    // If node ORIGIN is ElkNode -> apply_node_layout
                    self.apply_node_layout(lnode, offset);
                }
                Some(Origin::ElkPort(port_id)) => {
                    // External port dummy — its origin is an ElkPort
                    if node_type == NodeType::ExternalPort && parent_node.is_none() {
                        if let Some(elk_port) = self.origin_store.get_port(*port_id) {
                            let (port_width, port_height) = {
                                let mut port_mut = elk_port.borrow_mut();
                                let shape = port_mut.connectable().shape();
                                (shape.width(), shape.height())
                            };
                            let port_pos = LGraphUtil::get_external_port_position(
                                lgraph,
                                lnode,
                                port_width,
                                port_height,
                            );
                            let mut port_mut = elk_port.borrow_mut();
                            let shape = port_mut.connectable().shape();
                            shape.set_location(port_pos.x, port_pos.y);
                        }
                    }
                }
                _ => {}
            }

            // Collect outgoing edges (filtering: !LGraphUtil::is_descendant(edge.target.node, lnode))
            let ports = lnode
                .lock()
                .ok()
                .map(|node_guard| node_guard.ports().clone())
                .unwrap_or_default();
            for port in &ports {
                let outgoing = port
                    .lock()
                    .ok()
                    .map(|port_guard| port_guard.outgoing_edges().clone())
                    .unwrap_or_default();
                for edge in outgoing {
                    // Check if edge target node is a descendant of lnode
                    let target_node = edge
                        .lock()
                        .ok()
                        .and_then(|edge_guard| edge_guard.target())
                        .and_then(|target_port| {
                            target_port.lock().ok().and_then(|port| port.node())
                        });
                    let is_descendant = target_node
                        .as_ref()
                        .map(|tn| LGraphUtil::is_descendant(tn, lnode))
                        .unwrap_or(false);
                    if !is_descendant {
                        let key = Arc::as_ptr(&edge) as usize;
                        if edge_seen.insert(key) {
                            edges.push(edge);
                        }
                    }
                }
                let incoming = port
                    .lock()
                    .ok()
                    .map(|port_guard| port_guard.incoming_edges().clone())
                    .unwrap_or_default();
                for edge in incoming {
                    // Check if edge source node is a descendant of lnode
                    let source_node = edge
                        .lock()
                        .ok()
                        .and_then(|edge_guard| edge_guard.source())
                        .and_then(|source_port| {
                            source_port.lock().ok().and_then(|port| port.node())
                        });
                    let is_descendant = source_node
                        .as_ref()
                        .map(|sn| LGraphUtil::is_descendant(sn, lnode))
                        .unwrap_or(false);
                    if !is_descendant {
                        let key = Arc::as_ptr(&edge) as usize;
                        if edge_seen.insert(key) {
                            edges.push(edge);
                        }
                    }
                }
            }
        }

        // 6. Collect hierarchical edges from parent LNode's ports (outgoing edges where target IS descendant)
        if let Some(ref parent_lnode) = parent_node {
            let parent_ports = parent_lnode
                .lock()
                .ok()
                .map(|node_guard| node_guard.ports().clone())
                .unwrap_or_default();
            for port in &parent_ports {
                let outgoing = port
                    .lock()
                    .ok()
                    .map(|port_guard| port_guard.outgoing_edges().clone())
                    .unwrap_or_default();
                for edge in outgoing {
                    let target_node = edge
                        .lock()
                        .ok()
                        .and_then(|edge_guard| edge_guard.target())
                        .and_then(|target_port| {
                            target_port.lock().ok().and_then(|port| port.node())
                        });
                    let is_descendant = target_node
                        .as_ref()
                        .map(|tn| LGraphUtil::is_descendant(tn, parent_lnode))
                        .unwrap_or(false);
                    if is_descendant {
                        let key = Arc::as_ptr(&edge) as usize;
                        if edge_seen.insert(key) {
                            edges.push(edge);
                        }
                    }
                }
                let incoming = port
                    .lock()
                    .ok()
                    .map(|port_guard| port_guard.incoming_edges().clone())
                    .unwrap_or_default();
                for edge in incoming {
                    let source_node = edge
                        .lock()
                        .ok()
                        .and_then(|edge_guard| edge_guard.source())
                        .and_then(|source_port| {
                            source_port.lock().ok().and_then(|port| port.node())
                        });
                    let is_descendant = source_node
                        .as_ref()
                        .map(|sn| LGraphUtil::is_descendant(sn, parent_lnode))
                        .unwrap_or(false);
                    if is_descendant {
                        let key = Arc::as_ptr(&edge) as usize;
                        if edge_seen.insert(key) {
                            edges.push(edge);
                        }
                    }
                }
            }
        }

        // 7. Get edge_routing from ElkNode property
        let edge_routing = {
            let mut elk_node_mut = elk_node.borrow_mut();
            elk_node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(LayeredOptions::EDGE_ROUTING)
                .unwrap_or(EdgeRouting::Undefined)
        };

        // 8. Apply edge layout to all collected edges
        for edge in &edges {
            self.apply_edge_layout(edge, offset, edge_routing);
        }

        // 9. Apply parent node layout
        self.apply_parent_node_layout(lgraph);

        // 10. Recursively process nested subgraphs
        for node in &all_nodes {
            let nested_graph = node
                .lock()
                .ok()
                .and_then(|node_guard| node_guard.nested_graph());
            if let Some(nested_graph) = nested_graph {
                self.apply_layout(&nested_graph);
            }
        }
    }

    fn apply_parent_node_layout(&self, lgraph: &LGraphRef) {
        // Get origin ElkNode
        let origin = lgraph
            .lock()
            .ok()
            .and_then(|mut g| g.get_property(InternalProperties::ORIGIN));
        let Some(Origin::ElkNode(graph_id)) = origin else {
            return;
        };
        let Some(elk_node) = self.origin_store.get_node(graph_id) else {
            return;
        };

        // Check if NODE_SIZE_CONSTRAINTS included PORT_LABELS before we potentially overwrite it
        let size_constraints_included_port_labels = {
            let mut elk_node_mut = elk_node.borrow_mut();
            let props = elk_node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            props
                .get_property(LayeredOptions::NODE_SIZE_CONSTRAINTS)
                .unwrap_or_else(SizeConstraint::fixed)
                .contains(&SizeConstraint::PortLabels)
        };

        // Only resize for the top-level graph (no parent node)
        let parent_node = lgraph.lock().ok().and_then(|g| g.parent_node());
        if parent_node.is_none() {
            let (graph_props, actual_graph_size) = {
                let mut graph_guard = match lgraph.lock() {
                    Ok(guard) => guard,
                    Err(_) => return,
                };
                let gp = graph_guard
                    .get_property(InternalProperties::GRAPH_PROPERTIES)
                    .unwrap_or_else(EnumSet::none_of);
                let size = graph_guard.actual_size();
                (gp, size)
            };

            if graph_props.contains(&GraphProperties::ExternalPorts) {
                // Set PORT_CONSTRAINTS to FixedPos
                {
                    let mut elk_node_mut = elk_node.borrow_mut();
                    elk_node_mut
                        .connectable()
                        .shape()
                        .graph_element()
                        .properties_mut()
                        .set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedPos));
                }
                // Resize: move_ports=false, move_labels=true
                ElkUtil::resize_node_with(
                    &elk_node,
                    actual_graph_size.x,
                    actual_graph_size.y,
                    false,
                    true,
                );
            } else {
                // Only resize if NODE_SIZE_FIXED_GRAPH_SIZE is false
                let fixed_graph_size = {
                    let mut elk_node_mut = elk_node.borrow_mut();
                    elk_node_mut
                        .connectable()
                        .shape()
                        .graph_element()
                        .properties_mut()
                        .get_property(CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE)
                        .unwrap_or(false)
                };
                if !fixed_graph_size {
                    // Resize: move_ports=true, move_labels=true
                    ElkUtil::resize_node_with(
                        &elk_node,
                        actual_graph_size.x,
                        actual_graph_size.y,
                        true,
                        true,
                    );
                }
            }
        }

        // Restore NODE_SIZE_CONSTRAINTS: if port labels was included, set to just PORT_LABELS;
        // otherwise set to SizeConstraint::fixed()
        {
            let mut elk_node_mut = elk_node.borrow_mut();
            let props = elk_node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            if size_constraints_included_port_labels {
                props.set_property(
                    LayeredOptions::NODE_SIZE_CONSTRAINTS,
                    Some(EnumSet::of(&[SizeConstraint::PortLabels])),
                );
            } else {
                props.set_property(
                    LayeredOptions::NODE_SIZE_CONSTRAINTS,
                    Some(SizeConstraint::fixed()),
                );
            }
        }
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

        // Get layer_id and position_id from lnode
        let (layer_id, position_id) = {
            let mut node_guard = match lnode.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let lid = node_guard
                .get_property(LayeredOptions::LAYERING_LAYER_ID)
                .unwrap_or(-1);
            let pid = node_guard
                .get_property(LayeredOptions::CROSSING_MINIMIZATION_POSITION_ID)
                .unwrap_or(-1);
            (lid, pid)
        };

        // Set CROSSING_MINIMIZATION_POSITION_ID and LAYERING_LAYER_ID on ElkNode
        let _elk_size_constraints = {
            let mut elk_node_mut = elk_node.borrow_mut();
            let props = elk_node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            // Set CROSSING_MINIMIZATION_POSITION_ID and LAYERING_LAYER_ID on ElkNode
            props.set_property(
                LayeredOptions::CROSSING_MINIMIZATION_POSITION_ID,
                Some(position_id),
            );
            props.set_property(LayeredOptions::LAYERING_LAYER_ID, Some(layer_id));

            props
                .get_property(LayeredOptions::NODE_SIZE_CONSTRAINTS)
                .unwrap_or_else(SizeConstraint::fixed)
        };

        // Set position
        {
            let mut elk_node_mut = elk_node.borrow_mut();
            let shape = elk_node_mut.connectable().shape();
            shape.set_x(position.x + offset.x);
            shape.set_y(position.y + offset.y);
        }

        // Set node size
        // Note: Java conditionally sets size based on size constraints, nested graph, and
        // NetworkSimplex flexibility. However, for parity with the current Rust layout pipeline,
        // we unconditionally set the size here since the LabelAndNodeSizeProcessor already
        // computed the correct size during layout.
        {
            let mut elk_node_mut = elk_node.borrow_mut();
            let shape = elk_node_mut.connectable().shape();
            shape.set_dimensions(size.x, size.y);
        }

        // Set port positions and PORT_SIDE
        for port in &ports {
            self.apply_port_layout(port);
        }

        // Set node label positions
        for label in &labels {
            self.apply_label_layout_with_size(label);
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
            self.apply_label_layout_with_size(&label);
        }
    }

    fn apply_label_layout_with_size(&self, llabel: &LLabelRef) {
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
        shape.set_dimensions(size.x, size.y);
        shape.set_location(position.x, position.y);
    }

    fn apply_edge_layout(&self, ledge: &LEdgeRef, offset: KVector, edge_routing: EdgeRouting) {
        let (origin, source, target, labels) = {
            let mut edge_guard = match ledge.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let origin = edge_guard.get_property(InternalProperties::ORIGIN);
            let source = edge_guard.source();
            let target = edge_guard.target();
            let labels = edge_guard.labels().clone();
            (origin, source, target, labels)
        };

        let Some(Origin::ElkEdge(edge_id)) = origin else {
            return;
        };
        let Some(elk_edge) = self.origin_store.get_edge(edge_id) else {
            return;
        };

        if std::env::var("ELK_TRACE").is_ok() {
            let edge_id_str = {
                let mut edge_mut = elk_edge.borrow_mut();
                edge_mut.element().identifier().map(|id| id.to_string())
            };
            if edge_id_str.as_deref() == Some("e2") {
                let bend_count = ledge
                    .lock()
                    .ok()
                    .map(|edge_guard| edge_guard.bend_points_ref().len())
                    .unwrap_or(0);
                eprintln!(
                    "[transferrer] e2 apply_layout source_present={} target_present={} bendpoints={}",
                    source.is_some(),
                    target.is_some(),
                    bend_count
                );
            }
        }

        let edge_offset = offset;

        // Determine source and target nodes for the is_descendant check
        let source_node = source.as_ref().and_then(|port| {
            port.lock().ok().and_then(|port_guard| port_guard.node())
        });
        let target_node = target.as_ref().and_then(|port| {
            port.lock().ok().and_then(|port_guard| port_guard.node())
        });

        // Source point logic
        let source_is_descendant_parent = match (&target_node, &source_node) {
            (Some(tn), Some(sn)) => LGraphUtil::is_descendant(tn, sn),
            _ => false,
        };

        let start = if source_is_descendant_parent {
            // Java: sourcePoint = KVector.sum(sourcePort.getPosition(), sourcePort.getAnchor());
            // sourcePoint.sub(offset);
            // Note: uses port position + anchor (relative to node), NOT absolute anchor
            source
                .as_ref()
                .and_then(|port| {
                    let mut port_guard = port.lock().ok()?;
                    let port_pos = *port_guard.shape().position_ref();
                    let anchor = *port_guard.anchor_ref();
                    Some(KVector::with_values(
                        port_pos.x + anchor.x - offset.x,
                        port_pos.y + anchor.y - offset.y,
                    ))
                })
                .unwrap_or_else(KVector::new)
        } else {
            source
                .as_ref()
                .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.absolute_anchor()))
                .unwrap_or_else(KVector::new)
        };

        // Target point: target.absolute_anchor() + TARGET_OFFSET if present
        let mut end = target
            .as_ref()
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.absolute_anchor()))
            .unwrap_or_else(KVector::new);

        // Add TARGET_OFFSET if present
        let target_offset = ledge
            .lock()
            .ok()
            .and_then(|mut edge_guard| edge_guard.get_property(InternalProperties::TARGET_OFFSET));
        if let Some(to) = target_offset {
            end.x += to.x;
            end.y += to.y;
        }

        // Build the bend point chain: source as first, then bend_points, then target as last
        let mut bend_point_chain = {
            let edge_guard = ledge.lock().ok();
            match edge_guard {
                Some(edge_guard) => {
                    let mut chain = edge_guard.bend_points_ref().clone();
                    chain.add_first_values(start.x, start.y);
                    chain.add_last_values(end.x, end.y);
                    chain
                }
                None => {
                    let mut chain = KVectorChain::new();
                    chain.add_first_values(start.x, start.y);
                    chain.add_last_values(end.x, end.y);
                    chain
                }
            }
        };

        // Offset all bend points by edge_offset
        bend_point_chain.offset(edge_offset.x, edge_offset.y);

        // Clear existing sections
        {
            let mut edge_mut = elk_edge.borrow_mut();
            edge_mut.sections().clear();
        }

        // Get incoming/outgoing shapes
        let (incoming_shape, outgoing_shape) = {
            let edge_ref = elk_edge.borrow();
            (
                first_shape(edge_ref.sources_ro()),
                first_shape(edge_ref.targets_ro()),
            )
        };

        // Create section
        let section = ElkEdgeSection::new();
        {
            let mut section_mut = section.borrow_mut();
            section_mut.set_incoming_shape(incoming_shape);
            section_mut.set_outgoing_shape(outgoing_shape);
        }

        // Apply bend points via ElkUtil::apply_vector_chain
        ElkUtil::apply_vector_chain(&bend_point_chain, &section);

        {
            let mut edge_mut = elk_edge.borrow_mut();
            edge_mut.sections().add(section);
        }

        // Apply label positions with edge_offset (Java also sets dimensions)
        for label in &labels {
            let (label_origin, label_position, label_size, include_label) = {
                let mut label_guard = match label.lock() {
                    Ok(guard) => guard,
                    Err(_) => continue,
                };
                let origin = label_guard.get_property(InternalProperties::ORIGIN);
                let position = *label_guard.shape().position_ref();
                let size = *label_guard.shape().size_ref();
                let include_label = label_guard.get_property(&INCLUDE_LABEL);
                (origin, position, size, include_label)
            };

            let Some(Origin::ElkLabel(label_id)) = label_origin else {
                continue;
            };
            let Some(elk_label) = self.origin_store.get_label(label_id) else {
                continue;
            };

            let mut elk_label_mut = elk_label.borrow_mut();
            let shape = elk_label_mut.shape();
            shape.set_dimensions(label_size.x, label_size.y);
            shape.set_location(label_position.x + edge_offset.x, label_position.y + edge_offset.y);
            if let Some(include_label) = include_label {
                shape
                    .graph_element()
                    .properties_mut()
                    .set_property(&INCLUDE_LABEL, Some(include_label));
            } else {
                shape
                    .graph_element()
                    .properties_mut()
                    .set_property::<bool>(&INCLUDE_LABEL, None);
            }
        }

        // Copy junction points (offset them) or set to None
        let junction_points = ledge
            .lock()
            .ok()
            .and_then(|mut edge_guard| edge_guard.get_property(LayeredOptions::JUNCTION_POINTS));

        {
            let mut edge_mut = elk_edge.borrow_mut();
            if let Some(mut jp) = junction_points {
                jp.offset(edge_offset.x, edge_offset.y);
                edge_mut
                    .element()
                    .properties_mut()
                    .set_property(LayeredOptions::JUNCTION_POINTS, Some(jp));
            } else {
                edge_mut
                    .element()
                    .properties_mut()
                    .set_property::<KVectorChain>(LayeredOptions::JUNCTION_POINTS, None);
            }
        }

        // Mark edge routing: SPLINES if routing == SPLINES, else set to None
        {
            let mut edge_mut = elk_edge.borrow_mut();
            if edge_routing == EdgeRouting::Splines {
                edge_mut
                    .element()
                    .properties_mut()
                    .set_property(CoreOptions::EDGE_ROUTING, Some(EdgeRouting::Splines));
            } else {
                edge_mut
                    .element()
                    .properties_mut()
                    .set_property::<EdgeRouting>(CoreOptions::EDGE_ROUTING, None);
            }
        }
    }
}

fn collect_nodes_from_graph(
    graph: &crate::org::eclipse::elk::alg::layered::graph::LGraph,
) -> Vec<LNodeRef> {
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

fn first_shape(
    list: &org_eclipse_elk_graph::org::eclipse::elk::graph::EdgeEndpointList,
) -> Option<ElkConnectableShapeRef> {
    list.iter().next().cloned()
}
