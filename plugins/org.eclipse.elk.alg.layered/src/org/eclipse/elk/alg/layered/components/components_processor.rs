use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;

use crate::org::eclipse::elk::alg::layered::components::ComponentOrderingStrategy;
use crate::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphRef, LGraphUtil, LNode, LNodeRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::{GraphProperties, InternalProperties, LayeredOptions};

#[derive(Default)]
pub struct ComponentsProcessor;

impl ComponentsProcessor {
    pub fn new() -> Self {
        ComponentsProcessor
    }

    pub fn split(&self, graph: &LGraphRef) -> Vec<LGraphRef> {
        let (separate, ext_ports, ext_port_constraints, consider_model_order, nodes, graph_props, graph_padding) =
            if let Ok(mut graph_guard) = graph.lock() {
                (
                    graph_guard
                        .get_property(LayeredOptions::SEPARATE_CONNECTED_COMPONENTS)
                        .unwrap_or(true),
                    graph_guard
                        .get_property(InternalProperties::GRAPH_PROPERTIES)
                        .unwrap_or_else(EnumSet::none_of)
                        .contains(&GraphProperties::ExternalPorts),
                    graph_guard
                        .get_property(LayeredOptions::PORT_CONSTRAINTS)
                        .unwrap_or(PortConstraints::Undefined),
                    graph_guard
                        .get_property(LayeredOptions::CONSIDER_MODEL_ORDER_COMPONENTS)
                        .unwrap_or(ComponentOrderingStrategy::None),
                    graph_guard.layerless_nodes().clone(),
                    graph_guard.graph_element().properties().clone(),
                    graph_guard.padding_ref().clone(),
                )
            } else {
                return vec![graph.clone()];
            };

        let compatible_port_constraints = !ext_port_constraints.is_order_fixed();
        if !(separate && (compatible_port_constraints || !ext_ports)) {
            return vec![graph.clone()];
        }

        let mut result: Vec<LGraphRef> = Vec::new();
        let mut visited: HashSet<usize> = HashSet::new();
        for node in &nodes {
            let key = Arc::as_ptr(node) as usize;
            if visited.contains(&key) {
                continue;
            }

            let mut component_nodes: Vec<LNodeRef> = Vec::new();
            let mut ext_port_sides: EnumSet<PortSide> = EnumSet::none_of();
            Self::dfs(node, &mut visited, &mut component_nodes, &mut ext_port_sides);

            if component_nodes.is_empty() {
                continue;
            }

            if result.is_empty() && component_nodes.len() == nodes.len() {
                return vec![graph.clone()];
            }

            let component_graph = LGraph::new();
            if let Ok(mut component_guard) = component_graph.lock() {
                *component_guard.graph_element().properties_mut() = graph_props.clone();
                component_guard.set_property(
                    InternalProperties::EXT_PORT_CONNECTIONS,
                    Some(ext_port_sides),
                );
                *component_guard.padding() = graph_padding.clone();
                component_guard.set_property(LayeredOptions::NODE_SIZE_MINIMUM, None::<org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector>);

                for component_node in &component_nodes {
                    component_guard.layerless_nodes_mut().push(component_node.clone());
                    if let Ok(mut node_guard) = component_node.lock() {
                        node_guard.set_graph(&component_graph);
                    }
                }
            }

            result.push(component_graph);
        }

        if consider_model_order != ComponentOrderingStrategy::None {
            result.sort_by_key(LGraphUtil::get_minimal_model_order);
        }

        result
    }

