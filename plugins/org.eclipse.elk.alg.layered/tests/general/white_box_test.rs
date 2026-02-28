use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p2layers::network_simplex_layerer::NetworkSimplexLayerer;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_importer::ElkGraphImporter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredOptions, LayeringStrategy,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

#[test]
fn white_box_test_no_layerless_nodes() {
    let lgraph = run_network_simplex_layerer();
    let graph_guard = lgraph.lock().expect("lgraph lock");
    assert!(
        graph_guard.layerless_nodes().is_empty(),
        "There are layerless nodes left!"
    );
}

#[test]
fn white_box_test_proper_layering() {
    let lgraph = run_network_simplex_layerer();
    let graph_guard = lgraph.lock().expect("lgraph lock");
    let layers = graph_guard.layers().clone();
    drop(graph_guard);

    for layer in &layers {
        let source_layer_index = layer_index(layer);
        let nodes = layer.lock().expect("layer lock").nodes().clone();
        for node in nodes {
            let outgoing = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();
            for edge in outgoing {
                let target_layer_index = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| edge_guard.target())
                    .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
                    .and_then(|target_node| target_node.lock().ok().and_then(|n| n.layer()))
                    .map(|layer_ref| layer_index(&layer_ref))
                    .unwrap_or(source_layer_index);
                assert!(
                    source_layer_index <= target_layer_index,
                    "Edge points leftwards!"
                );
            }
        }
    }
}

fn run_network_simplex_layerer(
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef {
    initialize_plain_java_layout();
    let (root, _nodes) = build_test_graph();
    set_node_property(
        &root,
        CoreOptions::ALGORITHM,
        "org.eclipse.elk.layered".to_string(),
    );
    set_node_property(
        &root,
        CoreOptions::HIERARCHY_HANDLING,
        HierarchyHandling::IncludeChildren,
    );
    set_node_property(
        &root,
        LayeredOptions::LAYERING_STRATEGY,
        LayeringStrategy::NetworkSimplex,
    );

    let mut origin_store = OriginStore::new();
    let mut importer = ElkGraphImporter::new(&mut origin_store);
    let lgraph = importer.import_graph(&root);

    let mut layerer = NetworkSimplexLayerer::new();
    let mut monitor = BasicProgressMonitor::new();
    if let Ok(mut graph_guard) = lgraph.lock() {
        layerer.process(&mut *graph_guard, &mut monitor);
    }

    lgraph
}

fn build_test_graph() -> (ElkNodeRef, Vec<ElkNodeRef>) {
    let root = ElkGraphUtil::create_graph();
    let compound_child = ElkGraphUtil::create_node(Some(root.clone()));
    let nested = ElkGraphUtil::create_node(Some(compound_child.clone()));

    set_dimensions(&compound_child, 30.0, 30.0);
    set_dimensions(&nested, 30.0, 30.0);

    (root, vec![compound_child, nested])
}

fn layer_index(
    layer: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LayerRef,
) -> usize {
    layer
        .lock()
        .ok()
        .and_then(|layer_guard| layer_guard.index())
        .unwrap_or(0)
}

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

fn set_node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
    value: T,
) {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}
