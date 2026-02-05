use org_eclipse_elk_alg_disco::org::eclipse::elk::alg::disco::options::{
    CompactionStrategy, DisCoMetaDataProvider, DisCoOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    LayoutMetaDataService, LayoutOptionTarget, LayoutOptionVisibility,
};

fn init_disco_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&DisCoMetaDataProvider);
}

#[test]
fn disco_algorithm_registered() {
    init_disco_options();

    let algo = LayoutMetaDataService::get_instance()
        .get_algorithm_data(DisCoOptions::ALGORITHM_ID)
        .expect("disco algorithm");
    assert_eq!(algo.name(), "ELK DisCo");
}

#[test]
fn compaction_strategy_metadata() {
    init_disco_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(DisCoOptions::COMPONENT_COMPACTION_STRATEGY.id())
        .expect("compaction strategy option");

    assert_eq!(option.group(), "componentCompaction");
    assert!(option.targets().contains(&LayoutOptionTarget::Parents));
    assert_eq!(option.visibility(), LayoutOptionVisibility::Visible);

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<CompactionStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, CompactionStrategy::Polyomino);
}

#[test]
fn debug_options_hidden() {
    init_disco_options();

    let graph_option = LayoutMetaDataService::get_instance()
        .get_option_data(DisCoOptions::DEBUG_DISCO_GRAPH.id())
        .expect("debug graph option");
    assert_eq!(graph_option.group(), "debug");
    assert_eq!(graph_option.visibility(), LayoutOptionVisibility::Hidden);

    let poly_option = LayoutMetaDataService::get_instance()
        .get_option_data(DisCoOptions::DEBUG_DISCO_POLYS.id())
        .expect("debug polys option");
    assert_eq!(poly_option.group(), "debug");
    assert_eq!(poly_option.visibility(), LayoutOptionVisibility::Hidden);
}
