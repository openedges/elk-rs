//! Mutable builder for [`super::LArena`].
//!
//! Uses `Vec<Vec<Id>>` adjacency lists during the mutation-heavy build phase.
//! Call [`LArenaBuilder::freeze`] to compact into CSR and produce an immutable
//! [`super::LArena`].  Use [`LArenaBuilder::thaw`] to convert back for further
//! topology mutations.

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;

use super::arena::LArena;
use super::arena_types::*;
use super::{LMargin, LPadding, NodeType};

pub struct LArenaBuilder {
    // ── Node attributes ─────────────────────────────────────────────
    node_pos: Vec<KVector>,
    node_size: Vec<KVector>,
    node_type: Vec<NodeType>,
    node_margin: Vec<LMargin>,
    node_padding: Vec<LPadding>,
    node_layer: Vec<LayerId>,
    node_element_id: Vec<i32>,
    node_properties: Vec<MapPropertyHolder>,

    // ── Port attributes ─────────────────────────────────────────────
    port_pos: Vec<KVector>,
    port_size: Vec<KVector>,
    port_side: Vec<PortSide>,
    port_anchor: Vec<KVector>,
    port_margin: Vec<LMargin>,
    port_owner: Vec<NodeId>,
    port_element_id: Vec<i32>,
    port_properties: Vec<MapPropertyHolder>,

    // ── Edge attributes ─────────────────────────────────────────────
    edge_source: Vec<PortId>,
    edge_target: Vec<PortId>,
    edge_bend_points: Vec<KVectorChain>,
    edge_element_id: Vec<i32>,
    edge_properties: Vec<MapPropertyHolder>,

    // ── Label attributes ────────────────────────────────────────────
    label_pos: Vec<KVector>,
    label_size: Vec<KVector>,
    label_text: Vec<String>,
    label_element_id: Vec<i32>,
    label_properties: Vec<MapPropertyHolder>,

    // ── Layer attributes ────────────────────────────────────────────
    layer_size: Vec<KVector>,
    layer_element_id: Vec<i32>,

    // ── Mutable adjacency (Vec<Vec<Id>>) ────────────────────────────
    node_ports: Vec<Vec<PortId>>,
    node_labels: Vec<Vec<LabelId>>,
    port_in_edges: Vec<Vec<EdgeId>>,
    port_out_edges: Vec<Vec<EdgeId>>,
    port_labels: Vec<Vec<LabelId>>,
    edge_labels: Vec<Vec<LabelId>>,
    layer_nodes: Vec<Vec<NodeId>>,
}

impl LArenaBuilder {
    pub fn new() -> Self {
        LArenaBuilder {
            node_pos: Vec::new(),
            node_size: Vec::new(),
            node_type: Vec::new(),
            node_margin: Vec::new(),
            node_padding: Vec::new(),
            node_layer: Vec::new(),
            node_element_id: Vec::new(),
            node_properties: Vec::new(),

            port_pos: Vec::new(),
            port_size: Vec::new(),
            port_side: Vec::new(),
            port_anchor: Vec::new(),
            port_margin: Vec::new(),
            port_owner: Vec::new(),
            port_element_id: Vec::new(),
            port_properties: Vec::new(),

            edge_source: Vec::new(),
            edge_target: Vec::new(),
            edge_bend_points: Vec::new(),
            edge_element_id: Vec::new(),
            edge_properties: Vec::new(),

            label_pos: Vec::new(),
            label_size: Vec::new(),
            label_text: Vec::new(),
            label_element_id: Vec::new(),
            label_properties: Vec::new(),

            layer_size: Vec::new(),
            layer_element_id: Vec::new(),

            node_ports: Vec::new(),
            node_labels: Vec::new(),
            port_in_edges: Vec::new(),
            port_out_edges: Vec::new(),
            port_labels: Vec::new(),
            edge_labels: Vec::new(),
            layer_nodes: Vec::new(),
        }
    }

    /// Add a layer and return its [`LayerId`].
    pub fn add_layer(&mut self, size: KVector, element_id: i32) -> LayerId {
        let id = LayerId(self.layer_size.len() as u32);
        self.layer_size.push(size);
        self.layer_element_id.push(element_id);
        self.layer_nodes.push(Vec::new());
        id
    }

