use std::time::{SystemTime, UNIX_EPOCH};

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::NodeMicroLayout;
use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{IElkProgressMonitor, Random};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::force::components_processor::ComponentsProcessor;
use crate::org::eclipse::elk::alg::force::elk_graph_importer::ElkGraphImporter;
use crate::org::eclipse::elk::alg::force::graph::FGraph;
use crate::org::eclipse::elk::alg::force::i_graph_importer::IGraphImporter;
use crate::org::eclipse::elk::alg::force::model::abstract_force_model::ForceModel;
use crate::org::eclipse::elk::alg::force::model::{EadesModel, FruchtermanReingoldModel};
use crate::org::eclipse::elk::alg::force::options::{ForceModelStrategy, ForceOptions, InternalProperties};

enum ForceModelKind {
    Eades(EadesModel),
    FruchtermanReingold(FruchtermanReingoldModel),
}

impl ForceModelKind {
    fn layout(&mut self, graph: &mut FGraph, monitor: &mut dyn IElkProgressMonitor) {
        match self {
            ForceModelKind::Eades(model) => model.layout(graph, monitor),
            ForceModelKind::FruchtermanReingold(model) => model.layout(graph, monitor),
        }
    }
}

pub struct ForceLayoutProvider {
    force_model: ForceModelKind,
    components_processor: ComponentsProcessor,
}

impl ForceLayoutProvider {
    pub fn new() -> Self {
        ForceLayoutProvider {
            force_model: ForceModelKind::FruchtermanReingold(FruchtermanReingoldModel::new()),
            components_processor: ComponentsProcessor::new(),
        }
    }

    fn set_options(&mut self, graph: &mut FGraph) {
        let random_seed = graph.get_property(ForceOptions::RANDOM_SEED);
        let random = match random_seed {
            Some(seed) if seed == 0 => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64;
                Random::new(now)
            }
            Some(seed) => Random::new(seed as u64),
            None => Random::new(1),
        };
        graph.set_property(InternalProperties::RANDOM, Some(random));
    }

    fn update_model(&mut self, strategy: ForceModelStrategy) {
        match strategy {
            ForceModelStrategy::Eades => {
                self.force_model = ForceModelKind::Eades(EadesModel::new());
            }
            ForceModelStrategy::FruchtermanReingold => {
                self.force_model =
                    ForceModelKind::FruchtermanReingold(FruchtermanReingoldModel::new());
            }
        }
    }
}

impl Default for ForceLayoutProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphLayoutEngine for ForceLayoutProvider {
    fn layout(&mut self, layout_graph: &ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("ELK Force", 1.0);

        let omit_micro = {
            let mut root = layout_graph.borrow_mut();
            let mut props = root.connectable().shape().graph_element().properties().clone();
            props
                .get_property(ForceOptions::OMIT_NODE_MICRO_LAYOUT)
                .unwrap_or(false)
        };
        if !omit_micro {
            NodeMicroLayout::for_graph(layout_graph.clone()).execute();
        }

        let mut graph_importer = ElkGraphImporter::new();
        let mut fgraph = match graph_importer.import_graph(layout_graph) {
            Some(graph) => graph,
            None => return,
        };

        self.set_options(&mut fgraph);
        let model = fgraph
            .get_property(ForceOptions::MODEL)
            .unwrap_or(ForceModelStrategy::FruchtermanReingold);
        self.update_model(model);

        let mut components = self.components_processor.split(fgraph);
        let comp_work = 1.0 / (components.len().max(1) as f32);
        for comp in components.iter_mut() {
            self.force_model
                .layout(comp, progress_monitor.sub_task(comp_work).as_mut());
        }

        let fgraph = self.components_processor.recombine(components);
        graph_importer.apply_layout(&fgraph);

        progress_monitor.done();
    }
}

impl AbstractLayoutProvider for ForceLayoutProvider {}
