use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LGraphRef, LNodeRef, LPortRef, LayerRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions, Origin};

#[derive(Clone)]
pub struct SweepCopy {
    node_order: Vec<Vec<LNodeRef>>,
    port_orders: Vec<Vec<Vec<LPortRef>>>,
}

impl SweepCopy {
    pub fn new(node_order_in: &[Vec<LNodeRef>]) -> Self {
        let node_order = deep_copy(node_order_in);
        let mut port_orders: Vec<Vec<Vec<LPortRef>>> = Vec::new();
        for layer in node_order_in {
            let mut layer_ports = Vec::new();
            for node in layer {
                let ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.ports().clone())
                    .unwrap_or_default();
                layer_ports.push(ports);
            }
            port_orders.push(layer_ports);
        }
        SweepCopy {
            node_order,
            port_orders,
        }
    }

    pub fn nodes(&self) -> &Vec<Vec<LNodeRef>> {
        &self.node_order
    }

    pub fn transfer_node_and_port_orders_to_graph(
        &self,
        l_graph: &LGraphRef,
        set_port_constraints: bool,
    ) {
        let layers = l_graph
            .lock()
            .ok()
            .map(|graph_guard| graph_guard.layers().clone())
            .unwrap_or_default();
        self.apply_to_layers(&layers, set_port_constraints);
    }

    pub fn transfer_node_and_port_orders_to_graph_guard(
        &self,
        graph: &mut LGraph,
        set_port_constraints: bool,
    ) {
        let layers = graph.layers().clone();
        self.apply_to_layers(&layers, set_port_constraints);
    }

    fn apply_to_layers(&self, layers: &[LayerRef], set_port_constraints: bool) {
        let mut update_port_order: Vec<LNodeRef> = Vec::new();

        for (layer_index, layer_ref) in layers.iter().enumerate() {
            let mut north_south_port_dummies: Vec<LNodeRef> = Vec::new();
            let node_count = self
                .node_order
                .get(layer_index)
                .map(|layer| layer.len())
                .unwrap_or(0);
            for node_index in 0..node_count {
                let node = self.node_order[layer_index][node_index].clone();
                if let Ok(mut node_guard) = node.lock() {
                    node_guard.shape().graph_element().id = node_index as i32;
                    if node_guard.node_type() == NodeType::NorthSouthPort {
                        north_south_port_dummies.push(node.clone());
                    }
                };

                if let Ok(mut layer_guard) = layer_ref.lock() {
                    if node_index < layer_guard.nodes().len() {
                        layer_guard.nodes_mut()[node_index] = node.clone();
                    } else {
                        layer_guard.nodes_mut().push(node.clone());
                    }
                };

                if let Ok(mut node_guard) = node.lock() {
                    node_guard.ports_mut().clear();
                    if let Some(ports) = self
                        .port_orders
                        .get(layer_index)
                        .and_then(|layer_ports| layer_ports.get(node_index))
                    {
                        node_guard.ports_mut().extend(ports.iter().cloned());
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
                };
            }

            for dummy in north_south_port_dummies {
                if let Some(origin) = assert_correct_port_sides(&dummy) {
                    push_unique(&mut update_port_order, &origin);
                    push_unique(&mut update_port_order, &dummy);
                }
            }
        }

        for node in update_port_order {
            if let Ok(mut node_guard) = node.lock() {
                let constraints = node_guard
                    .get_property(LayeredOptions::PORT_CONSTRAINTS)
                    .unwrap_or(PortConstraints::Undefined);
                sort_ports_combined(node_guard.ports_mut(), constraints);
                node_guard.cache_port_sides();
            };
        }
    }
}

fn deep_copy(node_order: &[Vec<LNodeRef>]) -> Vec<Vec<LNodeRef>> {
    node_order.iter().map(|layer| layer.clone()).collect()
}

fn assert_correct_port_sides(dummy: &LNodeRef) -> Option<LNodeRef> {
    let origin = dummy
        .lock()
        .ok()
        .and_then(|mut node_guard| node_guard.get_property(InternalProperties::IN_LAYER_LAYOUT_UNIT));
    let origin = origin?;
    let dummy_ports = dummy
        .lock()
        .ok()
        .map(|node_guard| node_guard.ports().clone())
        .unwrap_or_default();
    let dummy_port = dummy_ports.get(0)?.clone();
    let dummy_origin_port = dummy_port
        .lock()
        .ok()
        .and_then(|mut port_guard| port_guard.get_property(InternalProperties::ORIGIN))
        .and_then(|origin| match origin {
            Origin::LPort(port) => Some(port),
            _ => None,
        });
    let Some(dummy_origin_port) = dummy_origin_port else { return Some(origin); };

    let origin_id = origin
        .lock()
        .ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id)
        .unwrap_or(0);
    let dummy_id = dummy
        .lock()
        .ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id)
        .unwrap_or(0);
    let ports = origin
        .lock()
        .ok()
        .map(|node_guard| node_guard.ports().clone())
        .unwrap_or_default();
    for port in ports {
        if Arc::ptr_eq(&port, &dummy_origin_port) {
            if let Ok(mut port_guard) = port.lock() {
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

fn sort_ports_combined(ports: &mut Vec<LPortRef>, constraints: PortConstraints) {
    ports.sort_by(|p1, p2| compare_ports(p1, p2, constraints));
}

fn compare_ports(
    p1: &LPortRef,
    p2: &LPortRef,
    constraints: PortConstraints,
) -> std::cmp::Ordering {
    let side1 = p1.lock().ok().map(|port_guard| port_guard.side()).unwrap_or(PortSide::Undefined);
    let side2 = p2.lock().ok().map(|port_guard| port_guard.side()).unwrap_or(PortSide::Undefined);
    let side_cmp = side1.cmp(&side2);
    if side_cmp != std::cmp::Ordering::Equal {
        return side_cmp;
    }

    if !constraints.is_order_fixed() {
        return std::cmp::Ordering::Equal;
    }

    if constraints == PortConstraints::FixedOrder {
        let idx1 = p1
            .lock()
            .ok()
            .and_then(|mut port_guard| port_guard.get_property(LayeredOptions::PORT_INDEX));
        let idx2 = p2
            .lock()
            .ok()
            .and_then(|mut port_guard| port_guard.get_property(LayeredOptions::PORT_INDEX));
        if let (Some(i1), Some(i2)) = (idx1, idx2) {
            if i1 != i2 {
                return i1.cmp(&i2);
            }
        }
    }

    let pos1 = p1
        .lock()
        .ok()
        .map(|mut port_guard| port_guard.shape().position_ref().clone())
        .unwrap_or_default();
    let pos2 = p2
        .lock()
        .ok()
        .map(|mut port_guard| port_guard.shape().position_ref().clone())
        .unwrap_or_default();
    match side1 {
        PortSide::North => pos1.x.partial_cmp(&pos2.x).unwrap_or(std::cmp::Ordering::Equal),
        PortSide::East => pos1.y.partial_cmp(&pos2.y).unwrap_or(std::cmp::Ordering::Equal),
        PortSide::South => pos2.x.partial_cmp(&pos1.x).unwrap_or(std::cmp::Ordering::Equal),
        PortSide::West => pos2.y.partial_cmp(&pos1.y).unwrap_or(std::cmp::Ordering::Equal),
        PortSide::Undefined => std::cmp::Ordering::Equal,
    }
}