    /// Add a node and return its [`NodeId`].
    ///
    /// The node is not assigned to any layer; call [`set_node_layer`] afterwards.
    #[allow(clippy::too_many_arguments)]
    pub fn add_node(
        &mut self,
        pos: KVector,
        size: KVector,
        node_type: NodeType,
        margin: LMargin,
        padding: LPadding,
        element_id: i32,
        properties: MapPropertyHolder,
    ) -> NodeId {
        let id = NodeId(self.node_pos.len() as u32);
        self.node_pos.push(pos);
        self.node_size.push(size);
        self.node_type.push(node_type);
        self.node_margin.push(margin);
        self.node_padding.push(padding);
        self.node_layer.push(LayerId::NONE);
        self.node_element_id.push(element_id);
        self.node_properties.push(properties);
        self.node_ports.push(Vec::new());
        self.node_labels.push(Vec::new());
        id
    }

    /// Add a port owned by `owner` and return its [`PortId`].
    ///
    /// Automatically registers the port in the owner node's port list.
    #[allow(clippy::too_many_arguments)]
    pub fn add_port(
        &mut self,
        owner: NodeId,
        pos: KVector,
        size: KVector,
        side: PortSide,
        anchor: KVector,
        margin: LMargin,
        element_id: i32,
        properties: MapPropertyHolder,
    ) -> PortId {
        let id = PortId(self.port_pos.len() as u32);
        self.port_pos.push(pos);
        self.port_size.push(size);
        self.port_side.push(side);
        self.port_anchor.push(anchor);
        self.port_margin.push(margin);
        self.port_owner.push(owner);
        self.port_element_id.push(element_id);
        self.port_properties.push(properties);
        self.port_in_edges.push(Vec::new());
        self.port_out_edges.push(Vec::new());
        self.port_labels.push(Vec::new());
        // Register port with its owner node
        self.node_ports[owner.idx()].push(id);
        id
    }

    /// Add an edge from `src` to `tgt` and return its [`EdgeId`].
    ///
    /// Automatically registers the edge in source port's outgoing list
    /// and target port's incoming list.
    pub fn add_edge(
        &mut self,
        src: PortId,
        tgt: PortId,
        bend_points: KVectorChain,
        element_id: i32,
        properties: MapPropertyHolder,
    ) -> EdgeId {
        let id = EdgeId(self.edge_source.len() as u32);
        self.edge_source.push(src);
        self.edge_target.push(tgt);
        self.edge_bend_points.push(bend_points);
        self.edge_element_id.push(element_id);
        self.edge_properties.push(properties);
        self.edge_labels.push(Vec::new());
        // Register edge with source and target ports
        self.port_out_edges[src.idx()].push(id);
        self.port_in_edges[tgt.idx()].push(id);
        id
    }

    /// Add a label and return its [`LabelId`].
    ///
    /// The label is not linked to any owner; use [`add_node_label`],
    /// [`add_port_label`], or [`add_edge_label`] to establish ownership.
    pub fn add_label(
        &mut self,
        pos: KVector,
        size: KVector,
        text: String,
        element_id: i32,
        properties: MapPropertyHolder,
    ) -> LabelId {
        let id = LabelId(self.label_pos.len() as u32);
        self.label_pos.push(pos);
        self.label_size.push(size);
        self.label_text.push(text);
        self.label_element_id.push(element_id);
        self.label_properties.push(properties);
        id
    }

    /// Link an existing label to a node.
    pub fn add_node_label(&mut self, node: NodeId, label: LabelId) {
        self.node_labels[node.idx()].push(label);
    }

    /// Link an existing label to a port.
    pub fn add_port_label(&mut self, port: PortId, label: LabelId) {
        self.port_labels[port.idx()].push(label);
    }

    /// Link an existing label to an edge.
    pub fn add_edge_label(&mut self, edge: EdgeId, label: LabelId) {
        self.edge_labels[edge.idx()].push(label);
    }

    /// Assign a node to a layer.
    pub fn set_node_layer(&mut self, node: NodeId, layer: LayerId) {
        self.node_layer[node.idx()] = layer;
        if !layer.is_none() {
            self.layer_nodes[layer.idx()].push(node);
        }
    }

