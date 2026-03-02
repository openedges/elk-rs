use rustc_hash::FxHashMap;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LNode, LNodeRef, LPortRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;

pub struct HyperedgeDummyMerger;

impl ILayoutProcessor<LGraph> for HyperedgeDummyMerger {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Hyperedge merging", 1.0);

        let hyperedge_ids = identify_hyperedges(layered_graph);
        let layers = layered_graph.layers().clone();

        for layer in layers {
            let mut node_index = 1usize;
            loop {
                let pair = {
                    let Ok(layer_guard) = layer.lock() else {
                        break;
                    };
                    if node_index >= layer_guard.nodes().len() {
                        None
                    } else {
                        Some((
                            layer_guard.nodes()[node_index].clone(),
                            layer_guard.nodes()[node_index - 1].clone(),
                        ))
                    }
                };
                let Some((curr_node, last_node)) = pair else {
                    break;
                };

                if is_long_edge_dummy(&curr_node) && is_long_edge_dummy(&last_node) {
                    let state = check_merge_allowed(&curr_node, &last_node, &hyperedge_ids);
                    if state.allow_merge {
                        merge_nodes(&curr_node, &last_node, state.same_source, state.same_target);
                        LNode::set_layer(&curr_node, None);
                        // Re-check at this index because the current node was removed.
                        continue;
                    }
                }

                node_index += 1;
            }
        }

        monitor.done();
    }
}

#[derive(Clone, Copy)]
struct MergeState {
    allow_merge: bool,
    same_source: bool,
    same_target: bool,
}

fn is_long_edge_dummy(node: &LNodeRef) -> bool {
    node.lock()
        .ok()
        .map(|node_guard| node_guard.node_type() == NodeType::LongEdge)
        .unwrap_or(false)
}

fn port_key(port: &LPortRef) -> usize {
    Arc::as_ptr(port) as usize
}

fn check_merge_allowed(
    curr_node: &LNodeRef,
    last_node: &LNodeRef,
    hyperedge_ids: &FxHashMap<usize, i32>,
) -> MergeState {
    let curr_has_label_dummies = curr_node
        .lock()
        .ok()
        .and_then(|mut node_guard| {
            node_guard.get_property(InternalProperties::LONG_EDGE_HAS_LABEL_DUMMIES)
        })
        .unwrap_or(false);
    let last_has_label_dummies = last_node
        .lock()
        .ok()
        .and_then(|mut node_guard| {
            node_guard.get_property(InternalProperties::LONG_EDGE_HAS_LABEL_DUMMIES)
        })
        .unwrap_or(false);

    let curr_source = curr_node
        .lock()
        .ok()
        .and_then(|mut node_guard| node_guard.get_property(InternalProperties::LONG_EDGE_SOURCE));
    let last_source = last_node
        .lock()
        .ok()
        .and_then(|mut node_guard| node_guard.get_property(InternalProperties::LONG_EDGE_SOURCE));
    let curr_target = curr_node
        .lock()
        .ok()
        .and_then(|mut node_guard| node_guard.get_property(InternalProperties::LONG_EDGE_TARGET));
    let last_target = last_node
        .lock()
        .ok()
        .and_then(|mut node_guard| node_guard.get_property(InternalProperties::LONG_EDGE_TARGET));

    let same_source = match (curr_source.as_ref(), last_source.as_ref()) {
        (Some(curr), Some(last)) => Arc::ptr_eq(curr, last),
        _ => false,
    };
    let same_target = match (curr_target.as_ref(), last_target.as_ref()) {
        (Some(curr), Some(last)) => Arc::ptr_eq(curr, last),
        _ => false,
    };

    if !curr_has_label_dummies && !last_has_label_dummies {
        let curr_first_port = curr_node
            .lock()
            .ok()
            .and_then(|node_guard| node_guard.ports().first().cloned());
        let last_first_port = last_node
            .lock()
            .ok()
            .and_then(|node_guard| node_guard.ports().first().cloned());

        let allow_merge = match (curr_first_port, last_first_port) {
            (Some(curr_port), Some(last_port)) => {
                let curr_id = hyperedge_ids.get(&port_key(&curr_port));
                let last_id = hyperedge_ids.get(&port_key(&last_port));
                curr_id.is_some() && curr_id == last_id
            }
            _ => false,
        };

        return MergeState {
            allow_merge,
            same_source,
            same_target,
        };
    }

    let curr_before_label_dummy = curr_node
        .lock()
        .ok()
        .and_then(|mut node_guard| {
            node_guard.get_property(InternalProperties::LONG_EDGE_BEFORE_LABEL_DUMMY)
        })
        .unwrap_or(false);
    let last_before_label_dummy = last_node
        .lock()
        .ok()
        .and_then(|mut node_guard| {
            node_guard.get_property(InternalProperties::LONG_EDGE_BEFORE_LABEL_DUMMY)
        })
        .unwrap_or(false);

    let eligible_for_source_merging = (!curr_has_label_dummies || curr_before_label_dummy)
        && (!last_has_label_dummies || last_before_label_dummy);
    let eligible_for_target_merging = (!curr_has_label_dummies || !curr_before_label_dummy)
        && (!last_has_label_dummies || !last_before_label_dummy);

    MergeState {
        allow_merge: (same_source && eligible_for_source_merging)
            || (same_target && eligible_for_target_merging),
        same_source,
        same_target,
    }
}

