use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_alg_graphviz_dot::org::eclipse::elk::alg::graphviz::dot::transform::{
    NeatoModel, OverlapMode,
};

use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;

pub struct GraphvizOptions;

pub static ADAPT_PORT_POSITIONS_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.graphviz.adaptPortPositions", true));

pub static CONCENTRATE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.graphviz.concentrate", false));

pub static EPSILON_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.graphviz.epsilon"));

pub static ITERATIONS_FACTOR_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.graphviz.iterationsFactor"));

pub static LABEL_ANGLE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.graphviz.labelAngle", -25.0));

pub static LABEL_DISTANCE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.graphviz.labelDistance", 1.0));

pub static LAYER_SPACING_FACTOR_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.graphviz.layerSpacingFactor", 1.0));

pub static MAXITER_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.graphviz.maxiter"));

pub static NEATO_MODEL_PROPERTY: LazyLock<Property<NeatoModel>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.graphviz.neatoModel", NeatoModel::Shortpath)
});

pub static OVERLAP_MODE_PROPERTY: LazyLock<Property<OverlapMode>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.graphviz.overlapMode", OverlapMode::Prism)
});

impl GraphvizOptions {
    pub const ADAPT_PORT_POSITIONS: &'static LazyLock<Property<bool>> =
        &ADAPT_PORT_POSITIONS_PROPERTY;
    pub const CONCENTRATE: &'static LazyLock<Property<bool>> = &CONCENTRATE_PROPERTY;
    pub const EPSILON: &'static LazyLock<Property<f64>> = &EPSILON_PROPERTY;
    pub const ITERATIONS_FACTOR: &'static LazyLock<Property<f64>> = &ITERATIONS_FACTOR_PROPERTY;
    pub const LABEL_ANGLE: &'static LazyLock<Property<f64>> = &LABEL_ANGLE_PROPERTY;
    pub const LABEL_DISTANCE: &'static LazyLock<Property<f64>> = &LABEL_DISTANCE_PROPERTY;
    pub const LAYER_SPACING_FACTOR: &'static LazyLock<Property<f64>> =
        &LAYER_SPACING_FACTOR_PROPERTY;
    pub const MAXITER: &'static LazyLock<Property<i32>> = &MAXITER_PROPERTY;
    pub const NEATO_MODEL: &'static LazyLock<Property<NeatoModel>> = &NEATO_MODEL_PROPERTY;
    pub const OVERLAP_MODE: &'static LazyLock<Property<OverlapMode>> = &OVERLAP_MODE_PROPERTY;

    pub const PADDING: &'static LazyLock<
        Property<org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding>,
    > = CoreOptions::PADDING;
    pub const DIRECTION: &'static LazyLock<
        Property<org_eclipse_elk_core::org::eclipse::elk::core::options::Direction>,
    > = CoreOptions::DIRECTION;
    pub const SPACING_NODE_NODE: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_NODE_NODE;
    pub const SPACING_EDGE_LABEL: &'static LazyLock<Property<f64>> =
        CoreOptions::SPACING_EDGE_LABEL;
    pub const NODE_SIZE_CONSTRAINTS: &'static LazyLock<Property<org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet<org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint>>> =
        CoreOptions::NODE_SIZE_CONSTRAINTS;
    pub const NODE_SIZE_OPTIONS: &'static LazyLock<
        Property<
            org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet<
                org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions,
            >,
        >,
    > = CoreOptions::NODE_SIZE_OPTIONS;
    pub const EDGE_ROUTING: &'static LazyLock<
        Property<org_eclipse_elk_core::org::eclipse::elk::core::options::EdgeRouting>,
    > = CoreOptions::EDGE_ROUTING;
    pub const DEBUG_MODE: &'static LazyLock<Property<bool>> = CoreOptions::DEBUG_MODE;
    pub const HIERARCHY_HANDLING: &'static LazyLock<Property<org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling>> =
        CoreOptions::HIERARCHY_HANDLING;
    pub const RANDOM_SEED: &'static LazyLock<Property<i32>> = CoreOptions::RANDOM_SEED;
    pub const INTERACTIVE: &'static LazyLock<Property<bool>> = CoreOptions::INTERACTIVE;
    pub const SEPARATE_CONNECTED_COMPONENTS: &'static LazyLock<Property<bool>> =
        CoreOptions::SEPARATE_CONNECTED_COMPONENTS;
}
