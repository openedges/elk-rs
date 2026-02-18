use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;

use super::spore_options::SporeCommonOptions;
use super::{
    CompactionStrategy, RootSelection, SpanningTreeCostFunction, StructureExtractionStrategy,
    TreeConstructionStrategy,
};

pub struct SporeCompactionOptions;

impl SporeCompactionOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.sporeCompaction";

    pub const UNDERLYING_LAYOUT_ALGORITHM: &'static LazyLock<Property<String>> =
        SporeCommonOptions::UNDERLYING_LAYOUT_ALGORITHM;
    pub const STRUCTURE_EXTRACTION_STRATEGY: &'static LazyLock<
        Property<StructureExtractionStrategy>,
    > = SporeCommonOptions::STRUCTURE_EXTRACTION_STRATEGY;
    pub const PROCESSING_ORDER_TREE_CONSTRUCTION: &'static LazyLock<
        Property<TreeConstructionStrategy>,
    > = SporeCommonOptions::PROCESSING_ORDER_TREE_CONSTRUCTION;
    pub const PROCESSING_ORDER_SPANNING_TREE_COST_FUNCTION: &'static LazyLock<
        Property<SpanningTreeCostFunction>,
    > = SporeCommonOptions::PROCESSING_ORDER_SPANNING_TREE_COST_FUNCTION;
    pub const PROCESSING_ORDER_PREFERRED_ROOT: &'static LazyLock<Property<String>> =
        SporeCommonOptions::PROCESSING_ORDER_PREFERRED_ROOT;
    pub const PROCESSING_ORDER_ROOT_SELECTION: &'static LazyLock<Property<RootSelection>> =
        SporeCommonOptions::PROCESSING_ORDER_ROOT_SELECTION;
    pub const COMPACTION_COMPACTION_STRATEGY: &'static LazyLock<Property<CompactionStrategy>> =
        SporeCommonOptions::COMPACTION_COMPACTION_STRATEGY;
    pub const COMPACTION_ORTHOGONAL: &'static LazyLock<Property<bool>> =
        SporeCommonOptions::COMPACTION_ORTHOGONAL;

    pub const PADDING: &'static LazyLock<Property<ElkPadding>> = SporeCommonOptions::PADDING;
    pub const SPACING_NODE_NODE: &'static LazyLock<Property<f64>> =
        SporeCommonOptions::SPACING_NODE_NODE;
    pub const DEBUG_MODE: &'static LazyLock<Property<bool>> = SporeCommonOptions::DEBUG_MODE;
}
