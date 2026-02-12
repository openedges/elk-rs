use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::nodespacing::NodeLabelAndSizeCalculator;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, EnumSet};
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{ElkGraphAdapters, PortAdapter};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkGraphElementRef, ElkLabelRef, ElkNodeRef, ElkPortRef,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

use crate::org::eclipse::elk::alg::layered::components::ComponentOrderingStrategy;
use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LGraphUtil, LLabel, LLabelRef, LNode, LNodeRef, LPort, LPortRef,
};
use crate::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use crate::org::eclipse::elk::alg::layered::options::{
    CycleBreakingStrategy, GraphProperties, InternalProperties, LayeredOptions, LayeringStrategy,
    NodePromotionStrategy, OrderingStrategy, Origin, OriginId, PortType,
};

pub struct ElkGraphImporter<'a> {
    origin_store: &'a mut OriginStore,
    node_map: HashMap<OriginId, LNodeRef>,
    port_map: HashMap<OriginId, LPortRef>,
    label_map: HashMap<OriginId, LLabelRef>,
}

impl<'a> ElkGraphImporter<'a> {
    pub fn new(origin_store: &'a mut OriginStore) -> Self {
        ElkGraphImporter {
            origin_store,
            node_map: HashMap::new(),
            port_map: HashMap::new(),
            label_map: HashMap::new(),
        }
    }

    pub fn import_graph(&mut self, elkgraph: &ElkNodeRef) -> LGraphRef {
        let lgraph = self.create_lgraph(elkgraph);

        if self
            .graph_property(elkgraph, CoreOptions::PARTITIONING_ACTIVATE)
            .unwrap_or(false)
        {
            if let Ok(mut graph_guard) = lgraph.lock() {
                let mut graph_properties = graph_guard
                    .get_property(InternalProperties::GRAPH_PROPERTIES)
                    .unwrap_or_else(EnumSet::none_of);
                graph_properties.insert(GraphProperties::Partitions);
                graph_guard.set_property(
                    InternalProperties::GRAPH_PROPERTIES,
                    Some(graph_properties),
                );
            }
        }

        let hierarchy_handling = self
            .graph_property(elkgraph, LayeredOptions::HIERARCHY_HANDLING)
            .unwrap_or(HierarchyHandling::Inherit);

        if hierarchy_handling == HierarchyHandling::IncludeChildren {
            self.import_hierarchical_graph(elkgraph, &lgraph);
        } else {
            self.import_flat_graph(elkgraph, &lgraph);
        }

        lgraph
    }

    fn import_hierarchical_graph(&mut self, elkgraph: &ElkNodeRef, lgraph: &LGraphRef) {
        self.import_flat_graph_nodes(elkgraph, lgraph);

        let children: Vec<ElkNodeRef> = {
            let mut graph_mut = elkgraph.borrow_mut();
            graph_mut.children().iter().cloned().collect()
        };

        for child in children {
            if self.should_skip_node(&child) {
                continue;
            }
            if child.borrow().is_hierarchical() {
                let Some(lnode) = self.node_for(&child) else {
                    continue;
                };
                let nested_graph = self.create_lgraph(&child);
                if let Ok(mut nested_guard) = nested_graph.lock() {
                    nested_guard.set_parent_node(Some(lnode.clone()));
                }
                if let Ok(mut node_guard) = lnode.lock() {
                    node_guard.set_nested_graph(Some(nested_graph.clone()));
                    node_guard.set_property(InternalProperties::COMPOUND_NODE, Some(true));
                }
                self.import_hierarchical_graph(&child, &nested_graph);
            }
        }

        self.import_flat_graph_edges(elkgraph, lgraph);
    }

    fn import_flat_graph(&mut self, elkgraph: &ElkNodeRef, lgraph: &LGraphRef) {
        self.import_flat_graph_nodes(elkgraph, lgraph);
        self.import_flat_graph_edges(elkgraph, lgraph);
    }

