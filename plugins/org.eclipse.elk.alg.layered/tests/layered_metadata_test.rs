use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::components::ComponentOrderingStrategy;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    CenterEdgeLabelPlacementStrategy, ConstraintCalculationStrategy, CrossingMinimizationStrategy,
    CycleBreakingStrategy, EdgeLabelSideSelection, EdgeStraighteningStrategy,
    GraphCompactionStrategy, GreedySwitchType, GroupOrderStrategy, LayerUnzippingStrategy,
    LayeredMetaDataProvider, LayeredOptions, LayeringStrategy, LongEdgeOrderingStrategy,
    NodeFlexibility, NodePlacementStrategy, OrderingStrategy, WrappingStrategy,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    LayoutMetaDataService, LayoutOptionTarget, LayoutOptionVisibility,
};

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

#[test]
fn spacing_base_value_metadata() {
    init_layered_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::SPACING_BASE_VALUE.id())
        .expect("spacing base value option");

    assert_eq!(option.group(), "spacing");
    assert!(option.targets().contains(&LayoutOptionTarget::Parents));
    assert!(option.default_value().is_none());

    let lower = option
        .lower_bound()
        .and_then(|value| value.downcast::<f64>().ok())
        .expect("lower bound");
    assert!((*lower - 0.0).abs() < f64::EPSILON);
}

#[test]
fn priority_direction_metadata() {
    init_layered_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::PRIORITY_DIRECTION.id())
        .expect("priority direction option");

    assert_eq!(option.group(), "priority");
    assert!(option.targets().contains(&LayoutOptionTarget::Edges));
    assert_eq!(option.visibility(), LayoutOptionVisibility::Advanced);

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<i32>().ok())
        .expect("default value");
    assert_eq!(*default, 0);
}

#[test]
fn wrapping_strategy_defaults() {
    init_layered_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::WRAPPING_STRATEGY.id())
        .expect("wrapping strategy option");

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<WrappingStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, WrappingStrategy::Off);
}

#[test]
fn layer_unzipping_defaults() {
    init_layered_options();

    let strategy = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::LAYER_UNZIPPING_STRATEGY.id())
        .expect("layer unzipping strategy option");
    let default = strategy
        .default_value()
        .and_then(|value| value.downcast::<LayerUnzippingStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, LayerUnzippingStrategy::None);

    let split = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::LAYER_UNZIPPING_LAYER_SPLIT.id())
        .expect("layer split option");
    let default = split
        .default_value()
        .and_then(|value| value.downcast::<i32>().ok())
        .expect("default value");
    assert_eq!(*default, 2);

    let lower = split
        .lower_bound()
        .and_then(|value| value.downcast::<i32>().ok())
        .expect("lower bound");
    assert_eq!(*lower, 1);
}

#[test]
fn cycle_breaking_and_layering_defaults() {
    init_layered_options();

    let cycle = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::CYCLE_BREAKING_STRATEGY.id())
        .expect("cycle breaking strategy option");
    let default = cycle
        .default_value()
        .and_then(|value| value.downcast::<CycleBreakingStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, CycleBreakingStrategy::Greedy);

    let layering = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::LAYERING_STRATEGY.id())
        .expect("layering strategy option");
    let default = layering
        .default_value()
        .and_then(|value| value.downcast::<LayeringStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, LayeringStrategy::NetworkSimplex);
}

#[test]
fn crossing_minimization_defaults() {
    init_layered_options();

    let strategy = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::CROSSING_MINIMIZATION_STRATEGY.id())
        .expect("crossing minimization strategy option");
    let default = strategy
        .default_value()
        .and_then(|value| value.downcast::<CrossingMinimizationStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, CrossingMinimizationStrategy::LayerSweep);

    let threshold = LayoutMetaDataService::get_instance()
        .get_option_data(
            LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_ACTIVATION_THRESHOLD.id(),
        )
        .expect("greedy switch activation threshold option");
    let default = threshold
        .default_value()
        .and_then(|value| value.downcast::<i32>().ok())
        .expect("default value");
    assert_eq!(*default, 40);

    let greedy_type = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE.id())
        .expect("greedy switch type option");
    let default = greedy_type
        .default_value()
        .and_then(|value| value.downcast::<GreedySwitchType>().ok())
        .expect("default value");
    assert_eq!(*default, GreedySwitchType::TwoSided);
}

