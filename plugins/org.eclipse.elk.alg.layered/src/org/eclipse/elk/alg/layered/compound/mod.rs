#![allow(clippy::mutable_key_type)]

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_label_placement::PortLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, EnumSet, IElkProgressMonitor};
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LGraphRef, LGraphUtil, LLabel, LLabelRef, LNodeRef, LPortRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::internal_properties::{Origin, PortRefKey};
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions, PortType,
};
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::OrthogonalRoutingGenerator;

#[derive(Clone)]
pub struct CrossHierarchyEdge {
    edge: LEdgeRef,
    graph: LGraphRef,
    port_type: PortType,
}

impl CrossHierarchyEdge {
    pub fn new(edge: LEdgeRef, graph: LGraphRef, port_type: PortType) -> Self {
        CrossHierarchyEdge {
            edge,
            graph,
            port_type,
        }
    }

    pub fn edge(&self) -> &LEdgeRef {
        &self.edge
    }

    pub fn graph(&self) -> &LGraphRef {
        &self.graph
    }

    pub fn port_type(&self) -> PortType {
        self.port_type
    }

    pub fn actual_source(&self) -> Option<LPortRef> {
        let source = self.edge.lock().source();
        let source = source?;
        let node = source.lock().node();
        let node = node?;
        let mut node_guard = node.lock();
        let node_type = node_guard.node_type();
        if node_type == NodeType::ExternalPort {
            let origin = node_guard.get_property(InternalProperties::ORIGIN);
            if let Some(Origin::LPort(port)) = origin {
                return Some(port);
            }
        }
        Some(source)
    }

    pub fn actual_target(&self) -> Option<LPortRef> {
        let target = self.edge.lock().target();
        let target = target?;
        let node = target.lock().node();
        let node = node?;
        let mut node_guard = node.lock();
        let node_type = node_guard.node_type();
        if node_type == NodeType::ExternalPort {
            let origin = node_guard.get_property(InternalProperties::ORIGIN);
            if let Some(Origin::LPort(port)) = origin {
                return Some(port);
            }
        }
        Some(target)
    }

    pub fn has_junction_points(&self) -> bool {
        let mut edge = self.edge.lock();
        edge.get_property(LayeredOptions::JUNCTION_POINTS)
            .map(|jps| !jps.is_empty())
            .unwrap_or(false)
    }
}

#[derive(Clone, Default)]
pub struct CrossHierarchyMap {
    entries: Vec<(LEdgeRef, Vec<CrossHierarchyEdge>)>,
}

impl CrossHierarchyMap {
    pub fn put(&mut self, edge: &LEdgeRef, cross_edge: CrossHierarchyEdge) {
        if let Some((_, edges)) = self
            .entries
            .iter_mut()
            .find(|(entry, _)| Arc::ptr_eq(entry, edge))
        {
            edges.push(cross_edge);
            return;
        }
        self.entries.push((edge.clone(), vec![cross_edge]));
    }

    pub fn get(&self, edge: &LEdgeRef) -> Vec<CrossHierarchyEdge> {
        self.entries
            .iter()
            .find(|(entry, _)| Arc::ptr_eq(entry, edge))
            .map(|(_, edges)| edges.clone())
            .unwrap_or_default()
    }

    pub fn keys(&self) -> Vec<LEdgeRef> {
        self.entries.iter().map(|(edge, _)| edge.clone()).collect()
    }
}

fn hierarchy_level(nested_graph: &LGraphRef, top_graph: &LGraphRef) -> i32 {
    let mut current = nested_graph.clone();
    let mut level = 0;
    loop {
        if Arc::ptr_eq(&current, top_graph) {
            return level;
        }
        let parent_node = current
            .lock().parent_node()
            .expect("graph is not a descendant of the given top-level graph");
        let parent_graph = parent_node
            .lock().graph()
            .expect("parent graph missing");
        current = parent_graph;
        level += 1;
    }
}

fn sort_cross_hierarchy_edges(edges: &mut [CrossHierarchyEdge], graph: &LGraphRef) {
    edges.sort_by(|edge1, edge2| {
        if edge1.port_type == PortType::Output && edge2.port_type == PortType::Input {
            return std::cmp::Ordering::Less;
        } else if edge1.port_type == PortType::Input && edge2.port_type == PortType::Output {
            return std::cmp::Ordering::Greater;
        }

        let level1 = hierarchy_level(edge1.graph(), graph);
        let level2 = hierarchy_level(edge2.graph(), graph);
        let diff = if edge1.port_type == PortType::Output {
            level2 - level1
        } else {
            level1 - level2
        };
        diff.cmp(&0)
    });
}

#[derive(Clone)]
struct ExternalPort {
    orig_edges: Vec<LEdgeRef>,
    new_edge: LEdgeRef,
    dummy_node: LNodeRef,
    dummy_port: LPortRef,
    port_type: PortType,
    exported: bool,
}

impl ExternalPort {
    fn new(
        orig_edge: LEdgeRef,
        new_edge: LEdgeRef,
        dummy_node: LNodeRef,
        dummy_port: LPortRef,
        port_type: PortType,
        exported: bool,
    ) -> Self {
        ExternalPort {
            orig_edges: vec![orig_edge],
            new_edge,
            dummy_node,
            dummy_port,
            port_type,
            exported,
        }
    }
}

#[derive(Default)]
pub struct CompoundGraphPreprocessor;

impl CompoundGraphPreprocessor {
    pub fn new() -> Self {
        CompoundGraphPreprocessor
    }

    pub fn process_with_ref(&mut self, graph: &LGraphRef, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Compound graph preprocessor", 1.0);

        let mut cross_hierarchy_map = CrossHierarchyMap::default();
        let mut dummy_node_map: HashMap<PortRefKey, LNodeRef> = HashMap::new();

        self.transform_hierarchy_edges(graph, None, &mut cross_hierarchy_map, &mut dummy_node_map);

        if ElkTrace::global().trace {
            for orig_edge in cross_hierarchy_map.keys() {
                let label_count = orig_edge
                    .lock_ok()
                    .map(|edge_guard| edge_guard.labels().len())
                    .unwrap_or(0);
                eprintln!("[compound-pre] cross edge labels={label_count}");
            }
        }
        self.move_labels_and_remove_original_edges(graph, &cross_hierarchy_map);
        self.set_sides_of_ports_to_sides_of_dummy_nodes(&dummy_node_map);

        if ElkTrace::global().trace {
            eprintln!(
                "[compound-pre] cross_hierarchy_map entries={}",
                cross_hierarchy_map.keys().len()
            );
        }

        {
            let mut graph_guard = graph.lock();
            graph_guard.set_property(
                InternalProperties::CROSS_HIERARCHY_MAP,
                Some(cross_hierarchy_map),
            );
        }

        monitor.done();
    }