    fn import_flat_graph_nodes(&mut self, elkgraph: &ElkNodeRef, lgraph: &LGraphRef) {
        let needs_model_order = self.needs_model_order_based_on_parent(elkgraph);
        let mut model_order_index = 0i32;
        let mut cb_group_model_orders: HashSet<i32> = HashSet::new();

        let children: Vec<ElkNodeRef> = {
            let mut graph_mut = elkgraph.borrow_mut();
            graph_mut.children().iter().cloned().collect()
        };

        for child in children {
            if self.should_skip_node(&child) {
                continue;
            }

            if self.needs_model_order(&child) {
                self.set_element_model_order_for_node(&child, model_order_index);
                model_order_index += 1;
                if self.has_graph_property(&child, LayeredOptions::GROUP_MODEL_ORDER_CYCLE_BREAKING_ID) {
                    if let Some(group_id) = self
                        .graph_property(&child, LayeredOptions::GROUP_MODEL_ORDER_CYCLE_BREAKING_ID)
                    {
                        cb_group_model_orders.insert(group_id);
                    }
                }
            }

            self.transform_node(&child, lgraph);
        }

        if let Ok(mut graph_guard) = lgraph.lock() {
            graph_guard.set_property(InternalProperties::MAX_MODEL_ORDER_NODES, Some(model_order_index));
            graph_guard.set_property(
                InternalProperties::CB_NUM_MODEL_ORDER_GROUPS,
                Some(cb_group_model_orders.len() as i32),
            );
        }
    }

    fn import_flat_graph_edges(&mut self, elkgraph: &ElkNodeRef, lgraph: &LGraphRef) {
        let needs_model_order = self.needs_model_order_based_on_parent(elkgraph);
        let edges: Vec<ElkEdgeRef> = {
            let mut graph_mut = elkgraph.borrow_mut();
            graph_mut.contained_edges().iter().cloned().collect()
        };

        let mut edge_model_order_index = 0i32;
        for edge in edges {
            if needs_model_order {
                self.set_element_model_order_for_edge(&edge, edge_model_order_index);
                edge_model_order_index += 1;
            }
            self.transform_edge(&edge, lgraph);
        }

        LGraphUtil::compute_graph_properties(lgraph);
    }

    fn should_skip_node(&self, node: &ElkNodeRef) -> bool {
        self.graph_property(node, CoreOptions::NO_LAYOUT).unwrap_or(false)
    }

    fn create_lgraph(&mut self, elkgraph: &ElkNodeRef) -> LGraphRef {
        let lgraph = LGraph::new();

        let node_label_padding = NodeLabelAndSizeCalculator::compute_inside_node_label_padding(
            &ElkGraphAdapters::adapt_single_node(elkgraph.clone()),
            Direction::Right,
        );

        let (properties, width, height, padding) = {
            let mut graph_mut = elkgraph.borrow_mut();
            let shape = graph_mut.connectable().shape();
            let width = shape.width();
            let height = shape.height();
            let mut props = shape.graph_element().properties().clone();
            let padding = props
                .get_property(CoreOptions::PADDING)
                .unwrap_or_default();
            (props, width, height, padding)
        };

        if let Ok(mut graph_guard) = lgraph.lock() {
            graph_guard
                .graph_element()
                .properties_mut()
                .copy_properties(&properties);
            let size = graph_guard.size();
            size.x = width;
            size.y = height;

            let lpadding = graph_guard.padding();
            lpadding.top = padding.top + node_label_padding.top;
            lpadding.right = padding.right + node_label_padding.right;
            lpadding.bottom = padding.bottom + node_label_padding.bottom;
            lpadding.left = padding.left + node_label_padding.left;

            let origin_id = self.origin_store.store(ElkGraphElementRef::Node(elkgraph.clone()));
            graph_guard.set_property(
                InternalProperties::ORIGIN,
                Some(Origin::ElkNode(origin_id)),
            );
            graph_guard.set_property(
                InternalProperties::GRAPH_PROPERTIES,
                Some(EnumSet::none_of()),
            );
        }

        lgraph
    }

