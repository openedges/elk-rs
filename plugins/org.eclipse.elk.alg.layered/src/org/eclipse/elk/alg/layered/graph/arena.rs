//! Frozen arena-based graph representation (SoA layout with CSR adjacency).
//!
//! [`LArena`] stores all graph elements in flat, contiguous arrays with
//! CSR (Compressed Sparse Row) encoded relationships.  This enables:
//! - Zero-allocation topology traversal (returns `&[Id]` slices)
//! - Cache-friendly iteration over individual fields
//! - No Arc/Mutex overhead for reads
//!
//! Topology (CSR) is immutable; per-element attributes (positions, sizes, etc.)
//! are mutable.  To modify topology, convert to [`super::LArenaBuilder`] via
//! [`LArenaBuilder::thaw`].

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;

use super::arena_types::*;
use super::{LMargin, LPadding, NodeType};

pub struct LArena {
    // ── Node attributes (indexed by NodeId) ─────────────────────────
    pub(crate) node_pos: Vec<KVector>,
    pub(crate) node_size: Vec<KVector>,
    pub(crate) node_type: Vec<NodeType>,
    pub(crate) node_margin: Vec<LMargin>,
    pub(crate) node_padding: Vec<LPadding>,
    pub(crate) node_layer: Vec<LayerId>,
    pub(crate) node_element_id: Vec<i32>,
    pub(crate) node_properties: Vec<MapPropertyHolder>,

    // ── Port attributes (indexed by PortId) ──────────────────────────
    pub(crate) port_pos: Vec<KVector>,
    pub(crate) port_size: Vec<KVector>,
    pub(crate) port_side: Vec<PortSide>,
    pub(crate) port_anchor: Vec<KVector>,
    pub(crate) port_margin: Vec<LMargin>,
    pub(crate) port_owner: Vec<NodeId>,
    pub(crate) port_element_id: Vec<i32>,
    pub(crate) port_properties: Vec<MapPropertyHolder>,

    // ── Edge attributes (indexed by EdgeId) ──────────────────────────
    pub(crate) edge_source: Vec<PortId>,
    pub(crate) edge_target: Vec<PortId>,
    pub(crate) edge_bend_points: Vec<KVectorChain>,
    pub(crate) edge_element_id: Vec<i32>,
    pub(crate) edge_properties: Vec<MapPropertyHolder>,

    // ── Label attributes (indexed by LabelId) ────────────────────────
    pub(crate) label_pos: Vec<KVector>,
    pub(crate) label_size: Vec<KVector>,
    pub(crate) label_text: Vec<String>,
    pub(crate) label_element_id: Vec<i32>,
    pub(crate) label_properties: Vec<MapPropertyHolder>,

    // ── Layer attributes (indexed by LayerId) ────────────────────────
    pub(crate) layer_size: Vec<KVector>,
    pub(crate) layer_element_id: Vec<i32>,

    // ── CSR: node → ports ────────────────────────────────────────────
    pub(crate) node_port_offset: Vec<u32>,
    pub(crate) node_port_ids: Vec<PortId>,

    // ── CSR: node → labels ───────────────────────────────────────────
    pub(crate) node_label_offset: Vec<u32>,
    pub(crate) node_label_ids: Vec<LabelId>,

    // ── CSR: port → incoming edges ───────────────────────────────────
    pub(crate) port_in_offset: Vec<u32>,
    pub(crate) port_in_edges: Vec<EdgeId>,

    // ── CSR: port → outgoing edges ──────────────────────────────────
    pub(crate) port_out_offset: Vec<u32>,
    pub(crate) port_out_edges: Vec<EdgeId>,

    // ── CSR: port → labels ──────────────────────────────────────────
    pub(crate) port_label_offset: Vec<u32>,
    pub(crate) port_label_ids: Vec<LabelId>,

    // ── CSR: edge → labels ──────────────────────────────────────────
    pub(crate) edge_label_offset: Vec<u32>,
    pub(crate) edge_label_ids: Vec<LabelId>,

    // ── CSR: layer → nodes ──────────────────────────────────────────
    pub(crate) layer_node_offset: Vec<u32>,
    pub(crate) layer_node_ids: Vec<NodeId>,

    // ── Counts ──────────────────────────────────────────────────────
    pub(crate) n_nodes: u32,
    pub(crate) n_ports: u32,
    pub(crate) n_edges: u32,
    pub(crate) n_labels: u32,
    pub(crate) n_layers: u32,
}

impl LArena {
    // ── Count accessors ─────────────────────────────────────────────

    #[inline]
    pub fn n_nodes(&self) -> u32 {
        self.n_nodes
    }

    #[inline]
    pub fn n_ports(&self) -> u32 {
        self.n_ports
    }

    #[inline]
    pub fn n_edges(&self) -> u32 {
        self.n_edges
    }

    #[inline]
    pub fn n_labels(&self) -> u32 {
        self.n_labels
    }

