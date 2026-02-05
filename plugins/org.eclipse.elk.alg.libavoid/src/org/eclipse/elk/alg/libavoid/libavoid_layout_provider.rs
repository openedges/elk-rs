use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::NodeMicroLayout;
use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{Direction, PortSide};
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkNodeRef, ElkPortRef};

use crate::org::eclipse::elk::alg::libavoid::options::LibavoidOptions;

pub struct LibavoidLayoutProvider;

impl LibavoidLayoutProvider {
    pub fn new() -> Self {
        LibavoidLayoutProvider
    }

    pub fn cancel_layouting(&self, _parent_node: &ElkNodeRef) -> bool {
        false
    }

    fn prepare_graph(parent_node: &ElkNodeRef) {
        let direction = node_get_property(parent_node, LibavoidOptions::DIRECTION)
            .unwrap_or(Direction::Undefined);

        let children: Vec<ElkNodeRef> = {
            let mut parent_mut = parent_node.borrow_mut();
            parent_mut.children().iter().cloned().collect()
        };

        for node in children {
            let ports: Vec<ElkPortRef> = {
                let mut node_mut = node.borrow_mut();
                node_mut.ports().iter().cloned().collect()
            };

            for port in ports {
                let side = port_get_property(&port, CoreOptions::PORT_SIDE)
                    .unwrap_or(PortSide::Undefined);
                if side == PortSide::Undefined {
                    let calculated = ElkUtil::calc_port_side(&port, direction);
                    port_set_property(&port, CoreOptions::PORT_SIDE, Some(calculated));
                }
            }
        }
    }
}

impl Default for LibavoidLayoutProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphLayoutEngine for LibavoidLayoutProvider {
    fn layout(&mut self, parent_node: &ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Libavoid layout", 1.0);
        let children_empty = {
            let mut parent_mut = parent_node.borrow_mut();
            parent_mut.children().is_empty()
        };
        if children_empty {
            progress_monitor.done();
            return;
        }

        if !node_get_property(parent_node, LibavoidOptions::OMIT_NODE_MICRO_LAYOUT).unwrap_or(false)
        {
            NodeMicroLayout::for_graph(parent_node.clone()).execute();
        }

        Self::prepare_graph(parent_node);
        // TODO: port libavoid server routing; keep no-op to avoid external dependency.

        progress_monitor.done();
    }
}

impl AbstractLayoutProvider for LibavoidLayoutProvider {}

fn node_get_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<T>,
) -> Option<T> {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(property)
}

fn port_get_property<T: Clone + Send + Sync + 'static>(
    port: &ElkPortRef,
    property: &org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<T>,
) -> Option<T> {
    let mut port_mut = port.borrow_mut();
    port_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(property)
}

fn port_set_property<T: Clone + Send + Sync + 'static>(
    port: &ElkPortRef,
    property: &org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<T>,
    value: Option<T>,
) {
    let mut port_mut = port.borrow_mut();
    port_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, value);
}
