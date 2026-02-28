use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutAlgorithmResolver;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::recursive_graph_layout_engine::RecursiveGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, NullElkProgressMonitor};
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::LayoutConfigurator;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};

#[test]
fn issue_562_inside_self_loop_options_do_not_crash() {
    init_layered_options();

    let graph = ElkGraphUtil::create_graph();
    set_node_property(
        &graph,
        CoreOptions::ALGORITHM,
        LayeredOptions::ALGORITHM_ID.to_string(),
    );

    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    let port1 = ElkGraphUtil::create_port(Some(node.clone()));
    let port2 = ElkGraphUtil::create_port(Some(node.clone()));
    let edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(port1),
        ElkConnectableShapeRef::Port(port2),
    );

    let mut configurator = LayoutConfigurator::new();
    configurator
        .configure_node(&node)
        .set_property(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE, Some(true));
    configurator
        .configure_edge(&edge)
        .set_property(CoreOptions::INSIDE_SELF_LOOPS_YO, Some(true));

    let mut resolver = LayoutAlgorithmResolver::new();
    ElkUtil::apply_visitors(&graph, &mut [&mut configurator, &mut resolver]);

    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = NullElkProgressMonitor;
    engine.layout(&graph, &mut monitor);
}

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn set_node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<T>,
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