    pub fn combine(&self, components: &[LGraphRef], target: &LGraphRef) {
        if components.len() == 1 {
            let source = components.first().cloned();
            if let Some(source) = source {
                if Arc::ptr_eq(&source, target) {
                    return;
                }

                if let Ok(mut target_guard) = target.lock() {
                    target_guard.layerless_nodes_mut().clear();
                }
                move_graph(target, &source, 0.0, 0.0);

                if let (Ok(mut target_guard), Ok(mut source_guard)) = (target.lock(), source.lock()) {
                    *target_guard.graph_element().properties_mut() =
                        source_guard.graph_element().properties().clone();
                    *target_guard.padding() = source_guard.padding_ref().clone();
                    target_guard.size().x = source_guard.size_ref().x;
                    target_guard.size().y = source_guard.size_ref().y;
                }
            }
            return;
        }

        if components.is_empty() {
            if let Ok(mut target_guard) = target.lock() {
                target_guard.layerless_nodes_mut().clear();
                target_guard.size().x = 0.0;
                target_guard.size().y = 0.0;
            }
            return;
        }

        let mut ordered_components = components.to_vec();
        sort_components_by_priority(&mut ordered_components, target);

        if let Ok(mut target_guard) = target.lock() {
            target_guard.layerless_nodes_mut().clear();
        }

        if let Some(first_component) = ordered_components.first() {
            if let (Ok(mut target_guard), Ok(mut first_guard)) = (target.lock(), first_component.lock()) {
                *target_guard.graph_element().properties_mut() =
                    first_guard.graph_element().properties().clone();
            }
        }

        let (aspect_ratio, component_spacing) = if let Ok(mut target_guard) = target.lock() {
            (
                target_guard
                    .get_property(LayeredOptions::ASPECT_RATIO)
                    .unwrap_or(1.6),
                target_guard
                    .get_property(LayeredOptions::SPACING_COMPONENT_COMPONENT)
                    .unwrap_or(20.0),
            )
        } else {
            (1.6, 20.0)
        };

        let mut max_row_width = 0.0f64;
        let mut total_area = 0.0f64;
        for component in &ordered_components {
            if let Ok(component_guard) = component.lock() {
                let size = component_guard.size_ref();
                max_row_width = max_row_width.max(size.x);
                total_area += size.x * size.y;
            }
        }
        max_row_width = max_row_width.max(total_area.sqrt() * aspect_ratio);

        place_components_in_rows(
            &ordered_components,
            target,
            max_row_width,
            component_spacing,
        );

        for component in &ordered_components {
            move_graph(target, component, 0.0, 0.0);
        }
    }

    fn dfs(
        node: &LNodeRef,
        visited: &mut HashSet<usize>,
        component_nodes: &mut Vec<LNodeRef>,
        ext_port_sides: &mut EnumSet<PortSide>,
    ) {
        let key = Arc::as_ptr(node) as usize;
        if !visited.insert(key) {
            return;
        }
        component_nodes.push(node.clone());

        let mut connected_nodes: Vec<LNodeRef> = Vec::new();
        if let Ok(mut node_guard) = node.lock() {
            if node_guard.node_type() == NodeType::ExternalPort {
                if let Some(side) = node_guard.get_property(InternalProperties::EXT_PORT_SIDE) {
                    ext_port_sides.insert(side);
                }
            }

            for port in node_guard.ports().clone() {
                if let Ok(port_guard) = port.lock() {
                    for connected_port in port_guard.connected_ports() {
                        if let Some(connected_node) =
                            connected_port.lock().ok().and_then(|port| port.node())
                        {
                            connected_nodes.push(connected_node);
                        }
                    }
                }
            }
        }

        for connected_node in connected_nodes {
            Self::dfs(&connected_node, visited, component_nodes, ext_port_sides);
        }
    }
}

fn sort_components_by_priority(components: &mut [LGraphRef], target: &LGraphRef) {
    let consider_model_order = target
        .lock()
        .ok()
        .and_then(|mut target_guard| target_guard.get_property(LayeredOptions::CONSIDER_MODEL_ORDER_COMPONENTS))
        .unwrap_or(ComponentOrderingStrategy::None);

    if consider_model_order != ComponentOrderingStrategy::None {
        components.sort_by_key(LGraphUtil::get_minimal_model_order);
        return;
    }

    components.sort_by(|left, right| {
        let (left_priority, left_area) = component_priority_and_area(left);
        let (right_priority, right_area) = component_priority_and_area(right);

        right_priority
            .cmp(&left_priority)
            .then_with(|| left_area.partial_cmp(&right_area).unwrap_or(Ordering::Equal))
    });
}

fn component_priority_and_area(graph: &LGraphRef) -> (i32, f64) {
    let priority = collect_component_nodes(graph)
        .iter()
        .filter_map(|node| {
            node.lock()
                .ok()
                .and_then(|mut node_guard| node_guard.get_property(CoreOptions::PRIORITY))
        })
        .sum::<i32>();

    if let Ok(graph_guard) = graph.lock() {
        let size = graph_guard.size_ref();
        (priority, size.x * size.y)
    } else {
        (priority, 0.0)
    }
}

