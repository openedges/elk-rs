use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::spore::elk_graph_importer::ElkGraphImporter;
use crate::org::eclipse::elk::alg::spore::i_graph_importer::IGraphImporter;
use crate::org::eclipse::elk::alg::spore::options::SporeCompactionOptions;
use crate::org::eclipse::elk::alg::spore::shrink_tree::ShrinkTree;

pub struct ShrinkTreeLayoutProvider {
    shrinktree: ShrinkTree,
}

impl ShrinkTreeLayoutProvider {
    pub fn new() -> Self {
        ShrinkTreeLayoutProvider {
            shrinktree: ShrinkTree::new(),
        }
    }
}

impl Default for ShrinkTreeLayoutProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphLayoutEngine for ShrinkTreeLayoutProvider {
    fn layout(
        &mut self,
        layout_graph: &ElkNodeRef,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        if has_property(
            layout_graph,
            SporeCompactionOptions::UNDERLYING_LAYOUT_ALGORITHM,
        ) {
            if let Some(requested) = property(
                layout_graph,
                SporeCompactionOptions::UNDERLYING_LAYOUT_ALGORITHM,
            ) {
                if !requested.trim().is_empty() {
                    let service = LayoutMetaDataService::get_instance();
                    if let Some(algorithm_data) = service.get_algorithm_data_by_suffix(&requested) {
                        if let Some(pool) = algorithm_data.provider_pool() {
                            let mut provider = pool.fetch();
                            let mut sub = progress_monitor.sub_task(1.0);
                            provider.layout(layout_graph, sub.as_mut());
                            pool.release(provider);
                        }
                    }
                }
            }
        }

        let mut importer = ElkGraphImporter::new();
        let mut graph = importer.import_graph(layout_graph);
        self.shrinktree
            .shrink(&mut graph, progress_monitor.sub_task(1.0).as_mut());
        importer.apply_positions(&graph);
    }
}

impl AbstractLayoutProvider for ShrinkTreeLayoutProvider {}

fn property<T: Clone + Send + Sync + 'static>(
    graph: &ElkNodeRef,
    property: &'static Property<T>,
) -> Option<T> {
    let mut graph_mut = graph.borrow_mut();
    graph_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(property)
}

fn has_property<T: Clone + Send + Sync + 'static>(
    graph: &ElkNodeRef,
    property: &'static Property<T>,
) -> bool {
    let mut graph_mut = graph.borrow_mut();
    graph_mut
        .connectable()
        .shape()
        .graph_element()
        .properties()
        .has_property(property)
}
