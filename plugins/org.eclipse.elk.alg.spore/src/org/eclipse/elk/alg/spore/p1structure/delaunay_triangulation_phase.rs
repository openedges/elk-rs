use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::bowyer_watson_triangulation::BowyerWatsonTriangulation;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::internal_properties::InternalProperties;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, IElkProgressMonitor};

use crate::org::eclipse::elk::alg::spore::graph::Graph;
use crate::org::eclipse::elk::alg::spore::spore_phases::SPOrEPhases;

pub struct DelaunayTriangulationPhase;

impl DelaunayTriangulationPhase {
    pub fn new() -> Self {
        DelaunayTriangulationPhase
    }
}

impl Default for DelaunayTriangulationPhase {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<SPOrEPhases, Graph> for DelaunayTriangulationPhase {
    fn process(&mut self, graph: &mut Graph, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Delaunay triangulation", 1.0);

        let vertices: Vec<_> = graph.vertices.iter().map(|v| v.original_vertex).collect();

        let debug_output = graph
            .get_property(InternalProperties::DEBUG_SVG)
            .unwrap_or(false)
            .then(|| ElkUtil::debug_folder_path(&["spore"]))
            .flatten()
            .map(|path| format!("{}10bw", path));

        let edges = BowyerWatsonTriangulation::triangulate(&vertices, debug_output.as_deref());
        match graph.t_edges.as_mut() {
            Some(existing) => {
                existing.extend(edges);
            }
            None => {
                graph.t_edges = Some(edges);
            }
        }

        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &Graph,
    ) -> Option<LayoutProcessorConfiguration<SPOrEPhases, Graph>> {
        Some(LayoutProcessorConfiguration::create())
    }
}
