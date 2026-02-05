use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::NodeMicroLayout;
use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::mrtree::components_processor::ComponentsProcessor;
use crate::org::eclipse::elk::alg::mrtree::elk_graph_importer::ElkGraphImporter;
use crate::org::eclipse::elk::alg::mrtree::i_graph_importer::IGraphImporter;
use crate::org::eclipse::elk::alg::mrtree::mrtree::MrTree;
use crate::org::eclipse::elk::alg::mrtree::options::MrTreeOptions;

pub struct TreeLayoutProvider {
    mr_tree: MrTree,
    components_processor: ComponentsProcessor,
    default_work: f32,
}

impl TreeLayoutProvider {
    pub fn new() -> Self {
        TreeLayoutProvider {
            mr_tree: MrTree::new(),
            components_processor: ComponentsProcessor::new(),
            default_work: 0.1,
        }
    }
}

impl Default for TreeLayoutProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphLayoutEngine for TreeLayoutProvider {
    fn layout(&mut self, layout_graph: &ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        let omit_micro = {
            let mut root = layout_graph.borrow_mut();
            let props = root
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            props
                .get_property(MrTreeOptions::OMIT_NODE_MICRO_LAYOUT)
                .unwrap_or(false)
        };

        if !omit_micro {
            NodeMicroLayout::for_graph(layout_graph.clone()).execute();
        }

        let mut importer = ElkGraphImporter::new();
        let mut pm = progress_monitor.sub_task(self.default_work);
        pm.begin("build tGraph", 1.0);
        let tgraph = importer.import_graph(layout_graph);
        pm.done();
        let Some(tgraph) = tgraph else { return };

        let mut pm = progress_monitor.sub_task(self.default_work);
        pm.begin("Split graph", 1.0);
        let components = self.components_processor.split(&tgraph);
        pm.done();

        let comp_work =
            (1.0 - self.default_work * 4.0) / (components.len().max(1) as f32);
        for comp in &components {
            self.mr_tree
                .do_layout(comp, Some(progress_monitor.sub_task(comp_work).as_mut()));
        }

        let mut pm = progress_monitor.sub_task(self.default_work);
        pm.begin("Pack components", 1.0);
        let packed = self.components_processor.pack(&components);
        pm.done();

        let mut pm = progress_monitor.sub_task(self.default_work);
        pm.begin("Apply layout results", 1.0);
        importer.apply_layout(&packed);
        pm.done();
    }
}

impl AbstractLayoutProvider for TreeLayoutProvider {}
