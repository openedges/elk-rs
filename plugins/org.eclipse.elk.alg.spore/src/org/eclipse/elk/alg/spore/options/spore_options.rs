use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;

use super::{CompactionStrategy, RootSelection, SpanningTreeCostFunction, StructureExtractionStrategy, TreeConstructionStrategy};

pub static UNDERLYING_LAYOUT_ALGORITHM_PROPERTY: LazyLock<Property<String>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.underlyingLayoutAlgorithm"));

pub static STRUCTURE_EXTRACTION_STRATEGY_PROPERTY: LazyLock<Property<StructureExtractionStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.structure.structureExtractionStrategy",
            StructureExtractionStrategy::DelaunayTriangulation,
        )
    });

pub static PROCESSING_ORDER_TREE_CONSTRUCTION_PROPERTY: LazyLock<Property<TreeConstructionStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.processingOrder.treeConstruction",
            TreeConstructionStrategy::MinimumSpanningTree,
        )
    });

pub static PROCESSING_ORDER_SPANNING_TREE_COST_FUNCTION_PROPERTY:
    LazyLock<Property<SpanningTreeCostFunction>> = LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.processingOrder.spanningTreeCostFunction",
            SpanningTreeCostFunction::CircleUnderlap,
        )
    });

pub static PROCESSING_ORDER_PREFERRED_ROOT_PROPERTY: LazyLock<Property<String>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.processingOrder.preferredRoot"));

pub static PROCESSING_ORDER_ROOT_SELECTION_PROPERTY: LazyLock<Property<RootSelection>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.processingOrder.rootSelection",
            RootSelection::CenterNode,
        )
    });

pub static COMPACTION_COMPACTION_STRATEGY_PROPERTY: LazyLock<Property<CompactionStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.compaction.compactionStrategy",
            CompactionStrategy::DepthFirst,
        )
    });

pub static COMPACTION_ORTHOGONAL_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.compaction.orthogonal", false));

pub static OVERLAP_REMOVAL_MAX_ITERATIONS_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.overlapRemoval.maxIterations", 64));

pub static OVERLAP_REMOVAL_RUN_SCANLINE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.overlapRemoval.runScanline", true));

pub struct SporeCommonOptions;

impl SporeCommonOptions {
    pub const UNDERLYING_LAYOUT_ALGORITHM: &'static LazyLock<Property<String>> =
        &UNDERLYING_LAYOUT_ALGORITHM_PROPERTY;
    pub const STRUCTURE_EXTRACTION_STRATEGY: &'static LazyLock<Property<StructureExtractionStrategy>> =
        &STRUCTURE_EXTRACTION_STRATEGY_PROPERTY;
    pub const PROCESSING_ORDER_TREE_CONSTRUCTION: &'static LazyLock<Property<TreeConstructionStrategy>> =
        &PROCESSING_ORDER_TREE_CONSTRUCTION_PROPERTY;
    pub const PROCESSING_ORDER_SPANNING_TREE_COST_FUNCTION:
        &'static LazyLock<Property<SpanningTreeCostFunction>> =
        &PROCESSING_ORDER_SPANNING_TREE_COST_FUNCTION_PROPERTY;
    pub const PROCESSING_ORDER_PREFERRED_ROOT: &'static LazyLock<Property<String>> =
        &PROCESSING_ORDER_PREFERRED_ROOT_PROPERTY;
    pub const PROCESSING_ORDER_ROOT_SELECTION: &'static LazyLock<Property<RootSelection>> =
        &PROCESSING_ORDER_ROOT_SELECTION_PROPERTY;
    pub const COMPACTION_COMPACTION_STRATEGY: &'static LazyLock<Property<CompactionStrategy>> =
        &COMPACTION_COMPACTION_STRATEGY_PROPERTY;
    pub const COMPACTION_ORTHOGONAL: &'static LazyLock<Property<bool>> =
        &COMPACTION_ORTHOGONAL_PROPERTY;
    pub const OVERLAP_REMOVAL_MAX_ITERATIONS: &'static LazyLock<Property<i32>> =
        &OVERLAP_REMOVAL_MAX_ITERATIONS_PROPERTY;
    pub const OVERLAP_REMOVAL_RUN_SCANLINE: &'static LazyLock<Property<bool>> =
        &OVERLAP_REMOVAL_RUN_SCANLINE_PROPERTY;

    pub const PADDING: &'static LazyLock<Property<ElkPadding>> = CoreOptions::PADDING;
    pub const SPACING_NODE_NODE: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_NODE_NODE;
    pub const DEBUG_MODE: &'static LazyLock<Property<bool>> = CoreOptions::DEBUG_MODE;
}
