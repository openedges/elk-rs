use std::collections::HashMap;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::{LNodeRef, NodeType};

/// Phase-local snapshot of graph adjacency for lock-free access during P3 crossing minimization.
///
/// Captures node/port/edge topology as flat indexed arrays (CSR format).
/// Built once after `init_initializables`; all subsequent lookups are lock-free.
///
/// Index spaces:
/// - **flat node index** (`u32`): globally unique, contiguous 0..n_nodes across all layers
/// - **port ID** (`u32`): equals `graph_element().id`, globally unique 0..n_ports
pub struct CrossMinSnapshot {
    // ── Primary lookup: Arc pointer → index ──────────────────────────
    /// Arc::as_ptr(node) as usize → flat node index
    node_map: HashMap<usize, u32>,
    /// Arc::as_ptr(port) as usize → port ID (= graph_element().id)
    port_map: HashMap<usize, u32>,

    // ── Node attributes (indexed by flat node index) ─────────────────
    /// graph_element().id of each node (= position within its layer)
    node_graph_id: Vec<u32>,
    /// Layer index of each node
    node_layer: Vec<u32>,
    /// Node type
    node_type: Vec<NodeType>,

    // ── Node → port CSR (indexed by flat node index) ─────────────────
    /// `node_port_offset[i]..node_port_offset[i+1]` indexes into `node_port_ids`
    node_port_offset: Vec<u32>,
    /// Flat list of port IDs belonging to each node (in iteration order)
    node_port_ids: Vec<u32>,

    // ── Port attributes (indexed by port ID) ─────────────────────────
    port_side: Vec<PortSide>,
    /// flat node index of the port's owning node
    port_owner: Vec<u32>,

    // ── Port → incoming predecessor CSR (indexed by port ID) ─────────
    /// `port_in_offset[p]..port_in_offset[p+1]` indexes into `port_in_src`
    port_in_offset: Vec<u32>,
    /// Source port IDs (predecessors) for all ports
    port_in_src: Vec<u32>,

    // ── Port → outgoing successor CSR (indexed by port ID) ───────────
    /// `port_out_offset[p]..port_out_offset[p+1]` indexes into `port_out_tgt`
    port_out_offset: Vec<u32>,
    /// Target port IDs (successors) for all ports
    port_out_tgt: Vec<u32>,

    n_nodes: u32,
    n_ports: u32,
}

