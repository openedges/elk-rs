use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphRef, LNodeRef, LPortRef, LayerRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions, Origin};
use crate::org::eclipse::elk::alg::layered::p3order::cross_min_snapshot::CrossMinSnapshot;

/// Snapshot of node and port orderings using compact u32 indices.
///
/// Stores flat node indices and port IDs from [`CrossMinSnapshot`] instead of
/// `Arc` references.  This makes `copy_from_sweep` a pure `u32` memcpy with
/// zero `Arc` refcounting.
#[derive(Clone)]
pub struct SweepCopy {
    /// Flat node indices (from CrossMinSnapshot) per layer.
    node_order: Vec<Vec<u32>>,
    /// Port IDs (graph_element().id) per node per layer.
    port_orders: Vec<Vec<Vec<u32>>>,
}

impl SweepCopy {
    pub fn new(node_order_in: &[Vec<LNodeRef>], snapshot: &CrossMinSnapshot) -> Self {
        let node_order: Vec<Vec<u32>> = node_order_in
            .iter()
            .map(|layer| {
                layer
                    .iter()
                    .map(|node| snapshot.node_flat_index(node))
                    .collect()
            })
            .collect();

        let port_orders: Vec<Vec<Vec<u32>>> = node_order_in
            .iter()
            .map(|layer| {
                layer
                    .iter()
                    .map(|node| {
                        {
                        let node_guard = node.lock();
                        node_guard
                            .ports()
                            .iter()
                            .map(|p| snapshot.port_id(p))
                            .collect()
                    }
                    })
                    .collect()
            })
            .collect();

        SweepCopy {
            node_order,
            port_orders,
        }
    }

    /// Reuse existing allocations to copy from a new node order.
    pub fn copy_from(&mut self, node_order_in: &[Vec<LNodeRef>], snapshot: &CrossMinSnapshot) {
        let target_len = node_order_in.len();

        // Reuse outer + inner Vec allocations for node_order
        self.node_order.resize_with(target_len, Vec::new);
        self.node_order.truncate(target_len);
        for (i, layer) in node_order_in.iter().enumerate() {
            self.node_order[i].clear();
            self.node_order[i]
                .extend(layer.iter().map(|n| snapshot.node_flat_index(n)));
        }

        // Reuse all 3 levels of Vec allocations for port_orders
        self.port_orders.resize_with(target_len, Vec::new);
        self.port_orders.truncate(target_len);
        for (i, layer) in node_order_in.iter().enumerate() {
            let inner = &mut self.port_orders[i];
            inner.resize_with(layer.len(), Vec::new);
            inner.truncate(layer.len());
            for (j, node) in layer.iter().enumerate() {
                inner[j].clear();
                {
                    let node_guard = node.lock();
                    inner[j].extend(node_guard.ports().iter().map(|p| snapshot.port_id(p)));
                }
            }
        }
    }

    /// Reuse existing allocations to copy from another SweepCopy.
    ///
    /// This is now a pure `u32` memcpy — no `Arc` refcounting at all.
    pub fn copy_from_sweep(&mut self, source: &SweepCopy) {
        let target_len = source.node_order.len();

        // Reuse outer + inner Vec allocations for node_order
        self.node_order.resize_with(target_len, Vec::new);
        self.node_order.truncate(target_len);
        for (i, layer) in source.node_order.iter().enumerate() {
            self.node_order[i].clear();
            self.node_order[i].extend_from_slice(layer);
        }

        // Reuse all 3 levels of Vec allocations for port_orders — copy from source's saved snapshot
        self.port_orders.resize_with(target_len, Vec::new);
        self.port_orders.truncate(target_len);
        for (i, layer_ports) in source.port_orders.iter().enumerate() {
            let inner = &mut self.port_orders[i];
            inner.resize_with(layer_ports.len(), Vec::new);
            inner.truncate(layer_ports.len());
            for (j, ports) in layer_ports.iter().enumerate() {
                inner[j].clear();
                inner[j].extend_from_slice(ports);
            }
        }
    }

