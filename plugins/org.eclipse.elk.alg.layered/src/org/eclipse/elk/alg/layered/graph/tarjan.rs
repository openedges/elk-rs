#![allow(clippy::mutable_key_type)]

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use crate::org::eclipse::elk::alg::layered::options::InternalProperties;

use super::{LEdgeRef, LGraphRef, LNodeRef};

#[derive(Clone)]
pub struct NodeRefKey(pub LNodeRef);

impl PartialEq for NodeRefKey {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for NodeRefKey {}

impl PartialOrd for NodeRefKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NodeRefKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_ptr = std::sync::Arc::as_ptr(&self.0) as usize;
        let other_ptr = std::sync::Arc::as_ptr(&other.0) as usize;
        self_ptr.cmp(&other_ptr)
    }
}

impl Hash for NodeRefKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = std::sync::Arc::as_ptr(&self.0) as usize;
        ptr.hash(state);
    }
}

pub struct Tarjan<'a> {
    edges_to_be_reversed: &'a [LEdgeRef],
    pub strongly_connected_components: &'a mut Vec<Vec<LNodeRef>>,
    pub node_to_scc_id: &'a mut BTreeMap<NodeRefKey, usize>,
    index: i32,
    stack: Vec<LNodeRef>,
}

impl<'a> Tarjan<'a> {
    pub fn new(
        edges_to_be_reversed: &'a [LEdgeRef],
        strongly_connected_components: &'a mut Vec<Vec<LNodeRef>>,
        node_to_scc_id: &'a mut BTreeMap<NodeRefKey, usize>,
    ) -> Self {
        Tarjan {
            edges_to_be_reversed,
            strongly_connected_components,
            node_to_scc_id,
            index: 0,
            stack: Vec::new(),
        }
    }

    pub fn tarjan(&mut self, graph: &LGraphRef) {
        self.index = 0;
        self.stack.clear();
        let nodes = graph
            .lock().layerless_nodes().clone();
        for node in nodes {
            let mut guard = node.lock();
            let id = guard
                .get_property(InternalProperties::TARJAN_ID)
                .unwrap_or(-1);
            drop(guard);
            if id == -1 {
                self.strongly_connected(&node);
                self.stack.clear();
            }
        }
    }

    fn edge_is_reversed(&self, edge: &LEdgeRef) -> bool {
        self.edges_to_be_reversed
            .iter()
            .any(|candidate| std::sync::Arc::ptr_eq(candidate, edge))
    }

    pub fn strongly_connected(&mut self, v: &LNodeRef) {
        {
            let mut guard = v.lock();
            guard.set_property(InternalProperties::TARJAN_ID, Some(self.index));
            guard.set_property(InternalProperties::TARJAN_LOWLINK, Some(self.index));
            self.index += 1;
            guard.set_property(InternalProperties::TARJAN_ON_STACK, Some(true));
        }
        self.stack.push(v.clone());

        let edges = v
            .lock().connected_edges();
        for edge in edges {
            let edge_guard = edge.lock();
            let source = edge_guard.source();
            let target = edge_guard.target();
            drop(edge_guard);

            let (source_node, target_node) = match (source, target) {
                (Some(source), Some(target)) => {
                    let source_node = source.lock().node();
                    let target_node = target.lock().node();
                    (source_node, target_node)
                }
                _ => (None, None),
            };

            let (source_node, target_node) = match (source_node, target_node) {
                (Some(source_node), Some(target_node)) => (source_node, target_node),
                _ => continue,
            };

            let source_is_v = std::sync::Arc::ptr_eq(&source_node, v);
            let reversed = self.edge_is_reversed(&edge);
            if !source_is_v && !reversed {
                continue;
            }
            if source_is_v && reversed {
                continue;
            }

            let target = if std::sync::Arc::ptr_eq(&target_node, v) {
                source_node
            } else {
                target_node
            };

            let target_id = target
                .lock()
                .get_property(InternalProperties::TARJAN_ID)
                .unwrap_or(-1);
            if target_id == -1 {
                self.strongly_connected(&target);
                let target_lowlink = target
                    .lock()
                    .get_property(InternalProperties::TARJAN_LOWLINK)
                    .unwrap_or(i32::MAX);
                {
                    let mut guard = v.lock();
                    let current = guard
                        .get_property(InternalProperties::TARJAN_LOWLINK)
                        .unwrap_or(i32::MAX);
                    guard.set_property(
                        InternalProperties::TARJAN_LOWLINK,
                        Some(current.min(target_lowlink)),
                    );
                }
            } else {
                let target_on_stack = target
                    .lock()
                    .get_property(InternalProperties::TARJAN_ON_STACK)
                    .unwrap_or(false);
                if target_on_stack {
                    {
                        let mut guard = v.lock();
                        let current = guard
                            .get_property(InternalProperties::TARJAN_LOWLINK)
                            .unwrap_or(i32::MAX);
                        guard.set_property(
                            InternalProperties::TARJAN_LOWLINK,
                            Some(current.min(target_id)),
                        );
                    }
                }
            }
        }

        let v_lowlink = v
            .lock()
            .get_property(InternalProperties::TARJAN_LOWLINK)
            .unwrap_or(i32::MAX);
        let v_id = v
            .lock()
            .get_property(InternalProperties::TARJAN_ID)
            .unwrap_or(-1);

        if v_lowlink == v_id {
            let mut scc = Vec::new();
            while let Some(n) = self.stack.pop() {
                {
                    let mut guard = n.lock();
                    guard.set_property(InternalProperties::TARJAN_ON_STACK, Some(false));
                }
                scc.push(n.clone());
                if std::sync::Arc::ptr_eq(&n, v) {
                    break;
                }
            }
            if scc.len() > 1 {
                let index = self.strongly_connected_components.len();
                self.strongly_connected_components.push(scc.clone());
                for node in scc {
                    self.node_to_scc_id.insert(NodeRefKey(node), index);
                }
            }
        }
    }

    pub fn reset_tarjan(&mut self, graph: &LGraphRef) {
        let nodes = graph
            .lock().layerless_nodes().clone();
        for node in nodes {
            {
                let mut guard = node.lock();
                guard.set_property(InternalProperties::TARJAN_ON_STACK, Some(false));
                guard.set_property(InternalProperties::TARJAN_LOWLINK, Some(-1));
                guard.set_property(InternalProperties::TARJAN_ID, Some(-1));
            }
            self.stack.clear();
            let edges = node
                .lock().connected_edges();
            for edge in edges {
                {
                    let mut edge_guard = edge.lock();
                    edge_guard.set_property(InternalProperties::IS_PART_OF_CYCLE, Some(false));
                }
            }
        }
    }
}
