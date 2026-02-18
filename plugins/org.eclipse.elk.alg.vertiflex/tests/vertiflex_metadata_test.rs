use org_eclipse_elk_alg_vertiflex::org::eclipse::elk::alg::vertiflex::options::{
    VertiFlexMetaDataProvider, VertiFlexOptions,
};
use org_eclipse_elk_alg_vertiflex::org::eclipse::elk::alg::vertiflex::EdgeRoutingStrategy;
use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    LayoutMetaDataService, LayoutOptionTarget,
};

fn init_vertiflex_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&VertiFlexMetaDataProvider);
}

#[test]
fn layout_strategy_defaults() {
    init_vertiflex_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(VertiFlexOptions::LAYOUT_STRATEGY.id())
        .expect("layout strategy option");

    assert!(option.targets().contains(&LayoutOptionTarget::Nodes));

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<EdgeRoutingStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, EdgeRoutingStrategy::Straight);
}

#[test]
fn layer_distance_defaults() {
    init_vertiflex_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(VertiFlexOptions::LAYER_DISTANCE.id())
        .expect("layer distance option");

    assert!(option.targets().contains(&LayoutOptionTarget::Parents));

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<f64>().ok())
        .expect("default value");
    assert!((*default - 50.0).abs() < f64::EPSILON);
}

#[test]
fn consider_model_order_defaults() {
    init_vertiflex_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(VertiFlexOptions::CONSIDER_NODE_MODEL_ORDER.id())
        .expect("consider node model order option");

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<bool>().ok())
        .expect("default value");
    assert!(*default);
}
