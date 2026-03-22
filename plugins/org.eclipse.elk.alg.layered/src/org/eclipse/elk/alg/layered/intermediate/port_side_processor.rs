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
                .lock().nodes().clone();
            for node in nodes {
                process_node(&node);
            }
        }

        monitor.done();
    }
}

fn process_node(node: &LNodeRef) {
    let constraints = {
        let mut node_guard = node.lock();
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
    }
    .unwrap_or(PortConstraints::Undefined);
    let ports = node
        .lock().ports().clone();

    if constraints.is_side_fixed() {
        for port in ports {
            let side = port
                .lock().side();
            if side == PortSide::Undefined {
                set_port_side(&port);
            }
        }
    } else {
        for port in ports {
            set_port_side(&port);
        }
        {
            let mut node_guard = node.lock();
            node_guard.set_property(
                LayeredOptions::PORT_CONSTRAINTS,
                Some(PortConstraints::FixedSide),
            );
        }
    }
}

pub fn set_port_side(port: &LPortRef) {
    let mut assigned_side: Option<PortSide> = None;
    {
        let mut port_guard = port.lock();
        let port_dummy = port_guard.get_property(InternalProperties::PORT_DUMMY);
        if let Some(port_dummy) = port_dummy.as_ref() {
            assigned_side = port_dummy.lock()
                .get_property(InternalProperties::EXT_PORT_SIDE);
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
