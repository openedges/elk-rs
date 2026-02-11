use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::testing::IWhiteBoxTestable;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::layered::elk_layered::{ElkLayered, TestExecutionState};
use crate::org::eclipse::elk::alg::layered::graph::transform::ElkGraphTransformer;
use crate::org::eclipse::elk::alg::layered::graph::transform::IGraphTransformer;
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::NodeMicroLayout;

pub struct LayeredLayoutProvider {
    elk_layered: ElkLayered,
}

impl LayeredLayoutProvider {
    pub fn new() -> Self {
        LayeredLayoutProvider {
            elk_layered: ElkLayered::new(),
        }
    }

    pub fn start_layout_test(&mut self, elkgraph: &ElkNodeRef) -> TestExecutionState {
        let mut transformer = ElkGraphTransformer::new();
        let layered_graph = transformer.import_graph(elkgraph);
        self.elk_layered.prepare_layout_test(&layered_graph)
    }

    pub fn layout_algorithm(&mut self) -> &mut ElkLayered {
        &mut self.elk_layered
    }
}

impl Default for LayeredLayoutProvider {
    fn default() -> Self {
        Self::new()
    }
}

fn apply_layered_algorithm_defaults(graph: &ElkNodeRef) {
    let mut graph_mut = graph.borrow_mut();
    let props = graph_mut.connectable().shape().graph_element().properties_mut();
    if !props.has_property_id(LayeredOptions::EDGE_ROUTING.id()) {
        props.set_property(&*LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
    }
    // NOTE: PORT_ALIGNMENT_DEFAULT is NOT set here. Java's Layered.melk declares
    // "supports portAlignment.default = JUSTIFIED" as metadata-only, not applied at runtime.
    // Java uses the Property global default (Distributed) via getProperty(CoreOptions.PORT_ALIGNMENT_DEFAULT).
    // Setting Justified here caused regressions in port positioning (182_minNodeSizeForHierarchicalNodes.elkt).
    if !props.has_property_id(LayeredOptions::SEPARATE_CONNECTED_COMPONENTS.id()) {
        props.set_property(&*LayeredOptions::SEPARATE_CONNECTED_COMPONENTS, Some(true));
    }
}

impl IGraphLayoutEngine for LayeredLayoutProvider {
    fn layout(&mut self, layout_graph: &ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        apply_layered_algorithm_defaults(layout_graph);

        let omit_micro_layout = {
            let mut graph_mut = layout_graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::OMIT_NODE_MICRO_LAYOUT)
                .unwrap_or(false)
        };
        if !omit_micro_layout {
            NodeMicroLayout::for_graph(layout_graph.clone()).execute();
        }

        let mut transformer = ElkGraphTransformer::new();
        let layered_graph = transformer.import_graph(layout_graph);

        let hierarchy_handling = {
            let mut graph_mut = layout_graph.borrow_mut();
            let mut props = graph_mut.connectable().shape().graph_element().properties().clone();
            props
                .get_property(LayeredOptions::HIERARCHY_HANDLING)
                .unwrap_or(HierarchyHandling::Inherit)
        };

        if hierarchy_handling == HierarchyHandling::IncludeChildren {
            self.elk_layered
                .do_compound_layout(&layered_graph, Some(progress_monitor));
        } else {
            self.elk_layered
                .do_layout(&layered_graph, Some(progress_monitor));
        }

        if !progress_monitor.is_canceled() {
            transformer.apply_layout(&layered_graph);
        }
    }
}

impl AbstractLayoutProvider for LayeredLayoutProvider {
    fn as_white_box_testable(&mut self) -> Option<&mut dyn IWhiteBoxTestable> {
        Some(self)
    }
}

impl IWhiteBoxTestable for LayeredLayoutProvider {
    fn set_test_controller(&mut self, controller: Option<*mut org_eclipse_elk_core::org::eclipse::elk::core::testing::TestController>) {
        self.elk_layered.set_test_controller(controller);
    }
}