    #[inline]
    pub fn n_layers(&self) -> u32 {
        self.n_layers
    }

    // ── Node attribute accessors ────────────────────────────────────

    #[inline]
    pub fn node_pos(&self, id: NodeId) -> KVector {
        self.node_pos[id.idx()]
    }

    #[inline]
    pub fn node_pos_mut(&mut self, id: NodeId) -> &mut KVector {
        &mut self.node_pos[id.idx()]
    }

    #[inline]
    pub fn node_size(&self, id: NodeId) -> KVector {
        self.node_size[id.idx()]
    }

    #[inline]
    pub fn node_size_mut(&mut self, id: NodeId) -> &mut KVector {
        &mut self.node_size[id.idx()]
    }

    #[inline]
    pub fn node_type(&self, id: NodeId) -> NodeType {
        self.node_type[id.idx()]
    }

    #[inline]
    pub fn node_margin(&self, id: NodeId) -> &LMargin {
        &self.node_margin[id.idx()]
    }

    #[inline]
    pub fn node_margin_mut(&mut self, id: NodeId) -> &mut LMargin {
        &mut self.node_margin[id.idx()]
    }

    #[inline]
    pub fn node_padding(&self, id: NodeId) -> &LPadding {
        &self.node_padding[id.idx()]
    }

    #[inline]
    pub fn node_padding_mut(&mut self, id: NodeId) -> &mut LPadding {
        &mut self.node_padding[id.idx()]
    }

    #[inline]
    pub fn node_layer_id(&self, id: NodeId) -> LayerId {
        self.node_layer[id.idx()]
    }

    #[inline]
    pub fn node_element_id(&self, id: NodeId) -> i32 {
        self.node_element_id[id.idx()]
    }

    #[inline]
    pub fn node_properties(&self, id: NodeId) -> &MapPropertyHolder {
        &self.node_properties[id.idx()]
    }

    #[inline]
    pub fn node_properties_mut(&mut self, id: NodeId) -> &mut MapPropertyHolder {
        &mut self.node_properties[id.idx()]
    }

    // ── Port attribute accessors ────────────────────────────────────

    #[inline]
    pub fn port_pos(&self, id: PortId) -> KVector {
        self.port_pos[id.idx()]
    }

    #[inline]
    pub fn port_pos_mut(&mut self, id: PortId) -> &mut KVector {
        &mut self.port_pos[id.idx()]
    }

    #[inline]
    pub fn port_size(&self, id: PortId) -> KVector {
        self.port_size[id.idx()]
    }

    #[inline]
    pub fn port_size_mut(&mut self, id: PortId) -> &mut KVector {
        &mut self.port_size[id.idx()]
    }

    #[inline]
    pub fn port_side(&self, id: PortId) -> PortSide {
        self.port_side[id.idx()]
    }

    #[inline]
    pub fn port_anchor(&self, id: PortId) -> KVector {
        self.port_anchor[id.idx()]
    }

    #[inline]
    pub fn port_anchor_mut(&mut self, id: PortId) -> &mut KVector {
        &mut self.port_anchor[id.idx()]
    }

    #[inline]
    pub fn port_margin(&self, id: PortId) -> &LMargin {
        &self.port_margin[id.idx()]
    }

    #[inline]
    pub fn port_margin_mut(&mut self, id: PortId) -> &mut LMargin {
        &mut self.port_margin[id.idx()]
    }

    #[inline]
    pub fn port_owner(&self, id: PortId) -> NodeId {
        self.port_owner[id.idx()]
    }

    #[inline]
    pub fn port_element_id(&self, id: PortId) -> i32 {
        self.port_element_id[id.idx()]
    }

    #[inline]
    pub fn port_properties(&self, id: PortId) -> &MapPropertyHolder {
        &self.port_properties[id.idx()]
    }

    #[inline]
    pub fn port_properties_mut(&mut self, id: PortId) -> &mut MapPropertyHolder {
        &mut self.port_properties[id.idx()]
    }

    // ── Edge attribute accessors ────────────────────────────────────

    #[inline]
    pub fn edge_source(&self, id: EdgeId) -> PortId {
        self.edge_source[id.idx()]
    }

    #[inline]
    pub fn edge_target(&self, id: EdgeId) -> PortId {
        self.edge_target[id.idx()]
    }

    #[inline]
    pub fn edge_bend_points(&self, id: EdgeId) -> &KVectorChain {
        &self.edge_bend_points[id.idx()]
    }

    #[inline]
    pub fn edge_bend_points_mut(&mut self, id: EdgeId) -> &mut KVectorChain {
        &mut self.edge_bend_points[id.idx()]
    }

    #[inline]
    pub fn edge_element_id(&self, id: EdgeId) -> i32 {
        self.edge_element_id[id.idx()]
    }

    #[inline]
    pub fn edge_properties(&self, id: EdgeId) -> &MapPropertyHolder {
        &self.edge_properties[id.idx()]
    }

