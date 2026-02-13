use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_util::ElkUtil;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkEdgeSection, ElkGraphElementRef, ElkNodeRef, ElkPortRef,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::sync::Arc;

use crate::org::eclipse::elk::alg::layered::graph::l_graph_util::LGraphUtil;
use crate::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use crate::org::eclipse::elk::alg::layered::graph::NodeType;
use crate::org::eclipse::elk::alg::layered::graph::{
    LEdgeRef, LGraphRef, LLabelRef, LNodeRef, LPortRef,
};
use crate::org::eclipse::elk::alg::layered::intermediate::INCLUDE_LABEL;
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions, Origin, OriginId,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::PortSide;

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

        let elk_edges = elk_node
            .borrow_mut()
            .contained_edges()
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        let ledge_by_origin = collect_edges_by_origin(&all_nodes);
        let edges: Vec<(LEdgeRef, OriginId)> = {
            let mut result = Vec::new();
            for elk_edge in &elk_edges {
                let origin_id =
                    self.origin_store
                        .get_id(&ElkGraphElementRef::Edge(elk_edge.clone()));
                let Some(origin_id) = origin_id else {
                    continue;
                };
                if let Some(ledge) = ledge_by_origin.get(&origin_id) {
                    result.push((ledge.clone(), origin_id));
                }
            }
            result
        };

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
        }

        // 6. Edges are collected from the ElkNode container via OriginStore.

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
        for (edge, edge_id) in &edges {
            self.apply_edge_layout(edge, *edge_id, offset, edge_routing);
        }

        self.apply_fallback_sections(&elk_edges, &elk_node);

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

        // 11. Ensure every contained edge has at least a fallback section
        self.apply_fallback_sections_recursive(&elk_node);
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

    fn apply_edge_layout(
        &self,
        ledge: &LEdgeRef,
        origin_id: OriginId,
        offset: KVector,
        edge_routing: EdgeRouting,
    ) {
        let (source, target, labels) = {
            let edge_guard = match ledge.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let source = edge_guard.source();
            let target = edge_guard.target();
            let labels = edge_guard.labels().clone();
            (source, target, labels)
        };

        let Some(elk_edge) = self.origin_store.get_edge(origin_id) else {
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

        let mut edge_offset = offset;
        let hierarchical_offset = self.calculate_hierarchical_offset(ledge);
        edge_offset.x += hierarchical_offset.x;
        edge_offset.y += hierarchical_offset.y;

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

    fn calculate_hierarchical_offset(&self, ledge: &LEdgeRef) -> KVector {
        let target_graph = ledge
            .lock()
            .ok()
            .and_then(|mut edge_guard| edge_guard.get_property(InternalProperties::COORDINATE_SYSTEM_ORIGIN));
        let Some(target_graph) = target_graph else {
            return KVector::new();
        };

        let mut result = KVector::new();
        let current_graph = ledge
            .lock()
            .ok()
            .and_then(|edge_guard| edge_guard.source())
            .and_then(|source| source.lock().ok().and_then(|port_guard| port_guard.node()))
            .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.graph()));

        let Some(mut current_graph) = current_graph else {
            return result;
        };

        let mut guard = 0usize;
        while !Arc::ptr_eq(&current_graph, &target_graph) {
            guard += 1;
            if guard > 512 {
                break;
            }

            let (parent_node, offset, padding) = {
                let graph_guard = match current_graph.lock() {
                    Ok(guard) => guard,
                    Err(_) => break,
                };
                (
                    graph_guard.parent_node(),
                    *graph_guard.offset_ref(),
                    graph_guard.padding_ref().clone(),
                )
            };

            let Some(parent_node) = parent_node else {
                break;
            };

            let (parent_pos, next_graph) = {
                let mut node_guard = match parent_node.lock() {
                    Ok(guard) => guard,
                    Err(_) => break,
                };
                (*node_guard.shape().position_ref(), node_guard.graph())
            };

            result.x += parent_pos.x + offset.x + padding.left;
            result.y += parent_pos.y + offset.y + padding.top;

            let Some(next_graph) = next_graph else {
                break;
            };
            current_graph = next_graph;
        }

        result
    }

    fn apply_fallback_sections(&self, elk_edges: &[ElkEdgeRef], container: &ElkNodeRef) {
        let container_abs = ElkUtil::absolute_position(&ElkGraphElementRef::Node(container.clone()))
            .unwrap_or_default();

        for edge in elk_edges {
            let has_sections = {
                let mut edge_mut = edge.borrow_mut();
                let has = edge_mut.sections().iter().next().is_some();
                has
            };
            if has_sections {
                continue;
            }

            let (incoming_shape, outgoing_shape) = {
                let edge_ref = edge.borrow();
                (
                    first_shape(edge_ref.sources_ro()),
                    first_shape(edge_ref.targets_ro()),
                )
            };

            let start = fallback_anchor_for_shape(&incoming_shape, container, &container_abs);
            let end = fallback_anchor_for_shape(&outgoing_shape, container, &container_abs);
            let (Some(start), Some(end)) = (start, end) else {
                continue;
            };

            let section = ElkEdgeSection::new();
            {
                let mut section_mut = section.borrow_mut();
                section_mut.set_incoming_shape(incoming_shape.clone());
                section_mut.set_outgoing_shape(outgoing_shape.clone());
            }

            let mut chain = KVectorChain::new();
            chain.add_first_values(start.x, start.y);
            chain.add_last_values(end.x, end.y);
            ElkUtil::apply_vector_chain(&chain, &section);

            let mut edge_mut = edge.borrow_mut();
            edge_mut.sections().clear();
            edge_mut.sections().add(section);
        }
    }

    fn apply_fallback_sections_recursive(&self, container: &ElkNodeRef) {
        let (edges, children) = {
            let mut node_mut = container.borrow_mut();
            let edges = node_mut.contained_edges().iter().cloned().collect::<Vec<_>>();
            let children = node_mut.children().iter().cloned().collect::<Vec<_>>();
            (edges, children)
        };

        self.apply_fallback_sections(&edges, container);
        for child in children {
            self.apply_fallback_sections_recursive(&child);
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

fn collect_edges_by_origin(nodes: &[LNodeRef]) -> HashMap<OriginId, LEdgeRef> {
    let mut edges: HashMap<OriginId, LEdgeRef> = HashMap::new();
    let mut seen: HashSet<usize> = HashSet::new();

    for node in nodes {
        let ports = match node.lock() {
            Ok(guard) => guard.ports().clone(),
            Err(_) => continue,
        };
        for port in ports {
            let (incoming, outgoing) = match port.lock() {
                Ok(port_guard) => (
                    port_guard.incoming_edges().clone(),
                    port_guard.outgoing_edges().clone(),
                ),
                Err(_) => continue,
            };
            for edge in incoming.iter().chain(outgoing.iter()) {
                let key = Arc::as_ptr(edge) as usize;
                if !seen.insert(key) {
                    continue;
                }
                if let Some(origin_id) = origin_id_for_edge(edge) {
                    edges.entry(origin_id).or_insert_with(|| edge.clone());
                }
            }
        }
    }

    edges
}

fn origin_id_for_edge(edge: &LEdgeRef) -> Option<OriginId> {
    let origin = edge
        .lock()
        .ok()
        .and_then(|mut guard| guard.get_property(InternalProperties::ORIGIN));
    match origin {
        Some(Origin::ElkEdge(origin_id)) => Some(origin_id),
        _ => None,
    }
}

fn resolve_port_for_fallback(container: &ElkNodeRef, port: &ElkPortRef) -> ElkPortRef {
    let identifier = {
        let mut port_mut = port.borrow_mut();
        port_mut
            .connectable()
            .shape()
            .graph_element()
            .identifier()
            .map(|value| value.to_string())
    };

    if let Some(identifier) = identifier {
        if let Some(found) = find_port_by_identifier(container, &identifier) {
            return found;
        }
    }

    port.clone()
}

fn find_port_by_identifier(graph: &ElkNodeRef, identifier: &str) -> Option<ElkPortRef> {
    let mut queue: VecDeque<ElkNodeRef> = VecDeque::new();
    queue.push_back(graph.clone());

    while let Some(node) = queue.pop_front() {
        for port in node.borrow_mut().ports().iter() {
            let matches = {
                let mut port_mut = port.borrow_mut();
                port_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    == Some(identifier)
            };
            if matches {
                return Some(port.clone());
            }
        }

        let children: Vec<_> = node.borrow_mut().children().iter().cloned().collect();
        queue.extend(children);
    }

    None
}

fn fallback_anchor_for_shape(
    shape: &Option<ElkConnectableShapeRef>,
    container: &ElkNodeRef,
    container_abs: &KVector,
) -> Option<KVector> {
    match shape {
        Some(ElkConnectableShapeRef::Port(port)) => {
            let resolved = resolve_port_for_fallback(container, port);
            fallback_anchor_for_port(&resolved, container, container_abs)
        }
        Some(ElkConnectableShapeRef::Node(node)) => {
            let (width, height) = {
                let mut node_mut = node.borrow_mut();
                let shape = node_mut.connectable().shape();
                (shape.width(), shape.height())
            };
            let node_abs =
                ElkUtil::absolute_position(&ElkGraphElementRef::Node(node.clone()))?;
            Some(KVector::with_values(
                node_abs.x + width / 2.0 - container_abs.x,
                node_abs.y + height / 2.0 - container_abs.y,
            ))
        }
        _ => None,
    }
}

fn fallback_anchor_for_port(
    port: &ElkPortRef,
    container: &ElkNodeRef,
    container_abs: &KVector,
) -> Option<KVector> {
    let parent = port.borrow().parent()?;
    let (port_x, port_y, port_w, port_h, side) = {
        let mut port_mut = port.borrow_mut();
        let shape = port_mut.connectable().shape();
        let side = shape
            .graph_element()
            .properties_mut()
            .get_property(LayeredOptions::PORT_SIDE)
            .unwrap_or(PortSide::Undefined);
        (
            shape.x(),
            shape.y(),
            shape.width(),
            shape.height(),
            side,
        )
    };

    let (node_w, _node_h) = {
        let mut node_mut = parent.borrow_mut();
        let shape = node_mut.connectable().shape();
        (shape.width(), shape.height())
    };

    let side = if side == PortSide::Undefined {
        if port_x <= 0.0 {
            PortSide::West
        } else if port_x + port_w >= node_w {
            PortSide::East
        } else if port_y <= 0.0 {
            PortSide::North
        } else {
            PortSide::South
        }
    } else {
        side
    };

    let on_container = Rc::ptr_eq(&parent, container);

    let (anchor_x, anchor_y) = match side {
        PortSide::West => (
            if on_container { port_x + port_w } else { port_x },
            port_y + port_h / 2.0,
        ),
        PortSide::East => (
            if on_container { port_x } else { port_x + port_w },
            port_y + port_h / 2.0,
        ),
        PortSide::North => (
            port_x + port_w / 2.0,
            if on_container { port_y + port_h } else { port_y },
        ),
        PortSide::South => (
            port_x + port_w / 2.0,
            if on_container { port_y } else { port_y + port_h },
        ),
        _ => (
            port_x + port_w / 2.0,
            port_y + port_h / 2.0,
        ),
    };

    let parent_abs = ElkUtil::absolute_position(&ElkGraphElementRef::Node(parent))?;
    Some(KVector::with_values(
        parent_abs.x + anchor_x - container_abs.x,
        parent_abs.y + anchor_y - container_abs.y,
    ))
}

fn first_shape(
    list: &org_eclipse_elk_graph::org::eclipse::elk::graph::EdgeEndpointList,
) -> Option<ElkConnectableShapeRef> {
    list.iter().next().cloned()
}
