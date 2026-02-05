use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkEdgeRef, ElkNodeRef, ElkPortRef,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

pub struct ElkGraphComponentsProcessor;

impl ElkGraphComponentsProcessor {
    pub fn split(graph: &ElkNodeRef) -> Vec<Vec<ElkNodeRef>> {
        let children: Vec<ElkNodeRef> = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.children().iter().cloned().collect()
        };

        let incidence_map = compute_incidences(&children);
        let mut visited: HashSet<usize> = HashSet::new();
        let mut components: Vec<Vec<ElkNodeRef>> = Vec::new();

        for node in &children {
            let id = node_id(node);
            if !visited.contains(&id) {
                let mut component: Vec<ElkNodeRef> = Vec::new();
                dfs(node, &incidence_map, &mut visited, &mut component);
                components.push(component);
            }
        }

        components
    }
}

fn compute_incidences(nodes: &[ElkNodeRef]) -> HashMap<usize, Vec<ElkNodeRef>> {
    let mut incidence_map: HashMap<usize, Vec<ElkNodeRef>> = HashMap::new();
    let mut adjacent_and_inside_parent: HashMap<usize, Vec<ElkNodeRef>> = HashMap::new();

    for node in nodes {
        let mut adjacent_ids: HashSet<usize> = HashSet::new();
        let mut adjacent_nodes: Vec<ElkNodeRef> = Vec::new();

        let incoming_edges = ElkGraphUtil::all_incoming_edges(node);
        for edge in &incoming_edges {
            if same_hierarchy_level(edge) {
                add_adjacent(&mut adjacent_nodes, &mut adjacent_ids, get_source_node(edge));
            }
        }

        for edge in &incoming_edges {
            if same_hierarchy_level(edge) {
                continue;
            }
            let source_node = get_source_node(edge);
            let target_node = get_target_node(edge);
            let target_parent = target_node.borrow().parent();
            if let Some(target_parent) = target_parent {
                if Rc::ptr_eq(&source_node, &target_parent) {
                    if let Some(port) = get_source_port(edge) {
                        let port_id = port_id(&port);
                        let nodes_at_port = adjacent_and_inside_parent
                            .entry(port_id)
                            .or_insert_with(|| get_inner_neighbors_of_port(&port));
                        for adj in nodes_at_port.clone() {
                            add_adjacent(&mut adjacent_nodes, &mut adjacent_ids, adj);
                        }
                    }
                }
            }
        }

        let outgoing_edges = ElkGraphUtil::all_outgoing_edges(node);
        for edge in &outgoing_edges {
            if same_hierarchy_level(edge) {
                add_adjacent(&mut adjacent_nodes, &mut adjacent_ids, get_target_node(edge));
            }
        }

        for edge in &outgoing_edges {
            if same_hierarchy_level(edge) {
                continue;
            }
            let source_node = get_source_node(edge);
            let source_parent = source_node.borrow().parent();
            let target_node = get_target_node(edge);
            if let Some(source_parent) = source_parent {
                if Rc::ptr_eq(&target_node, &source_parent) {
                    if let Some(port) = get_target_port(edge) {
                        let port_id = port_id(&port);
                        let nodes_at_port = adjacent_and_inside_parent
                            .entry(port_id)
                            .or_insert_with(|| get_inner_neighbors_of_port(&port));
                        for adj in nodes_at_port.clone() {
                            add_adjacent(&mut adjacent_nodes, &mut adjacent_ids, adj);
                        }
                    }
                }
            }
        }

        incidence_map.insert(node_id(node), adjacent_nodes);
    }

    incidence_map
}

