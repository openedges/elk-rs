use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredMetaDataProvider, LayeredOptions, LayeredSpacings,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkNodeRef,
};

const EPSILON: f64 = 1.0e-4;

fn factor_for(
    builder: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::layered_spacings::LayeredSpacingsBuilder,
    property: &Property<f64>,
) -> f64 {
    builder
        .factors()
        .iter()
        .find(|factor| factor.property.id() == property.id())
        .map(|factor| factor.factor)
        .unwrap_or(0.0)
}

fn create_simple_graph() -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    let node1 = ElkGraphUtil::create_node(Some(graph.clone()));
    let node2 = ElkGraphUtil::create_node(Some(graph.clone()));
    set_dimensions(&node1, 30.0, 30.0);
    set_dimensions(&node2, 30.0, 30.0);
    let _edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node1),
        ElkConnectableShapeRef::Node(node2),
    );
    graph
}

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

fn node_pos(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y())
}

fn node_pos_size(node: &ElkNodeRef) -> (f64, f64, f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y(), shape.width(), shape.height())
}

fn graph_property_f64(graph: &ElkNodeRef, property: &Property<f64>) -> f64 {
    let mut graph_mut = graph.borrow_mut();
    graph_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(property)
        .unwrap_or(0.0)
}

fn graph_has_property<T: Clone + Send + Sync + 'static>(
    graph: &ElkNodeRef,
    property: &Property<T>,
) -> bool {
    let mut graph_mut = graph.borrow_mut();
    graph_mut
        .connectable()
        .shape()
        .graph_element()
        .properties()
        .has_property(property)
}

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

#[test]
fn layered_spacing_factors_match_defaults() {
    init_layered_options();
    let builder = LayeredSpacings::with_base_value(33.0);
    let base_default = LayeredOptions::SPACING_NODE_NODE
        .get_default()
        .unwrap_or(0.0);

    let options: [&'static std::sync::LazyLock<Property<f64>>; 12] = [
        LayeredOptions::SPACING_EDGE_EDGE,
        LayeredOptions::SPACING_EDGE_LABEL,
        LayeredOptions::SPACING_EDGE_NODE,
        LayeredOptions::SPACING_LABEL_LABEL,
        LayeredOptions::SPACING_LABEL_NODE,
        LayeredOptions::SPACING_LABEL_PORT_HORIZONTAL,
        LayeredOptions::SPACING_LABEL_PORT_VERTICAL,
        LayeredOptions::SPACING_NODE_SELF_LOOP,
        LayeredOptions::SPACING_PORT_PORT,
        LayeredOptions::SPACING_EDGE_EDGE_BETWEEN_LAYERS,
        LayeredOptions::SPACING_EDGE_NODE_BETWEEN_LAYERS,
        LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS,
    ];

    for option in options {
        let expected = option.get_default().unwrap_or(0.0) / base_default;
        let actual = factor_for(&builder, option);
        assert!(
            (actual - expected).abs() <= EPSILON,
            "factor mismatch for {} (expected {}, got {})",
            option.id(),
            expected,
            actual
        );
    }
}

#[test]
fn layered_spacing_applies_between_layers() {
    init_layered_options();
    let graph = create_simple_graph();
    let mut builder = LayeredSpacings::with_base_value(33.0);
    builder.with_factor(LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS, 2.0);
    {
        let mut graph_mut = graph.borrow_mut();
        let props = graph_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut();
        builder.apply_to_properties(props);
        props.set_property(CoreOptions::DIRECTION, Some(Direction::Right));
        props.set_property(CoreOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
    }
    let configured = graph_property_f64(&graph, LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS);
    assert!(
        (configured - 66.0).abs() <= EPSILON,
        "unexpected configured spacing: {configured}"
    );

    let mut provider = LayeredLayoutProvider::new();
    provider.layout(&graph, &mut BasicProgressMonitor::new());

    let edges: Vec<ElkEdgeRef> = graph
        .borrow_mut()
        .contained_edges()
        .iter()
        .cloned()
        .collect();

    for edge in edges {
        let (source, target) = {
            let edge_ref = edge.borrow();
            let source_shape = edge_ref.sources_ro().get(0).expect("edge source");
            let target_shape = edge_ref.targets_ro().get(0).expect("edge target");
            (
                ElkGraphUtil::connectable_shape_to_node(&source_shape).expect("source node"),
                ElkGraphUtil::connectable_shape_to_node(&target_shape).expect("target node"),
            )
        };

        let (delta_x, delta_y, spacing) = {
            let (source_x, source_y, source_w, source_h) = node_pos_size(&source);
            let (target_x, target_y) = node_pos(&target);
            let delta_x = target_x - (source_x + source_w);
            let delta_y = target_y - (source_y + source_h);
            let spacing = if delta_x.abs() >= delta_y.abs() {
                delta_x
            } else {
                delta_y
            };
            (delta_x, delta_y, spacing)
        };

        assert!(
            (spacing.abs() - 66.0).abs() <= EPSILON,
            "unexpected spacing between layers: {spacing} (dx={delta_x}, dy={delta_y})"
        );
    }
}

#[test]
fn spacing_overrides_default_for_layered_property() {
    init_layered_options();
    let graph = create_simple_graph();
    let value = graph_property_f64(&graph, LayeredOptions::SPACING_EDGE_NODE);
    let expected = LayeredOptions::SPACING_EDGE_NODE
        .get_default()
        .unwrap_or(0.0);
    assert!((value - expected).abs() <= EPSILON);
}

#[test]
fn spacing_overrides_default_for_inheriting_property() {
    init_layered_options();
    let graph = create_simple_graph();
    let default = LayeredOptions::SPACING_EDGE_NODE
        .get_default()
        .unwrap_or(0.0);
    let custom_property = Property::from_property(LayeredOptions::SPACING_EDGE_NODE, default + 3.0);
    let value = graph_property_f64(&graph, &custom_property);
    assert!((value - (default + 3.0)).abs() <= EPSILON);
}

#[test]
fn spacing_defaults_after_configuration() {
    init_layered_options();
    let graph = create_simple_graph();
    let builder = LayeredSpacings::with_base_value(35.0);
    {
        let mut graph_mut = graph.borrow_mut();
        let props = graph_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut();
        builder.apply_to_properties(props);
    }

    assert!(graph_has_property(
        &graph,
        LayeredOptions::SPACING_EDGE_NODE
    ));

    let factor = factor_for(&builder, LayeredOptions::SPACING_EDGE_NODE);
    let expected = 35.0 * factor;
    let custom_property =
        Property::from_property(LayeredOptions::SPACING_EDGE_NODE, expected + 3.0);
    let value = graph_property_f64(&graph, &custom_property);
    assert!((value - expected).abs() <= EPSILON);
    let layered_value = graph_property_f64(&graph, LayeredOptions::SPACING_EDGE_NODE);
    assert!((layered_value - expected).abs() <= EPSILON);
}