    fn set_sides_of_ports_to_sides_of_dummy_nodes(
        &self,
        dummy_node_map: &HashMap<PortRefKey, LNodeRef>,
    ) {
        for (port_key, dummy_node) in dummy_node_map {
            let external_port = port_key.0.clone();
            let dummy_side = if let Some(mut dummy_guard) = dummy_node.lock_ok() {
                dummy_guard.set_property(
                    InternalProperties::ORIGIN,
                    Some(Origin::LPort(external_port.clone())),
                );
                dummy_guard
                    .get_property(InternalProperties::EXT_PORT_SIDE)
                    .unwrap_or(PortSide::Undefined)
            } else {
                PortSide::Undefined
            };

            let node_ref = if let Some(mut port_guard) = external_port.lock_ok() {
                port_guard.set_property(InternalProperties::PORT_DUMMY, Some(dummy_node.clone()));
                port_guard.set_property(InternalProperties::INSIDE_CONNECTIONS, Some(true));
                port_guard.set_side(dummy_side);
                port_guard.node()
            } else {
                None
            };
            let node_graph = node_ref
                .as_ref()
                .and_then(|node| node.lock().graph());

            if let Some(node_ref) = &node_ref {
                {
                    let mut node_guard = node_ref.lock();
                    node_guard.set_property(
                        LayeredOptions::PORT_CONSTRAINTS,
                        Some(PortConstraints::FixedSide),
                    );
                }
            }

            if let Some(graph) = node_graph {
                {
                    let mut graph_guard = graph.lock();
                    let mut props = graph_guard
                        .get_property(InternalProperties::GRAPH_PROPERTIES)
                        .unwrap_or_else(EnumSet::none_of);
                    props.insert(GraphProperties::NonFreePorts);
                    graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(props));
                }
            }
        }
    }

    fn transform_hierarchy_edges(
        &self,
        graph: &LGraphRef,
        parent_node: Option<LNodeRef>,
        cross_hierarchy_map: &mut CrossHierarchyMap,
        dummy_node_map: &mut HashMap<PortRefKey, LNodeRef>,
    ) -> Vec<ExternalPort> {
        let layerless_nodes = graph
            .lock().layerless_nodes().clone();

        let mut contained_external_ports: Vec<ExternalPort> = Vec::new();

        for node in layerless_nodes {
            let nested_graph = node.lock().nested_graph();
            if let Some(nested_graph) = nested_graph {
                let child_ports = self.transform_hierarchy_edges(
                    &nested_graph,
                    Some(node.clone()),
                    cross_hierarchy_map,
                    dummy_node_map,
                );
                contained_external_ports.extend(child_ports);

                self.process_inside_self_loops(
                    &nested_graph,
                    &node,
                    cross_hierarchy_map,
                    dummy_node_map,
                );

                let has_external_ports = nested_graph
                    .lock_ok()
                    .and_then(|mut graph_guard| {
                        graph_guard.get_property(InternalProperties::GRAPH_PROPERTIES)
                    })
                    .map(|props| props.contains(&GraphProperties::ExternalPorts))
                    .unwrap_or(false);

                if has_external_ports {
                    let (port_constraints, port_label_placement, ports) =
                        {
                            let mut node_guard = node.lock();
                            (
                                node_guard
                                    .get_property(LayeredOptions::PORT_CONSTRAINTS)
                                    .unwrap_or(PortConstraints::Undefined),
                                node_guard
                                    .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                                    .unwrap_or_else(PortLabelPlacement::outside),
                                node_guard.ports().clone(),
                            )
                        };

                    let inside_port_labels =
                        port_label_placement.contains(&PortLabelPlacement::Inside);
                    let direction = LGraphUtil::get_direction(&nested_graph);

                    for port in ports {
                        let port_key = PortRefKey(port.clone());
                        let mut dummy_node = dummy_node_map.get(&port_key).cloned();

                        if dummy_node.is_none() {
                            let (port_side, port_size, port_net_flow) =
                                {
                                    let mut port_guard = port.lock();
                                    (
                                        port_guard.side(),
                                        *port_guard.shape().size_ref(),
                                        port_guard.net_flow() as i32,
                                    )
                                };
                            let port_node_size = KVector::new();
                            let port_position = KVector::new();
                            let mut props = port_property_holder(&port);
                            let dummy = LGraphUtil::create_external_port_dummy(
                                &mut props,
                                port_constraints,
                                port_side,
                                -port_net_flow,
                                &port_node_size,
                                &port_position,
                                &port_size,
                                direction,
                                &nested_graph,
                            );
                            {
                                let mut dummy_guard = dummy.lock();
                                dummy_guard.set_property(
                                    InternalProperties::ORIGIN,
                                    Some(Origin::LPort(port.clone())),
                                );
                            }
                            dummy_node_map.insert(port_key.clone(), dummy.clone());
                            {
                                let mut graph_guard = nested_graph.lock();
                                graph_guard.layerless_nodes_mut().push(dummy.clone());
                            }
                            dummy_node = Some(dummy);
                        }

                        let dummy_node = match dummy_node {
                            Some(node) => node,
                            None => continue,
                        };

                        let dummy_port = dummy_node
                            .lock_ok()
                            .and_then(|node| node.ports().first().cloned());
                        let Some(dummy_port) = dummy_port else {
                            continue;
                        };

                        let (labels, port_size, port_side_cached) = if let Some(mut port_guard) = port.lock_ok() {
                            (
                                port_guard.labels().clone(),
                                *port_guard.shape().size_ref(),
                                port_guard.side(),
                            )
                        } else {
                            (Vec::new(), KVector::default(), PortSide::Undefined)
                        };

                        for label in labels {
                            let (label_size, label_pos) = if let Some(mut label_guard) = label.lock_ok()
                            {
                                (
                                    *label_guard.shape().size_ref(),
                                    *label_guard.shape().position_ref(),
                                )
                            } else {
                                (KVector::new(), KVector::new())
                            };

                            let dummy_label = Arc::new(Mutex::new(LLabel::new()));
                            {
                                let mut dummy_label_guard = dummy_label.lock();
                                dummy_label_guard.shape().size().x = label_size.x;
                                dummy_label_guard.shape().size().y = label_size.y;

                                if !inside_port_labels {
                                    let mut inside_part = 0.0;
                                    if PortLabelPlacement::is_fixed(&port_label_placement) {
                                        inside_part = ElkUtil::compute_inside_part(
                                            &label_pos,
                                            &label_size,
                                            &port_size,
                                            0.0,
                                            port_side_cached,
                                        );
                                    }

                                    if port_constraints == PortConstraints::Free
                                        || port_side_cached == PortSide::East
                                        || port_side_cached == PortSide::West
                                    {
                                        dummy_label_guard.shape().size().x = inside_part;
                                    } else {
                                        dummy_label_guard.shape().size().y = inside_part;
                                    }
                                }
                            }

                            {
                                let mut dummy_port_guard = dummy_port.lock();
                                dummy_port_guard.labels_mut().push(dummy_label);
                            }
                        }
                    }
                }
            }
        }

        let mut exported_external_ports: Vec<ExternalPort> = Vec::new();

        self.process_inner_hierarchical_edge_segments(
            graph,
            parent_node.clone(),
            &contained_external_ports,
            &mut exported_external_ports,
            cross_hierarchy_map,
            dummy_node_map,
        );

        if let Some(parent_node) = parent_node {
            self.process_outer_hierarchical_edge_segments(
                graph,
                &parent_node,
                &mut exported_external_ports,
                cross_hierarchy_map,
                dummy_node_map,
            );
        }

        exported_external_ports
    }

    fn move_labels_and_remove_original_edges(
        &self,
        graph: &LGraphRef,
        cross_hierarchy_map: &CrossHierarchyMap,
    ) {
        for orig_edge in cross_hierarchy_map.keys() {
            let has_labels = orig_edge
                .lock_ok()
                .map(|edge_guard| !edge_guard.labels().is_empty())
                .unwrap_or(false);

            if has_labels {
                let mut edge_segments = cross_hierarchy_map.get(&orig_edge);
                sort_cross_hierarchy_edges(&mut edge_segments, graph);

                let mut label_moves: Vec<(LLabelRef, usize)> = Vec::new();
                {
                    let mut edge_guard = orig_edge.lock();
                    let idx = 0;
                    while idx < edge_guard.labels().len() {
                        let label_ref = edge_guard.labels()[idx].clone();
                        let placement = label_ref
                            .lock_ok()
                            .and_then(|mut label| {
                                label.get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
                            })
                            .unwrap_or(EdgeLabelPlacement::Center);

                        let target_index = match placement {
                            EdgeLabelPlacement::Head => edge_segments.len().saturating_sub(1),
                            EdgeLabelPlacement::Center => {
                                self.get_shallowest_edge_segment(&edge_segments)
                            }
                            EdgeLabelPlacement::Tail => 0,
                        };

                        label_moves.push((label_ref.clone(), target_index));
                        {
                            let mut label_guard = label_ref.lock();
                            if ElkTrace::global().trace && label_guard.text() == "e2" {
                                eprintln!(
                                    "[compound-pre] moving e2 label (segments={})",
                                    edge_segments.len()
                                );
                            }
                            label_guard.set_property(
                                InternalProperties::ORIGINAL_LABEL_EDGE,
                                Some(orig_edge.clone()),
                            );
                        }
                        edge_guard.labels_mut().remove(idx);
                    }
                }

                for (label_ref, target_index) in label_moves {
                    if let Some(target_segment) = edge_segments.get(target_index) {
                        {
                            let mut target_edge_guard = target_segment.edge().lock();
                            target_edge_guard.labels_mut().push(label_ref.clone());
                        }

                        let graph_ref = target_segment
                            .edge()
                            .lock().source()
                            .and_then(|port| port.lock().node())
                            .and_then(|node| node.lock().graph());

                        if let Some(graph_ref) = graph_ref {
                            {
                                let mut graph_guard = graph_ref.lock();
                                let mut props = graph_guard
                                    .get_property(InternalProperties::GRAPH_PROPERTIES)
                                    .unwrap_or_else(EnumSet::none_of);
                                props.insert(GraphProperties::EndLabels);
                                props.insert(GraphProperties::CenterLabels);
                                graph_guard.set_property(
                                    InternalProperties::GRAPH_PROPERTIES,
                                    Some(props),
                                );
                            }
                        }
                    }
                }
            }

            LEdge::set_source(&orig_edge, None);
            LEdge::set_target(&orig_edge, None);
        }
    }

    fn get_shallowest_edge_segment(&self, edge_segments: &[CrossHierarchyEdge]) -> usize {
        let mut result = 0;
        for (index, segment) in edge_segments.iter().enumerate() {
            if segment.port_type() == PortType::Input {
                result = if index == 0 { 0 } else { index - 1 };
                break;
            } else if index == edge_segments.len().saturating_sub(1) {
                result = index;
            }
        }
        result
    }

    fn process_inner_hierarchical_edge_segments(
        &self,
        graph: &LGraphRef,
        parent_node: Option<LNodeRef>,
        contained_external_ports: &[ExternalPort],
        exported_external_ports: &mut Vec<ExternalPort>,
        cross_hierarchy_map: &mut CrossHierarchyMap,
        dummy_node_map: &mut HashMap<PortRefKey, LNodeRef>,
    ) {
        let mut created_external_ports: Vec<ExternalPort> = Vec::new();

        for external_port in contained_external_ports {
            if external_port.port_type == PortType::Output {
                let mut current_external_port_index: Option<usize> = None;
                for out_edge in &external_port.orig_edges {
                    let target_port = out_edge
                        .lock().target();
                    let target_port = match target_port {
                        Some(port) => port,
                        None => continue,
                    };
                    let target_node = match target_port.lock().node() {
                        Some(node) => node,
                        None => continue,
                    };

                    let target_graph = target_node.lock().graph();

                    if let Some(target_graph) = target_graph {
                        if Arc::ptr_eq(&target_graph, graph) {
                            self.connect_child(
                                graph,
                                external_port,
                                out_edge,
                                &external_port.dummy_port,
                                &target_port,
                                cross_hierarchy_map,
                            );
                        } else if parent_node
                            .as_ref()
                            .map(|parent| LGraphUtil::is_descendant(&target_node, parent))
                            .unwrap_or(true)
                        {
                            self.connect_siblings(
                                graph,
                                external_port,
                                contained_external_ports,
                                out_edge,
                                cross_hierarchy_map,
                            );
                        } else {
                            let default_port = current_external_port_index
                                .and_then(|idx| created_external_ports.get_mut(idx));
                            let new_port = self.introduce_hierarchical_edge_segment(
                                graph,
                                parent_node.as_ref().unwrap(),
                                out_edge,
                                &external_port.dummy_port,
                                PortType::Output,
                                default_port,
                                cross_hierarchy_map,
                                dummy_node_map,
                            );
                            if let Some(new_port) = new_port {
                                let idx = created_external_ports.len();
                                created_external_ports.push(new_port);
                                if created_external_ports[idx].exported {
                                    current_external_port_index = Some(idx);
                                }
                            }
                        }
                    }
                }
            } else {
                let mut current_external_port_index: Option<usize> = None;
                for in_edge in &external_port.orig_edges {
                    let source_port = in_edge
                        .lock().source();
                    let source_port = match source_port {
                        Some(port) => port,
                        None => continue,
                    };
                    let source_node = match source_port.lock().node() {
                        Some(node) => node,
                        None => continue,
                    };

                    let source_graph = source_node.lock().graph();

                    if let Some(source_graph) = source_graph {
                        if Arc::ptr_eq(&source_graph, graph) {
                            self.connect_child(
                                graph,
                                external_port,
                                in_edge,
                                &source_port,
                                &external_port.dummy_port,
                                cross_hierarchy_map,
                            );
                        } else if parent_node
                            .as_ref()
                            .map(|parent| LGraphUtil::is_descendant(&source_node, parent))
                            .unwrap_or(true)
                        {
                            continue;
                        } else {
                            let default_port = current_external_port_index
                                .and_then(|idx| created_external_ports.get_mut(idx));
                            let new_port = self.introduce_hierarchical_edge_segment(
                                graph,
                                parent_node.as_ref().unwrap(),
                                in_edge,
                                &external_port.dummy_port,
                                PortType::Input,
                                default_port,
                                cross_hierarchy_map,
                                dummy_node_map,
                            );
                            if let Some(new_port) = new_port {
                                let idx = created_external_ports.len();
                                created_external_ports.push(new_port);
                                if created_external_ports[idx].exported {
                                    current_external_port_index = Some(idx);
                                }
                            }
                        }
                    }
                }
            }
        }

        for external_port in created_external_ports {
            {
                let mut graph_guard = graph.lock();
                if !graph_guard
                    .layerless_nodes()
                    .iter()
                    .any(|node| Arc::ptr_eq(node, &external_port.dummy_node))
                {
                    graph_guard
                        .layerless_nodes_mut()
                        .push(external_port.dummy_node.clone());
                }
            }

            if external_port.exported {
                exported_external_ports.push(external_port);
            }
        }
    }

    fn connect_child(
        &self,
        graph: &LGraphRef,
        external_port: &ExternalPort,
        orig_edge: &LEdgeRef,
        source_port: &LPortRef,
        target_port: &LPortRef,
        cross_hierarchy_map: &mut CrossHierarchyMap,
    ) {
        let dummy_edge = create_dummy_edge(orig_edge);
        LEdge::set_source(&dummy_edge, Some(source_port.clone()));
        LEdge::set_target(&dummy_edge, Some(target_port.clone()));

        cross_hierarchy_map.put(
            orig_edge,
            CrossHierarchyEdge::new(dummy_edge, graph.clone(), external_port.port_type),
        );
    }

    fn connect_siblings(
        &self,
        graph: &LGraphRef,
        external_output_port: &ExternalPort,
        contained_external_ports: &[ExternalPort],
        orig_edge: &LEdgeRef,
        cross_hierarchy_map: &mut CrossHierarchyMap,
    ) {
        let target_external_port = contained_external_ports
            .iter()
            .find(|port| {
                !std::ptr::eq(*port, external_output_port)
                    && port
                        .orig_edges
                        .iter()
                        .any(|edge| Arc::ptr_eq(edge, orig_edge))
            })
            .expect("target external port missing");

        let dummy_edge = create_dummy_edge(orig_edge);
        LEdge::set_source(&dummy_edge, Some(external_output_port.dummy_port.clone()));
        LEdge::set_target(&dummy_edge, Some(target_external_port.dummy_port.clone()));

        cross_hierarchy_map.put(
            orig_edge,
            CrossHierarchyEdge::new(dummy_edge, graph.clone(), external_output_port.port_type),
        );
    }

    fn process_outer_hierarchical_edge_segments(
        &self,
        graph: &LGraphRef,
        parent_node: &LNodeRef,
        exported_external_ports: &mut Vec<ExternalPort>,
        cross_hierarchy_map: &mut CrossHierarchyMap,
        dummy_node_map: &mut HashMap<PortRefKey, LNodeRef>,
    ) {
        let child_nodes = graph
            .lock().layerless_nodes().clone();

        let mut created_external_ports: Vec<ExternalPort> = Vec::new();

        for child_node in child_nodes {
            let ports = child_node
                .lock().ports().clone();

            if ElkTrace::global().trace {
                let origin_id = child_node
                    .lock_ok()
                    .and_then(|mut node_guard| node_guard.get_property(InternalProperties::ORIGIN))
                    .and_then(|origin| match origin {
                        Origin::ElkNode(id) => Some(id),
                        _ => None,
                    })
                    .unwrap_or(0);
                eprintln!(
                    "[compound-pre] child node origin={origin_id} ports={}",
                    ports.len()
                );
                for port in &ports {
                    let (in_len, out_len) = port
                        .lock_ok()
                        .map(|port_guard| {
                            (
                                port_guard.incoming_edges().len(),
                                port_guard.outgoing_edges().len(),
                            )
                        })
                        .unwrap_or((0, 0));
                    eprintln!("[compound-pre] port edges in={in_len} out={out_len}");
                }
            }

            for child_port in ports {
                let (outgoing_edges, incoming_edges) = child_port
                    .lock_ok()
                    .map(|port_guard| {
                        (
                            port_guard.outgoing_edges().clone(),
                            port_guard.incoming_edges().clone(),
                        )
                    })
                    .unwrap_or_default();

                let mut current_external_output_index: Option<usize> = None;
                for out_edge in outgoing_edges {
                    let (source_port, target_port) = {
                        let edge_guard = out_edge.lock();
                        (edge_guard.source(), edge_guard.target())
                    };
                    let target_node = target_port
                        .and_then(|port| port.lock().node());
                    let target_node = match target_node {
                        Some(node) => node,
                        None => continue,
                    };

                    if !LGraphUtil::is_descendant(&target_node, parent_node) {
                        let default_port = current_external_output_index
                            .and_then(|idx| created_external_ports.get_mut(idx));
                        let opposite_port = source_port.unwrap();
                        let new_port = self.introduce_hierarchical_edge_segment(
                            graph,
                            parent_node,
                            &out_edge,
                            &opposite_port,
                            PortType::Output,
                            default_port,
                            cross_hierarchy_map,
                            dummy_node_map,
                        );
                        if let Some(new_port) = new_port {
                            let idx = created_external_ports.len();
                            created_external_ports.push(new_port);
                            if created_external_ports[idx].exported {
                                current_external_output_index = Some(idx);
                            }
                        }
                    }
                }

                let mut current_external_input_index: Option<usize> = None;
                for in_edge in incoming_edges {
                    let (source_port, target_port, has_e2_label) = {
                        let edge_guard = in_edge.lock();
                        let has_e2 = if ElkTrace::global().trace {
                            edge_guard.labels().iter().any(|label_ref| {
                                label_ref
                                    .lock_ok()
                                    .map(|label_guard| label_guard.text() == "e2")
                                    .unwrap_or(false)
                            })
                        } else {
                            false
                        };
                        (edge_guard.source(), edge_guard.target(), has_e2)
                    };
                    let source_node = source_port
                        .and_then(|port| port.lock().node());
                    let source_node = match source_node {
                        Some(node) => node,
                        None => continue,
                    };

                    if ElkTrace::global().trace && has_e2_label {
                        let is_desc = LGraphUtil::is_descendant(&source_node, parent_node);
                        eprintln!("[compound-pre] saw e2 incoming, source_descendant={is_desc}");
                    }

                    if !LGraphUtil::is_descendant(&source_node, parent_node) {
                        let default_port = current_external_input_index
                            .and_then(|idx| created_external_ports.get_mut(idx));
                        let opposite_port = target_port.unwrap();
                        let new_port = self.introduce_hierarchical_edge_segment(
                            graph,
                            parent_node,
                            &in_edge,
                            &opposite_port,
                            PortType::Input,
                            default_port,
                            cross_hierarchy_map,
                            dummy_node_map,
                        );
                        if let Some(new_port) = new_port {
                            let idx = created_external_ports.len();
                            created_external_ports.push(new_port);
                            if created_external_ports[idx].exported {
                                current_external_input_index = Some(idx);
                            }
                        }
                    }
                }
            }
        }

        for external_port in created_external_ports {
            {
                let mut graph_guard = graph.lock();
                if !graph_guard
                    .layerless_nodes()
                    .iter()
                    .any(|node| Arc::ptr_eq(node, &external_port.dummy_node))
                {
                    graph_guard
                        .layerless_nodes_mut()
                        .push(external_port.dummy_node.clone());
                }
            }
            if external_port.exported {
                exported_external_ports.push(external_port);
            }
        }
    }

    fn process_inside_self_loops(
        &self,
        nested_graph: &LGraphRef,
        node: &LNodeRef,
        cross_hierarchy_map: &mut CrossHierarchyMap,
        dummy_node_map: &mut HashMap<PortRefKey, LNodeRef>,
    ) {
        let trace_inside = ElkTrace::global().inside_yo;
        let (inside_self_loops_active, ports) = node
            .lock_ok()
            .map(|mut node_guard| {
                let active = node_guard
                    .get_property(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE)
                    .unwrap_or(false);
                let ports = node_guard.ports().clone();
                (active, ports)
            })
            .unwrap_or((false, Vec::new()));
        if trace_inside {
            let node_key = Arc::as_ptr(node) as usize;
            eprintln!(
                "[inside-self] node={} inside_self_loops_active={}",
                node_key, inside_self_loops_active
            );
        }
        if !inside_self_loops_active {
            return;
        }

        for port in ports {
            let out_edges = port
                .lock().outgoing_edges().clone();

            for out_edge in out_edges {
                let (is_self_loop, inside_self_loop) = out_edge
                    .lock_ok()
                    .map(|mut edge_guard| {
                        let is_self_loop = edge_guard.is_self_loop();
                        let inside_self_loop = is_self_loop
                            && edge_guard
                                .get_property(CoreOptions::INSIDE_SELF_LOOPS_YO)
                                .unwrap_or(false);
                        (is_self_loop, inside_self_loop)
                    })
                    .unwrap_or((false, false));
                if trace_inside {
                    let edge_key = Arc::as_ptr(&out_edge) as usize;
                    eprintln!(
                        "[inside-self] edge={} is_self_loop={} inside_self_loop={}",
                        edge_key, is_self_loop, inside_self_loop
                    );
                }

                if !is_self_loop || !inside_self_loop {
                    continue;
                }

                let (source_port, target_port) = {
                    let edge_guard = out_edge.lock_ok().unwrap();
                    (edge_guard.source().unwrap(), edge_guard.target().unwrap())
                };

                let source_dummy = self.ensure_external_port_dummy(
                    nested_graph,
                    &source_port,
                    PortConstraints::Free,
                    -1,
                    dummy_node_map,
                );
                let target_dummy = self.ensure_external_port_dummy(
                    nested_graph,
                    &target_port,
                    PortConstraints::Free,
                    1,
                    dummy_node_map,
                );

                let dummy_edge = create_dummy_edge(&out_edge);
                let source_port_dummy = source_dummy
                    .lock_ok()
                    .and_then(|node| node.ports().first().cloned())
                    .unwrap();
                let target_port_dummy = target_dummy
                    .lock_ok()
                    .and_then(|node| node.ports().first().cloned())
                    .unwrap();
                LEdge::set_source(&dummy_edge, Some(source_port_dummy));
                LEdge::set_target(&dummy_edge, Some(target_port_dummy));

                cross_hierarchy_map.put(
                    &out_edge,
                    CrossHierarchyEdge::new(dummy_edge, nested_graph.clone(), PortType::Output),
                );
                if trace_inside {
                    let edge_key = Arc::as_ptr(&out_edge) as usize;
                    eprintln!("[inside-self] edge={} promoted_to_inside_dummy", edge_key);
                }

                {
                    let mut graph_guard = nested_graph.lock();
                    let mut props = graph_guard
                        .get_property(InternalProperties::GRAPH_PROPERTIES)
                        .unwrap_or_else(EnumSet::none_of);
                    props.insert(GraphProperties::ExternalPorts);
                    graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(props));
                }
            }
        }
    }

    fn ensure_external_port_dummy(
        &self,
        graph: &LGraphRef,
        port: &LPortRef,
        fallback_constraints: PortConstraints,
        net_flow: i32,
        dummy_node_map: &mut HashMap<PortRefKey, LNodeRef>,
    ) -> LNodeRef {
        let port_key = PortRefKey(port.clone());
        if let Some(existing) = dummy_node_map.get(&port_key) {
            return existing.clone();
        }

        let (port_side, port_size, direction) = {
            let (port_side, port_size) = port
                .lock_ok()
                .map(|mut port_guard| (port_guard.side(), *port_guard.shape().size_ref()))
                .unwrap_or((PortSide::Undefined, KVector::new()));
            (port_side, port_size, LGraphUtil::get_direction(graph))
        };

        let mut props = port_property_holder(port);
        // Java inside-self-loop path uses PortConstraints.FREE with null position/node-size context.
        // Match that behavior by keeping FREE constraints and zero vectors.
        let dummy_node = LGraphUtil::create_external_port_dummy(
            &mut props,
            fallback_constraints,
            port_side,
            net_flow,
            &KVector::new(),
            &KVector::new(),
            &port_size,
            direction,
            graph,
        );
        {
            let mut dummy_guard = dummy_node.lock();
            dummy_guard.set_property(
                InternalProperties::ORIGIN,
                Some(Origin::LPort(port.clone())),
            );
        }
        dummy_node_map.insert(port_key, dummy_node.clone());

        {
            let mut graph_guard = graph.lock();
            graph_guard.layerless_nodes_mut().push(dummy_node.clone());
            let mut props = graph_guard
                .get_property(InternalProperties::GRAPH_PROPERTIES)
                .unwrap_or_else(EnumSet::none_of);
            props.insert(GraphProperties::ExternalPorts);
            graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(props));

            let graph_constraints = graph_guard
                .get_property(LayeredOptions::PORT_CONSTRAINTS)
                .unwrap_or(PortConstraints::Undefined);
            let new_constraints = if graph_constraints.is_side_fixed() {
                PortConstraints::FixedSide
            } else {
                PortConstraints::Free
            };
            graph_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(new_constraints));
        }

        dummy_node
    }

    #[allow(clippy::too_many_arguments)]
    fn introduce_hierarchical_edge_segment(
        &self,
        graph: &LGraphRef,
        parent_node: &LNodeRef,
        orig_edge: &LEdgeRef,
        opposite_port: &LPortRef,
        port_type: PortType,
        default_external_port: Option<&mut ExternalPort>,
        cross_hierarchy_map: &mut CrossHierarchyMap,
        dummy_node_map: &mut HashMap<PortRefKey, LNodeRef>,
    ) -> Option<ExternalPort> {
        let merge_external_ports = graph
            .lock_ok()
            .and_then(|mut graph_guard| {
                graph_guard.get_property(LayeredOptions::MERGE_HIERARCHY_EDGES)
            })
            .unwrap_or(true);

        let parent_end_port = if port_type == PortType::Input {
            orig_edge
                .lock().source()
                .and_then(|port| {
                    port.lock().node().map(|node| {
                        if Arc::ptr_eq(&node, parent_node) {
                            Some(port.clone())
                        } else {
                            None
                        }
                    })
                })
                .flatten()
        } else if port_type == PortType::Output {
            orig_edge
                .lock().target()
                .and_then(|port| {
                    port.lock().node().map(|node| {
                        if Arc::ptr_eq(&node, parent_node) {
                            Some(port.clone())
                        } else {
                            None
                        }
                    })
                })
                .flatten()
        } else {
            None
        };

        if merge_external_ports && parent_end_port.is_none() {
            if let Some(default_port) = default_external_port {
                default_port.orig_edges.push(orig_edge.clone());
                let orig_thickness = orig_edge
                    .lock_ok()
                    .and_then(|mut edge_guard| edge_guard.get_property(CoreOptions::EDGE_THICKNESS))
                    .unwrap_or(1.0);
                {
                    let mut edge_guard = default_port.new_edge.lock();
                    let existing_thickness = edge_guard
                        .get_property(CoreOptions::EDGE_THICKNESS)
                        .unwrap_or(1.0);
                    let thickness = existing_thickness.max(orig_thickness);
                    edge_guard.set_property(CoreOptions::EDGE_THICKNESS, Some(thickness));
                }

                cross_hierarchy_map.put(
                    orig_edge,
                    CrossHierarchyEdge::new(
                        default_port.new_edge.clone(),
                        graph.clone(),
                        port_type,
                    ),
                );
                if ElkTrace::global().trace
                    && orig_edge
                        .lock_ok()
                        .map(|edge_guard| {
                            edge_guard.labels().iter().any(|label_ref| {
                                label_ref
                                    .lock_ok()
                                    .map(|label_guard| label_guard.text() == "e2")
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(false)
                {
                    eprintln!(
                        "[compound-pre] e2 added via merge port_type={:?}",
                        port_type
                    );
                }
                return None;
            }
        }

        let port_side = if let Some(parent_end_port) = parent_end_port.as_ref() {
            parent_end_port
                .lock().side()
        } else if parent_node
            .lock_ok()
            .and_then(|mut node_guard| node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS))
            .unwrap_or(PortConstraints::Undefined)
            .is_side_fixed()
        {
            if port_type == PortType::Input {
                PortSide::West
            } else {
                PortSide::East
            }
        } else {
            PortSide::Undefined
        };

        let dummy_node = self.create_external_port_dummy(
            graph,
            parent_node,
            port_type,
            port_side,
            orig_edge,
            dummy_node_map,
        );

        let dummy_port = dummy_node
            .lock_ok()
            .and_then(|node| node.ports().first().cloned())
            .unwrap();

        let dummy_edge = create_dummy_edge(orig_edge);
        if port_type == PortType::Input {
            LEdge::set_source(&dummy_edge, Some(dummy_port));
            LEdge::set_target(&dummy_edge, Some(opposite_port.clone()));
        } else {
            LEdge::set_source(&dummy_edge, Some(opposite_port.clone()));
            LEdge::set_target(&dummy_edge, Some(dummy_port));
        }

        let origin_port = dummy_node
            .lock_ok()
            .and_then(|mut node| node.get_property(InternalProperties::ORIGIN))
            .and_then(|origin| match origin {
                Origin::LPort(port) => Some(port),
                _ => None,
            })
            .unwrap_or_else(|| opposite_port.clone());

        let external_port = ExternalPort::new(
            orig_edge.clone(),
            dummy_edge.clone(),
            dummy_node,
            origin_port,
            port_type,
            parent_end_port.is_none(),
        );

        cross_hierarchy_map.put(
            orig_edge,
            CrossHierarchyEdge::new(dummy_edge, graph.clone(), port_type),
        );
        if ElkTrace::global().trace
            && orig_edge
                .lock_ok()
                .map(|edge_guard| {
                    edge_guard.labels().iter().any(|label_ref| {
                        label_ref
                            .lock_ok()
                            .map(|label_guard| label_guard.text() == "e2")
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        {
            eprintln!("[compound-pre] e2 added via new port_type={:?}", port_type);
        }

        Some(external_port)
    }

    fn create_external_port_dummy(
        &self,
        graph: &LGraphRef,
        parent_node: &LNodeRef,
        port_type: PortType,
        port_side: PortSide,
        edge: &LEdgeRef,
        dummy_node_map: &mut HashMap<PortRefKey, LNodeRef>,
    ) -> LNodeRef {
        let outside_port = {
            let edge_guard = edge.lock_ok().expect("edge missing endpoints");
            if port_type == PortType::Input {
                edge_guard.source()
            } else {
                edge_guard.target()
            }
            .expect("edge missing endpoints")
        };

        let layout_direction = LGraphUtil::get_direction(graph);

        let is_parent_port = outside_port
            .lock().node()
            .map(|node| Arc::ptr_eq(&node, parent_node))
            .unwrap_or(false);
        let dummy_node = if is_parent_port {
            let port_key = PortRefKey(outside_port.clone());
            if let Some(existing) = dummy_node_map.get(&port_key) {
                return existing.clone();
            }
            let port_constraints = parent_node
                .lock_ok()
                .and_then(|mut node_guard| {
                    node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS)
                })
                .unwrap_or(PortConstraints::Undefined);
            let net_flow = self.calculate_net_flow(&outside_port);
            let port_node_size = KVector::new();
            let (port_position, port_size) = if let Some(mut port_guard) = outside_port.lock_ok() {
                (
                    *port_guard.shape().position_ref(),
                    *port_guard.shape().size_ref(),
                )
            } else {
                (KVector::new(), KVector::new())
            };
            let mut props = port_property_holder(&outside_port);
            let dummy = LGraphUtil::create_external_port_dummy(
                &mut props,
                port_constraints,
                port_side,
                net_flow,
                &port_node_size,
                &port_position,
                &port_size,
                layout_direction,
                graph,
            );
            {
                let mut dummy_guard = dummy.lock();
                dummy_guard.set_property(
                    InternalProperties::ORIGIN,
                    Some(Origin::LPort(outside_port.clone())),
                );
            }
            dummy_node_map.insert(port_key, dummy.clone());
            dummy
        } else {
            let port_constraints = parent_node
                .lock_ok()
                .and_then(|mut node_guard| {
                    node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS)
                })
                .unwrap_or(PortConstraints::Undefined);
            let net_flow = if port_type == PortType::Input { -1 } else { 1 };
            let port_node_size = KVector::new();
            let port_position = KVector::new();
            let port_size = KVector::new();
            let mut props = create_external_port_properties(graph);
            let dummy = LGraphUtil::create_external_port_dummy(
                &mut props,
                port_constraints,
                port_side,
                net_flow,
                &port_node_size,
                &port_position,
                &port_size,
                layout_direction,
                graph,
            );
            let dummy_port = self.create_port_for_dummy(&dummy, parent_node, port_type);
            {
                let mut dummy_guard = dummy.lock();
                dummy_guard.set_property(
                    InternalProperties::ORIGIN,
                    Some(Origin::LPort(dummy_port.clone())),
                );
            }
            dummy_node_map.insert(PortRefKey(dummy_port), dummy.clone());
            dummy
        };

        {
            let mut graph_guard = graph.lock();
            let mut props = graph_guard
                .get_property(InternalProperties::GRAPH_PROPERTIES)
                .unwrap_or_else(EnumSet::none_of);
            props.insert(GraphProperties::ExternalPorts);
            graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(props));

            let graph_constraints = graph_guard
                .get_property(LayeredOptions::PORT_CONSTRAINTS)
                .unwrap_or(PortConstraints::Undefined);
            if graph_constraints.is_side_fixed() {
                graph_guard.set_property(
                    LayeredOptions::PORT_CONSTRAINTS,
                    Some(PortConstraints::FixedSide),
                );
            } else {
                graph_guard.set_property(
                    LayeredOptions::PORT_CONSTRAINTS,
                    Some(PortConstraints::Free),
                );
            }
        }

        dummy_node
    }

    fn calculate_net_flow(&self, port: &LPortRef) -> i32 {
        let (node, outgoing_edges, incoming_edges) = {
            let port_guard = port.lock_ok().expect("port without node");
            (
                port_guard.node().expect("port without node"),
                port_guard.outgoing_edges().clone(),
                port_guard.incoming_edges().clone(),
            )
        };
        let inside_self_loops_enabled = node
            .lock_ok()
            .and_then(|mut node_guard| {
                node_guard.get_property(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE)
            })
            .unwrap_or(false);

        let mut output_port_vote = 0;
        let mut input_port_vote = 0;

        for outgoing_edge in outgoing_edges {
            let (is_self_loop, is_inside_self_loop, target_node) = outgoing_edge
                .lock_ok()
                .map(|mut edge_guard| {
                    (
                        edge_guard.is_self_loop(),
                        edge_guard
                            .get_property(CoreOptions::INSIDE_SELF_LOOPS_YO)
                            .unwrap_or(false),
                        edge_guard
                            .target()
                            .and_then(|port| port.lock().node()),
                    )
                })
                .unwrap_or((false, false, None));

            let is_inside_self_loop =
                is_self_loop && inside_self_loops_enabled && is_inside_self_loop;

            if is_self_loop && is_inside_self_loop {
                input_port_vote += 1;
            } else if is_self_loop && !is_inside_self_loop {
                output_port_vote += 1;
            } else if let Some(target_node) = target_node {
                let parent = target_node
                    .lock().graph()
                    .and_then(|graph| graph.lock().parent_node());
                if let Some(parent) = parent {
                    if Arc::ptr_eq(&parent, &node) {
                        input_port_vote += 1;
                    } else {
                        output_port_vote += 1;
                    }
                } else {
                    output_port_vote += 1;
                }
            } else {
                output_port_vote += 1;
            }
        }

        for incoming_edge in incoming_edges {
            let (is_self_loop, is_inside_self_loop, source_node) = incoming_edge
                .lock_ok()
                .map(|mut edge_guard| {
                    (
                        edge_guard.is_self_loop(),
                        edge_guard
                            .get_property(CoreOptions::INSIDE_SELF_LOOPS_YO)
                            .unwrap_or(false),
                        edge_guard
                            .source()
                            .and_then(|port| port.lock().node()),
                    )
                })
                .unwrap_or((false, false, None));

            let is_inside_self_loop =
                is_self_loop && inside_self_loops_enabled && is_inside_self_loop;

            if is_self_loop && is_inside_self_loop {
                output_port_vote += 1;
            } else if is_self_loop && !is_inside_self_loop {
                input_port_vote += 1;
            } else if let Some(source_node) = source_node {
                let parent = source_node
                    .lock().graph()
                    .and_then(|graph| graph.lock().parent_node());
                if let Some(parent) = parent {
                    if Arc::ptr_eq(&parent, &node) {
                        output_port_vote += 1;
                    } else {
                        input_port_vote += 1;
                    }
                } else {
                    input_port_vote += 1;
                }
            } else {
                input_port_vote += 1;
            }
        }

        output_port_vote - input_port_vote
    }

    fn create_port_for_dummy(
        &self,
        dummy_node: &LNodeRef,
        parent_node: &LNodeRef,
        port_type: PortType,
    ) -> LPortRef {
        let graph = parent_node
            .lock().graph()
            .expect("parent node without graph");
        let direction = LGraphUtil::get_direction(&graph);

        let port = crate::org::eclipse::elk::alg::layered::graph::LPort::new();
        crate::org::eclipse::elk::alg::layered::graph::LPort::set_node(
            &port,
            Some(parent_node.clone()),
        );

        {
            let mut port_guard = port.lock();
            match port_type {
                PortType::Input => {
                    port_guard.set_side(PortSide::from_direction(direction).opposed());
                }
                PortType::Output => {
                    port_guard.set_side(PortSide::from_direction(direction));
                }
                PortType::Undefined => {}
            }
            let border_offset = dummy_node
                .lock_ok()
                .and_then(|mut dummy_guard| {
                    dummy_guard.get_property(LayeredOptions::PORT_BORDER_OFFSET)
                })
                .unwrap_or(0.0);
            port_guard.set_property(LayeredOptions::PORT_BORDER_OFFSET, Some(border_offset));
        }

        port
    }
}

impl ILayoutProcessor<LGraph> for CompoundGraphPreprocessor {
    fn process(&mut self, _graph: &mut LGraph, _progress_monitor: &mut dyn IElkProgressMonitor) {
        panic!("CompoundGraphPreprocessor::process must be called via process_with_ref");
    }
}

#[derive(Default)]
pub struct CompoundGraphPostprocessor;

impl CompoundGraphPostprocessor {
    pub fn new() -> Self {
        CompoundGraphPostprocessor
    }

    pub fn process_with_ref(&mut self, graph: &LGraphRef, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Compound graph postprocessor", 1.0);

        let (add_unnecessary_bendpoints, cross_hierarchy_map) = {
            let mut graph_guard = graph.lock();
            let add = graph_guard
                .get_property(LayeredOptions::UNNECESSARY_BENDPOINTS)
                .unwrap_or(false);
            let map = graph_guard.get_property(InternalProperties::CROSS_HIERARCHY_MAP);
            (add, map)
        };
        let Some(cross_hierarchy_map) = cross_hierarchy_map else {
            monitor.done();
            return;
        };

        let mut dummy_edges: Vec<LEdgeRef> = Vec::new();
        let mut seen_edges: HashSet<usize> = HashSet::new();

        for orig_edge in cross_hierarchy_map.keys() {
            let mut cross_edges = cross_hierarchy_map.get(&orig_edge);
            if cross_edges.is_empty() {
                continue;
            }
            sort_cross_hierarchy_edges(&mut cross_edges, graph);

            let trace_edge = ElkTrace::global().trace
                && (cross_edges.iter().any(|edge| {
                    edge.edge()
                        .lock_ok()
                        .map(|edge_guard| {
                            edge_guard.labels().iter().any(|label_ref| {
                                label_ref
                                    .lock_ok()
                                    .map(|label_guard| label_guard.text() == "e2")
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(false)
                }) || orig_edge
                    .lock_ok()
                    .map(|edge_guard| {
                        edge_guard.labels().iter().any(|label_ref| {
                            label_ref
                                .lock_ok()
                                .map(|label_guard| label_guard.text() == "e2")
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false));

            let source_port = cross_edges.first().and_then(|edge| edge.actual_source());
            let target_port = cross_edges.last().and_then(|edge| edge.actual_target());
            if trace_edge {
                eprintln!(
                    "[compound-post] e2: segments={} source_present={} target_present={}",
                    cross_edges.len(),
                    source_port.is_some(),
                    target_port.is_some()
                );
            }

            let (Some(source_port), Some(target_port)) = (source_port, target_port) else {
                continue;
            };

            let reference_node = source_port
                .lock().node()
                .expect("source port missing node");
            let target_node = target_port
                .lock().node()
                .expect("target port missing node");
            let reference_graph = reference_node.lock_ok().and_then(|node| {
                if LGraphUtil::is_descendant(&target_node, &reference_node) {
                    node.nested_graph()
                } else {
                    node.graph()
                }
            });
            let Some(reference_graph) = reference_graph else {
                continue;
            };

            let has_junction_points = cross_edges.iter().any(|edge| edge.has_junction_points());
            let mut junction_points = if has_junction_points {
                Some(KVectorChain::new())
            } else {
                None
            };

            {
                let mut edge_guard = orig_edge.lock();
                if has_junction_points {
                    edge_guard
                        .set_property(LayeredOptions::JUNCTION_POINTS, Some(KVectorChain::new()));
                } else {
                    edge_guard.set_property(LayeredOptions::JUNCTION_POINTS, None);
                }
                edge_guard.bend_points().clear();
            }

            let mut last_point: Option<KVector> = None;

            for cross_edge in &cross_edges {
                let mut offset = KVector::new();
                LGraphUtil::change_coord_system(&mut offset, cross_edge.graph(), &reference_graph);

                let ledge = cross_edge.edge().clone();
                let (bend_points, source_point, target_point) = {
                    let edge_guard = ledge.lock();
                    let mut bend_points = edge_guard.bend_points_ref().clone();
                    bend_points.offset(offset.x, offset.y);

                    let source_point = edge_guard
                        .source()
                        .and_then(|port| port.lock().absolute_anchor())
                        .unwrap_or_default();
                    let target_point = edge_guard
                        .target()
                        .and_then(|port| port.lock().absolute_anchor())
                        .unwrap_or_default();

                    let mut source_point = source_point;
                    let mut target_point = target_point;
                    source_point.add(&offset);
                    target_point.add(&offset);

                    (bend_points, source_point, target_point)
                };

                if let Some(last_point) = last_point {
                    let next_point = if bend_points.is_empty() {
                        target_point
                    } else {
                        bend_points.get_first()
                    };

                    let x_diff_enough =
                        (last_point.x - next_point.x).abs() > OrthogonalRoutingGenerator::TOLERANCE;
                    let y_diff_enough =
                        (last_point.y - next_point.y).abs() > OrthogonalRoutingGenerator::TOLERANCE;

                    let should_add = if add_unnecessary_bendpoints {
                        x_diff_enough || y_diff_enough
                    } else {
                        x_diff_enough && y_diff_enough
                    };
                    {
                        let mut edge_guard = orig_edge.lock();
                        if should_add {
                            edge_guard.bend_points().add_vector(source_point);
                        }
                        edge_guard.bend_points().add_all(&bend_points.to_array());
                    }
                } else {
                    let mut edge_guard = orig_edge.lock();
                    edge_guard.bend_points().add_all(&bend_points.to_array());
                }

                last_point = if bend_points.is_empty() {
                    Some(source_point)
                } else {
                    Some(bend_points.get_last())
                };

                copy_junction_points(&ledge, &mut junction_points, &offset);

                if let Some(actual_target) = cross_edge.actual_target() {
                    if Arc::ptr_eq(&actual_target, &target_port) {
                        let target_graph = target_port
                            .lock().node()
                            .and_then(|node| node.lock().graph());
                        let mut target_offset = offset;
                        if let Some(target_graph) = target_graph {
                            if !Arc::ptr_eq(&target_graph, cross_edge.graph()) {
                                target_offset = KVector::new();
                                LGraphUtil::change_coord_system(
                                    &mut target_offset,
                                    &target_graph,
                                    &reference_graph,
                                );
                            }
                        }
                        {
                            let mut edge_guard = orig_edge.lock();
                            edge_guard.set_property(
                                InternalProperties::TARGET_OFFSET,
                                Some(target_offset),
                            );
                        }
                    }
                }

                copy_labels_back(&ledge, &orig_edge, &reference_graph);

                let edge_key = Arc::as_ptr(&ledge) as usize;
                if seen_edges.insert(edge_key) {
                    dummy_edges.push(ledge);
                }
            }

            LEdge::set_source(&orig_edge, Some(source_port.clone()));
            LEdge::set_target(&orig_edge, Some(target_port.clone()));
            if let Some(jps) = junction_points {
                {
                    let mut edge_guard = orig_edge.lock();
                    edge_guard.set_property(LayeredOptions::JUNCTION_POINTS, Some(jps));
                }
            }
        }

        for dummy_edge in dummy_edges {
            LEdge::set_source(&dummy_edge, None);
            LEdge::set_target(&dummy_edge, None);
        }

        monitor.done();
    }
}

impl ILayoutProcessor<LGraph> for CompoundGraphPostprocessor {
    fn process(&mut self, _graph: &mut LGraph, _progress_monitor: &mut dyn IElkProgressMonitor) {
        panic!("CompoundGraphPostprocessor::process must be called via process_with_ref");
    }
}

fn copy_junction_points(
    source_edge: &LEdgeRef,
    target: &mut Option<KVectorChain>,
    offset: &KVector,
) {
    let Some(target_chain) = target else { return };
    let jps = source_edge
        .lock_ok()
        .and_then(|mut edge_guard| edge_guard.get_property(LayeredOptions::JUNCTION_POINTS));
    if let Some(mut jps) = jps {
        jps.offset(offset.x, offset.y);
        target_chain.add_all(&jps.to_array());
    }
}

fn copy_labels_back(
    hierarchy_segment: &LEdgeRef,
    orig_edge: &LEdgeRef,
    reference_graph: &LGraphRef,
) {
    let segment_source_graph = hierarchy_segment
        .lock().source()
        .and_then(|port| port.lock().node())
        .and_then(|node| node.lock().graph());
    let Some(segment_source_graph) = segment_source_graph else {
        return;
    };

    let mut labels_to_move: Vec<LLabelRef> = Vec::new();
    {
        let mut edge_guard = hierarchy_segment.lock();
        let mut idx = 0;
        while idx < edge_guard.labels().len() {
            let label_ref = edge_guard.labels()[idx].clone();
            let matches = label_ref
                .lock_ok()
                .and_then(|mut label_guard| {
                    label_guard.get_property(InternalProperties::ORIGINAL_LABEL_EDGE)
                })
                .map(|edge| Arc::ptr_eq(&edge, orig_edge))
                .unwrap_or(false);
            if matches {
                edge_guard.labels_mut().remove(idx);
                labels_to_move.push(label_ref);
            } else {
                idx += 1;
            }
        }
    }

    for label in labels_to_move {
        {
            let mut label_guard = label.lock();
            LGraphUtil::change_coord_system(
                label_guard.shape().position(),
                &segment_source_graph,
                reference_graph,
            );
        }
        {
            let mut edge_guard = orig_edge.lock();
            edge_guard.labels_mut().push(label);
        }
    }
}

fn create_external_port_properties(graph: &LGraphRef) -> MapPropertyHolder {
    let mut property_holder = MapPropertyHolder::new();
    let spacing = graph
        .lock_ok()
        .and_then(|mut graph_guard| graph_guard.get_property(LayeredOptions::SPACING_EDGE_EDGE))
        .or_else(|| LayeredOptions::SPACING_EDGE_EDGE.get_default())
        .unwrap_or(0.0);
    property_holder.set_property(LayeredOptions::PORT_BORDER_OFFSET, Some(spacing / 2.0));
    property_holder
}

fn port_property_holder(port: &LPortRef) -> MapPropertyHolder {
    let mut property_holder = MapPropertyHolder::new();
    {
        let mut port_guard = port.lock();
        if let Some(value) = port_guard.get_property(LayeredOptions::PORT_BORDER_OFFSET) {
            property_holder.set_property(LayeredOptions::PORT_BORDER_OFFSET, Some(value));
        }
        if let Some(value) = port_guard.get_property(LayeredOptions::PORT_ANCHOR) {
            property_holder.set_property(LayeredOptions::PORT_ANCHOR, Some(value));
        }
        if let Some(value) = port_guard.get_property(LayeredOptions::PORT_INDEX) {
            property_holder.set_property(LayeredOptions::PORT_INDEX, Some(value));
        }
    }
    property_holder
}

fn create_dummy_edge(orig_edge: &LEdgeRef) -> LEdgeRef {
    let dummy_edge = LEdge::new();
    {
        let mut orig_guard = orig_edge.lock();
        let props = orig_guard.graph_element().properties().clone();
        {
            let mut dummy_guard = dummy_edge.lock();
            dummy_guard
                .graph_element()
                .properties_mut()
                .copy_properties(&props);
            dummy_guard.set_property(LayeredOptions::JUNCTION_POINTS, None);
            dummy_guard.set_property(
                InternalProperties::ORIGIN,
                Some(Origin::LEdge(orig_edge.clone())),
            );
        }
    }
    dummy_edge
}
