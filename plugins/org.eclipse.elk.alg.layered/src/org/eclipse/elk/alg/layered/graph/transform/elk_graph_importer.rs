use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, LazyLock};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::nodespacing::NodeLabelAndSizeCalculator;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_label_placement::PortLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{
    ElkGraphAdapters, PortAdapter,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, EnumSet};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkGraphElementRef, ElkLabelRef, ElkNodeRef, ElkPortRef,
};

use crate::org::eclipse::elk::alg::layered::components::ComponentOrderingStrategy;
use crate::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LGraphUtil, LLabel, LLabelRef, LNode, LNodeRef, LPort, LPortRef,
};
use crate::org::eclipse::elk::alg::layered::options::{
    CycleBreakingStrategy, GraphProperties, InternalProperties, LayeredOptions, LayeringStrategy,
    NodePromotionStrategy, OrderingStrategy, Origin, OriginId, PortType,
};

static TRACE_COMPOUND_WIDTH: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_COMPOUND_WIDTH").is_some());
static TRACE_IMPORT_EDGE_SELECTION: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_IMPORT_EDGE_SELECTION").is_some());
static TRACE_IMPORT_PORT_ORDER: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_IMPORT_PORT_ORDER").is_some());
static TRACE_EDGE_ORIGIN_MAP: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_EDGE_ORIGIN_MAP").is_some());
static TRACE_INSIDE_YO: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_INSIDE_YO").is_some());

pub struct ElkGraphImporter<'a> {
    origin_store: &'a mut OriginStore,
    node_map: HashMap<OriginId, LNodeRef>,
    port_map: HashMap<OriginId, LPortRef>,
    label_map: HashMap<OriginId, LLabelRef>,
    top_level_elkgraph: Option<ElkNodeRef>,
    top_level_lgraph: Option<LGraphRef>,
}

impl<'a> ElkGraphImporter<'a> {
    pub fn new(origin_store: &'a mut OriginStore) -> Self {
        ElkGraphImporter {
            origin_store,
            node_map: HashMap::new(),
            port_map: HashMap::new(),
            label_map: HashMap::new(),
            top_level_elkgraph: None,
            top_level_lgraph: None,
        }
    }

    pub fn import_graph(&mut self, elkgraph: &ElkNodeRef) -> LGraphRef {
        let lgraph = self.create_lgraph(elkgraph);
        self.top_level_elkgraph = Some(elkgraph.clone());
        self.top_level_lgraph = Some(lgraph.clone());

        if self
            .graph_property(elkgraph, CoreOptions::PARTITIONING_ACTIVATE)
            .unwrap_or(false)
        {
            if let Some(mut graph_guard) = lgraph.lock_ok() {
                let mut graph_properties = graph_guard
                    .get_property(InternalProperties::GRAPH_PROPERTIES)
                    .unwrap_or_else(EnumSet::none_of);
                graph_properties.insert(GraphProperties::Partitions);
                graph_guard
                    .set_property(InternalProperties::GRAPH_PROPERTIES, Some(graph_properties));
            }
        }

        let ports: Vec<ElkPortRef> = {
            let mut graph_mut = elkgraph.borrow_mut();
            graph_mut.ports().iter().cloned().collect()
        };
        for port in &ports {
            self.ensure_defined_port_side(&lgraph, port);
        }

        let has_external_ports = if let Some(mut graph_guard) = lgraph.lock_ok() {
            let mut graph_properties = graph_guard
                .get_property(InternalProperties::GRAPH_PROPERTIES)
                .unwrap_or_else(EnumSet::none_of);
            self.check_external_ports(elkgraph, &mut graph_properties);
            let has_external_ports = graph_properties.contains(&GraphProperties::ExternalPorts);
            graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(graph_properties));
            has_external_ports
        } else {
            false
        };
        if has_external_ports {
            for port in &ports {
                self.transform_external_port(elkgraph, &lgraph, port);
            }
        }

        if self.should_calculate_minimum_graph_size(elkgraph) {
            self.calculate_minimum_graph_size(elkgraph, &lgraph);
        }

        let hierarchy_handling = self
            .graph_property(elkgraph, LayeredOptions::HIERARCHY_HANDLING)
            .unwrap_or(HierarchyHandling::Inherit);

        if hierarchy_handling == HierarchyHandling::IncludeChildren {
            self.import_hierarchical_graph(elkgraph, &lgraph);
        } else {
            self.import_flat_graph(elkgraph, &lgraph);
        }

        if *TRACE_COMPOUND_WIDTH {
            if let Some(graph_guard) = lgraph.lock_ok() {
                let node_count = graph_guard.layerless_nodes().len();
                let edge_count: usize = graph_guard.layerless_nodes().iter().filter_map(|n| {
                    n.lock_ok().map(|ng| ng.ports().iter().filter_map(|p| {
                        p.lock_ok().map(|pg| pg.outgoing_edges().len())
                    }).sum::<usize>())
                }).sum();
                let graph_id = elkgraph.borrow_mut().connectable().shape().graph_element()
                    .identifier().unwrap_or("<no-id>").to_owned();
                eprintln!("[compound-width] after_import: graph={} nodes={} edges={}", graph_id, node_count, edge_count);
            }
        }

