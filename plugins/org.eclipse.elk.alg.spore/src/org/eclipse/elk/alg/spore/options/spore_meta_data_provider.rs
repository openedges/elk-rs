use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::sync::{Arc, LazyLock, OnceLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutAlgorithmData, LayoutMetaDataRegistry, LayoutOptionData,
    LayoutOptionTarget, LayoutOptionType, LayoutOptionVisibility,
};
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;

use super::{
    CompactionStrategy, OverlapRemovalStrategy, RootSelection, SpanningTreeCostFunction,
    SporeCompactionOptions, SporeOverlapRemovalOptions, StructureExtractionStrategy,
    TreeConstructionStrategy,
};

pub struct SporeMetaDataProvider;

impl ILayoutMetaDataProvider for SporeMetaDataProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        init_reflect();
        register_algorithms(registry);
        register_options(registry);
        register_supports(registry);
    }
}

const TARGET_PARENTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Parents];

fn register_algorithms(registry: &mut dyn LayoutMetaDataRegistry) {
    let mut overlap = LayoutAlgorithmData::new(SporeOverlapRemovalOptions::ALGORITHM_ID);
    overlap
        .set_name("ELK SPOrE Overlap Removal")
        .set_description(concat!(
            "A node overlap removal algorithm proposed by Nachmanson et al. in \"Node overlap removal ",
            "by growing a tree\"."
        ))
        .set_preview_image_path(Some("images/overlap-removal.png"));
    registry.register_algorithm(overlap);

    let mut compaction = LayoutAlgorithmData::new(SporeCompactionOptions::ALGORITHM_ID);
    compaction
        .set_name("ELK SPOrE Compaction")
        .set_description(concat!(
            "ShrinkTree is a compaction algorithm that maintains the topology of a layout. ",
            "The relocation of diagram elements is based on contracting a spanning tree."
        ))
        .set_preview_image_path(Some("images/compaction-example.png"));
    registry.register_algorithm(compaction);
}

fn register_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        SporeCompactionOptions::UNDERLYING_LAYOUT_ALGORITHM,
        LayoutOptionType::String,
        "Underlying Layout Algorithm",
        concat!(
            "A layout algorithm that is applied to the graph before it is compacted. ",
            "If this is null, nothing is applied before compaction."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );

    register_option(
        registry,
        SporeCompactionOptions::STRUCTURE_EXTRACTION_STRATEGY,
        LayoutOptionType::Enum,
        "Structure Extraction Strategy",
        "This option defines what kind of triangulation or other partitioning of the plane is applied to the vertices.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("structure"),
        None,
    );

    register_option(
        registry,
        SporeCompactionOptions::PROCESSING_ORDER_TREE_CONSTRUCTION,
        LayoutOptionType::Enum,
        "Tree Construction Strategy",
        "Whether a minimum spanning tree or a maximum spanning tree should be constructed.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("processingOrder"),
        None,
    );

    register_option(
        registry,
        SporeCompactionOptions::PROCESSING_ORDER_SPANNING_TREE_COST_FUNCTION,
        LayoutOptionType::Enum,
        "Cost Function for Spanning Tree",
        "The cost function is used in the creation of the spanning tree.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("processingOrder"),
        None,
    );

    register_option(
        registry,
        SporeCompactionOptions::PROCESSING_ORDER_PREFERRED_ROOT,
        LayoutOptionType::String,
        "Root node for spanning tree construction",
        concat!(
            "The identifier of the node that is preferred as the root of the spanning tree. ",
            "If this is null, the first node is chosen."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("processingOrder"),
        None,
    );

    register_option(
        registry,
        SporeCompactionOptions::PROCESSING_ORDER_ROOT_SELECTION,
        LayoutOptionType::Enum,
        "Root selection for spanning tree",
        "This sets the method used to select a root node for the construction of a spanning tree",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("processingOrder"),
        None,
    );

    register_option(
        registry,
        SporeCompactionOptions::COMPACTION_COMPACTION_STRATEGY,
        LayoutOptionType::Enum,
        "Compaction Strategy",
        "This option defines how the compaction is applied.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("compaction"),
        None,
    );

    register_option(
        registry,
        SporeCompactionOptions::COMPACTION_ORTHOGONAL,
        LayoutOptionType::Boolean,
        "Orthogonal Compaction",
        "Restricts the translation of nodes to orthogonal directions in the compaction phase.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("compaction"),
        None,
    );

    register_option(
        registry,
        SporeOverlapRemovalOptions::OVERLAP_REMOVAL_MAX_ITERATIONS,
        LayoutOptionType::Int,
        "Upper limit for iterations of overlap removal",
        "",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("overlapRemoval"),
        None,
    );

    register_option(
        registry,
        SporeOverlapRemovalOptions::OVERLAP_REMOVAL_RUN_SCANLINE,
        LayoutOptionType::Boolean,
        "Whether to run a supplementary scanline overlap check.",
        "",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("overlapRemoval"),
        None,
    );
}

