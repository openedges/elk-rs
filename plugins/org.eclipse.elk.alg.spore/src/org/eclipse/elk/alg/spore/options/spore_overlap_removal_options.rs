use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;

use super::spore_options::SporeCommonOptions;
use super::StructureExtractionStrategy;

pub struct SporeOverlapRemovalOptions;

impl SporeOverlapRemovalOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.sporeOverlap";

    pub const UNDERLYING_LAYOUT_ALGORITHM: &'static LazyLock<Property<String>> =
        SporeCommonOptions::UNDERLYING_LAYOUT_ALGORITHM;
    pub const STRUCTURE_EXTRACTION_STRATEGY: &'static LazyLock<
        Property<StructureExtractionStrategy>,
    > = SporeCommonOptions::STRUCTURE_EXTRACTION_STRATEGY;
    pub const OVERLAP_REMOVAL_MAX_ITERATIONS: &'static LazyLock<Property<i32>> =
        SporeCommonOptions::OVERLAP_REMOVAL_MAX_ITERATIONS;
    pub const OVERLAP_REMOVAL_RUN_SCANLINE: &'static LazyLock<Property<bool>> =
        SporeCommonOptions::OVERLAP_REMOVAL_RUN_SCANLINE;

    pub const PADDING: &'static LazyLock<Property<ElkPadding>> = SporeCommonOptions::PADDING;
    pub const SPACING_NODE_NODE: &'static LazyLock<Property<f64>> =
        SporeCommonOptions::SPACING_NODE_NODE;
    pub const DEBUG_MODE: &'static LazyLock<Property<bool>> = SporeCommonOptions::DEBUG_MODE;
}
