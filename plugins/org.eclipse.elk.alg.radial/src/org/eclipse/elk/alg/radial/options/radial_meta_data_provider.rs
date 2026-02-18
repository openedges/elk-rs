use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::sync::{Arc, LazyLock, OnceLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutAlgorithmData, LayoutMetaDataRegistry, LayoutOptionData,
    LayoutOptionTarget, LayoutOptionType, LayoutOptionVisibility,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;

use crate::org::eclipse::elk::alg::radial::options::{
    AnnulusWedgeCriteria, CompactionStrategy, RadialOptions, RadialTranslationStrategy,
    SortingStrategy,
};

pub struct RadialMetaDataProvider;

impl ILayoutMetaDataProvider for RadialMetaDataProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        init_reflect();
        register_algorithm(registry);
        register_options(registry);
        register_supports(registry);
    }
}

const TARGET_PARENTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Parents];
const TARGET_NODES: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Nodes];

fn register_algorithm(registry: &mut dyn LayoutMetaDataRegistry) {
    let mut data = LayoutAlgorithmData::new(RadialOptions::ALGORITHM_ID);
    data.set_name("ELK Radial")
        .set_description(concat!(
            "A radial layout provider which is based on the algorithm of Peter Eades published in ",
            "\"Drawing free trees.\", published by International Institute for Advanced Study of Social ",
            "Information Science, Fujitsu Limited in 1991. The radial layouter takes a tree and places ",
            "the nodes in radial order around the root. The nodes of the same tree level are placed ",
            "on the same radius."
        ))
        .set_preview_image_path(Some("images/radial_layout.png"))
        .set_category_id(Some("org.eclipse.elk.radial"))
        .set_defining_bundle_id(Some("org.eclipse.elk.alg.radial"));
    registry.register_algorithm(data);
}

fn register_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        RadialOptions::CENTER_ON_ROOT,
        LayoutOptionType::Boolean,
        "Center On Root",
        "Centers the layout on the root of the tree so that the central node is also the center node of the final layout.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        RadialOptions::ORDER_ID,
        LayoutOptionType::Int,
        "Order ID",
        "The id can be used to define an order for nodes of one radius.",
        &TARGET_NODES,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        RadialOptions::RADIUS,
        LayoutOptionType::Double,
        "Radius",
        "The radius option can be used to set the initial radius for the radial layouter.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );

    register_option(
        registry,
        RadialOptions::ROTATE,
        LayoutOptionType::Boolean,
        "Rotate",
        "Determines whether a rotation of the layout should be performed.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        RadialOptions::ROTATION_TARGET_ANGLE,
        LayoutOptionType::Double,
        "Target Angle",
        "The angle in radians that the layout should be rotated to after layout.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("rotation"),
        None,
    );
    register_option(
        registry,
        RadialOptions::ROTATION_COMPUTE_ADDITIONAL_WEDGE_SPACE,
        LayoutOptionType::Boolean,
        "Additional Wedge Space",
        "If set to true, modifies the target angle by rotating further such that space is left for an edge to pass in between the nodes.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("rotation"),
        None,
    );
    register_option(
        registry,
        RadialOptions::ROTATION_OUTGOING_EDGE_ANGLES,
        LayoutOptionType::Boolean,
        "Outgoing Edge Angles",
        "Calculate the required angle of connected nodes to leave space for an incoming edge.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("rotation"),
        None,
    );

    register_option(
        registry,
        RadialOptions::COMPACTOR,
        LayoutOptionType::Enum,
        "Compaction",
        "Determine how compaction on the graph is done.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        RadialOptions::COMPACTION_STEP_SIZE,
        LayoutOptionType::Int,
        "Compaction Step Size",
        "Determine the size of steps with which the compaction is done.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        Some(Arc::new(0_i32)),
    );

    register_option(
        registry,
        RadialOptions::SORTER,
        LayoutOptionType::Enum,
        "Sorter",
        "Sort the nodes per radius according to the sorting algorithm.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        RadialOptions::WEDGE_CRITERIA,
        LayoutOptionType::Enum,
        "Annulus Wedge Criteria",
        "Determine how the wedge for the node placement is calculated.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        RadialOptions::OPTIMIZATION_CRITERIA,
        LayoutOptionType::Enum,
        "Translation Optimization",
        "Find the optimal translation of the nodes of the first radii according to this criteria.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
}

fn register_supports(registry: &mut dyn LayoutMetaDataRegistry) {
    let algo = RadialOptions::ALGORITHM_ID;
    registry.add_option_support(algo, CoreOptions::POSITION.id(), None);
    registry.add_option_support(algo, CoreOptions::SPACING_NODE_NODE.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_CONSTRAINTS.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_MINIMUM.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_OPTIONS.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_LABELS_PLACEMENT.id(), None);
    registry.add_option_support(algo, CoreOptions::OMIT_NODE_MICRO_LAYOUT.id(), None);
    registry.add_option_support(algo, CoreOptions::PORT_LABELS_PLACEMENT.id(), None);

    registry.add_option_support(algo, RadialOptions::COMPACTION_STEP_SIZE.id(), None);
    registry.add_option_support(algo, RadialOptions::COMPACTOR.id(), None);
    registry.add_option_support(algo, RadialOptions::ROTATE.id(), None);
    registry.add_option_support(algo, RadialOptions::ROTATION_TARGET_ANGLE.id(), None);
    registry.add_option_support(
        algo,
        RadialOptions::ROTATION_COMPUTE_ADDITIONAL_WEDGE_SPACE.id(),
        None,
    );
    registry.add_option_support(
        algo,
        RadialOptions::ROTATION_OUTGOING_EDGE_ANGLES.id(),
        None,
    );
    registry.add_option_support(algo, RadialOptions::OPTIMIZATION_CRITERIA.id(), None);
    registry.add_option_support(algo, RadialOptions::ORDER_ID.id(), None);
    registry.add_option_support(algo, RadialOptions::RADIUS.id(), None);
    registry.add_option_support(algo, RadialOptions::SORTER.id(), None);
    registry.add_option_support(algo, RadialOptions::WEDGE_CRITERIA.id(), None);
    registry.add_option_support(algo, RadialOptions::CENTER_ON_ROOT.id(), None);
}

#[allow(clippy::too_many_arguments)]
fn register_option<T: Clone + Send + Sync + 'static>(
    registry: &mut dyn LayoutMetaDataRegistry,
    property: &'static LazyLock<Property<T>>,
    option_type: LayoutOptionType,
    name: &'static str,
    description: &'static str,
    targets: &'static [LayoutOptionTarget],
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
    if lower_bound.is_some() {
        builder = builder.lower_bound(lower_bound);
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
            Some(|| CompactionStrategy::None),
            Some(|v: &CompactionStrategy| *v),
        );
        ElkReflect::register(
            Some(|| SortingStrategy::None),
            Some(|v: &SortingStrategy| *v),
        );
        ElkReflect::register(
            Some(|| AnnulusWedgeCriteria::NodeSize),
            Some(|v: &AnnulusWedgeCriteria| *v),
        );
        ElkReflect::register(
            Some(|| RadialTranslationStrategy::None),
            Some(|v: &RadialTranslationStrategy| *v),
        );
    });
}
