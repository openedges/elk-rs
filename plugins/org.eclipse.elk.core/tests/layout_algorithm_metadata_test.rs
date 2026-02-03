use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, EdgeRouting, PortAlignment, TopdownNodeTypes,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::GraphFeature;

fn expect_value<T: 'static + Send + Sync>(
    value: Option<Arc<dyn std::any::Any + Send + Sync>>,
) -> Arc<T> {
    value
        .and_then(|value| value.downcast::<T>().ok())
        .expect("expected value to be set")
}

#[test]
fn layered_metadata_has_features() {
    let algo = LayoutMetaDataService::get_instance()
        .get_algorithm_data("org.eclipse.elk.layered")
        .expect("layered algorithm");
    assert!(algo.supports_feature(GraphFeature::Compound));
    assert!(algo.supports_feature(GraphFeature::Clusters));
}

#[test]
fn layered_metadata_defaults_match_core() {
    let algo = LayoutMetaDataService::get_instance()
        .get_algorithm_data("org.eclipse.elk.layered")
        .expect("layered algorithm");

    let padding = expect_value::<ElkPadding>(algo.default_value_any(CoreOptions::PADDING.id()));
    assert_eq!(*padding, ElkPadding::with_any(12.0));

    let routing = expect_value::<EdgeRouting>(algo.default_value_any(CoreOptions::EDGE_ROUTING.id()));
    assert_eq!(*routing, EdgeRouting::Orthogonal);

    let border_offset =
        expect_value::<f64>(algo.default_value_any(CoreOptions::PORT_BORDER_OFFSET.id()));
    assert!((*border_offset - 0.0).abs() < f64::EPSILON);

    let seed = expect_value::<i32>(algo.default_value_any(CoreOptions::RANDOM_SEED.id()));
    assert_eq!(*seed, 1);

    let aspect = expect_value::<f64>(algo.default_value_any(CoreOptions::ASPECT_RATIO.id()));
    assert!((*aspect - 1.6).abs() < f64::EPSILON);

    let priority = expect_value::<i32>(algo.default_value_any(CoreOptions::PRIORITY.id()));
    assert_eq!(*priority, 0);

    let separate =
        expect_value::<bool>(algo.default_value_any(CoreOptions::SEPARATE_CONNECTED_COMPONENTS.id()));
    assert_eq!(*separate, true);

    let port_alignment = expect_value::<PortAlignment>(
        algo.default_value_any(CoreOptions::PORT_ALIGNMENT_DEFAULT.id()),
    );
    assert_eq!(*port_alignment, PortAlignment::Justified);

    let node_type =
        expect_value::<TopdownNodeTypes>(algo.default_value_any(CoreOptions::TOPDOWN_NODE_TYPE.id()));
    assert_eq!(*node_type, TopdownNodeTypes::HierarchicalNode);

    assert!(algo.knows_option(CoreOptions::SPACING_NODE_NODE.id()));
    assert!(algo.knows_option(CoreOptions::SPACING_COMMENT_COMMENT.id()));
    assert!(algo.knows_option(CoreOptions::SPACING_COMMENT_NODE.id()));
}
