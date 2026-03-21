use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::content_alignment::ContentAlignment;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_label_placement::PortLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_util::ElkUtil;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkEdgeSection, ElkGraphElementRef, ElkNodeRef, ElkPortRef,
};
use std::collections::VecDeque;
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
    GraphProperties, InternalProperties, LayeredOptions, NodeFlexibility,
    NodePlacementStrategy, Origin, OriginId,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

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
        let (origin, mut offset, padding) = {
            let mut graph_guard = match lgraph.lock_ok() {
            Some(guard) => guard,
            None => return,
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

        // Apply content alignment adjustments to offset (like HierarchicalNodeResizingProcessor)
        // This centers/aligns content when minimum size constraints create extra space
        self.apply_content_alignment_offset(lgraph, &mut offset);

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
        let parent_node = lgraph.lock_ok().and_then(|g| g.parent_node());
        let all_nodes = {
            let graph_guard = match lgraph.lock_ok() {
            Some(guard) => guard,
            None => return,
            };
            collect_nodes_from_graph(&graph_guard)
        };

        let elk_edges = elk_node
            .borrow_mut()
            .contained_edges()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let mut edge_list: Vec<LEdgeRef> = Vec::new();

        for lnode in &all_nodes {
            let (node_origin, node_type) = {
                let mut node_guard = match lnode.lock_ok() {
            Some(guard) => guard,
            None => continue,
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

            let ports = lnode
                .lock_ok()
                .map(|node_guard| node_guard.ports().clone())
                .unwrap_or_default();
            for port in ports {
                let outgoing_edges = port
                    .lock_ok()
                    .map(|port_guard| port_guard.outgoing_edges().clone())
                    .unwrap_or_default();
                for edge in outgoing_edges {
                    let include_edge = edge
                        .lock_ok()
                        .and_then(|edge_guard| edge_guard.target())
                        .and_then(|target_port| target_port.lock_ok().and_then(|p| p.node()))
                        .map(|target_node| !LGraphUtil::is_descendant(&target_node, lnode))
                        .unwrap_or(true);
                    if include_edge {
                        edge_list.push(edge);
                    }
                }
            }
        }

        if let Some(parent_lnode) = &parent_node {
            let ports = parent_lnode
                .lock_ok()
                .map(|node_guard| node_guard.ports().clone())
                .unwrap_or_default();
            for port in ports {
                let outgoing_edges = port
                    .lock_ok()
                    .map(|port_guard| port_guard.outgoing_edges().clone())
                    .unwrap_or_default();
                for edge in outgoing_edges {
                    let include_edge = edge
                        .lock_ok()
                        .and_then(|edge_guard| edge_guard.target())
                        .and_then(|target_port| target_port.lock_ok().and_then(|p| p.node()))
                        .map(|target_node| LGraphUtil::is_descendant(&target_node, parent_lnode))
                        .unwrap_or(false);
                    if include_edge {
                        edge_list.push(edge);
                    }
                }
            }
        }

        if ElkTrace::global().debug_edges {
            eprintln!(
                "DEBUG: collected {} ledges by traversal, {} elk_edges",
                edge_list.len(),
                elk_edges.len()
            );
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

        if ElkTrace::global().edge_apply {
            eprintln!(
                "[transferrer-graph] graph_origin={} edges={} offset=({}, {})",
                graph_id,
                edge_list.len(),
                offset.x,
                offset.y
            );
        }

        // 8. Apply edge layout to all collected edges
        for edge in &edge_list {
            let edge_id = edge
                .lock_ok()
                .and_then(|mut edge_guard| edge_guard.get_property(InternalProperties::ORIGIN))
                .and_then(|origin| match origin {
                    Origin::ElkEdge(id) => Some(id),
                    _ => None,
                });
            let Some(edge_id) = edge_id else {
                continue;
            };
            if ElkTrace::global().edge_apply {
                eprintln!(
                    "[transferrer-graph-edge] graph_origin={} edge_origin={}",
                    graph_id, edge_id
                );
            }
            self.apply_edge_layout(edge, edge_id, offset, edge_routing);
        }

        self.apply_fallback_sections(&elk_edges, &elk_node);

        // 9. Apply parent node layout
        self.apply_parent_node_layout(lgraph);

        // 10. Recursively process nested subgraphs
        for node in &all_nodes {
            let nested_graph = node
                .lock_ok()
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
            .lock_ok()
            .and_then(|mut g| g.get_property(InternalProperties::ORIGIN));
        let Some(Origin::ElkNode(graph_id)) = origin else {
            return;
        };
        let Some(elk_node) = self.origin_store.get_node(graph_id) else {
            return;
        };

        let size_constraints_included_port_labels = {
            let mut elk_node_mut = elk_node.borrow_mut();
            let shape = elk_node_mut.connectable().shape();
            let props = shape.graph_element().properties_mut();
            let size_constraints = props
                .get_property(LayeredOptions::NODE_SIZE_CONSTRAINTS)
                .unwrap_or_else(SizeConstraint::fixed);
            size_constraints.contains(&SizeConstraint::PortLabels)
        };
        let parent_node = lgraph.lock_ok().and_then(|g| g.parent_node());

        if ElkTrace::global().compound_width {
            let origin_id_str = elk_node
                .borrow_mut()
                .connectable()
                .shape()
                .graph_element()
                .identifier()
                .unwrap_or("<no-id>")
                .to_owned();
            let (graph_size, graph_offset, lpadding) = {
                let graph_guard = lgraph.lock();                let size = *graph_guard.size_ref();
                let offset = *graph_guard.offset_ref();
                let pad = graph_guard.padding_ref().clone();
                (size, offset, pad)
            };
            let actual = lgraph.lock().actual_size();
            let has_parent = parent_node.is_some();
            eprintln!(
                "[compound-width] apply_layout: node={} has_parent_node={} graph_size=({},{}) offset=({},{}) padding=({},{},{},{}) actual_size=({},{})",
                origin_id_str, has_parent, graph_size.x, graph_size.y, graph_offset.x, graph_offset.y,
                lpadding.top, lpadding.right, lpadding.bottom, lpadding.left,
                actual.x, actual.y
            );
        }

        // Java parity: parent node resize is only applied for root graphs
        // (graphs without a representing parent node). Nested graphs are
        // already resized by the layered pipeline and must not be inflated here.
        if parent_node.is_none() {
            let (graph_props, actual_graph_size) = {
                let mut graph_guard = match lgraph.lock_ok() {
            Some(guard) => guard,
            None => return,
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
                        .set_property(
                            LayeredOptions::PORT_CONSTRAINTS,
                            Some(PortConstraints::FixedPos),
                        );
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

        if ElkTrace::global().compound_width {
            let origin_id_str = elk_node
                .borrow_mut()
                .connectable()
                .shape()
                .graph_element()
                .identifier()
                .unwrap_or("<no-id>")
                .to_owned();
            let (w, h) = {
                let mut node_mut = elk_node.borrow_mut();
                let shape = node_mut.connectable().shape();
                (shape.width(), shape.height())
            };
            eprintln!(
                "[compound-width] after_resize: node={} width={} height={}",
                origin_id_str, w, h
            );
        }

        // Keep node size constraints aligned with Java transfer behavior.
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

    fn apply_node_layout(&self, lnode: &LNodeRef, offset: KVector) {
        let (
            origin,
            position,
            size,
            ports,
            labels,
            has_nested_graph,
            node_has_label_placement,
            port_labels_are_fixed,
        ) = {
            let mut node_guard = match lnode.lock_ok() {
            Some(guard) => guard,
            None => return,
            };
            let origin = node_guard.get_property(InternalProperties::ORIGIN);
            let position = *node_guard.shape().position_ref();
            let size = *node_guard.shape().size_ref();
            let ports = node_guard.ports().clone();
            let labels = node_guard.labels().clone();
            let has_nested_graph = node_guard.nested_graph().is_some();
            let node_has_label_placement = !node_guard
                .get_property(LayeredOptions::NODE_LABELS_PLACEMENT)
                .unwrap_or_else(EnumSet::none_of)
                .is_empty();
            let port_labels_are_fixed = PortLabelPlacement::is_fixed(
                &node_guard
                    .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                    .unwrap_or_else(PortLabelPlacement::outside),
            );
            (
                origin,
                position,
                size,
                ports,
                labels,
                has_nested_graph,
                node_has_label_placement,
                port_labels_are_fixed,
            )
        };

        let Some(Origin::ElkNode(node_id)) = origin else {
            return;
        };
        let Some(elk_node) = self.origin_store.get_node(node_id) else {
            return;
        };

        // Get layer_id and position_id from lnode
        let (layer_id, position_id) = {
            let mut node_guard = match lnode.lock_ok() {
            Some(guard) => guard,
            None => return,
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
        let elk_size_constraints = {
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
                .unwrap_or_else(EnumSet::none_of)
        };

        // Set position
        {
            let mut elk_node_mut = elk_node.borrow_mut();
            let shape = elk_node_mut.connectable().shape();
            shape.set_x(position.x + offset.x);
            shape.set_y(position.y + offset.y);
        }

        // Java parity: set node size when size constraints are non-empty
        // or network-simplex node flexibility allows size adaptation.
        let should_set_size = !elk_size_constraints.is_empty()
            || has_nested_graph
            || uses_network_simplex_flexible_size(lnode);
        if should_set_size {
            let mut elk_node_mut = elk_node.borrow_mut();
            let shape = elk_node_mut.connectable().shape();
            shape.set_dimensions(size.x, size.y);
        }

        // Set port positions and PORT_SIDE
        for port in &ports {
            self.apply_port_layout(port, !port_labels_are_fixed);
        }

        // Java parity: set node label positions only when node/label placement is configured.
        for label in &labels {
            let label_has_label_placement = label
                .lock_ok()
                .map(|mut label_guard| {
                    label_guard
                        .shape()
                        .graph_element()
                        .properties()
                        .has_property(LayeredOptions::NODE_LABELS_PLACEMENT)
                })
                .unwrap_or(false);
            if node_has_label_placement || label_has_label_placement {
                self.apply_label_layout_with_size(label);
            }
        }
    }

    fn apply_port_layout(&self, lport: &LPortRef, apply_labels: bool) {
        let (origin, position, size, labels, side) = {
            let mut port_guard = match lport.lock_ok() {
            Some(guard) => guard,
            None => return,
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

        if apply_labels {
            for label in labels {
                self.apply_label_layout_with_size(&label);
            }
        }
    }

    fn apply_label_layout_with_size(&self, llabel: &LLabelRef) {
        let (origin, position, size) = {
            let mut label_guard = match llabel.lock_ok() {
            Some(guard) => guard,
            None => return,
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
            let edge_guard = match ledge.lock_ok() {
            Some(guard) => guard,
            None => return,
            };
            let source = edge_guard.source();
            let target = edge_guard.target();
            let labels = edge_guard.labels().clone();
            (source, target, labels)
        };

        let Some(elk_edge) = self.origin_store.get_edge(origin_id) else {
            return;
        };

        let inside_self_loop_yo = {
            let mut edge_mut = elk_edge.borrow_mut();
            edge_mut
                .element()
                .properties_mut()
                .get_property(CoreOptions::INSIDE_SELF_LOOPS_YO)
                .unwrap_or(false)
        };
        if inside_self_loop_yo {
            let already_has_geometry = {
                let mut edge_mut = elk_edge.borrow_mut();
                if let Some(section) = edge_mut.sections().get(0) {
                    let section_ref = section.borrow();
                    section_ref.start_x() != 0.0
                        || section_ref.start_y() != 0.0
                        || section_ref.end_x() != 0.0
                        || section_ref.end_y() != 0.0
                } else {
                    false
                }
            };
            if already_has_geometry {
                return;
            }
        }

        if ElkTrace::global().trace {
            let edge_id_str = {
                let mut edge_mut = elk_edge.borrow_mut();
                edge_mut.element().identifier().map(|id| id.to_string())
            };
            if edge_id_str.as_deref() == Some("e2") {
                let bend_count = ledge
                    .lock_ok()
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
        let source_node = source
            .as_ref()
            .and_then(|port| port.lock_ok().and_then(|port_guard| port_guard.node()));
        let target_node = target
            .as_ref()
            .and_then(|port| port.lock_ok().and_then(|port_guard| port_guard.node()));

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
                    let mut port_guard = port.lock_ok()?;
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
                .and_then(|port| {
                    port.lock_ok()
                        .and_then(|port_guard| port_guard.absolute_anchor())
                })
                .unwrap_or_else(KVector::new)
        };

        // Target point: target.absolute_anchor() + TARGET_OFFSET if present
        let mut end = target
            .as_ref()
            .and_then(|port| {
                port.lock_ok()
                    .and_then(|port_guard| port_guard.absolute_anchor())
            })
            .unwrap_or_else(KVector::new);

        // Add TARGET_OFFSET if present
        let target_offset = ledge
            .lock_ok()
            .and_then(|mut edge_guard| edge_guard.get_property(InternalProperties::TARGET_OFFSET));
        if let Some(to) = target_offset {
            end.x += to.x;
            end.y += to.y;
        }

        if ElkTrace::global().edge_offsets {
            let edge_identifier = {
                let mut edge_mut = elk_edge.borrow_mut();
                edge_mut
                    .element()
                    .identifier()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "<no-edge-id>".to_string())
            };
            let target_offset_text = target_offset
                .map(|to| format!("{},{}", to.x, to.y))
                .unwrap_or_else(|| "-".to_string());
            eprintln!(
                "[transferrer-offset] origin={} edge={} base_offset=({}, {}) hier_offset=({}, {}) edge_offset=({}, {}) start=({}, {}) end=({}, {}) target_offset={}",
                origin_id,
                edge_identifier,
                offset.x,
                offset.y,
                hierarchical_offset.x,
                hierarchical_offset.y,
                edge_offset.x,
                edge_offset.y,
                start.x,
                start.y,
                end.x,
                end.y,
                target_offset_text
            );
        }

        // Build the bend point chain: source as first, then bend_points, then target as last
        let mut bend_point_chain = {
            let edge_guard = ledge.lock();
            let mut chain = edge_guard.bend_points_ref().clone();
            chain.add_first_values(start.x, start.y);
            chain.add_last_values(end.x, end.y);
            chain
        };

        // Offset all bend points by edge_offset
        bend_point_chain.offset(edge_offset.x, edge_offset.y);

        // Get incoming/outgoing shapes
        let (incoming_shape, outgoing_shape) = {
            let edge_ref = elk_edge.borrow();
            (
                first_shape(edge_ref.sources_ro()),
                first_shape(edge_ref.targets_ro()),
            )
        };

        // Mirror Java ElkGraphUtil.firstEdgeSection(elkedge, true, true):
        // reuse the first section (preserving its identifier/properties), reset geometry, and drop extras.
        let section = {
            let mut edge_mut = elk_edge.borrow_mut();
            if let Some(first_section) = edge_mut.sections().get(0) {
                {
                    let mut first_mut = first_section.borrow_mut();
                    first_mut.bend_points().clear();
                    first_mut.set_start_x(0.0);
                    first_mut.set_start_y(0.0);
                    first_mut.set_end_x(0.0);
                    first_mut.set_end_y(0.0);
                }
                edge_mut.sections().clear();
                edge_mut.sections().add(first_section.clone());
                first_section
            } else {
                let new_section = ElkEdgeSection::new();
                edge_mut.sections().add(new_section.clone());
                new_section
            }
        };

        {
            let mut section_mut = section.borrow_mut();
            section_mut.set_incoming_shape(incoming_shape);
            section_mut.set_outgoing_shape(outgoing_shape);
        }

        // Apply bend points via ElkUtil::apply_vector_chain
        ElkUtil::apply_vector_chain(&bend_point_chain, &section);

        if ElkTrace::global().edge_offsets {
            let (sx, sy, ex, ey) = {
                let section_ref = section.borrow();
                (
                    section_ref.start_x(),
                    section_ref.start_y(),
                    section_ref.end_x(),
                    section_ref.end_y(),
                )
            };
            let edge_identifier = {
                let mut edge_mut = elk_edge.borrow_mut();
                edge_mut
                    .element()
                    .identifier()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "<no-edge-id>".to_string())
            };
            eprintln!(
                "[transferrer-post-section] edge={} section_start=({}, {}) section_end=({}, {})",
                edge_identifier, sx, sy, ex, ey
            );
        }

        // Apply label positions with edge_offset (Java also sets dimensions)
        for label in &labels {
            let (label_origin, label_position, label_size, include_label) = {
                let mut label_guard = match label.lock_ok() {
            Some(guard) => guard,
            None => continue,
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
            shape.set_location(
                label_position.x + edge_offset.x,
                label_position.y + edge_offset.y,
            );
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
            .lock_ok()
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
        let target_graph = ledge.lock_ok().and_then(|mut edge_guard| {
            edge_guard.get_property(InternalProperties::COORDINATE_SYSTEM_ORIGIN)
        });
        let Some(target_graph) = target_graph else {
            return KVector::new();
        };

        let mut result = KVector::new();
        let current_graph = ledge
            .lock_ok()
            .and_then(|edge_guard| edge_guard.source())
            .and_then(|source| source.lock_ok().and_then(|port_guard| port_guard.node()))
            .and_then(|node| node.lock_ok().and_then(|node_guard| node_guard.graph()));

        let Some(mut current_graph) = current_graph else {
            return result;
        };

        let mut guard = 0usize;
        while !Arc::ptr_eq(&current_graph, &target_graph) {
            guard += 1;
            if guard > 512 {
                break;
            }

            // Java parity: when moving one hierarchy level up, add
            // `representingNode.position + parentGraph.offset + parentGraph.padding`.
            // The previous implementation used the current graph's offset/padding,
            // which over-shifts hierarchical edge sections for nested sources.
            let parent_node = {
                let graph_guard = match current_graph.lock_ok() {
            Some(guard) => guard,
            None => break,
                };
                graph_guard.parent_node()
            };
            let Some(parent_node) = parent_node else {
                break;
            };

            let (parent_pos, parent_graph) = {
                let mut node_guard = match parent_node.lock_ok() {
            Some(guard) => guard,
            None => break,
                };
                (*node_guard.shape().position_ref(), node_guard.graph())
            };
            let Some(parent_graph) = parent_graph else {
                break;
            };

            let (parent_offset, parent_padding) = {
                let parent_graph_guard = match parent_graph.lock_ok() {
            Some(guard) => guard,
            None => break,
                };
                (
                    *parent_graph_guard.offset_ref(),
                    parent_graph_guard.padding_ref().clone(),
                )
            };

            result.x += parent_pos.x + parent_offset.x + parent_padding.left;
            result.y += parent_pos.y + parent_offset.y + parent_padding.top;
            current_graph = parent_graph;
        }

        result
    }

    fn apply_fallback_sections(&self, elk_edges: &[ElkEdgeRef], container: &ElkNodeRef) {
        let container_abs =
            ElkUtil::absolute_position(&ElkGraphElementRef::Node(container.clone()))
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
            let edges = node_mut
                .contained_edges()
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            let children = node_mut.children().iter().cloned().collect::<Vec<_>>();
            (edges, children)
        };

        self.apply_fallback_sections(&edges, container);
        for child in children {
            self.apply_fallback_sections_recursive(&child);
        }
    }

    /// Applies content alignment adjustments to the offset when the actual graph size
    /// (with minimum size constraints) is larger than the calculated size.
    /// This is analogous to HierarchicalNodeResizingProcessor::resizeGraphNoReallyIMeanIt.
    fn apply_content_alignment_offset(&self, lgraph: &LGraphRef, offset: &mut KVector) {
        let (size_constraint, content_alignment, calculated_size) = {
            let graph_guard = match lgraph.lock_ok() {
            Some(guard) => guard,
            None => return,
            };
            let size_constraint = graph_guard
                .get_property_ref(LayeredOptions::NODE_SIZE_CONSTRAINTS)
                .unwrap_or_else(EnumSet::none_of);
            let content_alignment = graph_guard
                .get_property_ref(CoreOptions::CONTENT_ALIGNMENT)
                .unwrap_or_else(EnumSet::none_of);
            let calculated_size = graph_guard.actual_size();
            (size_constraint, content_alignment, calculated_size)
        };

        // Only apply content alignment if there's a minimum size constraint
        if !size_constraint.contains(&SizeConstraint::MinimumSize) {
            return;
        }

        // Get minimum size (same logic as HierarchicalNodeResizingProcessor::resize_graph)
        let mut min_size = lgraph
            .lock_ok()
            .and_then(|g| g.get_property_ref(LayeredOptions::NODE_SIZE_MINIMUM))
            .unwrap_or_default();

        let size_options = lgraph
            .lock_ok()
            .and_then(|g| g.get_property_ref(CoreOptions::NODE_SIZE_OPTIONS))
            .unwrap_or_else(EnumSet::none_of);

        if size_options.contains(&SizeOptions::DefaultMinimumSize) {
            if min_size.x <= 0.0 {
                min_size.x = ElkUtil::DEFAULT_MIN_WIDTH;
            }
            if min_size.y <= 0.0 {
                min_size.y = ElkUtil::DEFAULT_MIN_HEIGHT;
            }
        }

        let actual_size = KVector::with_values(
            calculated_size.x.max(min_size.x),
            calculated_size.y.max(min_size.y),
        );

        // Apply horizontal alignment
        if actual_size.x > calculated_size.x {
            if content_alignment.contains(&ContentAlignment::HCenter) {
                offset.x += (actual_size.x - calculated_size.x) / 2.0;
            } else if content_alignment.contains(&ContentAlignment::HRight) {
                offset.x += actual_size.x - calculated_size.x;
            }
        }

        // Apply vertical alignment
        if actual_size.y > calculated_size.y {
            if content_alignment.contains(&ContentAlignment::VCenter) {
                offset.y += (actual_size.y - calculated_size.y) / 2.0;
            } else if content_alignment.contains(&ContentAlignment::VBottom) {
                offset.y += actual_size.y - calculated_size.y;
            }
        }
    }
}

fn uses_network_simplex_flexible_size(lnode: &LNodeRef) -> bool {
    let graph_ref = lnode.lock_ok().and_then(|node_guard| node_guard.graph());
    let Some(graph_ref) = graph_ref else {
        return false;
    };

    let node_placement_strategy = graph_ref
        .lock_ok()
        .and_then(|mut graph_guard| {
            graph_guard.get_property(LayeredOptions::NODE_PLACEMENT_STRATEGY)
        })
        .unwrap_or_default();
    if node_placement_strategy != NodePlacementStrategy::NetworkSimplex {
        return false;
    }

    let node_flexibility = lnode
        .lock_ok()
        .and_then(|mut node_guard| {
            if node_guard
                .shape()
                .graph_element()
                .properties()
                .has_property(LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY)
            {
                node_guard.get_property(
                    LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY,
                )
            } else {
                None
            }
        })
        .or_else(|| {
            graph_ref.lock_ok().and_then(|mut graph_guard| {
                graph_guard.get_property(
                    LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT,
                )
            })
        })
        .or_else(|| {
            LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT.get_default()
        })
        .unwrap_or(NodeFlexibility::None);

    node_flexibility.is_flexible_size_where_space_permits()
}

fn collect_nodes_from_graph(
    graph: &crate::org::eclipse::elk::alg::layered::graph::LGraph,
) -> Vec<LNodeRef> {
    use std::collections::HashSet;
    let mut seen: HashSet<usize> = HashSet::new();
    let mut nodes = Vec::new();

    for node in graph.layerless_nodes() {
        let key = Arc::as_ptr(node) as usize;
        if seen.insert(key) {
            nodes.push(node.clone());
        }
    }

    for layer in graph.layers() {
        if let Some(layer_guard) = layer.lock_ok() {
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
                port_mut.connectable().shape().graph_element().identifier() == Some(identifier)
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
            let node_abs = ElkUtil::absolute_position(&ElkGraphElementRef::Node(node.clone()))?;
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
        (shape.x(), shape.y(), shape.width(), shape.height(), side)
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
            if on_container {
                port_x + port_w
            } else {
                port_x
            },
            port_y + port_h / 2.0,
        ),
        PortSide::East => (
            if on_container {
                port_x
            } else {
                port_x + port_w
            },
            port_y + port_h / 2.0,
        ),
        PortSide::North => (
            port_x + port_w / 2.0,
            if on_container {
                port_y + port_h
            } else {
                port_y
            },
        ),
        PortSide::South => (
            port_x + port_w / 2.0,
            if on_container {
                port_y
            } else {
                port_y + port_h
            },
        ),
        _ => (port_x + port_w / 2.0, port_y + port_h / 2.0),
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
