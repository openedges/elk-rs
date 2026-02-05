use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::testing::IWhiteBoxTestable;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::layered::elk_layered::{ElkLayered, TestExecutionState};
use crate::org::eclipse::elk::alg::layered::graph::transform::ElkGraphTransformer;
use crate::org::eclipse::elk::alg::layered::graph::transform::IGraphTransformer;
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;

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

impl IGraphLayoutEngine for LayeredLayoutProvider {
    fn layout(&mut self, layout_graph: &ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
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