    fn transform_node(&mut self, elknode: &ElkNodeRef, lgraph: &LGraphRef) -> Option<LNodeRef> {
        let lnode = LNode::new(lgraph);
        let origin_id = self.origin_store.store(ElkGraphElementRef::Node(elknode.clone()));

        let (mut properties, position, mut size, labels, ports, is_hierarchical) = {
            let mut node_mut = elknode.borrow_mut();
            let shape = node_mut.connectable().shape();
            let props = shape.graph_element().properties().clone();
            let position = (shape.x(), shape.y());
            let size = (shape.width(), shape.height());
            let labels: Vec<ElkLabelRef> = shape.graph_element().labels().iter().cloned().collect();
            let ports: Vec<ElkPortRef> = node_mut.ports().iter().cloned().collect();
            (props, position, size, labels, ports, node_mut.is_hierarchical())
        };

        let inside_self_loops_active = properties
            .get_property(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE)
            .unwrap_or(false);
        let minimum_size = properties
            .get_property(CoreOptions::NODE_SIZE_MINIMUM)
            .unwrap_or_default();
        if inside_self_loops_active
            && self.has_inside_self_loop_edge(elknode)
            && size.0 == 0.0
            && size.1 == 0.0
            && minimum_size.x <= 0.0
            && minimum_size.y <= 0.0
        {
            // Java parity: inside-self-loop-only nodes get a small non-zero baseline size.
            size = (4.0, 24.0);
        }

        if let Ok(mut node_guard) = lnode.lock() {
            node_guard
                .shape()
                .graph_element()
                .properties_mut()
                .copy_properties(&properties);
            let pos = node_guard.shape().position();
            pos.x = position.0;
            pos.y = position.1;
            let size_vec = node_guard.shape().size();
            size_vec.x = size.0;
            size_vec.y = size.1;
            node_guard.set_property(InternalProperties::ORIGIN, Some(Origin::ElkNode(origin_id)));
            if is_hierarchical {
                node_guard.set_property(InternalProperties::COMPOUND_NODE, Some(true));
            }

            // Explicitly transfer PARTITIONING_PARTITION property with correct type
            if let Some(partition) = self.graph_property(elknode, CoreOptions::PARTITIONING_PARTITION) {
                node_guard.set_property(CoreOptions::PARTITIONING_PARTITION, Some(partition));
            }
        }

        if let Ok(mut graph_guard) = lgraph.lock() {
            graph_guard.layerless_nodes_mut().push(lnode.clone());
        }
        self.node_map.insert(origin_id, lnode.clone());

        let assign_port_model_order = self.needs_model_order(elknode);
        for (port_index, port) in ports.into_iter().enumerate() {
            if assign_port_model_order {
                self.set_element_model_order_for_port(&port, port_index as i32);
            }
            self.transform_port(&port, &lnode, lgraph);
        }

        for label in labels {
            let llabel = self.transform_label(&label);
            if let Ok(mut node_guard) = lnode.lock() {
                node_guard.labels_mut().push(llabel);
            }
        }

        Some(lnode)
    }

    fn transform_port(
        &mut self,
        elkport: &ElkPortRef,
        lnode: &LNodeRef,
        lgraph: &LGraphRef,
    ) -> Option<LPortRef> {
        let lport = LPort::new();
        LPort::set_node(&lport, Some(lnode.clone()));
        let origin_id = self.origin_store.store(ElkGraphElementRef::Port(elkport.clone()));

        let (properties, position, size, labels, anchor) = {
            let mut port_mut = elkport.borrow_mut();
            let shape = port_mut.connectable().shape();
            let mut props = shape.graph_element().properties().clone();
            let position = (shape.x(), shape.y());
            let size = (shape.width(), shape.height());
            let labels: Vec<ElkLabelRef> = shape.graph_element().labels().iter().cloned().collect();
            let anchor = props.get_property(CoreOptions::PORT_ANCHOR);
            (props, position, size, labels, anchor)
        };

        if let Ok(mut port_guard) = lport.lock() {
            port_guard
                .shape()
                .graph_element()
                .properties_mut()
                .copy_properties(&properties);
            let pos = port_guard.shape().position();
            pos.x = position.0;
            pos.y = position.1;
            let size_vec = port_guard.shape().size();
            size_vec.x = size.0;
            size_vec.y = size.1;

            if let Some(anchor) = anchor {
                let port_anchor = port_guard.anchor();
                port_anchor.x = anchor.x;
                port_anchor.y = anchor.y;
                port_guard.set_explicitly_supplied_port_anchor(true);
            }

            let port_side = self.determine_port_side(elkport, lgraph);
            port_guard.set_side(port_side);

            port_guard.set_property(InternalProperties::ORIGIN, Some(Origin::ElkPort(origin_id)));
            if ElkGraphAdapters::adapt_single_port(elkport.clone()).has_compound_connections() {
                port_guard.set_property(InternalProperties::INSIDE_CONNECTIONS, Some(true));
            }
        }

        self.port_map.insert(origin_id, lport.clone());

        for label in labels {
            let llabel = self.transform_label(&label);
            if let Ok(mut port_guard) = lport.lock() {
                port_guard.labels_mut().push(llabel);
            }
        }

        Some(lport)
    }