fn place_components_in_rows(
    components: &[LGraphRef],
    target: &LGraphRef,
    max_row_width: f64,
    component_spacing: f64,
) {
    let mut xpos = 0.0f64;
    let mut ypos = 0.0f64;
    let mut highest_box = 0.0f64;
    let mut broadest_row = component_spacing;

    for component in components {
        let (size_x, size_y, offset_x, offset_y) = if let Ok(graph_guard) = component.lock() {
            (
                graph_guard.size_ref().x,
                graph_guard.size_ref().y,
                graph_guard.offset_ref().x,
                graph_guard.offset_ref().y,
            )
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

        if xpos + size_x > max_row_width {
            xpos = 0.0;
            ypos += highest_box + component_spacing;
            highest_box = 0.0;
        }

        offset_graph(component, xpos + offset_x, ypos + offset_y);
        if let Ok(mut graph_guard) = component.lock() {
            graph_guard.offset().x = 0.0;
            graph_guard.offset().y = 0.0;
        }

        broadest_row = broadest_row.max(xpos + size_x);
        highest_box = highest_box.max(size_y);
        xpos += size_x + component_spacing;
    }

    if let Ok(mut target_guard) = target.lock() {
        target_guard.size().x = broadest_row;
        target_guard.size().y = ypos + highest_box;
    }
}

fn move_graph(destination: &LGraphRef, source: &LGraphRef, offset_x: f64, offset_y: f64) {
    let (graph_offset_x, graph_offset_y) = if let Ok(source_guard) = source.lock() {
        (
            source_guard.offset_ref().x + offset_x,
            source_guard.offset_ref().y + offset_y,
        )
    } else {
        (offset_x, offset_y)
    };
    let source_nodes = collect_component_nodes(source);

    for node in source_nodes {
        shift_node_and_outgoing_edges(&node, graph_offset_x, graph_offset_y);
        LNode::set_layer(&node, None);
        if let Ok(mut destination_guard) = destination.lock() {
            destination_guard.layerless_nodes_mut().push(node.clone());
        }
        if let Ok(mut node_guard) = node.lock() {
            node_guard.set_graph(destination);
        }
    }
}

fn offset_graph(graph: &LGraphRef, offset_x: f64, offset_y: f64) {
    let nodes = collect_component_nodes(graph);

    for node in nodes {
        shift_node_and_outgoing_edges(&node, offset_x, offset_y);
    }
}

fn collect_component_nodes(graph: &LGraphRef) -> Vec<LNodeRef> {
    let (layerless_nodes, layers) = if let Ok(graph_guard) = graph.lock() {
        (
            graph_guard.layerless_nodes().clone(),
            graph_guard.layers().clone(),
        )
    } else {
        return Vec::new();
    };

    let mut seen: HashSet<usize> = HashSet::new();
    let mut nodes: Vec<LNodeRef> = Vec::new();

    for node in layerless_nodes {
        let key = Arc::as_ptr(&node) as usize;
        if seen.insert(key) {
            nodes.push(node);
        }
    }

    for layer in layers {
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

fn shift_node_and_outgoing_edges(node: &LNodeRef, offset_x: f64, offset_y: f64) {
    let ports = if let Ok(mut node_guard) = node.lock() {
        let position = node_guard.shape().position();
        position.x += offset_x;
        position.y += offset_y;
        node_guard.ports().clone()
    } else {
        Vec::new()
    };

    for port in ports {
        let edges = port
            .lock()
            .ok()
            .map(|port_guard| port_guard.outgoing_edges().clone())
            .unwrap_or_default();

        for edge in edges {
            if let Ok(mut edge_guard) = edge.lock() {
                edge_guard.bend_points().offset(offset_x, offset_y);
                if let Some(mut junction_points) = edge_guard.get_property(LayeredOptions::JUNCTION_POINTS) {
                    junction_points.offset(offset_x, offset_y);
                    edge_guard.set_property(LayeredOptions::JUNCTION_POINTS, Some(junction_points));
                }

                let labels = edge_guard.labels().clone();
                drop(edge_guard);
                for label in labels {
                    if let Ok(mut label_guard) = label.lock() {
                        let position = label_guard.shape().position();
                        position.x += offset_x;
                        position.y += offset_y;
                    }
                }
            }
        }
    }
}
