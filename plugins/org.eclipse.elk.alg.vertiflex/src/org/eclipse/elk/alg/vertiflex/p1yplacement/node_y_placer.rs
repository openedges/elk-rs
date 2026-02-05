use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::vertiflex::vertiflex_layout_phases::VertiFlexLayoutPhases;
use crate::org::eclipse::elk::alg::vertiflex::vertiflex_util::VertiFlexUtil;
use crate::org::eclipse::elk::alg::vertiflex::options::VertiFlexOptions;

pub struct NodeYPlacer {
    layer_distance: f64,
    node_node_spacing: f64,
}

impl NodeYPlacer {
    pub fn new() -> Self {
        NodeYPlacer {
            layer_distance: 0.0,
            node_node_spacing: 0.0,
        }
    }

    fn set_y_levels(&self, node: &ElkNodeRef, mut min_height: f64) {
        if node_has_property(node, VertiFlexOptions::VERTICAL_CONSTRAINT) {
            if let Some(value) = node_get_property(node, VertiFlexOptions::VERTICAL_CONSTRAINT) {
                min_height = value;
            }
        }

        {
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            shape.set_y(min_height);
        }

        let height = {
            let mut node_mut = node.borrow_mut();
            node_mut.connectable().shape().height()
        };
        let margins_bottom = node_get_property(node, CoreOptions::MARGINS)
            .unwrap_or_else(org_eclipse_elk_core::org::eclipse::elk::core::math::ElkMargin::new)
            .bottom;

        let new_min_height = min_height + self.layer_distance + height + margins_bottom.max(self.node_node_spacing);
        for edge in ElkGraphUtil::all_outgoing_edges(node) {
            if let Some(child) = edge_target_node(&edge) {
                self.set_y_levels(&child, new_min_height);
            }
        }
    }
}

impl Default for NodeYPlacer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<VertiFlexLayoutPhases, ElkNodeRef> for NodeYPlacer {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("YPlacer", 1.0);

        self.layer_distance = node_get_property(graph, VertiFlexOptions::LAYER_DISTANCE).unwrap_or(0.0);
        self.node_node_spacing = node_get_property(graph, CoreOptions::SPACING_NODE_NODE).unwrap_or(0.0);

        let has_children = {
            let mut graph_mut = graph.borrow_mut();
            !graph_mut.children().is_empty()
        };
        if has_children {
            if let Some(parent) = VertiFlexUtil::find_root(graph) {
                self.set_y_levels(&parent, 0.0);
            }
        }

        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &ElkNodeRef,
    ) -> Option<LayoutProcessorConfiguration<VertiFlexLayoutPhases, ElkNodeRef>> {
        None
    }
}

fn node_get_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
) -> Option<T> {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    props.get_property(property)
}

fn node_has_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
) -> bool {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    props.has_property(property)
}

fn edge_target_node(edge: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeRef) -> Option<ElkNodeRef> {
    let edge_borrow = edge.borrow();
    let target = edge_borrow.targets_ro().get(0)?;
    drop(edge_borrow);
    org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil::connectable_shape_to_node(&target)
}