    fn transform_edge(&mut self, elkedge: &ElkEdgeRef, lgraph: &LGraphRef) {
        let (sources, targets, mut properties, labels) = {
            let mut edge_mut = elkedge.borrow_mut();
            let sources: Vec<ElkConnectableShapeRef> = edge_mut.sources_ro().iter().cloned().collect();
            let targets: Vec<ElkConnectableShapeRef> = edge_mut.targets_ro().iter().cloned().collect();
            let props = edge_mut.element().properties().clone();
            let labels: Vec<ElkLabelRef> = edge_mut.element().labels().iter().cloned().collect();
            (sources, targets, props, labels)
        };

        let Some(source_shape) = sources.first() else {
            return;
        };
        let Some(target_shape) = targets.first() else {
            return;
        };

        let source_port = match self.resolve_port(source_shape, PortType::Output, lgraph) {
            Some(port) => port,
            None => return,
        };
        let target_port = match self.resolve_port(target_shape, PortType::Input, lgraph) {
            Some(port) => port,
            None => return,
        };

        let inside_self_loops = properties
            .get_property(CoreOptions::INSIDE_SELF_LOOPS_YO)
            .unwrap_or(false);
        if let (Some(source_node), Some(target_node)) = (
            ElkGraphUtil::connectable_shape_to_node(source_shape),
            ElkGraphUtil::connectable_shape_to_node(target_shape),
        ) {
            let source_inside = ElkGraphUtil::is_descendant(&target_node, &source_node)
                || (inside_self_loops && Rc::ptr_eq(&target_node, &source_node));
            let target_inside = ElkGraphUtil::is_descendant(&source_node, &target_node)
                || (inside_self_loops && Rc::ptr_eq(&source_node, &target_node));

            if source_inside {
                if let Ok(mut port_guard) = source_port.lock() {
                    port_guard.set_property(InternalProperties::INSIDE_CONNECTIONS, Some(true));
                }
            }
            if target_inside {
                if let Ok(mut port_guard) = target_port.lock() {
                    port_guard.set_property(InternalProperties::INSIDE_CONNECTIONS, Some(true));
                }
            }
        }

        let ledge = LEdge::new();
        LEdge::set_source(&ledge, Some(source_port));
        LEdge::set_target(&ledge, Some(target_port));

        if let Ok(mut edge_guard) = ledge.lock() {
            edge_guard
                .graph_element()
                .properties_mut()
                .copy_properties(&properties);
            let origin_id = self.origin_store.store(ElkGraphElementRef::Edge(elkedge.clone()));
            edge_guard.set_property(InternalProperties::ORIGIN, Some(Origin::ElkEdge(origin_id)));

            for label in labels {
                let llabel = self.transform_label(&label);
                edge_guard.labels_mut().push(llabel);
            }
        };
    }

    fn transform_label(&mut self, elklabel: &ElkLabelRef) -> LLabelRef {
        let origin_id = self.origin_store.store(ElkGraphElementRef::Label(elklabel.clone()));
        let (text, position, size, properties) = {
            let mut label_mut = elklabel.borrow_mut();
            let text = label_mut.text().to_owned();
            let shape = label_mut.shape();
            let position = (shape.x(), shape.y());
            let size = (shape.width(), shape.height());
            let props = shape.graph_element().properties().clone();
            (text, position, size, props)
        };

        let llabel = std::sync::Arc::new(std::sync::Mutex::new(LLabel::with_text(text)));
        if let Ok(mut label_guard) = llabel.lock() {
            label_guard
                .shape()
                .graph_element()
                .properties_mut()
                .copy_properties(&properties);
            let pos = label_guard.shape().position();
            pos.x = position.0;
            pos.y = position.1;
            let size_vec = label_guard.shape().size();
            size_vec.x = size.0;
            size_vec.y = size.1;
            label_guard.set_property(InternalProperties::ORIGIN, Some(Origin::ElkLabel(origin_id)));
        }

        self.label_map.insert(origin_id, llabel.clone());
        llabel
    }

