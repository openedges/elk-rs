use org_eclipse_elk_alg_topdownpacking::org::eclipse::elk::alg::topdownpacking::node_arrangement_strategy::NodeArrangementStrategy;
use org_eclipse_elk_alg_topdownpacking::org::eclipse::elk::alg::topdownpacking::options::{
    TopdownpackingMetaDataProvider, TopdownpackingOptions,
};
use org_eclipse_elk_alg_topdownpacking::org::eclipse::elk::alg::topdownpacking::whitespace_elimination_strategy::WhitespaceEliminationStrategy;
use org_eclipse_elk_core::org::eclipse::elk::core::data::{LayoutMetaDataService, LayoutOptionTarget};

fn init_topdownpacking_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&TopdownpackingMetaDataProvider);
}

#[test]
fn arrangement_strategy_defaults() {
    init_topdownpacking_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(TopdownpackingOptions::NODE_ARRANGEMENT_STRATEGY.id())
        .expect("node arrangement option");

    assert_eq!(option.group(), "nodeArrangement");
    assert!(option.targets().contains(&LayoutOptionTarget::Parents));

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<NodeArrangementStrategy>().ok())
        .expect("default value");
    assert_eq!(
        *default,
        NodeArrangementStrategy::LeftRightTopDownNodePlacer
    );
}

#[test]
fn whitespace_elimination_defaults() {
    init_topdownpacking_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(TopdownpackingOptions::WHITESPACE_ELIMINATION_STRATEGY.id())
        .expect("whitespace elimination option");

    assert_eq!(option.group(), "whitespaceElimination");
    assert!(option.targets().contains(&LayoutOptionTarget::Parents));

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<WhitespaceEliminationStrategy>().ok())
        .expect("default value");
    assert_eq!(
        *default,
        WhitespaceEliminationStrategy::BottomRowEqualWhitespaceEliminator
    );
}