    /// Get the node order as flat node indices.
    pub fn node_indices(&self) -> &Vec<Vec<u32>> {
        &self.node_order
    }

    pub fn transfer_node_and_port_orders_to_graph(
        &self,
        l_graph: &LGraphRef,
        set_port_constraints: bool,
        snapshot: &CrossMinSnapshot,
    ) {
        let layers = {
            let graph_guard = l_graph.lock();
            graph_guard.layers().clone()
        };
        self.apply_to_layers(&layers, set_port_constraints, snapshot);
    }

    pub fn transfer_node_and_port_orders_to_graph_guard(
        &self,
        graph: &mut LGraph,
        set_port_constraints: bool,
        snapshot: &CrossMinSnapshot,
    ) {
        let layers = graph.layers().clone();
        self.apply_to_layers(&layers, set_port_constraints, snapshot);
    }

    fn apply_to_layers(
        &self,
        layers: &[LayerRef],
        set_port_constraints: bool,
        snapshot: &CrossMinSnapshot,
    ) {
        let mut update_port_order: Vec<LNodeRef> = Vec::new();

        for (layer_index, layer_ref) in layers.iter().enumerate() {
            let mut north_south_port_dummies: Vec<LNodeRef> = Vec::new();
            let node_count = self
                .node_order
                .get(layer_index)
                .map(|layer| layer.len())
                .unwrap_or(0);
            for node_index in 0..node_count {
                let flat_idx = self.node_order[layer_index][node_index];
                let node = snapshot.node_ref(flat_idx).clone();
                // Single node lock for id, type check, port orders, and constraints
                {
                    let mut node_guard = node.lock();
                    node_guard.shape().graph_element().id = node_index as i32;
                    if node_guard.node_type() == NodeType::NorthSouthPort {
                        north_south_port_dummies.push(node.clone());
                    }
                    node_guard.ports_mut().clear();
                    if let Some(ports) = self
                        .port_orders
                        .get(layer_index)
                        .and_then(|layer_ports| layer_ports.get(node_index))
                    {
                        for &pid in ports {
                            if let Some(port_ref) = snapshot.port_ref_opt(pid) {
                                node_guard.ports_mut().push(port_ref.clone());
                            }
                        }
                    }
                    if set_port_constraints {
                        let constraints = node_guard
                            .get_property(LayeredOptions::PORT_CONSTRAINTS)
                            .unwrap_or(PortConstraints::Undefined);
                        if !constraints.is_order_fixed() {
                            node_guard.set_property(
                                LayeredOptions::PORT_CONSTRAINTS,
                                Some(PortConstraints::FixedOrder),
                            );
                        }
                    }
                }

                {
                    let mut layer_guard = layer_ref.lock();
                    if node_index < layer_guard.nodes().len() {
                        layer_guard.nodes_mut()[node_index] = node.clone();
                    } else {
                        layer_guard.nodes_mut().push(node.clone());
                    }
                }
            }

            for dummy in north_south_port_dummies {
                if let Some(origin) = assert_correct_port_sides(&dummy) {
                    push_unique(&mut update_port_order, &origin);
                    push_unique(&mut update_port_order, &dummy);
                }
            }
        }

        for node in update_port_order {
            {
                let mut node_guard = node.lock();
                let constraints = node_guard
                    .get_property(LayeredOptions::PORT_CONSTRAINTS)
                    .unwrap_or(PortConstraints::Undefined);
                sort_ports_combined(node_guard.ports_mut(), constraints);
                node_guard.cache_port_sides();
            }
        }
    }
}