fn merge_nodes(
    merge_source: &LNodeRef,
    merge_target: &LNodeRef,
    keep_source_port: bool,
    keep_target_port: bool,
) {
    let merge_target_input_port = merge_target
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.ports_by_side(PortSide::West).first().cloned());
    let merge_target_output_port = merge_target
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.ports_by_side(PortSide::East).first().cloned());
    let (Some(merge_target_input_port), Some(merge_target_output_port)) =
        (merge_target_input_port, merge_target_output_port)
    else {
        return;
    };

    let ports = merge_source
        .lock()
        .ok()
        .map(|node_guard| node_guard.ports().clone())
        .unwrap_or_default();

    for port in ports {
        loop {
            let edge = port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.incoming_edges().first().cloned());
            let Some(edge) = edge else {
                break;
            };
            LEdge::set_target(&edge, Some(merge_target_input_port.clone()));
        }

        loop {
            let edge = port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.outgoing_edges().first().cloned());
            let Some(edge) = edge else {
                break;
            };
            LEdge::set_source(&edge, Some(merge_target_output_port.clone()));
        }
    }

    if !keep_source_port {
        if let Ok(mut target_guard) = merge_target.lock() {
            target_guard.set_property(InternalProperties::LONG_EDGE_SOURCE, None::<LPortRef>);
        }
    }
    if !keep_target_port {
        if let Ok(mut target_guard) = merge_target.lock() {
            target_guard.set_property(InternalProperties::LONG_EDGE_TARGET, None::<LPortRef>);
        }
    }
}

fn identify_hyperedges(layered_graph: &LGraph) -> FxHashMap<usize, i32> {
    let mut ports: Vec<LPortRef> = Vec::new();
    for layer in layered_graph.layers() {
        let layer_ports = layer
            .lock()
            .ok()
            .map(|layer_guard| {
                layer_guard
                    .nodes()
                    .iter()
                    .flat_map(|node| {
                        node.lock()
                            .ok()
                            .map(|node_guard| node_guard.ports().clone())
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        ports.extend(layer_ports);
    }

    let mut component_ids: FxHashMap<usize, i32> = FxHashMap::default();
    let mut next_id = 0i32;
    for port in ports {
        if component_ids.contains_key(&port_key(&port)) {
            continue;
        }
        mark_hyperedge_component(&port, next_id, &mut component_ids);
        next_id += 1;
    }
    component_ids
}

fn mark_hyperedge_component(
    start: &LPortRef,
    component_id: i32,
    component_ids: &mut FxHashMap<usize, i32>,
) {
    let mut stack: Vec<LPortRef> = vec![start.clone()];
    while let Some(port) = stack.pop() {
        let key = port_key(&port);
        if component_ids.contains_key(&key) {
            continue;
        }
        component_ids.insert(key, component_id);

        let connected_ports = port
            .lock()
            .ok()
            .map(|port_guard| port_guard.connected_ports())
            .unwrap_or_default();
        for connected in connected_ports {
            if !component_ids.contains_key(&port_key(&connected)) {
                stack.push(connected);
            }
        }

        let (is_long_edge, node_ports) = port
            .lock()
            .ok()
            .and_then(|port_guard| port_guard.node())
            .and_then(|node| {
                node.lock().ok().map(|node_guard| {
                    (
                        node_guard.node_type() == NodeType::LongEdge,
                        node_guard.ports().clone(),
                    )
                })
            })
            .unwrap_or((false, Vec::new()));
        if is_long_edge {
            for sibling_port in node_ports {
                if !Arc::ptr_eq(&sibling_port, &port)
                    && !component_ids.contains_key(&port_key(&sibling_port))
                {
                    stack.push(sibling_port);
                }
            }
        }
    }
}
