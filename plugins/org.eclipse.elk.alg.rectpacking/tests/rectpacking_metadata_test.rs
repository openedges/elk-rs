use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::options::{
    RectPackingMetaDataProvider, RectPackingOptions,
};
use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::p1widthapproximation::WidthApproximationStrategy;
use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::p3whitespaceelimination::WhiteSpaceEliminationStrategy;
use org_eclipse_elk_core::org::eclipse::elk::core::data::{LayoutMetaDataService, LayoutOptionTarget};

fn init_rectpacking_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&RectPackingMetaDataProvider);
}

#[test]
fn width_approximation_defaults() {
    init_rectpacking_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(RectPackingOptions::WIDTH_APPROXIMATION_STRATEGY.id())
        .expect("width approximation strategy option");

    assert_eq!(option.group(), "widthApproximation");
    assert!(option.targets().contains(&LayoutOptionTarget::Parents));

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<WidthApproximationStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, WidthApproximationStrategy::Greedy);
}

#[test]
fn whitespace_elimination_defaults() {
    init_rectpacking_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(RectPackingOptions::WHITE_SPACE_ELIMINATION_STRATEGY.id())
        .expect("whitespace elimination option");

    assert_eq!(option.group(), "whiteSpaceElimination");
    assert!(option.targets().contains(&LayoutOptionTarget::Parents));

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<WhiteSpaceEliminationStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, WhiteSpaceEliminationStrategy::None);
}

#[test]
fn compaction_iterations_default() {
    init_rectpacking_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(RectPackingOptions::PACKING_COMPACTION_ITERATIONS.id())
        .expect("compaction iterations option");

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<i32>().ok())
        .expect("default value");
    assert_eq!(*default, 1);
}
