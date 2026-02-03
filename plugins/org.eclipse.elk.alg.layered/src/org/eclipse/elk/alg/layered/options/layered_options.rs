use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use super::{CuttingStrategy, LayerUnzippingStrategy, ValidifyStrategy, WrappingStrategy};

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

pub static WRAPPING_STRATEGY_PROPERTY: LazyLock<Property<WrappingStrategy>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.layered.wrapping.strategy", WrappingStrategy::Off));

pub static WRAPPING_ADDITIONAL_EDGE_SPACING_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.wrapping.additionalEdgeSpacing",
        10.0,
    )
});

pub static WRAPPING_CORRECTION_FACTOR_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.alg.layered.wrapping.correctionFactor", 1.0)
});

pub static WRAPPING_CUTTING_STRATEGY_PROPERTY: LazyLock<Property<CuttingStrategy>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.wrapping.cutting.strategy",
        CuttingStrategy::Msd,
    )
});

pub static WRAPPING_CUTTING_CUTS_PROPERTY: LazyLock<Property<Vec<i32>>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.alg.layered.wrapping.cutting.cuts"));

pub static WRAPPING_CUTTING_MSD_FREEDOM_PROPERTY: LazyLock<Property<i32>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.alg.layered.wrapping.cutting.msd.freedom", 1)
});

pub static WRAPPING_VALIDIFY_STRATEGY_PROPERTY: LazyLock<Property<ValidifyStrategy>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.wrapping.validify.strategy",
        ValidifyStrategy::Greedy,
    )
});

pub static WRAPPING_VALIDIFY_FORBIDDEN_INDICES_PROPERTY: LazyLock<Property<Vec<i32>>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.alg.layered.wrapping.validify.forbiddenIndices"));

pub static WRAPPING_MULTI_EDGE_IMPROVE_CUTS_PROPERTY: LazyLock<Property<bool>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.alg.layered.wrapping.multiEdge.improveCuts", true)
});

pub static WRAPPING_MULTI_EDGE_DISTANCE_PENALTY_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.wrapping.multiEdge.distancePenalty",
        2.0,
    )
});

pub static WRAPPING_MULTI_EDGE_IMPROVE_WRAPPED_EDGES_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.wrapping.multiEdge.improveWrappedEdges",
            true,
        )
    });

pub static LAYER_UNZIPPING_STRATEGY_PROPERTY: LazyLock<Property<LayerUnzippingStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.layerUnzipping.strategy",
            LayerUnzippingStrategy::None,
        )
    });

pub static LAYER_UNZIPPING_MINIMIZE_EDGE_LENGTH_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.layerUnzipping.minimizeEdgeLength",
            false,
        )
    });

pub static LAYER_UNZIPPING_LAYER_SPLIT_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.layered.layerUnzipping.layerSplit", 2));

pub static LAYER_UNZIPPING_RESET_ON_LONG_EDGES_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.layerUnzipping.resetOnLongEdges",
            true,
        )
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

    pub const WRAPPING_STRATEGY: &'static LazyLock<Property<WrappingStrategy>> =
        &WRAPPING_STRATEGY_PROPERTY;
    pub const WRAPPING_ADDITIONAL_EDGE_SPACING: &'static LazyLock<Property<f64>> =
        &WRAPPING_ADDITIONAL_EDGE_SPACING_PROPERTY;
    pub const WRAPPING_CORRECTION_FACTOR: &'static LazyLock<Property<f64>> =
        &WRAPPING_CORRECTION_FACTOR_PROPERTY;
    pub const WRAPPING_CUTTING_STRATEGY: &'static LazyLock<Property<CuttingStrategy>> =
        &WRAPPING_CUTTING_STRATEGY_PROPERTY;
    pub const WRAPPING_CUTTING_CUTS: &'static LazyLock<Property<Vec<i32>>> =
        &WRAPPING_CUTTING_CUTS_PROPERTY;
    pub const WRAPPING_CUTTING_MSD_FREEDOM: &'static LazyLock<Property<i32>> =
        &WRAPPING_CUTTING_MSD_FREEDOM_PROPERTY;
    pub const WRAPPING_VALIDIFY_STRATEGY: &'static LazyLock<Property<ValidifyStrategy>> =
        &WRAPPING_VALIDIFY_STRATEGY_PROPERTY;
    pub const WRAPPING_VALIDIFY_FORBIDDEN_INDICES: &'static LazyLock<Property<Vec<i32>>> =
        &WRAPPING_VALIDIFY_FORBIDDEN_INDICES_PROPERTY;
    pub const WRAPPING_MULTI_EDGE_IMPROVE_CUTS: &'static LazyLock<Property<bool>> =
        &WRAPPING_MULTI_EDGE_IMPROVE_CUTS_PROPERTY;
    pub const WRAPPING_MULTI_EDGE_DISTANCE_PENALTY: &'static LazyLock<Property<f64>> =
        &WRAPPING_MULTI_EDGE_DISTANCE_PENALTY_PROPERTY;
    pub const WRAPPING_MULTI_EDGE_IMPROVE_WRAPPED_EDGES: &'static LazyLock<Property<bool>> =
        &WRAPPING_MULTI_EDGE_IMPROVE_WRAPPED_EDGES_PROPERTY;

    pub const LAYER_UNZIPPING_STRATEGY: &'static LazyLock<Property<LayerUnzippingStrategy>> =
        &LAYER_UNZIPPING_STRATEGY_PROPERTY;
    pub const LAYER_UNZIPPING_MINIMIZE_EDGE_LENGTH: &'static LazyLock<Property<bool>> =
        &LAYER_UNZIPPING_MINIMIZE_EDGE_LENGTH_PROPERTY;
    pub const LAYER_UNZIPPING_LAYER_SPLIT: &'static LazyLock<Property<i32>> =
        &LAYER_UNZIPPING_LAYER_SPLIT_PROPERTY;
    pub const LAYER_UNZIPPING_RESET_ON_LONG_EDGES: &'static LazyLock<Property<bool>> =
        &LAYER_UNZIPPING_RESET_ON_LONG_EDGES_PROPERTY;
}