    #[inline]
    pub fn edge_properties_mut(&mut self, id: EdgeId) -> &mut MapPropertyHolder {
        &mut self.edge_properties[id.idx()]
    }

    // ── Label attribute accessors ───────────────────────────────────

    #[inline]
    pub fn label_pos(&self, id: LabelId) -> KVector {
        self.label_pos[id.idx()]
    }

    #[inline]
    pub fn label_pos_mut(&mut self, id: LabelId) -> &mut KVector {
        &mut self.label_pos[id.idx()]
    }

    #[inline]
    pub fn label_size(&self, id: LabelId) -> KVector {
        self.label_size[id.idx()]
    }

    #[inline]
    pub fn label_text(&self, id: LabelId) -> &str {
        &self.label_text[id.idx()]
    }

    #[inline]
    pub fn label_element_id(&self, id: LabelId) -> i32 {
        self.label_element_id[id.idx()]
    }

    #[inline]
    pub fn label_properties(&self, id: LabelId) -> &MapPropertyHolder {
        &self.label_properties[id.idx()]
    }

    #[inline]
    pub fn label_properties_mut(&mut self, id: LabelId) -> &mut MapPropertyHolder {
        &mut self.label_properties[id.idx()]
    }

    // ── Layer attribute accessors ───────────────────────────────────

    #[inline]
    pub fn layer_size(&self, id: LayerId) -> KVector {
        self.layer_size[id.idx()]
    }

    #[inline]
    pub fn layer_size_mut(&mut self, id: LayerId) -> &mut KVector {
        &mut self.layer_size[id.idx()]
    }

    #[inline]
    pub fn layer_element_id(&self, id: LayerId) -> i32 {
        self.layer_element_id[id.idx()]
    }

    // ── CSR topology accessors (immutable) ──────────────────────────

    /// Port IDs belonging to a node.
    #[inline]
    pub fn node_ports(&self, id: NodeId) -> &[PortId] {
        let i = id.idx();
        let start = self.node_port_offset[i] as usize;
        let end = self.node_port_offset[i + 1] as usize;
        &self.node_port_ids[start..end]
    }

    /// Label IDs belonging to a node.
    #[inline]
    pub fn node_labels(&self, id: NodeId) -> &[LabelId] {
        let i = id.idx();
        let start = self.node_label_offset[i] as usize;
        let end = self.node_label_offset[i + 1] as usize;
        &self.node_label_ids[start..end]
    }

    /// Incoming edge IDs for a port.
    #[inline]
    pub fn port_incoming_edges(&self, id: PortId) -> &[EdgeId] {
        let i = id.idx();
        let start = self.port_in_offset[i] as usize;
        let end = self.port_in_offset[i + 1] as usize;
        &self.port_in_edges[start..end]
    }

    /// Outgoing edge IDs for a port.
    #[inline]
    pub fn port_outgoing_edges(&self, id: PortId) -> &[EdgeId] {
        let i = id.idx();
        let start = self.port_out_offset[i] as usize;
        let end = self.port_out_offset[i + 1] as usize;
        &self.port_out_edges[start..end]
    }

    /// Label IDs belonging to a port.
    #[inline]
    pub fn port_labels(&self, id: PortId) -> &[LabelId] {
        let i = id.idx();
        let start = self.port_label_offset[i] as usize;
        let end = self.port_label_offset[i + 1] as usize;
        &self.port_label_ids[start..end]
    }

    /// Label IDs belonging to an edge.
    #[inline]
    pub fn edge_labels(&self, id: EdgeId) -> &[LabelId] {
        let i = id.idx();
        let start = self.edge_label_offset[i] as usize;
        let end = self.edge_label_offset[i + 1] as usize;
        &self.edge_label_ids[start..end]
    }

    /// Node IDs belonging to a layer (in order).
    #[inline]
    pub fn layer_nodes(&self, id: LayerId) -> &[NodeId] {
        let i = id.idx();
        let start = self.layer_node_offset[i] as usize;
        let end = self.layer_node_offset[i + 1] as usize;
        &self.layer_node_ids[start..end]
    }

    // ── Derived lookups ─────────────────────────────────────────────

    /// Get the layer index of a port's owning node.
    #[inline]
    pub fn port_owner_layer(&self, id: PortId) -> LayerId {
        self.node_layer[self.port_owner[id.idx()].idx()]
    }

    /// Source port IDs of incoming edges to a port (predecessors).
    pub fn port_predecessors(&self, id: PortId) -> Vec<PortId> {
        self.port_incoming_edges(id)
            .iter()
            .map(|&eid| self.edge_source[eid.idx()])
            .collect()
    }

    /// Target port IDs of outgoing edges from a port (successors).
    pub fn port_successors(&self, id: PortId) -> Vec<PortId> {
        self.port_outgoing_edges(id)
            .iter()
            .map(|&eid| self.edge_target[eid.idx()])
            .collect()
    }
}
