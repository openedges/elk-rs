use std::collections::VecDeque;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LEdgeRef, LGraph, LNodeRef};
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;

const PARTITION_CONSTRAINT_EDGE_PRIORITY: i32 = 1_000;

pub struct PartitionPreprocessor;

impl ILayoutProcessor<LGraph> for PartitionPreprocessor {
    fn process(&mut self, lgraph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Partition preprocessing", 1.0);

        let partitioned_nodes: Vec<LNodeRef> = lgraph
            .layerless_nodes()
            .iter()
            .filter_map(|node| {
                if node_partition(node).is_some() {
                    Some(node.clone())
                } else {
                    None
                }
            })
            .collect();

        let mut edges_to_reverse: Vec<LEdgeRef> = Vec::new();
        for node in &partitioned_nodes {
            let Some(source_partition) = node_partition(node) else {
                continue;
            };
            let outgoing_edges = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();
            for edge in outgoing_edges {
                if must_be_reversed(&edge, source_partition, &partitioned_nodes) {
                    edges_to_reverse.push(edge);
                }
            }
        }

        for edge in edges_to_reverse {
            reverse_edge(&edge);
        }

        monitor.done();
    }
}

fn node_partition(node: &LNodeRef) -> Option<i32> {
    node.lock()
        .ok()
        .and_then(|mut node_guard| node_guard.get_property(CoreOptions::PARTITIONING_PARTITION))
}

fn must_be_reversed(
    edge: &LEdgeRef,
    source_partition: i32,
    partitioned_nodes: &[LNodeRef],
) -> bool {
    let target_node = edge
        .lock()
        .ok()
        .and_then(|edge_guard| edge_guard.target())
        .and_then(|target| target.lock().ok().and_then(|port_guard| port_guard.node()));
    let Some(target_node) = target_node else {
        return false;
    };

    if let Some(target_partition) = node_partition(&target_node) {
        return source_partition > target_partition;
    }

    let lower_partition_nodes: Vec<LNodeRef> = partitioned_nodes
        .iter()
        .filter_map(|node| {
            let partition = node_partition(node)?;
            if partition < source_partition {
                Some(node.clone())
            } else {
                None
            }
        })
        .collect();

    if lower_partition_nodes.is_empty() {
        return false;
    }

    let source_node = edge
        .lock()
        .ok()
        .and_then(|edge_guard| edge_guard.source())
        .and_then(|source| source.lock().ok().and_then(|port_guard| port_guard.node()));
    let Some(source_node) = source_node else {
        return false;
    };

    let mut queue = VecDeque::new();
    let mut visited: Vec<LNodeRef> = vec![source_node.clone()];
    queue.push_back(source_node);

    while let Some(current_node) = queue.pop_front() {
        if lower_partition_nodes
            .iter()
            .any(|candidate| Arc::ptr_eq(candidate, &current_node))
        {
            return true;
        }

        let outgoing = current_node
            .lock()
            .ok()
            .map(|node_guard| node_guard.outgoing_edges())
            .unwrap_or_default();
        for outgoing_edge in outgoing {
            let target = outgoing_edge
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.target())
                .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
            let Some(target) = target else {
                continue;
            };

            if visited
                .iter()
                .any(|visited_node| Arc::ptr_eq(visited_node, &target))
            {
                continue;
            }

            visited.push(target.clone());
            queue.push_back(target);
        }
    }

    false
}

fn reverse_edge(edge: &LEdgeRef) {
    let graph_ref = edge
        .lock()
        .ok()
        .and_then(|edge_guard| edge_guard.source())
        .and_then(|source| source.lock().ok().and_then(|port_guard| port_guard.node()))
        .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.graph()));
    let Some(graph_ref) = graph_ref else {
        return;
    };

    LEdge::reverse(edge, &graph_ref, true);

    if let Ok(mut edge_guard) = edge.lock() {
        let mut priority = PARTITION_CONSTRAINT_EDGE_PRIORITY;
        if let Some(existing_priority) = edge_guard.get_property(LayeredOptions::PRIORITY_DIRECTION)
        {
            priority += existing_priority;
        }
        edge_guard.set_property(LayeredOptions::PRIORITY_DIRECTION, Some(priority));
    }
}
