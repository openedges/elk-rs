use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph, LPort};
use crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfLoopHolder;
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions,
};

pub struct SelfLoopPreProcessor;

impl ILayoutProcessor<LGraph> for SelfLoopPreProcessor {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Self-Loop pre-processing", 1.0);

        for lnode in graph.layerless_nodes().clone() {
            if SelfLoopHolder::needs_self_loop_processing(&lnode) {
                let sl_holder = SelfLoopHolder::install(&lnode);
                hide_self_loops(&sl_holder);
                hide_ports(&sl_holder);
            }
        }

        monitor.done();
    }
}

fn hide_self_loops(holder: &crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfLoopHolderRef) {
    let edges = holder
        .lock()
        .ok()
        .map(|holder_guard| holder_guard.all_self_loop_edges())
        .unwrap_or_default();

    for edge in edges {
        LEdge::set_source(&edge, None);
        LEdge::set_target(&edge, None);
    }
}

fn hide_ports(holder: &crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfLoopHolderRef) {
    let node = holder
        .lock()
        .ok()
        .map(|holder_guard| holder_guard.l_node().clone());
    let Some(node) = node else {
        return;
    };

    let (order_fixed, nested_graph) = node
        .lock()
        .ok()
        .map(|mut node_guard| {
            let constraints = node_guard
                .get_property(LayeredOptions::PORT_CONSTRAINTS)
                .unwrap_or(PortConstraints::Undefined);
            (constraints.is_order_fixed(), node_guard.nested_graph())
        })
        .unwrap_or((false, None));

    let hierarchy_mode = nested_graph
        .and_then(|graph| {
            graph.lock().ok().and_then(|mut graph_guard| {
                graph_guard.get_property(InternalProperties::GRAPH_PROPERTIES)
            })
        })
        .is_some_and(|graph_props| graph_props.contains(&GraphProperties::ExternalPorts));

    if order_fixed || hierarchy_mode {
        return;
    }

    let sl_ports = holder
        .lock()
        .ok()
        .map(|holder_guard| holder_guard.sl_port_values())
        .unwrap_or_default();

    for sl_port in sl_ports {
        let (had_only_self_loops, l_port) = sl_port
            .lock()
            .ok()
            .map(|port_guard| (port_guard.had_only_self_loops(), port_guard.l_port().clone()))
            .unwrap_or_else(|| panic!("self loop port lock poisoned"));

        if !had_only_self_loops {
            continue;
        }

        LPort::set_node(&l_port, None);

        if let Ok(mut sl_port_guard) = sl_port.lock() {
            sl_port_guard.set_hidden(true);
        }
        if let Ok(mut holder_guard) = holder.lock() {
            holder_guard.set_ports_hidden(true);
        }

        debug_assert!(
            l_port
                .lock()
                .ok()
                .and_then(|mut port_guard| port_guard.get_property(InternalProperties::PORT_DUMMY))
                .is_none()
        );
    }
}
