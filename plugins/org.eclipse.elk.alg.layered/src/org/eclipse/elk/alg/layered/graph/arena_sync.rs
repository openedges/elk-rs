//! Bidirectional bridge between Arc<Mutex<T>> graph and [`super::LArena`].
//!
//! [`ArenaSync`] builds an arena from the existing graph (single lock pass)
//! and provides methods to synchronize results back to the Arc graph.

use rustc_hash::FxHashMap;
use std::sync::Arc;

use super::arena::LArena;
use super::arena_builder::LArenaBuilder;
use super::arena_types::*;
use super::{LEdgeRef, LGraphRef, LLabelRef, LNodeRef, LPortRef, LayerRef};

pub struct ArenaSync {
    arena: LArena,

    // ── Arc pointer → arena ID ──────────────────────────────────────
    node_arc_to_id: FxHashMap<usize, NodeId>,
    port_arc_to_id: FxHashMap<usize, PortId>,
    edge_arc_to_id: FxHashMap<usize, EdgeId>,

    // ── Arena ID → Arc ref (for sync-back) ──────────────────────────
    node_id_to_arc: Vec<LNodeRef>,
    port_id_to_arc: Vec<LPortRef>,
    edge_id_to_arc: Vec<LEdgeRef>,
    label_id_to_arc: Vec<LLabelRef>,
    layer_id_to_arc: Vec<LayerRef>,
}

impl ArenaSync {
    /// Build an arena from the current state of the Arc-based graph.
    ///
    /// Performs two passes:
    /// 1. Layers, nodes, ports, and their labels (establishes all port IDs)
    /// 2. Edges and their labels (both source and target ports are known)
    pub fn from_graph(graph: &LGraphRef) -> Self {
        let mut builder = LArenaBuilder::new();

        let mut node_arc_to_id: FxHashMap<usize, NodeId> = FxHashMap::default();
        let mut port_arc_to_id: FxHashMap<usize, PortId> = FxHashMap::default();
        let mut edge_arc_to_id: FxHashMap<usize, EdgeId> = FxHashMap::default();

        let mut node_id_to_arc: Vec<LNodeRef> = Vec::new();
        let mut port_id_to_arc: Vec<LPortRef> = Vec::new();
        let mut edge_id_to_arc: Vec<LEdgeRef> = Vec::new();
        let mut label_id_to_arc: Vec<LLabelRef> = Vec::new();
        let mut layer_id_to_arc: Vec<LayerRef> = Vec::new();

        // Collect layer and node refs under a single graph lock
        let (layer_refs, layerless_refs) = {
            let graph_guard = graph.lock();            (graph_guard.layers().clone(), graph_guard.layerless_nodes().clone())
        };

        // ── Pass 1: layers, nodes, ports, labels ────────────────────

        for layer_ref in &layer_refs {
            let (layer_size, layer_eid, node_refs) = {
                let mut layer_guard = layer_ref.lock();                let size = *layer_guard.size_ref();
                let eid = layer_guard.graph_element().id;
                let nodes = layer_guard.nodes().clone();
                (size, eid, nodes)
            };

            let layer_id = builder.add_layer(layer_size, layer_eid);
            layer_id_to_arc.push(layer_ref.clone());

            for node_ref in &node_refs {
                let node_id = add_node_to_builder(
                    &mut builder,
                    node_ref,
                    &mut node_arc_to_id,
                    &mut node_id_to_arc,
                    &mut port_arc_to_id,
                    &mut port_id_to_arc,
                    &mut label_id_to_arc,
                );
                builder.set_node_layer(node_id, layer_id);
            }
        }

        // Handle layerless nodes (before P2 or after layer dissolution)
        for node_ref in &layerless_refs {
            add_node_to_builder(
                &mut builder,
                node_ref,
                &mut node_arc_to_id,
                &mut node_id_to_arc,
                &mut port_arc_to_id,
                &mut port_id_to_arc,
                &mut label_id_to_arc,
            );
            // node_layer remains LayerId::NONE (set by add_node default)
        }

        // ── Pass 2: edges and edge labels ───────────────────────────

        for port_ref in &port_id_to_arc {
            let outgoing = {
                let port_guard = port_ref.lock();                port_guard.outgoing_edges().clone()
            };

            for edge_ref in &outgoing {
                let edge_ptr = Arc::as_ptr(edge_ref) as usize;
                if edge_arc_to_id.contains_key(&edge_ptr) {
                    continue; // already processed
                }

                let (src_port_ref, tgt_port_ref, bend_points, eid, props, edge_label_refs) = {
                    let mut edge_guard = edge_ref.lock();                    let src = edge_guard.source().unwrap();
                    let tgt = edge_guard.target().unwrap();
                    let bp = edge_guard.bend_points_ref().clone();
                    let eid = edge_guard.graph_element().id;
                    let props = edge_guard.graph_element().properties().clone();
                    let labels = edge_guard.labels().clone();
                    (src, tgt, bp, eid, props, labels)
                };

                let src_ptr = Arc::as_ptr(&src_port_ref) as usize;
                let tgt_ptr = Arc::as_ptr(&tgt_port_ref) as usize;

                // Skip edges whose ports are outside this graph
                let src_pid = match port_arc_to_id.get(&src_ptr) {
                    Some(&pid) => pid,
                    None => continue,
                };
                let tgt_pid = match port_arc_to_id.get(&tgt_ptr) {
                    Some(&pid) => pid,
                    None => continue,
                };

                let edge_id = builder.add_edge(src_pid, tgt_pid, bend_points, eid, props);
                edge_arc_to_id.insert(edge_ptr, edge_id);
                edge_id_to_arc.push(edge_ref.clone());

                // Edge labels
                for label_ref in &edge_label_refs {
                    let label_id = add_label_to_builder(&mut builder, label_ref, &mut label_id_to_arc);
                    builder.add_edge_label(edge_id, label_id);
                }
            }
        }

        let arena = builder.freeze();

        ArenaSync {
            arena,
            node_arc_to_id,
            port_arc_to_id,
            edge_arc_to_id,
            node_id_to_arc,
            port_id_to_arc,
            edge_id_to_arc,
            label_id_to_arc,
            layer_id_to_arc,
        }
    }