fn get_inner_neighbors_of_port(port: &ElkPortRef) -> Vec<ElkNodeRef> {
    let port_parent = match port.borrow().parent() {
        Some(parent) => parent,
        None => return Vec::new(),
    };

    let mut edges: Vec<ElkEdgeRef> = Vec::new();
    {
        let mut port_mut = port.borrow_mut();
        edges.extend(port_mut.connectable().incoming_edges().iter());
        edges.extend(port_mut.connectable().outgoing_edges().iter());
    }

    let mut result: Vec<ElkNodeRef> = Vec::new();
    let mut seen: HashSet<usize> = HashSet::new();

    for edge in edges {
        let source = get_source_node(&edge);
        let target = get_target_node(&edge);
        let source_parent = source.borrow().parent();
        let target_parent = target.borrow().parent();
        let inwards = match (source_parent, target_parent) {
            (Some(sp), _) if Rc::ptr_eq(&sp, &port_parent) => true,
            (_, Some(tp)) if Rc::ptr_eq(&tp, &port_parent) => true,
            _ => false,
        };
        if !inwards {
            continue;
        }
        let neighbor = if Rc::ptr_eq(&source, &port_parent) { target } else { source };
        add_adjacent(&mut result, &mut seen, neighbor);
    }

    result
}

fn dfs(
    start: &ElkNodeRef,
    incidence_map: &HashMap<usize, Vec<ElkNodeRef>>,
    visited: &mut HashSet<usize>,
    component: &mut Vec<ElkNodeRef>,
) {
    let id = node_id(start);
    visited.insert(id);
    component.push(start.clone());
    if let Some(adjacent_nodes) = incidence_map.get(&id) {
        for node in adjacent_nodes {
            let node_id = node_id(node);
            if !visited.contains(&node_id) {
                dfs(node, incidence_map, visited, component);
            }
        }
    }
}

fn add_adjacent(
    nodes: &mut Vec<ElkNodeRef>,
    ids: &mut HashSet<usize>,
    node: ElkNodeRef,
) {
    let id = node_id(&node);
    if ids.insert(id) {
        nodes.push(node);
    }
}

fn same_hierarchy_level(edge: &ElkEdgeRef) -> bool {
    let source = get_source_node(edge);
    let target = get_target_node(edge);
    let source_parent = source.borrow().parent();
    let target_parent = target.borrow().parent();
    match (source_parent, target_parent) {
        (Some(sp), Some(tp)) => Rc::ptr_eq(&sp, &tp),
        (None, None) => true,
        _ => false,
    }
}

fn get_source_node(edge: &ElkEdgeRef) -> ElkNodeRef {
    let (source, target) = edge_endpoints(edge);
    let _ = target;
    ElkGraphUtil::connectable_shape_to_node(&source)
        .expect("Passed edge is not 'simple'.")
}

fn get_target_node(edge: &ElkEdgeRef) -> ElkNodeRef {
    let (_source, target) = edge_endpoints(edge);
    ElkGraphUtil::connectable_shape_to_node(&target)
        .expect("Passed edge is not 'simple'.")
}

fn get_source_port(edge: &ElkEdgeRef) -> Option<ElkPortRef> {
    let (source, _target) = edge_endpoints(edge);
    ElkGraphUtil::connectable_shape_to_port(&source)
}

fn get_target_port(edge: &ElkEdgeRef) -> Option<ElkPortRef> {
    let (_source, target) = edge_endpoints(edge);
    ElkGraphUtil::connectable_shape_to_port(&target)
}

fn edge_endpoints(
    edge: &ElkEdgeRef,
) -> (
    org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef,
    org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef,
) {
    let edge_borrow = edge.borrow();
    if edge_borrow.sources_ro().len() != 1 || edge_borrow.targets_ro().len() != 1 {
        panic!("Passed edge is not 'simple'.");
    }
    let source = edge_borrow.sources_ro().get(0).expect("missing source").clone();
    let target = edge_borrow.targets_ro().get(0).expect("missing target").clone();
    (source, target)
}

fn node_id(node: &ElkNodeRef) -> usize {
    Rc::as_ptr(node) as usize
}

fn port_id(port: &ElkPortRef) -> usize {
    Rc::as_ptr(port) as usize
}
