use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::NodeMicroLayout;
use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::force::components_processor::ComponentsProcessor;
use crate::org::eclipse::elk::alg::force::elk_graph_importer::ElkGraphImporter;
use crate::org::eclipse::elk::alg::force::force_layout_provider::ForceLayoutProvider;
use crate::org::eclipse::elk::alg::force::i_graph_importer::IGraphImporter;
use crate::org::eclipse::elk::alg::force::options::StressOptions;
use crate::org::eclipse::elk::alg::force::stress::StressMajorization;

pub struct StressLayoutProvider {
    components_processor: ComponentsProcessor,
    stress_majorization: StressMajorization,
}

impl StressLayoutProvider {
    pub fn new() -> Self {
        StressLayoutProvider {
            components_processor: ComponentsProcessor::new(),
            stress_majorization: StressMajorization::new(),
        }
    }
}

impl Default for StressLayoutProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphLayoutEngine for StressLayoutProvider {
    fn layout(
        &mut self,
        layout_graph: &ElkNodeRef,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        progress_monitor.begin("ELK Stress", 1.0);

        let interactive = {
            let mut root = layout_graph.borrow_mut();
            let mut props = root
                .connectable()
                .shape()
                .graph_element()
                .properties()
                .clone();
            props
                .get_property(StressOptions::INTERACTIVE)
                .unwrap_or(false)
        };

        if !interactive {
            let mut force = ForceLayoutProvider::new();
            force.layout(layout_graph, progress_monitor.sub_task(1.0).as_mut());
        } else {
            let omit_micro = {
                let mut root = layout_graph.borrow_mut();
                let mut props = root
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties()
                    .clone();
                props
                    .get_property(StressOptions::OMIT_NODE_MICRO_LAYOUT)
                    .unwrap_or(false)
            };
            if !omit_micro {
                NodeMicroLayout::for_graph(layout_graph.clone()).execute();
            }
        }

        let mut importer = ElkGraphImporter::new();
        let fgraph = match importer.import_graph(layout_graph) {
            Some(graph) => graph,
            None => return,
        };

        let mut components = self.components_processor.split(fgraph);
        for subgraph in components.iter_mut() {
            if subgraph.nodes().len() <= 1 {
                continue;
            }
            self.stress_majorization.initialize(subgraph);
            self.stress_majorization.execute(subgraph);

            for label in subgraph.labels() {
                if let Ok(mut label_guard) = label.lock() {
                    label_guard.refresh_position();
                }
            }
        }

        let fgraph = self.components_processor.recombine(components);
        importer.apply_layout(&fgraph);

        progress_monitor.done();
    }
}

impl AbstractLayoutProvider for StressLayoutProvider {}