impl CrossMinSnapshot {
    /// Build a snapshot from the current node order.
    ///
    /// Must be called AFTER `init_initializables` so that all `graph_element().id`
    /// values are finalized. Locks each node/port/edge once during construction.
    pub fn build(order: &[Vec<LNodeRef>], n_ports: usize) -> Self {
        let total_nodes: usize = order.iter().map(|l| l.len()).sum();

        let mut node_map = HashMap::with_capacity(total_nodes);
        let mut port_map = HashMap::with_capacity(n_ports);

        let mut node_graph_id = Vec::with_capacity(total_nodes);
        let mut node_layer = Vec::with_capacity(total_nodes);
        let mut node_type_vec = Vec::with_capacity(total_nodes);
        let mut node_port_offset = Vec::with_capacity(total_nodes + 1);
        let mut node_port_ids = Vec::with_capacity(n_ports);

        let mut port_side = vec![PortSide::Undefined; n_ports];
        let mut port_owner = vec![0u32; n_ports];

        // Temp storage for per-port edge lists (converted to CSR after traversal)
        let mut port_in_lists: Vec<Vec<u32>> = vec![Vec::new(); n_ports];
        let mut port_out_lists: Vec<Vec<u32>> = vec![Vec::new(); n_ports];

        let mut flat_node_idx: u32 = 0;

        for (layer_index, layer_nodes) in order.iter().enumerate() {
            for node_ref in layer_nodes {
                let node_ptr = Arc::as_ptr(node_ref) as usize;
                node_map.insert(node_ptr, flat_node_idx);

                if let Ok(mut node_guard) = node_ref.lock() {
                    node_graph_id.push(node_guard.shape().graph_element().id as u32);
                    node_layer.push(layer_index as u32);
                    node_type_vec.push(node_guard.node_type());

                    node_port_offset.push(node_port_ids.len() as u32);

                    for port_ref in node_guard.ports() {
                        let port_ptr = Arc::as_ptr(port_ref) as usize;

                        if let Ok(mut port_guard) = port_ref.lock() {
                            let pid = port_guard.shape().graph_element().id as u32;
                            let pid_usize = pid as usize;

                            port_map.insert(port_ptr, pid);
                            node_port_ids.push(pid);

                            if pid_usize < n_ports {
                                port_side[pid_usize] = port_guard.side();
                                port_owner[pid_usize] = flat_node_idx;

                                // Collect incoming edge source ports
                                for edge in port_guard.incoming_edges() {
                                    if let Ok(edge_guard) = edge.lock() {
                                        if let Some(src_port) = edge_guard.source() {
                                            if let Ok(mut src_guard) = src_port.lock() {
                                                let src_pid =
                                                    src_guard.shape().graph_element().id as u32;
                                                port_in_lists[pid_usize].push(src_pid);
                                            }
                                        }
                                    }
                                }

                                // Collect outgoing edge target ports
                                for edge in port_guard.outgoing_edges() {
                                    if let Ok(edge_guard) = edge.lock() {
                                        if let Some(tgt_port) = edge_guard.target() {
                                            if let Ok(mut tgt_guard) = tgt_port.lock() {
                                                let tgt_pid =
                                                    tgt_guard.shape().graph_element().id as u32;
                                                port_out_lists[pid_usize].push(tgt_pid);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                flat_node_idx += 1;
            }
        }
        // Sentinel for last node
        node_port_offset.push(node_port_ids.len() as u32);

        // Convert per-port edge lists to CSR
        let mut port_in_offset = Vec::with_capacity(n_ports + 1);
        let mut port_in_src = Vec::new();
        port_in_offset.push(0);
        for list in &port_in_lists {
            port_in_src.extend_from_slice(list);
            port_in_offset.push(port_in_src.len() as u32);
        }

        let mut port_out_offset = Vec::with_capacity(n_ports + 1);
        let mut port_out_tgt = Vec::new();
        port_out_offset.push(0);
        for list in &port_out_lists {
            port_out_tgt.extend_from_slice(list);
            port_out_offset.push(port_out_tgt.len() as u32);
        }

        CrossMinSnapshot {
            node_map,
            port_map,
            node_graph_id,
            node_layer,
            node_type: node_type_vec,
            node_port_offset,
            node_port_ids,
            port_side,
            port_owner,
            port_in_offset,
            port_in_src,
            port_out_offset,
            port_out_tgt,
            n_nodes: flat_node_idx,
            n_ports: n_ports as u32,
        }
    }

    // ── Lock-free ID lookups (replaces port_id() / node_id() / layer_index()) ──

    /// Get port's `graph_element().id` from its Arc pointer, without locking.
    #[inline]
    pub fn port_id(&self, port: &crate::org::eclipse::elk::alg::layered::graph::LPortRef) -> u32 {
        let ptr = Arc::as_ptr(port) as usize;
        self.port_map.get(&ptr).copied().unwrap_or(0)
    }

    /// Get node's `graph_element().id` (per-layer position) from its Arc pointer.
    #[inline]
    pub fn node_id(&self, node: &LNodeRef) -> u32 {
        let ptr = Arc::as_ptr(node) as usize;
        self.node_map
            .get(&ptr)
            .map(|&flat| self.node_graph_id[flat as usize])
            .unwrap_or(0)
    }

    /// Get node's layer index from its Arc pointer.
    #[inline]
    pub fn node_layer_index(&self, node: &LNodeRef) -> u32 {
        let ptr = Arc::as_ptr(node) as usize;
        self.node_map
            .get(&ptr)
            .map(|&flat| self.node_layer[flat as usize])
            .unwrap_or(0)
    }

    /// Get the flat (global) node index from its Arc pointer.
    #[inline]
    pub fn node_flat_index(&self, node: &LNodeRef) -> u32 {
        let ptr = Arc::as_ptr(node) as usize;
        self.node_map.get(&ptr).copied().unwrap_or(0)
    }

    /// Check if two nodes are in the same layer, without locking.
    #[inline]
    pub fn same_layer(&self, a: &LNodeRef, b: &LNodeRef) -> bool {
        self.node_layer_index(a) == self.node_layer_index(b)
    }

    // ── CSR accessors ──────────────────────────────────────────────────

    /// Port IDs belonging to a node (by flat node index).
    #[inline]
    pub fn node_ports(&self, flat_node: u32) -> &[u32] {
        let start = self.node_port_offset[flat_node as usize] as usize;
        let end = self.node_port_offset[flat_node as usize + 1] as usize;
        &self.node_port_ids[start..end]
    }

    /// Source port IDs of incoming edges to a port (predecessors).
    #[inline]
    pub fn port_predecessors(&self, port_id: u32) -> &[u32] {
        let pid = port_id as usize;
        if pid >= self.n_ports as usize {
            return &[];
        }
        let start = self.port_in_offset[pid] as usize;
        let end = self.port_in_offset[pid + 1] as usize;
        &self.port_in_src[start..end]
    }

    /// Target port IDs of outgoing edges from a port (successors).
    #[inline]
    pub fn port_successors(&self, port_id: u32) -> &[u32] {
        let pid = port_id as usize;
        if pid >= self.n_ports as usize {
            return &[];
        }
        let start = self.port_out_offset[pid] as usize;
        let end = self.port_out_offset[pid + 1] as usize;
        &self.port_out_tgt[start..end]
    }

    /// Get the flat node index of a port's owning node.
    #[inline]
    pub fn port_owner_flat(&self, port_id: u32) -> u32 {
        let pid = port_id as usize;
        if pid < self.n_ports as usize {
            self.port_owner[pid]
        } else {
            0
        }
    }

    /// Get the layer index of a port's owning node.
    #[inline]
    pub fn port_owner_layer(&self, port_id: u32) -> u32 {
        let flat = self.port_owner_flat(port_id) as usize;
        if flat < self.n_nodes as usize {
            self.node_layer[flat]
        } else {
            0
        }
    }

    /// Get the side of a port.
    #[inline]
    pub fn port_side_of(&self, port_id: u32) -> PortSide {
        let pid = port_id as usize;
        if pid < self.n_ports as usize {
            self.port_side[pid]
        } else {
            PortSide::Undefined
        }
    }

    /// Get the node type for a flat node index.
    #[inline]
    pub fn node_type_of(&self, flat_node: u32) -> NodeType {
        let idx = flat_node as usize;
        if idx < self.n_nodes as usize {
            self.node_type[idx]
        } else {
            NodeType::Normal
        }
    }

    /// Get the graph_element().id (per-layer position) for a flat node index.
    #[inline]
    pub fn node_graph_id_of(&self, flat_node: u32) -> u32 {
        let idx = flat_node as usize;
        if idx < self.n_nodes as usize {
            self.node_graph_id[idx]
        } else {
            0
        }
    }

    /// Get the layer index for a flat node index.
    #[inline]
    pub fn node_layer_of(&self, flat_node: u32) -> u32 {
        let idx = flat_node as usize;
        if idx < self.n_nodes as usize {
            self.node_layer[idx]
        } else {
            0
        }
    }

    /// Total number of ports in the snapshot.
    #[inline]
    pub fn n_ports(&self) -> u32 {
        self.n_ports
    }

    /// Total number of nodes in the snapshot.
    #[inline]
    pub fn n_nodes(&self) -> u32 {
        self.n_nodes
    }
}