fn register_supports(registry: &mut dyn LayoutMetaDataRegistry) {
    fn arc_any<T: Any + Send + Sync>(value: T) -> Option<Arc<dyn Any + Send + Sync>> {
        Some(Arc::new(value))
    }

    let overlap = SporeOverlapRemovalOptions::ALGORITHM_ID;
    registry.add_option_support(overlap, SporeOverlapRemovalOptions::UNDERLYING_LAYOUT_ALGORITHM.id(), None);
    registry.add_option_support(overlap, CoreOptions::PADDING.id(), arc_any(ElkPadding::with_any(8.0)));
    registry.add_option_support(overlap, CoreOptions::SPACING_NODE_NODE.id(), arc_any(8.0_f64));
    registry.add_option_support(overlap, SporeOverlapRemovalOptions::STRUCTURE_EXTRACTION_STRATEGY.id(), None);
    registry.add_option_support(overlap, SporeOverlapRemovalOptions::OVERLAP_REMOVAL_MAX_ITERATIONS.id(), None);
    registry.add_option_support(overlap, SporeOverlapRemovalOptions::OVERLAP_REMOVAL_RUN_SCANLINE.id(), None);
    registry.add_option_support(overlap, CoreOptions::DEBUG_MODE.id(), arc_any(false));

    let compaction = SporeCompactionOptions::ALGORITHM_ID;
    registry.add_option_support(compaction, SporeCompactionOptions::UNDERLYING_LAYOUT_ALGORITHM.id(), None);
    registry.add_option_support(compaction, SporeCompactionOptions::PROCESSING_ORDER_TREE_CONSTRUCTION.id(), None);
    registry.add_option_support(
        compaction,
        SporeCompactionOptions::PROCESSING_ORDER_SPANNING_TREE_COST_FUNCTION.id(),
        None,
    );
    registry.add_option_support(compaction, SporeCompactionOptions::PROCESSING_ORDER_PREFERRED_ROOT.id(), None);
    registry.add_option_support(compaction, SporeCompactionOptions::PROCESSING_ORDER_ROOT_SELECTION.id(), None);
    registry.add_option_support(compaction, CoreOptions::PADDING.id(), arc_any(ElkPadding::with_any(8.0)));
    registry.add_option_support(compaction, CoreOptions::SPACING_NODE_NODE.id(), arc_any(8.0_f64));
    registry.add_option_support(compaction, SporeCompactionOptions::STRUCTURE_EXTRACTION_STRATEGY.id(), None);
    registry.add_option_support(compaction, SporeCompactionOptions::COMPACTION_COMPACTION_STRATEGY.id(), None);
    registry.add_option_support(compaction, SporeCompactionOptions::COMPACTION_ORTHOGONAL.id(), None);
    registry.add_option_support(compaction, CoreOptions::DEBUG_MODE.id(), arc_any(false));
}

#[allow(clippy::too_many_arguments)]
fn register_option<T: Clone + Send + Sync + 'static>(
    registry: &mut dyn LayoutMetaDataRegistry,
    property: &'static LazyLock<Property<T>>,
    option_type: LayoutOptionType,
    name: &'static str,
    description: &'static str,
    targets: &[LayoutOptionTarget],
    visibility: LayoutOptionVisibility,
    group: Option<&'static str>,
    lower_bound: Option<Arc<dyn Any + Send + Sync>>,
) {
    let default_value = property_default_any(property);
    let mut builder = LayoutOptionData::builder()
        .id(property.id())
        .option_type(option_type)
        .default_value(default_value)
        .name(name)
        .description(description)
        .targets(targets.iter().copied().collect::<HashSet<_>>())
        .visibility(visibility)
        .value_type_id(TypeId::of::<T>());
    if let Some(group) = group {
        builder = builder.group(group);
    }
    if let Some(lower_bound) = lower_bound {
        builder = builder.lower_bound(Some(lower_bound));
    }
    registry.register_option(builder.create());
}

fn property_default_any<T: Clone + Send + Sync + 'static>(
    property: &'static LazyLock<Property<T>>,
) -> Option<Arc<dyn Any + Send + Sync>> {
    if !property.is_cloneable() {
        return None;
    }
    property
        .get_default()
        .map(|value| Arc::new(value) as Arc<dyn Any + Send + Sync>)
}

fn init_reflect() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        ElkReflect::register(
            Some(|| StructureExtractionStrategy::DelaunayTriangulation),
            Some(|v: &StructureExtractionStrategy| *v),
        );
        ElkReflect::register(
            Some(|| TreeConstructionStrategy::MinimumSpanningTree),
            Some(|v: &TreeConstructionStrategy| *v),
        );
        ElkReflect::register(
            Some(|| CompactionStrategy::DepthFirst),
            Some(|v: &CompactionStrategy| *v),
        );
        ElkReflect::register(
            Some(|| RootSelection::CenterNode),
            Some(|v: &RootSelection| *v),
        );
        ElkReflect::register(
            Some(|| SpanningTreeCostFunction::CircleUnderlap),
            Some(|v: &SpanningTreeCostFunction| *v),
        );
        ElkReflect::register(
            Some(|| OverlapRemovalStrategy::GrowTree),
            Some(|v: &OverlapRemovalStrategy| *v),
        );
    });
}
