use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::elk_layered::ElkLayered;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_importer::ElkGraphImporter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredMetaDataProvider, LayeredOptions, LayeringStrategy,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};

#[test]
fn coffman_graham_no_layerless_nodes() {
    assert_no_layerless_nodes(LayeringStrategy::CoffmanGraham);
}

#[test]
fn coffman_graham_no_empty_layers() {
    assert_no_empty_layers(LayeringStrategy::CoffmanGraham);
}

#[test]
fn coffman_graham_edges_point_towards_next_layers() {
    assert_edges_point_towards_next_layers(LayeringStrategy::CoffmanGraham);
}

#[test]
fn interactive_no_layerless_nodes() {
    assert_no_layerless_nodes(LayeringStrategy::Interactive);
}

#[test]
fn interactive_no_empty_layers() {
    assert_no_empty_layers(LayeringStrategy::Interactive);
}

#[test]
fn interactive_edges_point_towards_next_layers() {
    assert_edges_point_towards_next_layers(LayeringStrategy::Interactive);
}

#[test]
fn longest_path_no_layerless_nodes() {
    assert_no_layerless_nodes(LayeringStrategy::LongestPath);
}

#[test]
fn longest_path_no_empty_layers() {
    assert_no_empty_layers(LayeringStrategy::LongestPath);
}

#[test]
fn longest_path_edges_point_towards_next_layers() {
    assert_edges_point_towards_next_layers(LayeringStrategy::LongestPath);
}

#[test]
fn min_width_no_layerless_nodes() {
    assert_no_layerless_nodes(LayeringStrategy::MinWidth);
}

#[test]
fn min_width_no_empty_layers() {
    assert_no_empty_layers(LayeringStrategy::MinWidth);
}

#[test]
fn min_width_edges_point_towards_next_layers() {
    assert_edges_point_towards_next_layers(LayeringStrategy::MinWidth);
}

#[test]
fn network_simplex_no_layerless_nodes() {
    assert_no_layerless_nodes(LayeringStrategy::NetworkSimplex);
}

#[test]
fn network_simplex_no_empty_layers() {
    assert_no_empty_layers(LayeringStrategy::NetworkSimplex);
}

#[test]
fn network_simplex_edges_point_towards_next_layers() {
    assert_edges_point_towards_next_layers(LayeringStrategy::NetworkSimplex);
}

#[test]
fn stretch_width_no_layerless_nodes() {
    assert_no_layerless_nodes(LayeringStrategy::StretchWidth);
}

#[test]
fn stretch_width_no_empty_layers() {
    assert_no_empty_layers(LayeringStrategy::StretchWidth);
}

#[test]
fn stretch_width_edges_point_towards_next_layers() {
    assert_edges_point_towards_next_layers(LayeringStrategy::StretchWidth);
}

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn build_test_graph() -> (ElkNodeRef, Vec<ElkNodeRef>) {
    let root = ElkGraphUtil::create_graph();
    let node_a = ElkGraphUtil::create_node(Some(root.clone()));
    let node_b = ElkGraphUtil::create_node(Some(root.clone()));
    let node_c = ElkGraphUtil::create_node(Some(root.clone()));
    let node_d = ElkGraphUtil::create_node(Some(root.clone()));

    set_dimensions(&node_a, 30.0, 30.0);
    set_dimensions(&node_b, 30.0, 30.0);
    set_dimensions(&node_c, 30.0, 30.0);
    set_dimensions(&node_d, 30.0, 30.0);

    let _e1 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node_a.clone()),
        ElkConnectableShapeRef::Node(node_b.clone()),
    );
    let _e2 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node_a.clone()),
        ElkConnectableShapeRef::Node(node_c.clone()),
    );
    let _e3 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node_b.clone()),
        ElkConnectableShapeRef::Node(node_d.clone()),
    );
    let _e4 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node_c.clone()),
        ElkConnectableShapeRef::Node(node_d.clone()),
    );

    (root, vec![node_a, node_b, node_c, node_d])
}

fn apply_interactive_positions(nodes: &[ElkNodeRef]) {
    for (idx, node) in nodes.iter().enumerate() {
        let x = idx as f64 * 100.0;
        let y = idx as f64 * 50.0;
        let mut node_mut = node.borrow_mut();
        node_mut.connectable().shape().set_location(x, y);
    }
}

fn import_lgraph(
    root: &ElkNodeRef,
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef {
    let mut origin_store = OriginStore::new();
    let mut importer = ElkGraphImporter::new(&mut origin_store);
    importer.import_graph(root)
}

fn run_layout_for_strategy(
    strategy: LayeringStrategy,
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef {
    init_layered_options();
    let (root, nodes) = build_test_graph();
    set_node_property(&root, CoreOptions::ALGORITHM, "org.eclipse.elk.layered".to_string());
    set_node_property(&root, LayeredOptions::LAYERING_STRATEGY, strategy);

    if strategy == LayeringStrategy::Interactive {
        apply_interactive_positions(&nodes);
    }

    let lgraph = import_lgraph(&root);
    let mut layered = ElkLayered::new();
    layered.do_layout(&lgraph, None);
    lgraph
}

fn assert_no_layerless_nodes(strategy: LayeringStrategy) {
    let lgraph = run_layout_for_strategy(strategy);
    let graph_guard = lgraph.lock().expect("lgraph lock");
    assert!(graph_guard.layerless_nodes().is_empty());
}

fn assert_no_empty_layers(strategy: LayeringStrategy) {
    let lgraph = run_layout_for_strategy(strategy);
    let graph_guard = lgraph.lock().expect("lgraph lock");
    let layers = graph_guard.layers().clone();
    drop(graph_guard);

    for layer in &layers {
        let layer_guard = layer.lock().expect("layer lock");
        assert!(!layer_guard.nodes().is_empty());
    }
}

fn assert_edges_point_towards_next_layers(
    strategy: LayeringStrategy,
) {
    let lgraph = run_layout_for_strategy(strategy);
    let graph_guard = lgraph.lock().expect("lgraph lock");
    let layers = graph_guard.layers().clone();
    drop(graph_guard);

    for layer in &layers {
        let layer_idx = layer_index(layer);
        let nodes = layer.lock().expect("layer lock").nodes().clone();
        for node in nodes {
            let outgoing = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();
            for edge in outgoing {
                let target_layer_idx = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| edge_guard.target())
                    .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
                    .and_then(|target_node| target_node.lock().ok().and_then(|n| n.layer()))
                    .map(|layer_ref| layer_index(&layer_ref))
                    .unwrap_or(layer_idx);
                assert!(layer_idx < target_layer_idx);
            }
        }
    }
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
    node_mut
        .connectable()
        .shape()
        .set_dimensions(width, height);
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
