use std::collections::HashSet;
use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::scanline_overlap_check::ScanlineOverlapCheck;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::t_edge::TEdge;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::utils::SVGImage;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkRectangle;
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use org_eclipse_elk_alg_spore::org::eclipse::elk::alg::spore::elk_graph_importer::ElkGraphImporter;
use org_eclipse_elk_alg_spore::org::eclipse::elk::alg::spore::i_graph_importer::IGraphImporter;
use org_eclipse_elk_alg_spore::org::eclipse::elk::alg::spore::options::{
    SporeMetaDataProvider, SporeOverlapRemovalOptions,
};
use org_eclipse_elk_alg_spore::org::eclipse::elk::alg::spore::overlap_removal_layout_provider::OverlapRemovalLayoutProvider;

#[test]
fn scanline_overlap_removal_test() {
    LayoutMetaDataService::get_instance();
    LayoutMetaDataService::get_instance()
        .register_layout_meta_data_provider(&SporeMetaDataProvider);
    let graph_with_scanline = build_graph(true);

    let mut importer = ElkGraphImporter::new();
    let spore_graph = importer.import_graph(&graph_with_scanline);
    let overlap_edges: Arc<Mutex<HashSet<TEdge>>> = Arc::new(Mutex::new(HashSet::new()));
    let handler_edges = overlap_edges.clone();
    let handler = move |n1: &org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::node::Node,
                        n2: &org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::node::Node| {
        if let Some(mut guard) = handler_edges.lock_ok() {
            guard.insert(TEdge::new(n1.original_vertex, n2.original_vertex));
        }
    };
    let mut scanline = ScanlineOverlapCheck::new(handler, SVGImage::new(None));
    scanline.sweep(&spore_graph.vertices);
    assert!(!overlap_edges.lock().is_empty());

    let mut provider = OverlapRemovalLayoutProvider::new();
    let mut monitor = BasicProgressMonitor::new();
    provider.layout(&graph_with_scanline, &mut monitor);

    assert!(!has_overlaps(&graph_with_scanline));
}

fn build_graph(run_scanline: bool) -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    let n0 = ElkGraphUtil::create_node(Some(graph.clone()));
    let n1 = ElkGraphUtil::create_node(Some(graph.clone()));
    let n2 = ElkGraphUtil::create_node(Some(graph.clone()));
    let n3 = ElkGraphUtil::create_node(Some(graph.clone()));

    set_node_dimensions(&n0, 160.0, 20.0);
    set_node_dimensions(&n1, 160.0, 20.0);
    set_node_dimensions(&n2, 20.0, 20.0);
    set_node_dimensions(&n3, 20.0, 20.0);

    set_node_location(&n0, 0.0, 30.0);
    set_node_location(&n1, 150.0, 40.0);
    set_node_location(&n2, 150.0, 0.0);
    set_node_location(&n3, 150.0, 70.0);

    graph
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(
            SporeOverlapRemovalOptions::OVERLAP_REMOVAL_RUN_SCANLINE,
            Some(run_scanline),
        );

    graph
}

fn set_node_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

fn set_node_location(node: &ElkNodeRef, x: f64, y: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_location(x, y);
}

fn has_overlaps(graph: &ElkNodeRef) -> bool {
    let nodes = {
        let mut graph_mut = graph.borrow_mut();
        graph_mut.children().iter().cloned().collect::<Vec<_>>()
    };

    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            let r1 = node_rect(&nodes[i]);
            let r2 = node_rect(&nodes[j]);
            if ElkRectangle::from_other(&r1).intersects(&r2) {
                let dist =
                    org_eclipse_elk_core::org::eclipse::elk::core::math::ElkMath::shortest_distance(
                        &r1, &r2,
                    );
                if dist < 0.0 {
                    return true;
                }
            }
        }
    }

    false
}

fn node_rect(node: &ElkNodeRef) -> ElkRectangle {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    ElkRectangle::with_values(shape.x(), shape.y(), shape.width(), shape.height())
}