fn assert_correct_port_sides(dummy: &LNodeRef) -> Option<LNodeRef> {
    // Batch-extract dummy data in a single lock
    let (origin, dummy_ports, dummy_id) = {
        let mut node_guard = dummy.lock();
        let origin = node_guard
            .get_property(InternalProperties::IN_LAYER_LAYOUT_UNIT)?;
        let ports = node_guard.ports().clone();
        let id = node_guard.shape().graph_element().id;
        (origin, ports, id)
    };
    // Batch-extract origin data in a single lock (constraints, id, ports)
    let (origin_constraints, origin_id, ports) = {
        let mut origin_guard = origin.lock();
        let c = origin_guard
            .get_property(LayeredOptions::PORT_CONSTRAINTS)
            .unwrap_or(PortConstraints::Undefined);
        let id = origin_guard.shape().graph_element().id;
        let p = origin_guard.ports().clone();
        (c, id, p)
    };
    if origin_constraints.is_pos_fixed() {
        return Some(origin);
    }
    let dummy_port = dummy_ports.first()?.clone();
    let dummy_origin_port = {
        let port_guard = dummy_port.lock();
        port_guard.get_property(InternalProperties::ORIGIN)
            .and_then(|origin| match origin {
                Origin::LPort(port) => Some(port),
                _ => None,
            })
    };
    let Some(dummy_origin_port) = dummy_origin_port else {
        return Some(origin);
    };
    for port in ports {
        if Arc::ptr_eq(&port, &dummy_origin_port) {
            {
                let mut port_guard = port.lock();
                let side = port_guard.side();
                if side == PortSide::North && dummy_id > origin_id {
                    port_guard.set_side(PortSide::South);
                    if port_guard.is_explicitly_supplied_port_anchor() {
                        let port_height = port_guard.shape().size_ref().y;
                        let anchor_y = port_guard.anchor_ref().y;
                        port_guard.anchor().y = port_height - anchor_y;
                    }
                } else if side == PortSide::South && origin_id > dummy_id {
                    port_guard.set_side(PortSide::North);
                    if port_guard.is_explicitly_supplied_port_anchor() {
                        let port_height = port_guard.shape().size_ref().y;
                        let anchor_y = port_guard.anchor_ref().y;
                        port_guard.anchor().y = -(port_height - anchor_y);
                    }
                }
            }
            break;
        }
    }
    Some(origin)
}

fn push_unique(list: &mut Vec<LNodeRef>, node: &LNodeRef) {
    if list.iter().any(|existing| Arc::ptr_eq(existing, node)) {
        return;
    }
    list.push(node.clone());
}

fn sort_ports_combined(ports: &mut [LPortRef], constraints: PortConstraints) {
    ports.sort_by(|p1, p2| compare_ports(p1, p2, constraints));
}

fn compare_ports(p1: &LPortRef, p2: &LPortRef, constraints: PortConstraints) -> std::cmp::Ordering {
    use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;

    // Batch-extract all needed data from each port in a single lock
    let extract = |port: &LPortRef| -> (PortSide, Option<i32>, KVector) {
        let mut port_guard = port.lock();
        let side = port_guard.side();
        let idx = if constraints == PortConstraints::FixedOrder {
            port_guard.get_property(LayeredOptions::PORT_INDEX)
        } else {
            None
        };
        let pos = *port_guard.shape().position_ref();
        (side, idx, pos)
    };
    let (side1, idx1, pos1) = extract(p1);
    let (side2, idx2, pos2) = extract(p2);

    let side_cmp = side1.cmp(&side2);
    if side_cmp != std::cmp::Ordering::Equal {
        return side_cmp;
    }

    if !constraints.is_order_fixed() {
        return std::cmp::Ordering::Equal;
    }

    if constraints == PortConstraints::FixedOrder {
        if let (Some(i1), Some(i2)) = (idx1, idx2) {
            if i1 != i2 {
                return i1.cmp(&i2);
            }
        }
    }

    match side1 {
        PortSide::North => pos1
            .x
            .partial_cmp(&pos2.x)
            .unwrap_or(std::cmp::Ordering::Equal),
        PortSide::East => pos1
            .y
            .partial_cmp(&pos2.y)
            .unwrap_or(std::cmp::Ordering::Equal),
        PortSide::South => pos2
            .x
            .partial_cmp(&pos1.x)
            .unwrap_or(std::cmp::Ordering::Equal),
        PortSide::West => pos2
            .y
            .partial_cmp(&pos1.y)
            .unwrap_or(std::cmp::Ordering::Equal),
        PortSide::Undefined => std::cmp::Ordering::Equal,
    }
}