    /// Compact mutable adjacency lists into CSR format, producing an
    /// immutable [`LArena`].
    pub fn freeze(self) -> LArena {
        let n_nodes = self.node_pos.len() as u32;
        let n_ports = self.port_pos.len() as u32;
        let n_edges = self.edge_source.len() as u32;
        let n_labels = self.label_pos.len() as u32;
        let n_layers = self.layer_size.len() as u32;

        let (node_port_offset, node_port_ids) = build_csr(&self.node_ports);
        let (node_label_offset, node_label_ids) = build_csr(&self.node_labels);
        let (port_in_offset, port_in_edges) = build_csr(&self.port_in_edges);
        let (port_out_offset, port_out_edges) = build_csr(&self.port_out_edges);
        let (port_label_offset, port_label_ids) = build_csr(&self.port_labels);
        let (edge_label_offset, edge_label_ids) = build_csr(&self.edge_labels);
        let (layer_node_offset, layer_node_ids) = build_csr(&self.layer_nodes);

        LArena {
            node_pos: self.node_pos,
            node_size: self.node_size,
            node_type: self.node_type,
            node_margin: self.node_margin,
            node_padding: self.node_padding,
            node_layer: self.node_layer,
            node_element_id: self.node_element_id,
            node_properties: self.node_properties,

            port_pos: self.port_pos,
            port_size: self.port_size,
            port_side: self.port_side,
            port_anchor: self.port_anchor,
            port_margin: self.port_margin,
            port_owner: self.port_owner,
            port_element_id: self.port_element_id,
            port_properties: self.port_properties,

            edge_source: self.edge_source,
            edge_target: self.edge_target,
            edge_bend_points: self.edge_bend_points,
            edge_element_id: self.edge_element_id,
            edge_properties: self.edge_properties,

            label_pos: self.label_pos,
            label_size: self.label_size,
            label_text: self.label_text,
            label_element_id: self.label_element_id,
            label_properties: self.label_properties,

            layer_size: self.layer_size,
            layer_element_id: self.layer_element_id,

            node_port_offset,
            node_port_ids,
            node_label_offset,
            node_label_ids,
            port_in_offset,
            port_in_edges,
            port_out_offset,
            port_out_edges,
            port_label_offset,
            port_label_ids,
            edge_label_offset,
            edge_label_ids,
            layer_node_offset,
            layer_node_ids,

            n_nodes,
            n_ports,
            n_edges,
            n_labels,
            n_layers,
        }
    }

    /// Convert a frozen [`LArena`] back into a mutable builder.
    pub fn thaw(arena: LArena) -> Self {
        let n_nodes = arena.n_nodes as usize;
        let n_ports = arena.n_ports as usize;
        let n_edges = arena.n_edges as usize;
        let n_layers = arena.n_layers as usize;

        LArenaBuilder {
            node_pos: arena.node_pos,
            node_size: arena.node_size,
            node_type: arena.node_type,
            node_margin: arena.node_margin,
            node_padding: arena.node_padding,
            node_layer: arena.node_layer,
            node_element_id: arena.node_element_id,
            node_properties: arena.node_properties,

            port_pos: arena.port_pos,
            port_size: arena.port_size,
            port_side: arena.port_side,
            port_anchor: arena.port_anchor,
            port_margin: arena.port_margin,
            port_owner: arena.port_owner,
            port_element_id: arena.port_element_id,
            port_properties: arena.port_properties,

            edge_source: arena.edge_source,
            edge_target: arena.edge_target,
            edge_bend_points: arena.edge_bend_points,
            edge_element_id: arena.edge_element_id,
            edge_properties: arena.edge_properties,

            label_pos: arena.label_pos,
            label_size: arena.label_size,
            label_text: arena.label_text,
            label_element_id: arena.label_element_id,
            label_properties: arena.label_properties,

            layer_size: arena.layer_size,
            layer_element_id: arena.layer_element_id,

            node_ports: split_csr(&arena.node_port_offset, &arena.node_port_ids, n_nodes),
            node_labels: split_csr(&arena.node_label_offset, &arena.node_label_ids, n_nodes),
            port_in_edges: split_csr(&arena.port_in_offset, &arena.port_in_edges, n_ports),
            port_out_edges: split_csr(&arena.port_out_offset, &arena.port_out_edges, n_ports),
            port_labels: split_csr(&arena.port_label_offset, &arena.port_label_ids, n_ports),
            edge_labels: split_csr(&arena.edge_label_offset, &arena.edge_label_ids, n_edges),
            layer_nodes: split_csr(&arena.layer_node_offset, &arena.layer_node_ids, n_layers),
        }
    }
}

impl Default for LArenaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ── CSR helpers ─────────────────────────────────────────────────────

/// Compact `Vec<Vec<T>>` into CSR offset + data arrays.
fn build_csr<T: Copy>(lists: &[Vec<T>]) -> (Vec<u32>, Vec<T>) {
    let mut offset = Vec::with_capacity(lists.len() + 1);
    let total: usize = lists.iter().map(|l| l.len()).sum();
    let mut data = Vec::with_capacity(total);
    offset.push(0);
    for list in lists {
        data.extend_from_slice(list);
        offset.push(data.len() as u32);
    }
    (offset, data)
}

/// Split CSR offset + data arrays back into `Vec<Vec<T>>`.
fn split_csr<T: Copy>(offset: &[u32], data: &[T], n: usize) -> Vec<Vec<T>> {
    (0..n)
        .map(|i| {
            let start = offset[i] as usize;
            let end = offset[i + 1] as usize;
            data[start..end].to_vec()
        })
        .collect()
}
