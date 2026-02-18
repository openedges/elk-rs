use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::naive_min_st::NaiveMinST;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::internal_properties::InternalProperties;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, IElkProgressMonitor};

use crate::org::eclipse::elk::alg::spore::graph::Graph;
use crate::org::eclipse::elk::alg::spore::p2processingorder::cost_function::{
    GraphCostFunction, InvertedCostFunction,
};
use crate::org::eclipse::elk::alg::spore::p2processingorder::min_st_phase::MinSTPhase;
use crate::org::eclipse::elk::alg::spore::spore_phases::SPOrEPhases;

pub struct MaxSTPhase {
    converter: MinSTPhase,
}

impl MaxSTPhase {
    pub fn new() -> Self {
        MaxSTPhase {
            converter: MinSTPhase::new(),
        }
    }
}

impl Default for MaxSTPhase {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<SPOrEPhases, Graph> for MaxSTPhase {
    fn process(&mut self, graph: &mut Graph, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Maximum spanning tree construction", 1.0);

        if graph.vertices.is_empty() {
            progress_monitor.done();
            return;
        }

        let root = graph
            .preferred_root()
            .map(|node| node.vertex)
            .unwrap_or(graph.vertices[0].vertex);

        let debug_output = graph
            .get_property(InternalProperties::DEBUG_SVG)
            .unwrap_or(false)
            .then(|| ElkUtil::debug_folder_path(&["spore"]))
            .flatten()
            .map(|path| format!("{}20minst", path));

        let Some(edges) = graph.t_edges.as_ref() else {
            progress_monitor.done();
            return;
        };

        let cost_function = GraphCostFunction::new(graph);
        let inverted = InvertedCostFunction::new(cost_function);

        let tree =
            NaiveMinST::create_spanning_tree(edges, &root, &inverted, debug_output.as_deref());
        self.converter.convert_tree(&tree, graph);

        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &Graph,
    ) -> Option<LayoutProcessorConfiguration<SPOrEPhases, Graph>> {
        Some(LayoutProcessorConfiguration::create())
    }
}
