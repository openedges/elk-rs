use org_eclipse_elk_alg_graphviz_dot::org::eclipse::elk::alg::graphviz::dot::transform::{
    NeatoModel, OverlapMode,
};
use org_eclipse_elk_alg_graphviz_layouter::org::eclipse::elk::alg::graphviz::layouter::options::{
    CircoOptions, DotOptions, FdpOptions, GraphvizMetaDataProvider, GraphvizOptions, NeatoOptions,
    TwopiOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    LayoutMetaDataService, LayoutOptionTarget,
};

fn init_graphviz_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&GraphvizMetaDataProvider);
}

#[test]
fn graphviz_algorithms_registered() {
    init_graphviz_options();
    let service = LayoutMetaDataService::get_instance();

    let dot = service
        .get_algorithm_data(DotOptions::ALGORITHM_ID)
        .expect("dot algorithm");
    assert_eq!(dot.name(), "Graphviz Dot");
    assert_eq!(dot.category_id(), Some("org.eclipse.elk.layered"));

    let neato = service
        .get_algorithm_data(NeatoOptions::ALGORITHM_ID)
        .expect("neato algorithm");
    assert_eq!(neato.name(), "Graphviz Neato");
    assert_eq!(neato.category_id(), Some("org.eclipse.elk.force"));

    let fdp = service
        .get_algorithm_data(FdpOptions::ALGORITHM_ID)
        .expect("fdp algorithm");
    assert_eq!(fdp.name(), "Graphviz FDP");
    assert_eq!(fdp.category_id(), Some("org.eclipse.elk.force"));

    let twopi = service
        .get_algorithm_data(TwopiOptions::ALGORITHM_ID)
        .expect("twopi algorithm");
    assert_eq!(twopi.name(), "Graphviz Twopi");
    assert_eq!(twopi.category_id(), Some("org.eclipse.elk.radial"));

    let circo = service
        .get_algorithm_data(CircoOptions::ALGORITHM_ID)
        .expect("circo algorithm");
    assert_eq!(circo.name(), "Graphviz Circo");
    assert_eq!(circo.category_id(), Some("org.eclipse.elk.circle"));
}

#[test]
fn neato_model_default() {
    init_graphviz_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(GraphvizOptions::NEATO_MODEL.id())
        .expect("neato model option");

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<NeatoModel>().ok())
        .expect("default neato model");
    assert_eq!(*default, NeatoModel::Shortpath);
}

#[test]
fn overlap_mode_default() {
    init_graphviz_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(GraphvizOptions::OVERLAP_MODE.id())
        .expect("overlap mode option");

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<OverlapMode>().ok())
        .expect("default overlap mode");
    assert_eq!(*default, OverlapMode::Prism);
}

#[test]
fn label_distance_targets_edges() {
    init_graphviz_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(GraphvizOptions::LABEL_DISTANCE.id())
        .expect("label distance option");

    assert!(option.targets().contains(&LayoutOptionTarget::Edges));

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<f64>().ok())
        .expect("default label distance");
    assert_eq!(*default, 1.0);
}