    // ── Accessors ───────────────────────────────────────────────────

    #[inline]
    pub fn arena(&self) -> &LArena {
        &self.arena
    }

    #[inline]
    pub fn arena_mut(&mut self) -> &mut LArena {
        &mut self.arena
    }

    /// Look up the arena [`NodeId`] for an Arc node reference.
    #[inline]
    pub fn node_id(&self, node: &LNodeRef) -> Option<NodeId> {
        let ptr = Arc::as_ptr(node) as usize;
        self.node_arc_to_id.get(&ptr).copied()
    }

    /// Look up the arena [`PortId`] for an Arc port reference.
    #[inline]
    pub fn port_id(&self, port: &LPortRef) -> Option<PortId> {
        let ptr = Arc::as_ptr(port) as usize;
        self.port_arc_to_id.get(&ptr).copied()
    }

    /// Look up the arena [`EdgeId`] for an Arc edge reference.
    #[inline]
    pub fn edge_id(&self, edge: &LEdgeRef) -> Option<EdgeId> {
        let ptr = Arc::as_ptr(edge) as usize;
        self.edge_arc_to_id.get(&ptr).copied()
    }

    /// Get the Arc node reference for an arena [`NodeId`].
    #[inline]
    pub fn node_ref(&self, id: NodeId) -> &LNodeRef {
        &self.node_id_to_arc[id.idx()]
    }

    /// Get the Arc port reference for an arena [`PortId`].
    #[inline]
    pub fn port_ref(&self, id: PortId) -> &LPortRef {
        &self.port_id_to_arc[id.idx()]
    }

    /// Get the Arc edge reference for an arena [`EdgeId`].
    #[inline]
    pub fn edge_ref(&self, id: EdgeId) -> &LEdgeRef {
        &self.edge_id_to_arc[id.idx()]
    }

    /// Get the Arc label reference for an arena [`LabelId`].
    #[inline]
    pub fn label_ref(&self, id: LabelId) -> &LLabelRef {
        &self.label_id_to_arc[id.idx()]
    }

    /// Get the Arc layer reference for an arena [`LayerId`].
    #[inline]
    pub fn layer_ref(&self, id: LayerId) -> &LayerRef {
        &self.layer_id_to_arc[id.idx()]
    }

    // ── Sync: arena → Arc graph ─────────────────────────────────────

    /// Write arena node/port/label positions and sizes back to the Arc graph.
    pub fn sync_positions_to_graph(&self) {
        // Nodes
        for (i, node_ref) in self.node_id_to_arc.iter().enumerate() {
            let id = NodeId(i as u32);
            let mut guard = node_ref.lock();            let pos = guard.shape().position();
            let arena_pos = self.arena.node_pos(id);
            pos.x = arena_pos.x;
            pos.y = arena_pos.y;
            let size = guard.shape().size();
            let arena_size = self.arena.node_size(id);
            size.x = arena_size.x;
            size.y = arena_size.y;
        }

        // Ports
        for (i, port_ref) in self.port_id_to_arc.iter().enumerate() {
            let id = PortId(i as u32);
            let mut guard = port_ref.lock();            let pos = guard.shape().position();
            let arena_pos = self.arena.port_pos(id);
            pos.x = arena_pos.x;
            pos.y = arena_pos.y;
            let size = guard.shape().size();
            let arena_size = self.arena.port_size(id);
            size.x = arena_size.x;
            size.y = arena_size.y;
        }

        // Labels
        for (i, label_ref) in self.label_id_to_arc.iter().enumerate() {
            let id = LabelId(i as u32);
            let mut guard = label_ref.lock();            let pos = guard.shape().position();
            let arena_pos = self.arena.label_pos(id);
            pos.x = arena_pos.x;
            pos.y = arena_pos.y;
            let size = guard.shape().size();
            let arena_size = self.arena.label_size(id);
            size.x = arena_size.x;
            size.y = arena_size.y;
        }
    }

