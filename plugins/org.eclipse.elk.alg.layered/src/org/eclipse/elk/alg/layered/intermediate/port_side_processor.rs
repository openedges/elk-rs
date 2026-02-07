use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNodeRef, LPortRef};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};

pub struct PortSideProcessor;

impl ILayoutProcessor<LGraph> for PortSideProcessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Port side processing", 1.0);

        for node in layered_graph.layerless_nodes().clone() {
            process_node(&node);
        }

        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                process_node(&node);
            }
        }

        monitor.done();
    }
}

fn process_node(node: &LNodeRef) {
    let constraints = node
        .lock()
        .ok()
        .and_then(|mut node_guard| {
            if node_guard
                .shape()
                .graph_element()
                .properties_mut()
                .has_property(LayeredOptions::PORT_CONSTRAINTS)
            {
                node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS)
            } else {
                None
            }
        })
        .unwrap_or(PortConstraints::Undefined);
    let ports = node
        .lock()
        .ok()
        .map(|node_guard| node_guard.ports().clone())
        .unwrap_or_default();

    if constraints.is_side_fixed() {
        for port in ports {
            let side = port
                .lock()
                .ok()
                .map(|port_guard| port_guard.side())
                .unwrap_or(PortSide::Undefined);
            if side == PortSide::Undefined {
                set_port_side(&port);
            }
        }
    } else {
        for port in ports {
            set_port_side(&port);
        }
        if let Ok(mut node_guard) = node.lock() {
            node_guard.set_property(
                LayeredOptions::PORT_CONSTRAINTS,
                Some(PortConstraints::FixedSide),
            );
        }
    }
}

pub fn set_port_side(port: &LPortRef) {
    let mut assigned_side: Option<PortSide> = None;
    if let Ok(mut port_guard) = port.lock() {
        if let Some(port_dummy) = port_guard.get_property(InternalProperties::PORT_DUMMY) {
            assigned_side = port_dummy
                .lock()
                .ok()
                .and_then(|mut dummy_guard| dummy_guard.get_property(InternalProperties::EXT_PORT_SIDE));
        }

        let side = assigned_side.unwrap_or_else(|| {
            if port_guard.net_flow() < 0 {
                PortSide::East
            } else {
                PortSide::West
            }
        });
        port_guard.set_side(side);
    }
}
