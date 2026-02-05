use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkMargin;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;

use crate::org::eclipse::elk::alg::vertiflex::EdgeRoutingStrategy;

pub struct VertiFlexOptions;

pub static VERTICAL_CONSTRAINT_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.vertiflex.verticalConstraint"));

pub static LAYOUT_STRATEGY_PROPERTY: LazyLock<Property<EdgeRoutingStrategy>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.vertiflex.layoutStrategy",
        EdgeRoutingStrategy::Straight,
    )
});

pub static LAYER_DISTANCE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.vertiflex.layerDistance", 50.0));

pub static CONSIDER_NODE_MODEL_ORDER_PROPERTY: LazyLock<Property<bool>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.vertiflex.considerNodeModelOrder", true)
});

impl VertiFlexOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.vertiflex";

    pub const VERTICAL_CONSTRAINT: &'static LazyLock<Property<f64>> =
        &VERTICAL_CONSTRAINT_PROPERTY;
    pub const LAYOUT_STRATEGY: &'static LazyLock<Property<EdgeRoutingStrategy>> =
        &LAYOUT_STRATEGY_PROPERTY;
    pub const LAYER_DISTANCE: &'static LazyLock<Property<f64>> = &LAYER_DISTANCE_PROPERTY;
    pub const CONSIDER_NODE_MODEL_ORDER: &'static LazyLock<Property<bool>> =
        &CONSIDER_NODE_MODEL_ORDER_PROPERTY;

    pub const SPACING_NODE_NODE: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_NODE_NODE;
    pub const PADDING: &'static LazyLock<Property<ElkPadding>> = CoreOptions::PADDING;
    pub const INTERACTIVE: &'static LazyLock<Property<bool>> = CoreOptions::INTERACTIVE;
    pub const PORT_CONSTRAINTS: &'static LazyLock<Property<PortConstraints>> =
        CoreOptions::PORT_CONSTRAINTS;
    pub const EDGE_LABELS_INLINE: &'static LazyLock<Property<bool>> = CoreOptions::EDGE_LABELS_INLINE;
    pub const OMIT_NODE_MICRO_LAYOUT: &'static LazyLock<Property<bool>> =
        CoreOptions::OMIT_NODE_MICRO_LAYOUT;
    pub const MARGINS: &'static LazyLock<Property<ElkMargin>> = CoreOptions::MARGINS;
}