    /// Write arena layer→node ordering back to the Arc graph.
    ///
    /// For each layer, replaces its node list with the arena's ordering.
    pub fn sync_order_to_graph(&self) {
        for (i, layer_ref) in self.layer_id_to_arc.iter().enumerate() {
            let layer_id = LayerId(i as u32);
            let arena_node_ids = self.arena.layer_nodes(layer_id);
            let mut layer_guard = layer_ref.lock();            let nodes = layer_guard.nodes_mut();
            nodes.clear();
            nodes.reserve(arena_node_ids.len());
            for &nid in arena_node_ids {
                nodes.push(self.node_id_to_arc[nid.idx()].clone());
            }
        }
    }

    /// Write arena bend points back to the Arc graph edges.
    pub fn sync_bend_points_to_graph(&self) {
        for (i, edge_ref) in self.edge_id_to_arc.iter().enumerate() {
            let id = EdgeId(i as u32);
            let mut guard = edge_ref.lock();            let bp = guard.bend_points();
            let arena_bp = self.arena.edge_bend_points(id);
            *bp = arena_bp.clone();
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Add a single node (with its ports and labels) to the builder.
fn add_node_to_builder(
    builder: &mut LArenaBuilder,
    node_ref: &LNodeRef,
    node_arc_to_id: &mut FxHashMap<usize, NodeId>,
    node_id_to_arc: &mut Vec<LNodeRef>,
    port_arc_to_id: &mut FxHashMap<usize, PortId>,
    port_id_to_arc: &mut Vec<LPortRef>,
    label_id_to_arc: &mut Vec<LLabelRef>,
) -> NodeId {
    let node_ptr = Arc::as_ptr(node_ref) as usize;

    let (pos, size, node_type, margin, padding, eid, props, port_refs, label_refs) = {
        let mut guard = node_ref.lock();        let pos = *guard.shape().position_ref();
        let size = *guard.shape().size_ref();
        let nt = guard.node_type();
        let margin = guard.margin().clone();
        let padding = guard.padding().clone();
        let eid = guard.shape().graph_element().id;
        let props = guard.shape().graph_element().properties().clone();
        let ports = guard.ports().clone();
        let labels = guard.labels().clone();
        (pos, size, nt, margin, padding, eid, props, ports, labels)
    };

    let node_id = builder.add_node(pos, size, node_type, margin, padding, eid, props);
    node_arc_to_id.insert(node_ptr, node_id);
    node_id_to_arc.push(node_ref.clone());

    // Ports
    for port_ref in &port_refs {
        let port_ptr = Arc::as_ptr(port_ref) as usize;

        let (p_pos, p_size, p_side, p_anchor, p_margin, p_eid, p_props, p_label_refs) = {
            let mut guard = port_ref.lock();            let pos = *guard.shape().position_ref();
            let size = *guard.shape().size_ref();
            let side = guard.side();
            let anchor = *guard.anchor_ref();
            let margin = guard.margin().clone();
            let eid = guard.shape().graph_element().id;
            let props = guard.shape().graph_element().properties().clone();
            let labels = guard.labels().clone();
            (pos, size, side, anchor, margin, eid, props, labels)
        };

        let port_id =
            builder.add_port(node_id, p_pos, p_size, p_side, p_anchor, p_margin, p_eid, p_props);
        port_arc_to_id.insert(port_ptr, port_id);
        port_id_to_arc.push(port_ref.clone());

        // Port labels
        for label_ref in &p_label_refs {
            let label_id = add_label_to_builder(builder, label_ref, label_id_to_arc);
            builder.add_port_label(port_id, label_id);
        }
    }

    // Node labels
    for label_ref in &label_refs {
        let label_id = add_label_to_builder(builder, label_ref, label_id_to_arc);
        builder.add_node_label(node_id, label_id);
    }

    node_id
}

/// Add a single label to the builder.
fn add_label_to_builder(
    builder: &mut LArenaBuilder,
    label_ref: &LLabelRef,
    label_id_to_arc: &mut Vec<LLabelRef>,
) -> LabelId {
    let (pos, size, text, eid, props) = {
        let mut guard = label_ref.lock();        let pos = *guard.shape().position_ref();
        let size = *guard.shape().size_ref();
        let text = guard.text().to_owned();
        let eid = guard.shape().graph_element().id;
        let props = guard.shape().graph_element().properties().clone();
        (pos, size, text, eid, props)
    };

    let label_id = builder.add_label(pos, size, text, eid, props);
    label_id_to_arc.push(label_ref.clone());
    label_id
}
