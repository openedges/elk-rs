use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph, LPort};
use crate::org::eclipse::elk::alg::layered::intermediate::loops::{
    SelfLoopHolder, SelfLoopHolderRef,
};
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions,
};

pub struct SelfLoopPreProcessor;

impl ILayoutProcessor<LGraph> for SelfLoopPreProcessor {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Self-Loop pre-processing", 1.0);

        // Read graph-level properties BEFORE iterating nodes (graph is &mut, no Mutex).
        let graph_spacing = graph.get_property(LayeredOptions::SPACING_LABEL_LABEL);
        let graph_direction = graph.get_property(LayeredOptions::DIRECTION);

        // Compute graph-level layout properties (Java: getIndividualOrInherited).
        // LNode.get_property returns Some(default) even when not explicitly set, so we
        // only use graph-level properties here to match Java's hasProperty() check.
        let label_label_spacing = graph_spacing
            .or_else(|| LayeredOptions::SPACING_LABEL_LABEL.get_default())
            .unwrap_or(0.0);
        let direction = graph_direction.unwrap_or(
            org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction::Right,
        );
        let layout_dir_horizontal = direction.is_horizontal();

        for lnode in graph.layerless_nodes().clone() {
            if SelfLoopHolder::needs_self_loop_processing(&lnode) {
                let sl_holder = SelfLoopHolder::install(&lnode);

                set_hyper_loop_label_properties(
                    &sl_holder,
                    label_label_spacing,
                    layout_dir_horizontal,
                );

                hide_self_loops(&sl_holder);
                hide_ports(&sl_holder);
            }
        }

        monitor.done();
    }
}

fn set_hyper_loop_label_properties(holder: &SelfLoopHolderRef, spacing: f64, horizontal: bool) {
    if let Ok(holder_guard) = holder.lock() {
        for hyper_loop in holder_guard.sl_hyper_loops() {
            if let Ok(mut loop_guard) = hyper_loop.lock() {
                if let Some(sl_labels) = loop_guard.sl_labels_mut() {
                    sl_labels.set_layout_direction_horizontal(horizontal);
                    sl_labels.set_label_label_spacing(spacing);
                }
            }
        }
    }
}

fn hide_self_loops(
    holder: &crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfLoopHolderRef,
) {
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

fn hide_ports(
    holder: &crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfLoopHolderRef,
) {
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
            .map(|port_guard| {
                (
                    port_guard.had_only_self_loops(),
                    port_guard.l_port().clone(),
                )
            })
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

        debug_assert!(l_port
            .lock()
            .ok()
            .and_then(|mut port_guard| port_guard.get_property(InternalProperties::PORT_DUMMY))
            .is_none());
    }
}