    fn resolve_port(
        &mut self,
        shape: &ElkConnectableShapeRef,
        port_type: PortType,
        lgraph: &LGraphRef,
    ) -> Option<LPortRef> {
        match shape {
            ElkConnectableShapeRef::Port(port) => {
                let origin_id = self.origin_store.store(ElkGraphElementRef::Port(port.clone()));
                if let Some(existing) = self.port_map.get(&origin_id) {
                    return Some(existing.clone());
                }
                let parent = port.borrow().parent()?;
                let lnode = self.node_for(&parent)?;
                self.transform_port(port, &lnode, lgraph)
            }
            ElkConnectableShapeRef::Node(node) => {
                let lnode = self.node_for(node)?;
                Some(LGraphUtil::create_port(&lnode, None, port_type, lgraph))
            }
        }
    }

    fn node_for(&mut self, node: &ElkNodeRef) -> Option<LNodeRef> {
        let origin_id = self.origin_store.store(ElkGraphElementRef::Node(node.clone()));
        self.node_map.get(&origin_id).cloned()
    }

    fn determine_port_side(&self, elkport: &ElkPortRef, lgraph: &LGraphRef) -> PortSide {
        let direction = lgraph
            .lock()
            .ok()
            .and_then(|mut graph_guard| graph_guard.get_property(LayeredOptions::DIRECTION))
            .unwrap_or(Direction::Right);

        let port_side = self
            .graph_property(elkport, LayeredOptions::PORT_SIDE)
            .unwrap_or(PortSide::Undefined);
        let port_constraints = self
            .graph_property(elkport, LayeredOptions::PORT_CONSTRAINTS)
            .unwrap_or(PortConstraints::Undefined);

        if port_constraints.is_side_fixed() {
            if port_side != PortSide::Undefined {
                return port_side;
            }
            let calculated = ElkUtil::calc_port_side(elkport, direction);
            if calculated != PortSide::Undefined {
                return calculated;
            }
            return PortSide::from_direction(direction);
        }

        if port_side != PortSide::Undefined {
            return port_side;
        }

        let net_flow = self.net_flow(elkport);
        let default_side = PortSide::from_direction(direction);
        if net_flow >= 0 {
            default_side
        } else {
            default_side.opposed()
        }
    }

    fn net_flow(&self, elkport: &ElkPortRef) -> isize {
        let (outgoing, incoming) = {
            let mut port_mut = elkport.borrow_mut();
            let outgoing = port_mut.connectable().outgoing_edges().len() as isize;
            let incoming = port_mut.connectable().incoming_edges().len() as isize;
            (outgoing, incoming)
        };
        outgoing - incoming
    }

    fn set_element_model_order_for_node(&self, node: &ElkNodeRef, model_order: i32) {
        let mut node_mut = node.borrow_mut();
        node_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(InternalProperties::MODEL_ORDER, Some(model_order));
    }

    fn set_element_model_order_for_edge(&self, edge: &ElkEdgeRef, model_order: i32) {
        let mut edge_mut = edge.borrow_mut();
        edge_mut
            .element()
            .properties_mut()
            .set_property(InternalProperties::MODEL_ORDER, Some(model_order));
    }

    fn set_element_model_order_for_port(&self, port: &ElkPortRef, model_order: i32) {
        let mut port_mut = port.borrow_mut();
        port_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(InternalProperties::MODEL_ORDER, Some(model_order));
    }

    fn needs_model_order(&self, child: &ElkNodeRef) -> bool {
        let parent = child.borrow().parent();
        parent
            .as_ref()
            .is_some_and(|graph| self.needs_model_order_based_on_parent(graph))
            && !self
                .graph_property(child, LayeredOptions::CONSIDER_MODEL_ORDER_NO_MODEL_ORDER)
                .unwrap_or(false)
    }

