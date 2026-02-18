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
use org_eclipse_elk_core::org::eclipse::elk::core::options::content_alignment::ContentAlignment;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::node_label_placement::NodeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_label_placement::PortLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;

use crate::org::eclipse::elk::alg::rectpacking::options::{OptimizationGoal, RectPackingOptions};
use crate::org::eclipse::elk::alg::rectpacking::p1widthapproximation::WidthApproximationStrategy;
use crate::org::eclipse::elk::alg::rectpacking::p2packing::PackingStrategy;
use crate::org::eclipse::elk::alg::rectpacking::p3whitespaceelimination::WhiteSpaceEliminationStrategy;

pub struct RectPackingMetaDataProvider;

impl ILayoutMetaDataProvider for RectPackingMetaDataProvider {
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
    let mut data = LayoutAlgorithmData::new(RectPackingOptions::ALGORITHM_ID);
    data.set_name("ELK Rectangle Packing")
        .set_description(concat!(
            "Algorithm for packing of unconnected boxes, i.e. graphs without edges. The given order of the boxes is ",
            "always preserved and the main reading direction of the boxes is left to right. The algorithm is divided ",
            "into two phases. One phase approximates the width in which the rectangles can be placed. The next phase ",
            "places the rectangles in rows using the previously calculated width as bounding width and bundles ",
            "rectangles with a similar height in blocks. A compaction step reduces the size of the drawing. Finally, ",
            "the rectangles are expanded to fill their bounding box and eliminate empty unused spaces.",
        ))
        .set_defining_bundle_id(Some("org.eclipse.elk.alg.rectpacking"));
    registry.register_algorithm(data);
}

