use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::sync::{Arc, LazyLock, OnceLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutMetaDataRegistry, LayoutOptionData, LayoutOptionTarget,
    LayoutOptionType, LayoutOptionVisibility,
};

use super::{HighLevelSortingCriterion, LowLevelSortingCriterion, TraversalStrategy};

pub struct PolyominoOptions;

pub static POLYOMINO_TRAVERSAL_STRATEGY_PROPERTY: LazyLock<Property<TraversalStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.polyomino.traversalStrategy",
            TraversalStrategy::QuadrantsLineByLine,
        )
    });

pub static POLYOMINO_LOW_LEVEL_SORT_PROPERTY: LazyLock<Property<LowLevelSortingCriterion>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.polyomino.lowLevelSort",
            LowLevelSortingCriterion::BySizeAndShape,
        )
    });

pub static POLYOMINO_HIGH_LEVEL_SORT_PROPERTY: LazyLock<Property<HighLevelSortingCriterion>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.polyomino.highLevelSort",
            HighLevelSortingCriterion::NumOfExternalSidesThanNumOfExtensionsLast,
        )
    });

pub static POLYOMINO_FILL_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.polyomino.fill", true));

impl PolyominoOptions {
    pub const POLYOMINO_TRAVERSAL_STRATEGY: &'static LazyLock<Property<TraversalStrategy>> =
        &POLYOMINO_TRAVERSAL_STRATEGY_PROPERTY;
    pub const POLYOMINO_LOW_LEVEL_SORT: &'static LazyLock<Property<LowLevelSortingCriterion>> =
        &POLYOMINO_LOW_LEVEL_SORT_PROPERTY;
    pub const POLYOMINO_HIGH_LEVEL_SORT: &'static LazyLock<Property<HighLevelSortingCriterion>> =
        &POLYOMINO_HIGH_LEVEL_SORT_PROPERTY;
    pub const POLYOMINO_FILL: &'static LazyLock<Property<bool>> = &POLYOMINO_FILL_PROPERTY;
}

impl ILayoutMetaDataProvider for PolyominoOptions {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        init_reflect();
        register_options(registry);
    }
}

const TARGET_PARENTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Parents];

fn register_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        PolyominoOptions::POLYOMINO_TRAVERSAL_STRATEGY,
        LayoutOptionType::Enum,
        "Polyomino Traversal Strategy",
        "Traversal strategy for trying different candidate positions for polyominoes.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("polyomino"),
    );
    register_option(
        registry,
        PolyominoOptions::POLYOMINO_LOW_LEVEL_SORT,
        LayoutOptionType::Enum,
        "Polyomino Secondary Sorting Criterion",
        concat!(
            "Possible secondary sorting criteria for the processing order of polyominoes. ",
            "They are used when polyominoes are equal according to the primary sorting criterion HighLevelSortingCriterion."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("polyomino"),
    );
    register_option(
        registry,
        PolyominoOptions::POLYOMINO_HIGH_LEVEL_SORT,
        LayoutOptionType::Enum,
        "Polyomino Primary Sorting Criterion",
        "Possible primary sorting criteria for the processing order of polyominoes.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("polyomino"),
    );
    register_option(
        registry,
        PolyominoOptions::POLYOMINO_FILL,
        LayoutOptionType::Boolean,
        "Fill Polyominoes",
        concat!(
            "Use the Profile Fill algorithm to fill polyominoes to prevent small polyominoes ",
            "from being placed inside of big polyominoes with large holes. Might increase packing area."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("polyomino"),
    );
}

fn register_option<T: Clone + Send + Sync + 'static>(
    registry: &mut dyn LayoutMetaDataRegistry,
    property: &'static LazyLock<Property<T>>,
    option_type: LayoutOptionType,
    name: &'static str,
    description: &'static str,
    targets: &[LayoutOptionTarget],
    visibility: LayoutOptionVisibility,
    group: Option<&'static str>,
) {
    let mut targets_set = HashSet::new();
    for target in targets {
        targets_set.insert(*target);
    }
    let mut builder = LayoutOptionData::builder()
        .id(property.id())
        .option_type(option_type)
        .name(name)
        .description(description)
        .targets(targets_set)
        .visibility(visibility)
        .default_value(property_default_any(property))
        .value_type_id(TypeId::of::<T>());
    if let Some(group) = group {
        builder = builder.group(group);
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
            Some(|| TraversalStrategy::QuadrantsLineByLine),
            Some(|v: &TraversalStrategy| *v),
        );
        ElkReflect::register(
            Some(|| LowLevelSortingCriterion::BySizeAndShape),
            Some(|v: &LowLevelSortingCriterion| *v),
        );
        ElkReflect::register(
            Some(|| HighLevelSortingCriterion::NumOfExternalSidesThanNumOfExtensionsLast),
            Some(|v: &HighLevelSortingCriterion| *v),
        );
    });
}
