use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

pub struct LayeredOptions;

pub static SPACING_BASE_VALUE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.alg.layered.spacing.baseValue"));

pub static SPACING_EDGE_NODE_BETWEEN_LAYERS_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.spacing.edgeNodeBetweenLayers",
        10.0,
    )
});

pub static SPACING_EDGE_EDGE_BETWEEN_LAYERS_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.spacing.edgeEdgeBetweenLayers",
        10.0,
    )
});

pub static SPACING_NODE_NODE_BETWEEN_LAYERS_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.spacing.nodeNodeBetweenLayers",
        20.0,
    )
});

pub static PRIORITY_DIRECTION_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.layered.priority.direction", 0));

pub static PRIORITY_SHORTNESS_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.layered.priority.shortness", 0));

pub static PRIORITY_STRAIGHTNESS_PROPERTY: LazyLock<Property<i32>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.alg.layered.priority.straightness", 0)
});

impl LayeredOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.layered";

    pub const SPACING_BASE_VALUE: &'static LazyLock<Property<f64>> = &SPACING_BASE_VALUE_PROPERTY;
    pub const SPACING_EDGE_NODE_BETWEEN_LAYERS: &'static LazyLock<Property<f64>> =
        &SPACING_EDGE_NODE_BETWEEN_LAYERS_PROPERTY;
    pub const SPACING_EDGE_EDGE_BETWEEN_LAYERS: &'static LazyLock<Property<f64>> =
        &SPACING_EDGE_EDGE_BETWEEN_LAYERS_PROPERTY;
    pub const SPACING_NODE_NODE_BETWEEN_LAYERS: &'static LazyLock<Property<f64>> =
        &SPACING_NODE_NODE_BETWEEN_LAYERS_PROPERTY;

    pub const PRIORITY_DIRECTION: &'static LazyLock<Property<i32>> = &PRIORITY_DIRECTION_PROPERTY;
    pub const PRIORITY_SHORTNESS: &'static LazyLock<Property<i32>> = &PRIORITY_SHORTNESS_PROPERTY;
    pub const PRIORITY_STRAIGHTNESS: &'static LazyLock<Property<i32>> = &PRIORITY_STRAIGHTNESS_PROPERTY;
}
