use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::elk_layered::ElkLayered;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_importer::ElkGraphImporter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    CrossingMinimizationStrategy, InternalProperties, LayeredMetaDataProvider, LayeredOptions,
    LayeringStrategy, OrderingStrategy,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};

#[test]
fn consider_model_order_configs_run() {
    init_layered_options();

    let configs = vec![
        Config::new(OrderingStrategy::NodesAndEdges),
        Config::new(OrderingStrategy::NodesAndEdges).with_weights(),
        Config::new(OrderingStrategy::PreferEdges),
        Config::new(OrderingStrategy::PreferEdges).with_weights(),
        Config::new(OrderingStrategy::PreferNodes),
        Config::new(OrderingStrategy::PreferNodes).with_weights(),
        Config::new(OrderingStrategy::PreferEdges).with_port_model_order(),
        Config::new(OrderingStrategy::NodesAndEdges).with_port_model_order(),
        Config::new(OrderingStrategy::PreferNodes).with_port_model_order(),
    ];

    let only_config = std::env::var("ELK_MODEL_ORDER_CONFIG")
        .ok()
        .and_then(|value| value.parse::<usize>().ok());

    for (index, config) in configs.into_iter().enumerate() {
        if let Some(only) = only_config {
            if only != index {
                continue;
            }
        }
        let root = build_test_graph();
        set_node_property(&root, CoreOptions::ALGORITHM, "org.eclipse.elk.layered".to_string());
        set_node_property(
            &root,
            LayeredOptions::LAYERING_STRATEGY,
            LayeringStrategy::NetworkSimplex,
        );
        set_node_property(
            &root,
            LayeredOptions::CROSSING_MINIMIZATION_STRATEGY,
            CrossingMinimizationStrategy::LayerSweep,
        );
        set_node_property(
            &root,
            LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY,
            config.strategy,
        );
        if let Some(value) = config.node_influence {
            set_node_property(
                &root,
                LayeredOptions::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_NODE_INFLUENCE,
                value,
            );
        }
        if let Some(value) = config.port_influence {
            set_node_property(
                &root,
                LayeredOptions::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_PORT_INFLUENCE,
                value,
            );
        }
        if config.port_model_order {
            set_node_property(
                &root,
                LayeredOptions::CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER,
                true,
            );
        }

        let lgraph = import_lgraph(&root);
        let mut layered = ElkLayered::new();
        layered.do_layout(&lgraph, None);
    }
}

#[derive(Clone, Copy)]
struct Config {
    strategy: OrderingStrategy,
    node_influence: Option<f64>,
    port_influence: Option<f64>,
    port_model_order: bool,
}

impl Config {
    fn new(strategy: OrderingStrategy) -> Self {
        Config {
            strategy,
            node_influence: None,
            port_influence: None,
            port_model_order: false,
        }
    }

    fn with_weights(mut self) -> Self {
        self.node_influence = Some(0.001);
        self.port_influence = Some(0.001);
        self
    }

    fn with_port_model_order(mut self) -> Self {
        self.port_model_order = true;
        self
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

    assign_port_model_order(&node_a);
    assign_port_model_order(&node_b);
    assign_port_model_order(&node_c);
    assign_port_model_order(&node_d);

    root
}

fn assign_port_model_order(node: &ElkNodeRef) {
    let ports: Vec<_> = node.borrow_mut().ports().iter().cloned().collect();
    for (idx, port) in ports.iter().enumerate() {
        let mut port_mut = port.borrow_mut();
        port_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(InternalProperties::MODEL_ORDER, Some(idx as i32));
    }
}

fn import_lgraph(
    root: &ElkNodeRef,
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef {
    let mut origin_store = OriginStore::new();
    let mut importer = ElkGraphImporter::new(&mut origin_store);
    importer.import_graph(root)
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