        lgraph
    }

    fn import_hierarchical_graph(&mut self, elkgraph: &ElkNodeRef, lgraph: &LGraphRef) {
        let parent_graph_direction = LGraphUtil::get_direction(lgraph);
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

            let has_children = {
                let mut child_ref = child.borrow_mut();
                !child_ref.children().is_empty()
            };
            let has_inside_self_loops = self.has_inside_self_loop_edge(&child);
            let child_hierarchy_handling = self
                .graph_property(&child, LayeredOptions::HIERARCHY_HANDLING)
                .unwrap_or(HierarchyHandling::Inherit);
            // Java resolves child INHERIT against the parent's INCLUDE_CHILDREN mode.
            // This importer already recurses only under INCLUDE_CHILDREN, so treat INHERIT
            // as enabled here to mirror Java nested-graph creation.
            let has_hierarchy_handling_enabled = matches!(
                child_hierarchy_handling,
                HierarchyHandling::IncludeChildren | HierarchyHandling::Inherit
            );
            let uses_elk_layered = self
                .graph_property(&child, CoreOptions::ALGORITHM)
                .map(|algorithm_id: String| {
                    LayeredOptions::ALGORITHM_ID.ends_with(algorithm_id.trim())
                })
                .unwrap_or(true);

            let mut nested_graph: Option<LGraphRef> = None;
            if uses_elk_layered
                && has_hierarchy_handling_enabled
                && (has_children || has_inside_self_loops)
            {
                let created_nested = self.create_lgraph(&child);
                if let Some(mut nested_guard) = created_nested.lock_ok() {
                    nested_guard.set_property(LayeredOptions::DIRECTION, Some(parent_graph_direction));
                }
                if self.should_calculate_minimum_graph_size(&child) {
                    let child_ports: Vec<ElkPortRef> = {
                        let mut child_mut = child.borrow_mut();
                        child_mut.ports().iter().cloned().collect()
                    };
                    for port in &child_ports {
                        self.ensure_defined_port_side(&created_nested, port);
                    }
                    self.calculate_minimum_graph_size(&child, &created_nested);
                }
                nested_graph = Some(created_nested);
            }

            let Some(lnode) = self.transform_node(&child, lgraph) else {
                continue;
            };

            if let Some(nested_graph) = nested_graph {
                if let Some(mut nested_guard) = nested_graph.lock_ok() {
                    nested_guard.set_parent_node(Some(lnode.clone()));
                }
                if let Some(mut node_guard) = lnode.lock_ok() {
                    node_guard.set_nested_graph(Some(nested_graph.clone()));
                    node_guard.set_property(InternalProperties::COMPOUND_NODE, Some(true));
                }
                self.import_hierarchical_graph(&child, &nested_graph);
            }
        }

        if let Some(mut graph_guard) = lgraph.lock_ok() {
            graph_guard.set_property(
                InternalProperties::MAX_MODEL_ORDER_NODES,
                Some(model_order_index),
            );
            graph_guard.set_property(
                InternalProperties::CB_NUM_MODEL_ORDER_GROUPS,
                Some(cb_group_model_orders.len() as i32),
            );
        }

        self.import_flat_graph_edges(elkgraph, lgraph);
    }

    fn import_flat_graph(&mut self, elkgraph: &ElkNodeRef, lgraph: &LGraphRef) {
        self.import_flat_graph_nodes(elkgraph, lgraph);
        self.import_flat_graph_edges(elkgraph, lgraph);
    }

    fn import_flat_graph_nodes(&mut self, elkgraph: &ElkNodeRef, lgraph: &LGraphRef) {
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
                if self
                    .has_graph_property(&child, LayeredOptions::GROUP_MODEL_ORDER_CYCLE_BREAKING_ID)
                {
                    if let Some(group_id) = self
                        .graph_property(&child, LayeredOptions::GROUP_MODEL_ORDER_CYCLE_BREAKING_ID)
                    {
                        cb_group_model_orders.insert(group_id);
                    }
                }
            }

            self.transform_node(&child, lgraph);
        }

        if let Some(mut graph_guard) = lgraph.lock_ok() {
            graph_guard.set_property(
                InternalProperties::MAX_MODEL_ORDER_NODES,
                Some(model_order_index),
            );
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
            let trace_edge_selection = *TRACE_IMPORT_EDGE_SELECTION;
            let edge_id_for_trace = if trace_edge_selection {
                edge.borrow_mut()
                    .element()
                    .identifier()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "<no-edge-id>".to_string())
            } else {
                String::new()
            };

            let (
                source_node,
                target_node,
                is_self_loop,
                inside_self_loop_yo,
                edge_no_layout,
                edge_has_no_layout_property,
            ) = {
                let mut edge_mut = edge.borrow_mut();
                let source_node = edge_mut
                    .sources_ro()
                    .get(0)
                    .as_ref()
                    .and_then(ElkGraphUtil::connectable_shape_to_node);
                let target_node = edge_mut
                    .targets_ro()
                    .get(0)
                    .as_ref()
                    .and_then(ElkGraphUtil::connectable_shape_to_node);
                let is_self_loop = edge_mut.is_selfloop();
                let inside_self_loop_yo = edge_mut
                    .element()
                    .properties_mut()
                    .get_property(CoreOptions::INSIDE_SELF_LOOPS_YO)
                    .unwrap_or(false);
                let edge_no_layout = edge_mut
                    .element()
                    .properties_mut()
                    .get_property(CoreOptions::NO_LAYOUT)
                    .unwrap_or(false);
                let edge_has_no_layout_property = edge_mut
                    .element()
                    .properties_mut()
                    .has_property(CoreOptions::NO_LAYOUT);
                (
                    source_node,
                    target_node,
                    is_self_loop,
                    inside_self_loop_yo,
                    edge_no_layout,
                    edge_has_no_layout_property,
                )
            };

            let (Some(source_node), Some(target_node)) = (source_node, target_node) else {
                if trace_edge_selection {
                    eprintln!(
                        "[import-edge-select] edge={} skip=missing-endpoint-node",
                        edge_id_for_trace
                    );
                }
                continue;
            };

            let source_no_layout = self
                .graph_property(&source_node, CoreOptions::NO_LAYOUT)
                .unwrap_or(false);
            let target_no_layout = self
                .graph_property(&target_node, CoreOptions::NO_LAYOUT)
                .unwrap_or(false);
            if edge_no_layout || source_no_layout || target_no_layout {
                if trace_edge_selection {
                    let edge_property_ids = edge
                        .borrow_mut()
                        .element()
                        .properties_mut()
                        .get_all_properties()
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(",");
                    eprintln!(
                        "[import-edge-select] edge={} skip=no-layout edge={} edge_has_no_layout={} inside_yo={} source={} target={} props=[{}]",
                        edge_id_for_trace,
                        edge_no_layout,
                        edge_has_no_layout_property,
                        inside_self_loop_yo,
                        source_no_layout,
                        target_no_layout,
                        edge_property_ids
                    );
                }
                continue;
            }

            let inside_self_loops_enabled = self
                .graph_property(&source_node, CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE)
                .unwrap_or(false);
            let is_inside_self_loop =
                is_self_loop && inside_self_loops_enabled && inside_self_loop_yo;

            let parent_elk_graph =
                if is_inside_self_loop || ElkGraphUtil::is_descendant(&target_node, &source_node) {
                    source_node
                } else if ElkGraphUtil::is_descendant(&source_node, &target_node) {
                    target_node
                } else {
                    elkgraph.clone()
                };

            let parent_lgraph = if let Some(parent_lnode) = self.node_for(&parent_elk_graph) {
                parent_lnode
                    .lock_ok()
                    .and_then(|node_guard| node_guard.nested_graph())
                    .unwrap_or_else(|| lgraph.clone())
            } else {
                lgraph.clone()
            };

            if trace_edge_selection {
                let has_target_graph_origin = parent_lgraph
                    .lock_ok()
                    .and_then(|mut g| g.get_property(InternalProperties::ORIGIN))
                    .is_some();
                eprintln!(
                    "[import-edge-select] edge={} inside={} has_target_graph_origin={}",
                    edge_id_for_trace, is_inside_self_loop, has_target_graph_origin
                );
            }

            self.transform_edge(&edge, &parent_lgraph);
        }

        let parent_graph = {
            let graph_ref = elkgraph.borrow();
            graph_ref.parent()
        };
        if let Some(parent_graph) = parent_graph {
            let parent_edges: Vec<ElkEdgeRef> = {
                let mut parent_graph_mut = parent_graph.borrow_mut();
                parent_graph_mut.contained_edges().iter().cloned().collect()
            };

            for edge in parent_edges {
                let (source_node, is_self_loop, inside_self_loop_yo) = {
                    let mut edge_mut = edge.borrow_mut();
                    let source_node = edge_mut
                        .sources_ro()
                        .get(0)
                        .as_ref()
                        .and_then(ElkGraphUtil::connectable_shape_to_node);
                    let is_self_loop = edge_mut.is_selfloop();
                    let inside_self_loop_yo = edge_mut
                        .element()
                        .properties_mut()
                        .get_property(CoreOptions::INSIDE_SELF_LOOPS_YO)
                        .unwrap_or(false);
                    (source_node, is_self_loop, inside_self_loop_yo)
                };

                let Some(source_node) = source_node else {
                    continue;
                };
                if !Rc::ptr_eq(&source_node, elkgraph) || !is_self_loop {
                    continue;
                }

                let inside_self_loops_enabled = self
                    .graph_property(&source_node, CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE)
                    .unwrap_or(false);
                if inside_self_loops_enabled && inside_self_loop_yo {
                    self.transform_edge(&edge, lgraph);
                }
            }
        }
    }

    fn should_skip_node(&self, node: &ElkNodeRef) -> bool {
        self.graph_property(node, CoreOptions::NO_LAYOUT)
            .unwrap_or(false)
    }

    fn should_calculate_minimum_graph_size(&self, elkgraph: &ElkNodeRef) -> bool {
        !self
            .graph_property(elkgraph, LayeredOptions::NODE_SIZE_CONSTRAINTS)
            .unwrap_or_else(EnumSet::none_of)
            .is_empty()
    }

    fn calculate_minimum_graph_size(&self, elkgraph: &ElkNodeRef, lgraph: &LGraphRef) {
        let has_parent = elkgraph.borrow().parent().is_some();
        if !has_parent {
            return;
        }

        let mut size_constraints = if let Some(graph_guard) = lgraph.lock_ok() {
            graph_guard
                .get_property_ref(LayeredOptions::NODE_SIZE_CONSTRAINTS)
                .unwrap_or_else(EnumSet::none_of)
        } else {
            return;
        };
        if size_constraints.is_empty() {
            return;
        }

        // Java parity guard: fixed port label placement on hierarchical nodes can carry
        // hand-authored absolute label coordinates. Recomputing and enforcing importer
        // minimums from those fixed coordinates over-constrains some compounds.
        if PortLabelPlacement::is_fixed(
            &self
                .graph_property(elkgraph, CoreOptions::PORT_LABELS_PLACEMENT)
                .unwrap_or_default(),
        ) {
            return;
        }

        if self
            .graph_property(elkgraph, LayeredOptions::PORT_CONSTRAINTS)
            .unwrap_or(PortConstraints::Undefined)
            == PortConstraints::Undefined
        {
            let mut node_mut = elkgraph.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .set_property(
                    LayeredOptions::PORT_CONSTRAINTS,
                    Some(PortConstraints::Free),
                );
        }

        let mut min_size = if let Some(graph_guard) = lgraph.lock_ok() {
            graph_guard
                .get_property_ref(LayeredOptions::NODE_SIZE_MINIMUM)
                .unwrap_or_default()
        } else {
            return;
        };

        // Match Java's importer path:
        // NodeLabelAndSizeCalculator.process(graphAdapter, nodeAdapter, false, true)
        // i.e. compute-only and ignore inside port labels for hierarchical minima.
        let layout_direction = self
            .graph_property(elkgraph, CoreOptions::DIRECTION)
            .unwrap_or(Direction::Right);
        let node_adapter = ElkGraphAdapters::adapt_single_node(elkgraph.clone());
        let calculated_min = NodeLabelAndSizeCalculator::process_with_options(
            &node_adapter,
            layout_direction,
            false,
            true,
        );
        min_size.x = min_size.x.max(calculated_min.x);
        min_size.y = min_size.y.max(calculated_min.y);

        size_constraints.insert(SizeConstraint::MinimumSize);

        if let Some(mut graph_guard) = lgraph.lock_ok() {
            graph_guard.set_property(
                LayeredOptions::NODE_SIZE_CONSTRAINTS,
                Some(size_constraints),
            );
            graph_guard.set_property(LayeredOptions::NODE_SIZE_MINIMUM, Some(min_size));
        }
    }

    fn create_lgraph(&mut self, elkgraph: &ElkNodeRef) -> LGraphRef {
        let lgraph = LGraph::new();

        let node_label_padding = NodeLabelAndSizeCalculator::compute_inside_node_label_padding(
            &ElkGraphAdapters::adapt_single_node(elkgraph.clone()),
            Direction::Right,
        );

        let (properties, padding) = {
            let mut graph_mut = elkgraph.borrow_mut();
            let shape = graph_mut.connectable().shape();
            let mut props = shape.graph_element().properties().clone();
            let padding = props.get_property(CoreOptions::PADDING).unwrap_or_default();
            (props, padding)
        };

        if let Some(mut graph_guard) = lgraph.lock_ok() {
            graph_guard
                .graph_element()
                .properties_mut()
                .copy_properties(&properties);

            let lpadding = graph_guard.padding();
            lpadding.top = padding.top + node_label_padding.top;
            lpadding.right = padding.right + node_label_padding.right;
            lpadding.bottom = padding.bottom + node_label_padding.bottom;
            lpadding.left = padding.left + node_label_padding.left;

            if *TRACE_COMPOUND_WIDTH {
                let graph_id = elkgraph
                    .borrow_mut()
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .unwrap_or("<no-id>")
                    .to_owned();
                eprintln!(
                    "[compound-width] create_lgraph: graph={} elk_padding=({},{},{},{}) node_label_padding=({},{},{},{}) lpadding=({},{},{},{})",
                    graph_id,
                    padding.top, padding.right, padding.bottom, padding.left,
                    node_label_padding.top, node_label_padding.right, node_label_padding.bottom, node_label_padding.left,
                    lpadding.top, lpadding.right, lpadding.bottom, lpadding.left,
                );
            }

            let origin_id = self
                .origin_store
                .store(ElkGraphElementRef::Node(elkgraph.clone()));
            graph_guard.set_property(InternalProperties::ORIGIN, Some(Origin::ElkNode(origin_id)));
            graph_guard.set_property(
                InternalProperties::GRAPH_PROPERTIES,
                Some(EnumSet::none_of()),
            );
        }

        lgraph
    }

    fn transform_node(&mut self, elknode: &ElkNodeRef, lgraph: &LGraphRef) -> Option<LNodeRef> {
        let lnode = LNode::new(lgraph);
        let origin_id = self
            .origin_store
            .store(ElkGraphElementRef::Node(elknode.clone()));

        let (properties, position, size, labels, ports, node_is_hierarchical) = {
            let mut node_mut = elknode.borrow_mut();
            let shape = node_mut.connectable().shape();
            let props = shape.graph_element().properties().clone();
            let position = (shape.x(), shape.y());
            let size = (shape.width(), shape.height());
            let labels: Vec<ElkLabelRef> = shape.graph_element().labels().iter().cloned().collect();
            let ports: Vec<ElkPortRef> = node_mut.ports().iter().cloned().collect();
            (
                props,
                position,
                size,
                labels,
                ports,
                node_mut.is_hierarchical(),
            )
        };
        let is_hierarchical = node_is_hierarchical || self.has_inside_self_loop_edge(elknode);

        if *TRACE_IMPORT_PORT_ORDER {
            let node_id = elknode
                .borrow_mut()
                .connectable()
                .shape()
                .graph_element()
                .identifier()
                .unwrap_or("<no-node-id>")
                .to_owned();
            let port_ids = ports
                .iter()
                .map(|port| {
                    port.borrow_mut()
                        .connectable()
                        .shape()
                        .graph_element()
                        .identifier()
                        .unwrap_or("<no-port-id>")
                        .to_owned()
                })
                .collect::<Vec<_>>()
                .join(", ");
            eprintln!(
                "rust-import-port-order: node={} ports=[{}]",
                node_id, port_ids
            );
        }

        if let Some(mut node_guard) = lnode.lock_ok() {
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
            if let Some(partition) =
                self.graph_property(elknode, CoreOptions::PARTITIONING_PARTITION)
            {
                node_guard.set_property(CoreOptions::PARTITIONING_PARTITION, Some(partition));
            }
        }

        if let Some(mut graph_guard) = lgraph.lock_ok() {
            graph_guard.layerless_nodes_mut().push(lnode.clone());
        }
        self.node_map.insert(origin_id, lnode.clone());

        let direction = LGraphUtil::get_direction(lgraph);
        let mut graph_properties = lgraph
            .lock_ok()
            .and_then(|mut graph_guard| graph_guard.get_property(InternalProperties::GRAPH_PROPERTIES))
            .unwrap_or_else(EnumSet::none_of);

        let mut port_constraints = lnode
            .lock_ok()
            .and_then(|mut node_guard| node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS))
            .unwrap_or(PortConstraints::Undefined);
        if port_constraints == PortConstraints::Undefined {
            if let Some(mut node_guard) = lnode.lock_ok() {
                node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::Free));
            }
            port_constraints = PortConstraints::Free;
        } else if port_constraints != PortConstraints::Free {
            graph_properties.insert(GraphProperties::NonFreePorts);
        }

        let assign_port_model_order = self.needs_model_order(elknode);
        for (port_index, port) in ports.into_iter().enumerate() {
            if assign_port_model_order {
                self.set_element_model_order_for_port(&port, port_index as i32);
            }
            if self
                .graph_property(&port, CoreOptions::NO_LAYOUT)
                    .unwrap_or(false)
            {
                continue;
            }
            self.transform_port(
                &port,
                &lnode,
                &mut graph_properties,
                direction,
                port_constraints,
            );
        }

        for label in labels {
            let (no_layout, text) = {
                let mut label_mut = label.borrow_mut();
                let no_layout = label_mut
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .get_property(CoreOptions::NO_LAYOUT)
                    .unwrap_or(false);
                let text = label_mut.text().to_owned();
                (no_layout, text)
            };
            if no_layout || text.is_empty() {
                continue;
            }
            let llabel = self.transform_label(&label);
            if let Some(mut node_guard) = lnode.lock_ok() {
                node_guard.labels_mut().push(llabel);
            }
        }

        if lnode
            .lock_ok()
            .and_then(|mut node_guard| node_guard.get_property(LayeredOptions::COMMENT_BOX))
            .unwrap_or(false)
        {
            graph_properties.insert(GraphProperties::Comments);
        }

        if lnode
            .lock_ok()
            .and_then(|mut node_guard| node_guard.get_property(LayeredOptions::HYPERNODE))
            .unwrap_or(false)
        {
            graph_properties.insert(GraphProperties::Hypernodes);
            graph_properties.insert(GraphProperties::Hyperedges);
            if let Some(mut node_guard) = lnode.lock_ok() {
                node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::Free));
            }
        }

        if let Some(mut graph_guard) = lgraph.lock_ok() {
            graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(graph_properties));
        }

        Some(lnode)
    }

    fn transform_port(
        &mut self,
        elkport: &ElkPortRef,
        lnode: &LNodeRef,
        graph_properties: &mut EnumSet<GraphProperties>,
        layout_direction: Direction,
        port_constraints: PortConstraints,
    ) -> Option<LPortRef> {
        let lport = LPort::new();
        LPort::set_node(&lport, Some(lnode.clone()));
        let origin_id = self
            .origin_store
            .store(ElkGraphElementRef::Port(elkport.clone()));

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

        if let Some(mut port_guard) = lport.lock_ok() {
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

            // Java parity: non-external ports keep the model-provided side (possibly Undefined)
            // and rely on LGraphUtil.initializePort / later processors for further resolution.
            let port_side = self
                .graph_property(elkport, LayeredOptions::PORT_SIDE)
                .unwrap_or(PortSide::Undefined);
            port_guard.set_side(port_side);

            port_guard.set_property(InternalProperties::ORIGIN, Some(Origin::ElkPort(origin_id)));
            if ElkGraphAdapters::adapt_single_port(elkport.clone()).has_compound_connections() {
                port_guard.set_property(InternalProperties::INSIDE_CONNECTIONS, Some(true));
            }
        }

        // Initialize port side, border offset, ratio, and anchor (Java: LGraphUtil.initializePort)
        LGraphUtil::initialize_port(&lport, port_constraints, layout_direction, anchor);

        let port_side = lport
            .lock_ok()
            .map(|port_guard| port_guard.side())
            .unwrap_or(PortSide::Undefined);
        match layout_direction {
            Direction::Left | Direction::Right => {
                if port_side == PortSide::North || port_side == PortSide::South {
                    graph_properties.insert(GraphProperties::NorthSouthPorts);
                }
            }
            Direction::Up | Direction::Down => {
                if port_side == PortSide::East || port_side == PortSide::West {
                    graph_properties.insert(GraphProperties::NorthSouthPorts);
                }
            }
            _ => {}
        }

        self.port_map.insert(origin_id, lport.clone());

        for label in labels {
            let (no_layout, text) = {
                let mut label_mut = label.borrow_mut();
                let no_layout = label_mut
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .get_property(CoreOptions::NO_LAYOUT)
                    .unwrap_or(false);
                let text = label_mut.text().to_owned();
                (no_layout, text)
            };
            if no_layout || text.is_empty() {
                continue;
            }
            let llabel = self.transform_label(&label);
            if let Some(mut port_guard) = lport.lock_ok() {
                port_guard.labels_mut().push(llabel);
            }
        }
        Some(lport)
    }

    fn transform_edge(&mut self, elkedge: &ElkEdgeRef, lgraph: &LGraphRef) {
        let (sources, targets, mut properties, labels, edge_id) = {
            let mut edge_mut = elkedge.borrow_mut();
            let sources: Vec<ElkConnectableShapeRef> =
                edge_mut.sources_ro().iter().cloned().collect();
            let targets: Vec<ElkConnectableShapeRef> =
                edge_mut.targets_ro().iter().cloned().collect();
            let props = edge_mut.element().properties().clone();
            let labels: Vec<ElkLabelRef> = edge_mut.element().labels().iter().cloned().collect();
            let edge_id = edge_mut
                .element()
                .identifier()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "<no-edge-id>".to_string());
            (sources, targets, props, labels, edge_id)
        };
        let trace_edge_origin_map = *TRACE_EDGE_ORIGIN_MAP;

        let Some(source_shape) = sources.first() else {
            return;
        };
        let Some(target_shape) = targets.first() else {
            return;
        };

        let source_port = match self.resolve_port(source_shape, PortType::Output, lgraph) {
            Some(port) => port,
            None => {
                // Cross-hierarchy edge: source not in current layout scope.
                // Java throws UnsupportedGraphException; skip gracefully in Rust.
                return;
            }
        };
        let target_port = match self.resolve_port(target_shape, PortType::Input, lgraph) {
            Some(port) => port,
            None => {
                // Cross-hierarchy edge: target not in current layout scope.
                // Java throws UnsupportedGraphException; skip gracefully in Rust.
                return;
            }
        };

        let inside_self_loops = properties
            .get_property(CoreOptions::INSIDE_SELF_LOOPS_YO)
            .unwrap_or(false);
        if *TRACE_INSIDE_YO {
            let has_inside_yo = properties.has_property(CoreOptions::INSIDE_SELF_LOOPS_YO);
            eprintln!(
                "[import-edge] edge={} has_inside_yo={} inside_yo={}",
                edge_id, has_inside_yo, inside_self_loops
            );
        }
        if let (Some(source_node), Some(target_node)) = (
            ElkGraphUtil::connectable_shape_to_node(source_shape),
            ElkGraphUtil::connectable_shape_to_node(target_shape),
        ) {
            let source_inside = ElkGraphUtil::is_descendant(&target_node, &source_node)
                || (inside_self_loops && Rc::ptr_eq(&target_node, &source_node));
            let target_inside = ElkGraphUtil::is_descendant(&source_node, &target_node)
                || (inside_self_loops && Rc::ptr_eq(&source_node, &target_node));

            if source_inside {
                if let Some(mut port_guard) = source_port.lock_ok() {
                    port_guard.set_property(InternalProperties::INSIDE_CONNECTIONS, Some(true));
                }
            }
            if target_inside {
                if let Some(mut port_guard) = target_port.lock_ok() {
                    port_guard.set_property(InternalProperties::INSIDE_CONNECTIONS, Some(true));
                }
            }
        }

        let ledge = LEdge::new();
        LEdge::set_source(&ledge, Some(source_port));
        LEdge::set_target(&ledge, Some(target_port));

        let mut graph_properties = lgraph
            .lock_ok()
            .and_then(|mut graph_guard| graph_guard.get_property(InternalProperties::GRAPH_PROPERTIES))
            .unwrap_or_else(EnumSet::none_of);

        // Java parity: self loops and hyperedges are tracked while importing edges.
        let is_self_loop = ledge
            .lock_ok()
            .is_some_and(|edge_guard| edge_guard.is_self_loop());
        if is_self_loop {
            graph_properties.insert(GraphProperties::SelfLoops);
        }

        let is_hyperedge = {
            let source_counts = ledge.lock_ok().and_then(|edge_guard| {
                edge_guard.source().and_then(|source| {
                    source
                        .lock_ok()
                        .map(|port_guard| (port_guard.incoming_edges().len(), port_guard.outgoing_edges().len()))
                })
            });
            let target_counts = ledge.lock_ok().and_then(|edge_guard| {
                edge_guard.target().and_then(|target| {
                    target
                        .lock_ok()
                        .map(|port_guard| (port_guard.incoming_edges().len(), port_guard.outgoing_edges().len()))
                })
            });
            source_counts
                .is_some_and(|(inc, out)| inc > 1 || out > 1)
                || target_counts.is_some_and(|(inc, out)| inc > 1 || out > 1)
        };
        if is_hyperedge {
            graph_properties.insert(GraphProperties::Hyperedges);
        }

        if let Some(mut edge_guard) = ledge.lock_ok() {
            edge_guard
                .graph_element()
                .properties_mut()
                .copy_properties(&properties);
            edge_guard
                .graph_element()
                .properties_mut()
                .set_property(CoreOptions::NO_LAYOUT, Some(false));
            let origin_id = self
                .origin_store
                .store(ElkGraphElementRef::Edge(elkedge.clone()));
            edge_guard.set_property(InternalProperties::ORIGIN, Some(Origin::ElkEdge(origin_id)));
            self.origin_store.register_ledge(origin_id, ledge.clone());

            if trace_edge_origin_map {
                let source_id = Self::connectable_shape_identifier(source_shape);
                let target_id = Self::connectable_shape_identifier(target_shape);
                eprintln!(
                    "[edge-origin-map] edge_origin={} edge_id={} source_shape={} target_shape={}",
                    origin_id, edge_id, source_id, target_id
                );
            }

            for label in labels {
                let (no_layout, text) = {
                    let mut label_mut = label.borrow_mut();
                    let no_layout = label_mut
                        .shape()
                        .graph_element()
                        .properties_mut()
                        .get_property(CoreOptions::NO_LAYOUT)
                        .unwrap_or(false);
                    let text = label_mut.text().to_owned();
                    (no_layout, text)
                };
                if no_layout || text.is_empty() {
                    continue;
                }

                let llabel = self.transform_label(&label);
                let placement = llabel
                    .lock_ok()
                    .and_then(|mut label_guard| {
                        label_guard.get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
                    })
                    .unwrap_or(EdgeLabelPlacement::Center);
                match placement {
                    EdgeLabelPlacement::Head | EdgeLabelPlacement::Tail => {
                        graph_properties.insert(GraphProperties::EndLabels);
                    }
                    EdgeLabelPlacement::Center => {
                        graph_properties.insert(GraphProperties::CenterLabels);
                        if let Some(mut label_guard) = llabel.lock_ok() {
                            label_guard.set_property(
                                LayeredOptions::EDGE_LABELS_PLACEMENT,
                                Some(EdgeLabelPlacement::Center),
                            );
                        }
                    }
                }
                edge_guard.labels_mut().push(llabel);
            }

            if let (Some(top_level_elkgraph), Some(top_level_lgraph)) =
                (&self.top_level_elkgraph, &self.top_level_lgraph)
            {
                let coord_origin = self.find_coordinate_system_origin(
                    elkedge,
                    top_level_elkgraph,
                    top_level_lgraph,
                );
                edge_guard.set_property(InternalProperties::COORDINATE_SYSTEM_ORIGIN, coord_origin);
            }
        };

        if let Some(mut graph_guard) = lgraph.lock_ok() {
            graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(graph_properties));
        }
    }

    fn connectable_shape_identifier(shape: &ElkConnectableShapeRef) -> String {
        match shape {
            ElkConnectableShapeRef::Node(node) => node
                .borrow_mut()
                .connectable()
                .shape()
                .graph_element()
                .identifier()
                .unwrap_or("<no-node-id>")
                .to_owned(),
            ElkConnectableShapeRef::Port(port) => port
                .borrow_mut()
                .connectable()
                .shape()
                .graph_element()
                .identifier()
                .unwrap_or("<no-port-id>")
                .to_owned(),
        }
    }

    fn find_coordinate_system_origin(
        &self,
        elkedge: &ElkEdgeRef,
        top_level_elkgraph: &ElkNodeRef,
        top_level_lgraph: &LGraphRef,
    ) -> Option<LGraphRef> {
        let (sources, targets, containing_node) = {
            let edge_ref = elkedge.borrow();
            let sources = edge_ref.sources_ro().iter().cloned().collect::<Vec<_>>();
            let targets = edge_ref.targets_ro().iter().cloned().collect::<Vec<_>>();
            let containing_node = edge_ref.containing_node();
            (sources, targets, containing_node)
        };

        let source = sources
            .first()
            .and_then(ElkGraphUtil::connectable_shape_to_node)?;
        let target = targets
            .first()
            .and_then(ElkGraphUtil::connectable_shape_to_node)?;

        let source_parent = source.borrow().parent();
        let target_parent = target.borrow().parent();
        match (source_parent, target_parent) {
            (Some(sp), Some(tp)) if Rc::ptr_eq(&sp, &tp) => return None,
            (None, None) => return None,
            _ => {}
        }

        if ElkGraphUtil::is_descendant(&target, &source) {
            return None;
        }

        let origin = containing_node?;
        if Rc::ptr_eq(&origin, top_level_elkgraph) {
            return Some(top_level_lgraph.clone());
        }

        let origin_id = self
            .origin_store
            .get_id(&ElkGraphElementRef::Node(origin.clone()))?;
        let lnode = self.node_map.get(&origin_id)?;
        lnode
            .lock_ok()
            .and_then(|node_guard| node_guard.nested_graph())
    }

    fn transform_label(&mut self, elklabel: &ElkLabelRef) -> LLabelRef {
        let origin_id = self
            .origin_store
            .store(ElkGraphElementRef::Label(elklabel.clone()));
        let (text, position, size, properties) = {
            let mut label_mut = elklabel.borrow_mut();
            let text = label_mut.text().to_owned();
            let shape = label_mut.shape();
            let position = (shape.x(), shape.y());
            let size = (shape.width(), shape.height());
            let props = shape.graph_element().properties().clone();
            (text, position, size, props)
        };

        let llabel = Arc::new(Mutex::new(LLabel::with_text(text)));
        if let Some(mut label_guard) = llabel.lock_ok() {
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
            label_guard.set_property(
                InternalProperties::ORIGIN,
                Some(Origin::ElkLabel(origin_id)),
            );
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
                let origin_id = self
                    .origin_store
                    .store(ElkGraphElementRef::Port(port.clone()));
                if let Some(existing) = self.port_map.get(&origin_id) {
                    return Some(existing.clone());
                }
                let parent = port.borrow().parent()?;
                let lnode = match self.node_for(&parent) {
                    Some(n) => n,
                    None => {
                        if *TRACE_COMPOUND_WIDTH {
                            let parent_id = parent
                                .borrow_mut()
                                .connectable()
                                .shape()
                                .graph_element()
                                .identifier()
                                .unwrap_or("<no-id>")
                                .to_owned();
                            let port_id = port
                                .borrow_mut()
                                .connectable()
                                .shape()
                                .graph_element()
                                .identifier()
                                .unwrap_or("<no-port-id>")
                                .to_owned();
                            eprintln!(
                                "[compound-width] resolve_port FAILED: port={} parent_node={} not in node_map",
                                port_id, parent_id
                            );
                        }
                        return None;
                    }
                };
                let direction = LGraphUtil::get_direction(lgraph);
                let mut graph_properties = lgraph
                    .lock_ok()
                    .and_then(|mut graph_guard| {
                        graph_guard.get_property(InternalProperties::GRAPH_PROPERTIES)
                    })
                    .unwrap_or_else(EnumSet::none_of);
                let mut port_constraints = lnode
                    .lock_ok()
                    .and_then(|mut node_guard| node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS))
                    .unwrap_or(PortConstraints::Undefined);
                if port_constraints == PortConstraints::Undefined {
                    if let Some(mut node_guard) = lnode.lock_ok() {
                        node_guard
                            .set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::Free));
                    }
                    port_constraints = PortConstraints::Free;
                } else if port_constraints != PortConstraints::Free {
                    graph_properties.insert(GraphProperties::NonFreePorts);
                }

                let result = self.transform_port(
                    port,
                    &lnode,
                    &mut graph_properties,
                    direction,
                    port_constraints,
                );

                if let Some(mut graph_guard) = lgraph.lock_ok() {
                    graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(graph_properties));
                }

                result
            }
            ElkConnectableShapeRef::Node(node) => {
                let lnode = self.node_for(node)?;
                // Java parity:
                // - source endpoint creation uses the current edge-containing graph (`lgraph`)
                // - target endpoint creation uses `targetLNode.getGraph()`
                // Here, `transform_edge` always resolves target with `PortType::Input`.
                let graph_for_port = if port_type == PortType::Input {
                    lnode
                        .lock_ok()
                        .and_then(|node_guard| node_guard.graph())
                        .unwrap_or_else(|| lgraph.clone())
                } else {
                    lgraph.clone()
                };
                Some(LGraphUtil::create_port(
                    &lnode,
                    None,
                    port_type,
                    &graph_for_port,
                ))
            }
        }
    }

    fn node_for(&mut self, node: &ElkNodeRef) -> Option<LNodeRef> {
        let origin_id = self
            .origin_store
            .store(ElkGraphElementRef::Node(node.clone()));
        self.node_map.get(&origin_id).cloned()
    }

    fn ensure_defined_port_side(&self, lgraph: &LGraphRef, elkport: &ElkPortRef) {
        let direction = LGraphUtil::get_direction(lgraph);
        let port_constraints = lgraph
            .lock_ok()
            .and_then(|mut graph_guard| graph_guard.get_property(LayeredOptions::PORT_CONSTRAINTS))
            .unwrap_or(PortConstraints::Undefined);

        let mut port_side = self
            .graph_property(elkport, LayeredOptions::PORT_SIDE)
            .unwrap_or(PortSide::Undefined);

        if !port_constraints.is_side_fixed() {
            let net_flow = self.calculate_external_port_net_flow(elkport);
            if net_flow > 0 {
                port_side = PortSide::from_direction(direction);
            } else {
                port_side = PortSide::from_direction(direction).opposed();
            }
        } else if port_side == PortSide::Undefined {
            let calculated = ElkUtil::calc_port_side(elkport, direction);
            if calculated != PortSide::Undefined {
                port_side = calculated;
            } else {
                port_side = PortSide::from_direction(direction);
            }
        }

        let mut port_mut = elkport.borrow_mut();
        port_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(LayeredOptions::PORT_SIDE, Some(port_side));
    }

    fn check_external_ports(
        &self,
        elkgraph: &ElkNodeRef,
        graph_properties: &mut EnumSet<GraphProperties>,
    ) {
        let enable_self_loops = self
            .graph_property(elkgraph, CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE)
            .unwrap_or(false);
        let port_label_placement = self
            .graph_property(elkgraph, CoreOptions::PORT_LABELS_PLACEMENT)
            .unwrap_or_else(PortLabelPlacement::outside);

        let ports: Vec<ElkPortRef> = {
            let mut graph_mut = elkgraph.borrow_mut();
            graph_mut.ports().iter().cloned().collect()
        };

        let mut has_external_ports = false;
        let mut has_hyperedges = false;

        for port in ports {
            if has_external_ports && has_hyperedges {
                break;
            }

            let mut external_port_edges = 0;
            let incident_edges = ElkGraphUtil::all_incident_edges_for_shape(
                &ElkConnectableShapeRef::Port(port.clone()),
            );
            for edge in incident_edges {
                let (
                    port_is_source,
                    port_is_target,
                    source_shape,
                    target_shape,
                    is_self_loop,
                    inside_loop,
                ) = {
                    let mut edge_mut = edge.borrow_mut();
                    let port_is_source = edge_mut.sources_ro().iter().any(|shape| {
                        matches!(shape, ElkConnectableShapeRef::Port(p) if Rc::ptr_eq(p, &port))
                    });
                    let port_is_target = edge_mut.targets_ro().iter().any(|shape| {
                        matches!(shape, ElkConnectableShapeRef::Port(p) if Rc::ptr_eq(p, &port))
                    });
                    let source_shape = edge_mut.sources_ro().get(0);
                    let target_shape = edge_mut.targets_ro().get(0);
                    let is_self_loop = edge_mut.is_selfloop();
                    let inside_loop = edge_mut
                        .element()
                        .properties_mut()
                        .get_property(CoreOptions::INSIDE_SELF_LOOPS_YO)
                        .unwrap_or(false);
                    (
                        port_is_source,
                        port_is_target,
                        source_shape,
                        target_shape,
                        is_self_loop,
                        inside_loop,
                    )
                };

                let is_inside_self_loop = enable_self_loops && is_self_loop && inside_loop;
                let connects_to_child = if port_is_source {
                    target_shape
                        .and_then(|shape| ElkGraphUtil::connectable_shape_to_node(&shape))
                        .and_then(|node| node.borrow().parent())
                        .is_some_and(|parent| Rc::ptr_eq(&parent, elkgraph))
                } else if port_is_target {
                    source_shape
                        .and_then(|shape| ElkGraphUtil::connectable_shape_to_node(&shape))
                        .and_then(|node| node.borrow().parent())
                        .is_some_and(|parent| Rc::ptr_eq(&parent, elkgraph))
                } else {
                    false
                };

                if is_inside_self_loop || connects_to_child {
                    external_port_edges += 1;
                    if external_port_edges > 1 {
                        break;
                    }
                }
            }

            if external_port_edges > 0 {
                has_external_ports = true;
            } else {
                let label_count = port
                    .borrow_mut()
                    .connectable()
                    .shape()
                    .graph_element()
                    .labels()
                    .len();
                if port_label_placement.contains(&PortLabelPlacement::Inside) && label_count > 0 {
                    has_external_ports = true;
                }
            }

            if external_port_edges > 1 {
                has_hyperedges = true;
            }
        }

        // Treat edgeless ports as external ports when the root graph has
        // portConstraints >= FIXED_SIDE.  Without this, root-level ports
        // that have no edges to internal nodes are never converted to
        // external-port dummies and therefore never positioned by the
        // layered pipeline.  Only apply at the root level — nested compound
        // nodes use regular port placement for edgeless ports.
        let is_root = elkgraph.borrow().parent().is_none();
        if is_root && !has_external_ports {
            let has_any_ports = {
                let mut graph_mut = elkgraph.borrow_mut();
                !graph_mut.ports().is_empty()
            };
            if has_any_ports {
                let port_constraints = {
                    let mut graph_mut = elkgraph.borrow_mut();
                    graph_mut
                        .connectable()
                        .shape()
                        .graph_element()
                        .properties_mut()
                        .get_property(CoreOptions::PORT_CONSTRAINTS)
                        .unwrap_or(PortConstraints::Undefined)
                };
                if port_constraints.is_side_fixed() {
                    has_external_ports = true;
                }
            }
        }

        if has_external_ports {
            graph_properties.insert(GraphProperties::ExternalPorts);
        }
        if has_hyperedges {
            graph_properties.insert(GraphProperties::Hyperedges);
        }
    }

    fn transform_external_port(
        &mut self,
        elkgraph: &ElkNodeRef,
        lgraph: &LGraphRef,
        elkport: &ElkPortRef,
    ) {
        let (port_x, port_y, port_width, port_height, port_side, labels) = {
            let mut port_mut = elkport.borrow_mut();
            let shape = port_mut.connectable().shape();
            let port_side = shape
                .graph_element()
                .properties_mut()
                .get_property(LayeredOptions::PORT_SIDE)
                .unwrap_or(PortSide::Undefined);
            let labels: Vec<ElkLabelRef> = shape.graph_element().labels().iter().cloned().collect();
            (
                shape.x(),
                shape.y(),
                shape.width(),
                shape.height(),
                port_side,
                labels,
            )
        };

        let port_position =
            KVector::with_values(port_x + port_width / 2.0, port_y + port_height / 2.0);
        let port_size = KVector::with_values(port_width, port_height);
        let net_flow = self.calculate_external_port_net_flow(elkport);
        let port_constraints = self
            .graph_property(elkgraph, LayeredOptions::PORT_CONSTRAINTS)
            .unwrap_or(PortConstraints::Undefined);
        let layout_direction = LGraphUtil::get_direction(lgraph);

        let graph_size = {
            let mut graph_mut = elkgraph.borrow_mut();
            let shape = graph_mut.connectable().shape();
            KVector::with_values(shape.width(), shape.height())
        };

        let needs_border_offset = {
            let mut port_mut = elkport.borrow_mut();
            let props = port_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            !props.has_property(LayeredOptions::PORT_BORDER_OFFSET)
        };
        let port_border_offset = if needs_border_offset {
            if port_x == 0.0 && port_y == 0.0 {
                0.0
            } else {
                ElkUtil::calc_port_offset(elkport, port_side)
            }
        } else {
            0.0
        };

        let dummy = {
            let mut port_mut = elkport.borrow_mut();
            let shape = port_mut.connectable().shape();
            let props = shape.graph_element().properties_mut();
            if needs_border_offset {
                props.set_property(LayeredOptions::PORT_BORDER_OFFSET, Some(port_border_offset));
            }
            LGraphUtil::create_external_port_dummy(
                props,
                port_constraints,
                port_side,
                net_flow,
                &graph_size,
                &port_position,
                &port_size,
                layout_direction,
                lgraph,
            )
        };

        let origin_id = self
            .origin_store
            .store(ElkGraphElementRef::Port(elkport.clone()));
        if let Some(mut dummy_guard) = dummy.lock_ok() {
            dummy_guard.set_property(InternalProperties::ORIGIN, Some(Origin::ElkPort(origin_id)));
            dummy_guard.set_property(
                CoreOptions::PORT_LABELS_PLACEMENT,
                Some(PortLabelPlacement::outside()),
            );
        }

        let dummy_port = dummy
            .lock_ok()
            .and_then(|dummy_guard| dummy_guard.ports().first().cloned());
        if let Some(dummy_port) = &dummy_port {
            if let Some(mut dummy_port_guard) = dummy_port.lock_ok() {
                dummy_port_guard
                    .set_connected_to_external_nodes(self.is_connected_to_external_nodes(elkport));
            }
        }

        let port_label_placement = self
            .graph_property(elkgraph, CoreOptions::PORT_LABELS_PLACEMENT)
            .unwrap_or_else(PortLabelPlacement::outside);
        let inside_port_labels = port_label_placement.contains(&PortLabelPlacement::Inside);
        let port_labels_fixed = PortLabelPlacement::is_fixed(&port_label_placement);

        for label in labels {
            let (text, no_layout, label_position, label_size) = {
                let mut label_mut = label.borrow_mut();
                let text = label_mut.text().to_owned();
                let no_layout = label_mut
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .get_property(CoreOptions::NO_LAYOUT)
                    .unwrap_or(false);
                let shape = label_mut.shape();
                let position = KVector::with_values(shape.x(), shape.y());
                let size = KVector::with_values(shape.width(), shape.height());
                (text, no_layout, position, size)
            };

            if no_layout || text.is_empty() {
                continue;
            }

            let llabel = self.transform_label(&label);
            if let Some(dummy_port) = &dummy_port {
                if let Some(mut dummy_port_guard) = dummy_port.lock_ok() {
                    dummy_port_guard.labels_mut().push(llabel.clone());
                }
            }

            if !inside_port_labels {
                let inside_part = if port_labels_fixed {
                    ElkUtil::compute_inside_part(
                        &label_position,
                        &label_size,
                        &port_size,
                        0.0,
                        port_side,
                    )
                } else {
                    0.0
                };
                if let Some(mut label_guard) = llabel.lock_ok() {
                    match port_side {
                        PortSide::East | PortSide::West => {
                            label_guard.shape().size().x = inside_part;
                        }
                        PortSide::North | PortSide::South => {
                            label_guard.shape().size().y = inside_part;
                        }
                        PortSide::Undefined => {}
                    }
                }
            }
        }

        if let Some(parent) = elkgraph.borrow().parent() {
            if let Some(mut dummy_guard) = dummy.lock_ok() {
                let spacing_label_port_horizontal = self
                    .graph_property(&parent, LayeredOptions::SPACING_LABEL_PORT_HORIZONTAL)
                    .unwrap_or(1.0);
                let spacing_label_port_vertical = self
                    .graph_property(&parent, LayeredOptions::SPACING_LABEL_PORT_VERTICAL)
                    .unwrap_or(1.0);
                let spacing_label_label = self
                    .graph_property(&parent, LayeredOptions::SPACING_LABEL_LABEL)
                    .unwrap_or(0.0);
                dummy_guard.set_property(
                    LayeredOptions::SPACING_LABEL_PORT_HORIZONTAL,
                    Some(spacing_label_port_horizontal),
                );
                dummy_guard.set_property(
                    LayeredOptions::SPACING_LABEL_PORT_VERTICAL,
                    Some(spacing_label_port_vertical),
                );
                dummy_guard.set_property(
                    LayeredOptions::SPACING_LABEL_LABEL,
                    Some(spacing_label_label),
                );
            }
        }

        if let Some(mut graph_guard) = lgraph.lock_ok() {
            graph_guard.layerless_nodes_mut().push(dummy.clone());
        }

        if let Some(dummy_port) = dummy_port {
            self.port_map.insert(origin_id, dummy_port);
        }
    }

    fn calculate_external_port_net_flow(&self, elkport: &ElkPortRef) -> i32 {
        let elkgraph = elkport
            .borrow()
            .parent()
            .expect("port must have a parent node");
        let inside_self_loops_enabled = self
            .graph_property(&elkgraph, CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE)
            .unwrap_or(false);

        let mut output_port_vote = 0;
        let mut input_port_vote = 0;

        let outgoing_edges = {
            let mut port_mut = elkport.borrow_mut();
            port_mut
                .connectable()
                .outgoing_edges()
                .iter()
                .collect::<Vec<_>>()
        };
        for edge in outgoing_edges {
            let (is_self_loop, inside_loop, target_node) = {
                let mut edge_mut = edge.borrow_mut();
                let is_self_loop = edge_mut.is_selfloop();
                let inside_loop = edge_mut
                    .element()
                    .properties_mut()
                    .get_property(CoreOptions::INSIDE_SELF_LOOPS_YO)
                    .unwrap_or(false);
                let target_node = edge_mut
                    .targets_ro()
                    .get(0)
                    .as_ref()
                    .and_then(ElkGraphUtil::connectable_shape_to_node);
                (is_self_loop, inside_loop, target_node)
            };

            let is_inside_self_loop = is_self_loop && inside_self_loops_enabled && inside_loop;

            if is_self_loop && is_inside_self_loop {
                input_port_vote += 1;
            } else if is_self_loop && !is_inside_self_loop {
                output_port_vote += 1;
            } else if let Some(target_node) = target_node {
                let parent = target_node.borrow().parent();
                if parent.as_ref().is_some_and(|p| Rc::ptr_eq(p, &elkgraph))
                    || Rc::ptr_eq(&target_node, &elkgraph)
                {
                    input_port_vote += 1;
                } else {
                    output_port_vote += 1;
                }
            } else {
                output_port_vote += 1;
            }
        }

        let incoming_edges = {
            let mut port_mut = elkport.borrow_mut();
            port_mut
                .connectable()
                .incoming_edges()
                .iter()
                .collect::<Vec<_>>()
        };
        for edge in incoming_edges {
            let (is_self_loop, inside_loop, source_node) = {
                let mut edge_mut = edge.borrow_mut();
                let is_self_loop = edge_mut.is_selfloop();
                let inside_loop = edge_mut
                    .element()
                    .properties_mut()
                    .get_property(CoreOptions::INSIDE_SELF_LOOPS_YO)
                    .unwrap_or(false);
                let source_node = edge_mut
                    .sources_ro()
                    .get(0)
                    .as_ref()
                    .and_then(ElkGraphUtil::connectable_shape_to_node);
                (is_self_loop, inside_loop, source_node)
            };

            let is_inside_self_loop = is_self_loop && inside_self_loops_enabled && inside_loop;

            if is_self_loop && is_inside_self_loop {
                output_port_vote += 1;
            } else if is_self_loop && !is_inside_self_loop {
                input_port_vote += 1;
            } else if let Some(source_node) = source_node {
                let parent = source_node.borrow().parent();
                if parent.as_ref().is_some_and(|p| Rc::ptr_eq(p, &elkgraph))
                    || Rc::ptr_eq(&source_node, &elkgraph)
                {
                    output_port_vote += 1;
                } else {
                    input_port_vote += 1;
                }
            } else {
                input_port_vote += 1;
            }
        }

        output_port_vote - input_port_vote
    }

    fn is_connected_to_external_nodes(&self, elkport: &ElkPortRef) -> bool {
        let parent = elkport
            .borrow()
            .parent()
            .expect("port must have a parent node");

        let outgoing_edges = {
            let mut port_mut = elkport.borrow_mut();
            port_mut
                .connectable()
                .outgoing_edges()
                .iter()
                .collect::<Vec<_>>()
        };
        for edge in outgoing_edges {
            let target_node = edge
                .borrow()
                .targets_ro()
                .get(0)
                .as_ref()
                .and_then(ElkGraphUtil::connectable_shape_to_node);
            if let Some(target_node) = target_node {
                if !ElkGraphUtil::is_descendant(&target_node, &parent) {
                    return true;
                }
            }
        }

        let incoming_edges = {
            let mut port_mut = elkport.borrow_mut();
            port_mut
                .connectable()
                .incoming_edges()
                .iter()
                .collect::<Vec<_>>()
        };
        for edge in incoming_edges {
            let source_node = edge
                .borrow()
                .sources_ro()
                .get(0)
                .as_ref()
                .and_then(ElkGraphUtil::connectable_shape_to_node);
            if let Some(source_node) = source_node {
                if !ElkGraphUtil::is_descendant(&source_node, &parent) {
                    return true;
                }
            }
        }

        false
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
            .graph_property(
                elkgraph,
                LayeredOptions::CROSSING_MINIMIZATION_FORCE_NODE_MODEL_ORDER,
            )
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
        if !self
            .graph_property(node, CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE)
            .unwrap_or(false)
        {
            return false;
        }

        for edge in ElkGraphUtil::all_outgoing_edges(node) {
            let mut edge_mut = edge.borrow_mut();
            let is_self_loop = edge_mut.is_selfloop();
            if !is_self_loop {
                continue;
            }
            let inside = edge_mut
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
        node_mut
            .connectable()
            .shape()
            .graph_element()
            .properties()
            .clone()
    }
}

impl GraphPropertyOwner for ElkPortRef {
    fn graph_properties(&self) -> MapPropertyHolder {
        let mut port_mut = self.borrow_mut();
        port_mut
            .connectable()
            .shape()
            .graph_element()
            .properties()
            .clone()
    }
}
