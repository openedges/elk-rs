use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::elk_layered::ElkLayered;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_importer::ElkGraphImporter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    CycleBreakingStrategy, InternalProperties, LayeredMetaDataProvider, LayeredOptions,
    LayeringStrategy,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};

#[test]
fn model_order_layering_runs() {
    init_layered_options();

    for strategy in [LayeringStrategy::BfModelOrder, LayeringStrategy::DfModelOrder] {
        let root = build_test_graph();
        set_node_property(&root, CoreOptions::ALGORITHM, "org.eclipse.elk.layered".to_string());
        set_node_property(
            &root,
            LayeredOptions::CYCLE_BREAKING_STRATEGY,
            CycleBreakingStrategy::ModelOrder,
        );
        set_node_property(&root, LayeredOptions::LAYERING_STRATEGY, strategy);

        let lgraph = import_lgraph(&root);
        let mut layered = ElkLayered::new();
        layered.do_layout(&lgraph, None);

        assert_layering_invariants(&lgraph);
    }
}

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
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
    set_node_property(&node_a, InternalProperties::MODEL_ORDER, 0);
    set_node_property(&node_b, InternalProperties::MODEL_ORDER, 1);
    set_node_property(&node_c, InternalProperties::MODEL_ORDER, 2);
    set_node_property(&node_d, InternalProperties::MODEL_ORDER, 3);

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