    fn needs_model_order_based_on_parent(&self, elkgraph: &ElkNodeRef) -> bool {
        let cycle_breaking = self
            .graph_property(elkgraph, LayeredOptions::CYCLE_BREAKING_STRATEGY)
            .unwrap_or_default();
        let model_order_cycle_breaking = matches!(
            cycle_breaking,
            CycleBreakingStrategy::ModelOrder
                | CycleBreakingStrategy::BfsNodeOrder
                | CycleBreakingStrategy::DfsNodeOrder
                | CycleBreakingStrategy::GreedyModelOrder
                | CycleBreakingStrategy::SccConnectivity
                | CycleBreakingStrategy::SccNodeType
        );

        let layering_strategy = self
            .graph_property(elkgraph, LayeredOptions::LAYERING_STRATEGY)
            .unwrap_or_default();
        let node_promotion_strategy = self
            .graph_property(elkgraph, LayeredOptions::LAYERING_NODE_PROMOTION_STRATEGY)
            .unwrap_or_default();
        let model_order_layering = matches!(
            layering_strategy,
            LayeringStrategy::BfModelOrder | LayeringStrategy::DfModelOrder
        ) || matches!(
            node_promotion_strategy,
            NodePromotionStrategy::ModelOrderLeftToRight
                | NodePromotionStrategy::ModelOrderRightToLeft
        );

        let ordering_strategy = self
            .graph_property(elkgraph, LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY)
            .unwrap_or(OrderingStrategy::None);
        let force_node_model_order = self
            .graph_property(elkgraph, LayeredOptions::CROSSING_MINIMIZATION_FORCE_NODE_MODEL_ORDER)
            .unwrap_or(false);
        let component_ordering = self
            .graph_property(elkgraph, LayeredOptions::CONSIDER_MODEL_ORDER_COMPONENTS)
            .unwrap_or(ComponentOrderingStrategy::None);
        let node_influence = self
            .graph_property(
                elkgraph,
                LayeredOptions::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_NODE_INFLUENCE,
            )
            .unwrap_or(0.0);
        let port_influence = self
            .graph_property(
                elkgraph,
                LayeredOptions::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_PORT_INFLUENCE,
            )
            .unwrap_or(0.0);
        let model_order_crossing_minimization = ordering_strategy != OrderingStrategy::None
            || force_node_model_order
            || component_ordering != ComponentOrderingStrategy::None
            || node_influence != 0.0
            || port_influence != 0.0;

        model_order_cycle_breaking || model_order_layering || model_order_crossing_minimization
    }

    fn has_inside_self_loop_edge(&self, node: &ElkNodeRef) -> bool {
        for edge in ElkGraphUtil::all_outgoing_edges(node) {
            let is_self_loop = edge.borrow().is_selfloop();
            if !is_self_loop {
                continue;
            }
            let inside = edge
                .borrow_mut()
                .element()
                .properties_mut()
                .get_property(CoreOptions::INSIDE_SELF_LOOPS_YO)
                .unwrap_or(false);
            if inside {
                return true;
            }
        }
        false
    }

    fn graph_property<T: Clone + Send + Sync + 'static>(
        &self,
        element: &impl GraphPropertyOwner,
        property: &org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<T>,
    ) -> Option<T> {
        let mut props = element.graph_properties();
        props.get_property(property)
    }

    fn has_graph_property<T: Clone + Send + Sync + 'static>(
        &self,
        element: &impl GraphPropertyOwner,
        property: &org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<T>,
    ) -> bool {
        let props = element.graph_properties();
        props.has_property(property)
    }
}

trait GraphPropertyOwner {
    fn graph_properties(&self) -> MapPropertyHolder;
}

impl GraphPropertyOwner for ElkNodeRef {
    fn graph_properties(&self) -> MapPropertyHolder {
        let mut node_mut = self.borrow_mut();
        node_mut.connectable().shape().graph_element().properties().clone()
    }
}

impl GraphPropertyOwner for ElkPortRef {
    fn graph_properties(&self) -> MapPropertyHolder {
        let mut port_mut = self.borrow_mut();
        port_mut.connectable().shape().graph_element().properties().clone()
    }
}
