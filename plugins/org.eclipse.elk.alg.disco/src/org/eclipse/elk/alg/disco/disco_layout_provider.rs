use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::disco::disco_polyomino_compactor::DisCoPolyominoCompactor;
use crate::org::eclipse::elk::alg::disco::i_compactor::ICompactor;
use crate::org::eclipse::elk::alg::disco::options::{CompactionStrategy, DisCoOptions};
use crate::org::eclipse::elk::alg::disco::transform::{ElkGraphTransformer, IGraphTransformer};

pub struct DisCoLayoutProvider;

impl DisCoLayoutProvider {
    pub fn new() -> Self {
        DisCoLayoutProvider
    }
}

impl Default for DisCoLayoutProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphLayoutEngine for DisCoLayoutProvider {
    fn layout(
        &mut self,
        layout_graph: &ElkNodeRef,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        progress_monitor.begin("Connected Components Compaction", 1.0);

        let component_spacing =
            property(layout_graph, DisCoOptions::SPACING_COMPONENT_COMPONENT).unwrap_or(0.0);

        if has_property(
            layout_graph,
            DisCoOptions::COMPONENT_COMPACTION_COMPONENT_LAYOUT_ALGORITHM,
        ) {
            if let Some(requested) = property(
                layout_graph,
                DisCoOptions::COMPONENT_COMPACTION_COMPONENT_LAYOUT_ALGORITHM,
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

        let mut transformer = ElkGraphTransformer::new(component_spacing);
        {
            let graph = transformer.import_graph(layout_graph);
            let strategy = graph
                .properties_mut()
                .get_property(DisCoOptions::COMPONENT_COMPACTION_STRATEGY)
                .unwrap_or_default();

            match strategy {
                CompactionStrategy::Polyomino => {
                    let mut compactor = DisCoPolyominoCompactor::new();
                    compactor.compact(graph);
                    if let Some(polys) = graph
                        .properties_mut()
                        .get_property(DisCoOptions::DEBUG_DISCO_POLYS)
                    {
                        set_property(layout_graph, DisCoOptions::DEBUG_DISCO_POLYS, polys);
                    }
                }
            }

            set_property(layout_graph, DisCoOptions::DEBUG_DISCO_GRAPH, graph.clone());
        }

        transformer.apply_layout();
        progress_monitor.done();
    }
}

impl AbstractLayoutProvider for DisCoLayoutProvider {}

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

fn set_property<T: Clone + Send + Sync + 'static>(
    graph: &ElkNodeRef,
    property: &'static Property<T>,
    value: T,
) {
    let mut graph_mut = graph.borrow_mut();
    graph_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}
