use std::sync::Once;

use org_eclipse_elk_conn_gmf::org::eclipse::elk::conn::gmf::layouter::{
    Draw2DMetaDataProvider, Draw2DOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{CoreOptions, Direction};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::GraphFeature;

static DRAW2D_OPTIONS_INIT: Once = Once::new();

fn init_draw2d_options() {
    DRAW2D_OPTIONS_INIT.call_once(|| {
        let service = LayoutMetaDataService::get_instance();
        service.register_layout_meta_data_provider(&Draw2DMetaDataProvider);
    });
}

#[test]
fn draw2d_algorithm_registered() {
    init_draw2d_options();

    let algo = LayoutMetaDataService::get_instance()
        .get_algorithm_data(Draw2DOptions::ALGORITHM_ID)
        .expect("draw2d algorithm");

    assert_eq!(algo.category_id(), Some("org.eclipse.elk.layered"));
    assert!(algo.supports_feature(GraphFeature::MultiEdges));
}

#[test]
fn draw2d_option_defaults_match_java_metadata() {
    init_draw2d_options();

    let algo = LayoutMetaDataService::get_instance()
        .get_algorithm_data(Draw2DOptions::ALGORITHM_ID)
        .expect("draw2d algorithm");

    let spacing = algo
        .default_value_any(CoreOptions::SPACING_NODE_NODE.id())
        .and_then(|value| value.downcast::<f64>().ok())
        .expect("spacing default");
    assert_eq!(*spacing, 16.0_f64);

    let padding = algo
        .default_value_any(CoreOptions::PADDING.id())
        .and_then(|value| value.downcast::<ElkPadding>().ok())
        .expect("padding default");
    assert_eq!(padding.top, 16.0_f64);
    assert_eq!(padding.right, 16.0_f64);
    assert_eq!(padding.bottom, 16.0_f64);
    assert_eq!(padding.left, 16.0_f64);

    let direction = algo
        .default_value_any(CoreOptions::DIRECTION.id())
        .and_then(|value| value.downcast::<Direction>().ok())
        .expect("direction default");
    assert_eq!(*direction, Direction::Right);
}