#[test]
fn node_placement_defaults() {
    init_layered_options();

    let strategy = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::NODE_PLACEMENT_STRATEGY.id())
        .expect("node placement strategy option");
    let default = strategy
        .default_value()
        .and_then(|value| value.downcast::<NodePlacementStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, NodePlacementStrategy::BrandesKoepf);

    let straightening = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::NODE_PLACEMENT_BK_EDGE_STRAIGHTENING.id())
        .expect("node placement edge straightening option");
    let default = straightening
        .default_value()
        .and_then(|value| value.downcast::<EdgeStraighteningStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, EdgeStraighteningStrategy::ImproveStraightness);

    let flexibility = LayoutMetaDataService::get_instance()
        .get_option_data(
            LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT.id(),
        )
        .expect("node flexibility default option");
    let default = flexibility
        .default_value()
        .and_then(|value| value.downcast::<NodeFlexibility>().ok())
        .expect("default value");
    assert_eq!(*default, NodeFlexibility::None);
}

#[test]
fn compaction_defaults() {
    init_layered_options();

    let strategy = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::COMPACTION_POST_COMPACTION_STRATEGY.id())
        .expect("compaction strategy option");
    let default = strategy
        .default_value()
        .and_then(|value| value.downcast::<GraphCompactionStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, GraphCompactionStrategy::None);

    let constraints = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::COMPACTION_POST_COMPACTION_CONSTRAINTS.id())
        .expect("compaction constraints option");
    let default = constraints
        .default_value()
        .and_then(|value| value.downcast::<ConstraintCalculationStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, ConstraintCalculationStrategy::Scanline);
}

#[test]
fn edge_label_defaults() {
    init_layered_options();

    let side = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::EDGE_LABELS_SIDE_SELECTION.id())
        .expect("edge label side selection");
    let default = side
        .default_value()
        .and_then(|value| value.downcast::<EdgeLabelSideSelection>().ok())
        .expect("default value");
    assert_eq!(*default, EdgeLabelSideSelection::SmartDown);

    let center = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY.id())
        .expect("edge label center placement");
    let default = center
        .default_value()
        .and_then(|value| value.downcast::<CenterEdgeLabelPlacementStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, CenterEdgeLabelPlacementStrategy::MedianLayer);
}

#[test]
fn consider_model_order_defaults() {
    init_layered_options();

    let strategy = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY.id())
        .expect("consider model order strategy");
    assert_eq!(strategy.group(), "considerModelOrder");
    assert!(strategy.targets().contains(&LayoutOptionTarget::Parents));
    let default = strategy
        .default_value()
        .and_then(|value| value.downcast::<OrderingStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, OrderingStrategy::None);

    let components = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::CONSIDER_MODEL_ORDER_COMPONENTS.id())
        .expect("consider model order components");
    let default = components
        .default_value()
        .and_then(|value| value.downcast::<ComponentOrderingStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, ComponentOrderingStrategy::None);

    let long_edge = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::CONSIDER_MODEL_ORDER_LONG_EDGE_STRATEGY.id())
        .expect("consider model order long edge strategy");
    let default = long_edge
        .default_value()
        .and_then(|value| value.downcast::<LongEdgeOrderingStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, LongEdgeOrderingStrategy::DummyNodeOver);
}

#[test]
fn group_model_order_defaults() {
    init_layered_options();

    let cycle = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::GROUP_MODEL_ORDER_CYCLE_BREAKING_ID.id())
        .expect("group model order cycle breaking id");
    assert_eq!(cycle.group(), "considerModelOrder.groupModelOrder");
    assert!(cycle.targets().contains(&LayoutOptionTarget::Nodes));
    let default = cycle
        .default_value()
        .and_then(|value| value.downcast::<i32>().ok())
        .expect("default value");
    assert_eq!(*default, 0);

    let crossing = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::GROUP_MODEL_ORDER_CROSSING_MINIMIZATION_ID.id())
        .expect("group model order crossing minimization id");
    assert!(crossing.targets().contains(&LayoutOptionTarget::Nodes));
    assert!(crossing.targets().contains(&LayoutOptionTarget::Edges));
    assert!(crossing.targets().contains(&LayoutOptionTarget::Ports));

    let strategy = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::GROUP_MODEL_ORDER_CB_GROUP_ORDER_STRATEGY.id())
        .expect("group model order cb group order strategy");
    let default = strategy
        .default_value()
        .and_then(|value| value.downcast::<GroupOrderStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, GroupOrderStrategy::OnlyWithinGroup);

    let enforced = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::GROUP_MODEL_ORDER_CM_ENFORCED_GROUP_ORDERS.id())
        .expect("group model order enforced groups");
    let default = enforced
        .default_value()
        .and_then(|value| value.downcast::<Vec<i32>>().ok())
        .expect("default value");
    assert_eq!(&*default, &[1, 2, 6, 7, 10, 11]);
}