fn register_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        RectPackingOptions::WIDTH_APPROXIMATION_STRATEGY,
        LayoutOptionType::Enum,
        "Width Approximation Strategy",
        "Strategy for finding an initial width of the drawing.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("widthApproximation"),
        None,
    );
    register_option(
        registry,
        RectPackingOptions::WIDTH_APPROXIMATION_TARGET_WIDTH,
        LayoutOptionType::Double,
        "Target Width",
        "Target width for placement; padding is added externally.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("widthApproximation"),
        None,
    );
    register_option(
        registry,
        RectPackingOptions::WIDTH_APPROXIMATION_OPTIMIZATION_GOAL,
        LayoutOptionType::Enum,
        "Optimization Goal",
        "Optimization goal for approximation of the bounding box given by the first iteration.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("widthApproximation"),
        None,
    );
    register_option(
        registry,
        RectPackingOptions::WIDTH_APPROXIMATION_LAST_PLACE_SHIFT,
        LayoutOptionType::Boolean,
        "Shift Last Placed",
        "Allow a shift when placing behind or below the last placed rectangle.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("widthApproximation"),
        None,
    );

    register_option(
        registry,
        RectPackingOptions::PACKING_STRATEGY,
        LayoutOptionType::Enum,
        "Compaction Strategy",
        "Strategy for finding an initial placement on nodes.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("packing"),
        None,
    );
    register_option(
        registry,
        RectPackingOptions::PACKING_COMPACTION_ROW_HEIGHT_REEVALUATION,
        LayoutOptionType::Boolean,
        "Row Height Reevaluation",
        "Reevaluate row height during compaction if needed.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("packing.compaction"),
        None,
    );
    register_option(
        registry,
        RectPackingOptions::PACKING_COMPACTION_ITERATIONS,
        LayoutOptionType::Int,
        "Compaction iterations",
        "Number of compaction iterations.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("packing.compaction"),
        Some(Arc::new(1_i32)),
    );

    register_option(
        registry,
        RectPackingOptions::WHITE_SPACE_ELIMINATION_STRATEGY,
        LayoutOptionType::Enum,
        "White Space Approximation Strategy",
        "Strategy for expanding nodes to eliminate whitespace.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("whiteSpaceElimination"),
        None,
    );

    register_option(
        registry,
        RectPackingOptions::TRYBOX,
        LayoutOptionType::Boolean,
        "Try box layout first",
        "Check whether regions are stackable to use box layout.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );
    register_option(
        registry,
        RectPackingOptions::CURRENT_POSITION,
        LayoutOptionType::Int,
        "Current position of a node in the order of nodes",
        "Specifies the current position of a node.",
        &TARGET_NODES,
        LayoutOptionVisibility::Visible,
        None,
        Some(Arc::new(-1_i32)),
    );
    register_option(
        registry,
        RectPackingOptions::DESIRED_POSITION,
        LayoutOptionType::Int,
        "Desired index of node",
        "Desired position of a node in the ordering.",
        &TARGET_NODES,
        LayoutOptionVisibility::Advanced,
        None,
        Some(Arc::new(-1_i32)),
    );
    register_option(
        registry,
        RectPackingOptions::IN_NEW_ROW,
        LayoutOptionType::Boolean,
        "In new Row",
        "If true, this node begins a new row.",
        &TARGET_NODES,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        RectPackingOptions::ORDER_BY_SIZE,
        LayoutOptionType::Boolean,
        "Order nodes by height",
        "Sort nodes by height before layout.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
}

fn register_supports(registry: &mut dyn LayoutMetaDataRegistry) {
    fn arc_any<T: Any + Send + Sync>(value: T) -> Option<Arc<dyn Any + Send + Sync>> {
        Some(Arc::new(value))
    }

    let algo = RectPackingOptions::ALGORITHM_ID;
    registry.add_option_support(algo, CoreOptions::ASPECT_RATIO.id(), arc_any(1.3_f64));
    registry.add_option_support(
        algo,
        CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE.id(),
        arc_any(false),
    );
    registry.add_option_support(
        algo,
        CoreOptions::PADDING.id(),
        arc_any(ElkPadding::with_any(15.0)),
    );
    registry.add_option_support(algo, CoreOptions::SPACING_NODE_NODE.id(), arc_any(15.0_f64));
    registry.add_option_support(algo, CoreOptions::CONTENT_ALIGNMENT.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_CONSTRAINTS.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_MINIMUM.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_OPTIONS.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_LABELS_PLACEMENT.id(), None);
    registry.add_option_support(algo, CoreOptions::OMIT_NODE_MICRO_LAYOUT.id(), None);
    registry.add_option_support(algo, CoreOptions::PORT_LABELS_PLACEMENT.id(), None);
    registry.add_option_support(
        algo,
        RectPackingOptions::WIDTH_APPROXIMATION_OPTIMIZATION_GOAL.id(),
        None,
    );
    registry.add_option_support(
        algo,
        RectPackingOptions::WIDTH_APPROXIMATION_LAST_PLACE_SHIFT.id(),
        None,
    );
    registry.add_option_support(
        algo,
        RectPackingOptions::WIDTH_APPROXIMATION_TARGET_WIDTH.id(),
        None,
    );
    registry.add_option_support(
        algo,
        RectPackingOptions::WIDTH_APPROXIMATION_STRATEGY.id(),
        None,
    );
    registry.add_option_support(algo, RectPackingOptions::PACKING_STRATEGY.id(), None);
    registry.add_option_support(
        algo,
        RectPackingOptions::PACKING_COMPACTION_ROW_HEIGHT_REEVALUATION.id(),
        None,
    );
    registry.add_option_support(
        algo,
        RectPackingOptions::PACKING_COMPACTION_ITERATIONS.id(),
        None,
    );
    registry.add_option_support(
        algo,
        RectPackingOptions::WHITE_SPACE_ELIMINATION_STRATEGY.id(),
        None,
    );
    registry.add_option_support(algo, CoreOptions::INTERACTIVE.id(), None);
    registry.add_option_support(algo, CoreOptions::INTERACTIVE_LAYOUT.id(), None);
    registry.add_option_support(algo, RectPackingOptions::DESIRED_POSITION.id(), None);
    registry.add_option_support(algo, RectPackingOptions::CURRENT_POSITION.id(), None);
    registry.add_option_support(algo, RectPackingOptions::IN_NEW_ROW.id(), None);
    registry.add_option_support(algo, RectPackingOptions::TRYBOX.id(), None);
    registry.add_option_support(algo, RectPackingOptions::ORDER_BY_SIZE.id(), None);
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
            Some(|| WidthApproximationStrategy::Greedy),
            Some(|v: &WidthApproximationStrategy| *v),
        );
        ElkReflect::register(
            Some(|| PackingStrategy::Compaction),
            Some(|v: &PackingStrategy| *v),
        );
        ElkReflect::register(
            Some(|| WhiteSpaceEliminationStrategy::None),
            Some(|v: &WhiteSpaceEliminationStrategy| *v),
        );
        ElkReflect::register(
            Some(|| OptimizationGoal::MaxScaleDriven),
            Some(|v: &OptimizationGoal| *v),
        );
        ElkReflect::register(
            Some(EnumSet::<ContentAlignment>::none_of),
            Some(|v: &EnumSet<ContentAlignment>| v.clone()),
        );
        ElkReflect::register(
            Some(EnumSet::<SizeConstraint>::none_of),
            Some(|v: &EnumSet<SizeConstraint>| v.clone()),
        );
        ElkReflect::register(
            Some(EnumSet::<SizeOptions>::none_of),
            Some(|v: &EnumSet<SizeOptions>| v.clone()),
        );
        ElkReflect::register(
            Some(EnumSet::<NodeLabelPlacement>::none_of),
            Some(|v: &EnumSet<NodeLabelPlacement>| v.clone()),
        );
        ElkReflect::register(
            Some(EnumSet::<PortLabelPlacement>::none_of),
            Some(|v: &EnumSet<PortLabelPlacement>| v.clone()),
        );
    });
}
