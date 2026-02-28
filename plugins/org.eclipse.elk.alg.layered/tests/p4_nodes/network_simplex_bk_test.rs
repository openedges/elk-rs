use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::elk_layered::ElkLayered;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_importer::ElkGraphImporter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    EdgeStraighteningStrategy, FixedAlignment, LayeredMetaDataProvider, LayeredOptions,
    LayeringStrategy, NodePlacementStrategy,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p2layers::network_simplex_layerer::NetworkSimplexLayerer;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};

#[test]
fn network_simplex_layering_invariants() {
    init_layered_options();

    let root = build_test_graph();
    set_node_property(
        &root,
        CoreOptions::ALGORITHM,
        "org.eclipse.elk.layered".to_string(),
    );
    set_node_property(
        &root,
        LayeredOptions::LAYERING_STRATEGY,
        LayeringStrategy::NetworkSimplex,
    );

    let lgraph = import_lgraph(&root);
    let mut layerer = NetworkSimplexLayerer::new();
    let mut monitor = BasicProgressMonitor::new();
    if let Ok(mut graph_guard) = lgraph.lock() {
        layerer.process(&mut graph_guard, &mut monitor);
    }

    assert_layering_invariants(&lgraph);
}

#[test]
fn bk_node_placement_orders_nodes() {
    init_layered_options();

    let root = build_test_graph();
    set_node_property(
        &root,
        CoreOptions::ALGORITHM,
        "org.eclipse.elk.layered".to_string(),
    );
    set_node_property(
        &root,
        LayeredOptions::NODE_PLACEMENT_STRATEGY,
        NodePlacementStrategy::BrandesKoepf,
    );
    set_node_property(
        &root,
        LayeredOptions::NODE_PLACEMENT_BK_EDGE_STRAIGHTENING,
        EdgeStraighteningStrategy::None,
    );
    set_node_property(
        &root,
        LayeredOptions::NODE_PLACEMENT_BK_FIXED_ALIGNMENT,
        FixedAlignment::None,
    );

    let lgraph = import_lgraph(&root);
    let mut layered = ElkLayered::new();
    layered.do_layout(&lgraph, None);

    assert_layer_node_positions_increasing(&lgraph);
}

#[test]
fn network_simplex_node_placement_orders_nodes() {
    init_layered_options();

    let root = build_test_graph();
    set_node_property(
        &root,
        CoreOptions::ALGORITHM,
        "org.eclipse.elk.layered".to_string(),
    );
    set_node_property(
        &root,
        LayeredOptions::NODE_PLACEMENT_STRATEGY,
        NodePlacementStrategy::NetworkSimplex,
    );

    let lgraph = import_lgraph(&root);
    let mut layered = ElkLayered::new();
    layered.do_layout(&lgraph, None);

    assert_layer_node_positions_increasing(&lgraph);
}

#[test]
fn simple_node_placement_orders_nodes() {
    init_layered_options();

    let root = build_test_graph();
    set_node_property(
        &root,
        CoreOptions::ALGORITHM,
        "org.eclipse.elk.layered".to_string(),
    );
    set_node_property(
        &root,
        LayeredOptions::NODE_PLACEMENT_STRATEGY,
        NodePlacementStrategy::Simple,
    );

    let lgraph = import_lgraph(&root);
    let mut layered = ElkLayered::new();
    layered.do_layout(&lgraph, None);

    assert_layer_node_positions_increasing(&lgraph);
}

#[test]
fn linear_segments_node_placement_orders_nodes() {
    init_layered_options();

    let root = build_test_graph();
    set_node_property(
        &root,
        CoreOptions::ALGORITHM,
        "org.eclipse.elk.layered".to_string(),
    );
    set_node_property(
        &root,
        LayeredOptions::NODE_PLACEMENT_STRATEGY,
        NodePlacementStrategy::LinearSegments,
    );

    let lgraph = import_lgraph(&root);
    let mut layered = ElkLayered::new();
    layered.do_layout(&lgraph, None);

    assert_layer_node_positions_increasing(&lgraph);
}

fn build_test_graph() -> ElkNodeRef {
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

    root
}

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn import_lgraph(
    root: &ElkNodeRef,
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef {
    let mut origin_store = OriginStore::new();
    let mut importer = ElkGraphImporter::new(&mut origin_store);
    importer.import_graph(root)
}

fn assert_layering_invariants(
    lgraph: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef,
) {
    let graph_guard = lgraph.lock().expect("lgraph lock");
    assert!(graph_guard.layerless_nodes().is_empty());

    let layers = graph_guard.layers().clone();
    drop(graph_guard);

    for layer in &layers {
        let layer_guard = layer.lock().expect("layer lock");
        assert!(!layer_guard.nodes().is_empty());
    }

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
                let target_layer_index = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| edge_guard.target())
                    .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
                    .and_then(|target_node| target_node.lock().ok().and_then(|n| n.layer()))
                    .map(|layer_ref| layer_index(&layer_ref))
                    .unwrap_or(layer_idx);
                assert!(layer_idx < target_layer_index);
            }
        }
    }
}

fn assert_layer_node_positions_increasing(
    lgraph: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef,
) {
    let layers = lgraph.lock().expect("lgraph lock").layers().clone();

    for layer in layers {
        let nodes = layer.lock().expect("layer lock").nodes().clone();
        let mut last_bottom = None;
        for node in nodes {
            if let Ok(mut node_guard) = node.lock() {
                let pos = node_guard.shape().position_ref().y;
                let size = node_guard.shape().size_ref().y;
                if let Some(last) = last_bottom {
                    assert!(last < pos);
                }
                last_bottom = Some(pos + size);
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
