use std::collections::HashSet;
use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::internal_properties::InternalProperties;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::scanline_overlap_check::ScanlineOverlapCheck;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::t_edge::TEdge;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::utils::SVGImage;
use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::algorithm_assembler::{
    AlgorithmAssembler, SharedProcessor,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, IElkProgressMonitor};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::spore::elk_graph_importer::ElkGraphImporter;
use crate::org::eclipse::elk::alg::spore::graph::Graph;
use crate::org::eclipse::elk::alg::spore::i_graph_importer::IGraphImporter;
use crate::org::eclipse::elk::alg::spore::options::{
    OverlapRemovalStrategy, RootSelection, SpanningTreeCostFunction, SporeCompactionOptions,
    SporeOverlapRemovalOptions, StructureExtractionStrategy, TreeConstructionStrategy,
};
use crate::org::eclipse::elk::alg::spore::spore_phases::SPOrEPhases;

pub struct OverlapRemovalLayoutProvider {
    algorithm_assembler: AlgorithmAssembler<SPOrEPhases, Graph>,
    algorithm: Vec<SharedProcessor<Graph>>,
}

impl OverlapRemovalLayoutProvider {
    pub fn new() -> Self {
        OverlapRemovalLayoutProvider {
            algorithm_assembler: AlgorithmAssembler::create(),
            algorithm: Vec::new(),
        }
    }
}

impl Default for OverlapRemovalLayoutProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphLayoutEngine for OverlapRemovalLayoutProvider {
    fn layout(
        &mut self,
        layout_graph: &ElkNodeRef,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        if has_property(
            layout_graph,
            SporeOverlapRemovalOptions::UNDERLYING_LAYOUT_ALGORITHM,
        ) {
            if let Some(requested) = property(
                layout_graph,
                SporeOverlapRemovalOptions::UNDERLYING_LAYOUT_ALGORITHM,
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

        set_property(
            layout_graph,
            SporeCompactionOptions::PROCESSING_ORDER_ROOT_SELECTION,
            RootSelection::CenterNode,
        );
        set_property(
            layout_graph,
            SporeCompactionOptions::PROCESSING_ORDER_SPANNING_TREE_COST_FUNCTION,
            SpanningTreeCostFunction::InvertedOverlap,
        );
        set_property(
            layout_graph,
            SporeCompactionOptions::PROCESSING_ORDER_TREE_CONSTRUCTION,
            TreeConstructionStrategy::MinimumSpanningTree,
        );

        let max_iterations = property(
            layout_graph,
            SporeOverlapRemovalOptions::OVERLAP_REMOVAL_MAX_ITERATIONS,
        )
        .unwrap_or(0)
        .max(0) as usize;

        progress_monitor.begin("Overlap removal", 1.0);

        let debug_output = if property(layout_graph, SporeOverlapRemovalOptions::DEBUG_MODE)
            .unwrap_or(false)
        {
            ElkUtil::debug_folder_path(&["spore"]).map(|path| format!("{}45scanlineOverlaps", path))
        } else {
            None
        };

        let overlap_edges: Arc<Mutex<HashSet<TEdge>>> = Arc::new(Mutex::new(HashSet::new()));
        let mut graph_importer = ElkGraphImporter::new();
        let mut graph = graph_importer.import_graph(layout_graph);

        let mut overlaps_existed = true;
        let mut iteration = 0usize;

        while iteration < max_iterations && overlaps_existed {
            if property(
                layout_graph,
                SporeOverlapRemovalOptions::OVERLAP_REMOVAL_RUN_SCANLINE,
            )
            .unwrap_or(true)
            {
                if let Some(mut guard) = overlap_edges.lock_ok() {
                    guard.clear();
                }
                let handler_edges = overlap_edges.clone();
                let handler = move |n1: &org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::node::Node,
                                    n2: &org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::node::Node| {
                    if let Some(mut guard) = handler_edges.lock_ok() {
                        guard.insert(TEdge::new(n1.original_vertex, n2.original_vertex));
                    }
                };
                let svg = SVGImage::new(debug_output.as_deref());
                let mut scanline = ScanlineOverlapCheck::new(handler, svg);
                scanline.sweep(&graph.vertices);

                let edges = overlap_edges
                    .lock_ok()
                    .map(|guard| guard.clone())
                    .unwrap_or_default();
                if edges.is_empty() {
                    break;
                }
                graph.t_edges = Some(edges);
            }

            self.algorithm_assembler.reset();
            self.algorithm_assembler
                .set_phase(
                    SPOrEPhases::P1Structure,
                    Arc::new(StructureExtractionStrategy::DelaunayTriangulation),
                )
                .set_phase(
                    SPOrEPhases::P2ProcessingOrder,
                    Arc::new(graph.tree_construction_strategy),
                )
                .set_phase(
                    SPOrEPhases::P3Execution,
                    Arc::new(OverlapRemovalStrategy::GrowTree),
                );

            self.algorithm = self.algorithm_assembler.build(&graph);
            let step = if self.algorithm.is_empty() {
                1.0
            } else {
                1.0 / self.algorithm.len() as f32
            };

            for processor in &self.algorithm {
                if progress_monitor.is_canceled() {
                    return;
                }
                if let Some(mut processor_guard) = processor.lock_ok() {
                    let mut sub = progress_monitor.sub_task(step);
                    processor_guard.process(&mut graph, sub.as_mut());
                }
            }

            graph.sync_vertices_from_tree();
            graph_importer.update_graph(&mut graph);
            overlaps_existed = graph
                .get_property(InternalProperties::OVERLAPS_EXISTED)
                .unwrap_or(false);
            iteration += 1;
        }

        graph_importer.apply_positions(&graph);
        progress_monitor.done();
    }
}

impl AbstractLayoutProvider for OverlapRemovalLayoutProvider {}

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
