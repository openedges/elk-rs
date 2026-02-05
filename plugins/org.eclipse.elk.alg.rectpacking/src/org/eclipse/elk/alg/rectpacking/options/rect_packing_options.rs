use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::content_alignment::ContentAlignment;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::node_label_placement::NodeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_label_placement::PortLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;

use crate::org::eclipse::elk::alg::rectpacking::options::OptimizationGoal;
use crate::org::eclipse::elk::alg::rectpacking::p1widthapproximation::WidthApproximationStrategy;
use crate::org::eclipse::elk::alg::rectpacking::p2packing::PackingStrategy;
use crate::org::eclipse::elk::alg::rectpacking::p3whitespaceelimination::WhiteSpaceEliminationStrategy;

pub struct RectPackingOptions;

pub static WIDTH_APPROXIMATION_STRATEGY_PROPERTY: LazyLock<Property<WidthApproximationStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.rectpacking.widthApproximation.strategy",
            WidthApproximationStrategy::Greedy,
        )
    });

pub static WIDTH_APPROXIMATION_TARGET_WIDTH_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.rectpacking.widthApproximation.targetWidth",
            -1.0,
        )
    });

pub static WIDTH_APPROXIMATION_OPTIMIZATION_GOAL_PROPERTY: LazyLock<Property<OptimizationGoal>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.rectpacking.widthApproximation.optimizationGoal",
            OptimizationGoal::MaxScaleDriven,
        )
    });

pub static WIDTH_APPROXIMATION_LAST_PLACE_SHIFT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.rectpacking.widthApproximation.lastPlaceShift",
            true,
        )
    });

pub static PACKING_STRATEGY_PROPERTY: LazyLock<Property<PackingStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.rectpacking.packing.strategy",
            PackingStrategy::Compaction,
        )
    });

pub static PACKING_COMPACTION_ROW_HEIGHT_REEVALUATION_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.rectpacking.packing.compaction.rowHeightReevaluation",
            false,
        )
    });

pub static PACKING_COMPACTION_ITERATIONS_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.rectpacking.packing.compaction.iterations",
            1,
        )
    });

pub static WHITE_SPACE_ELIMINATION_STRATEGY_PROPERTY: LazyLock<Property<WhiteSpaceEliminationStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.rectpacking.whiteSpaceElimination.strategy",
            WhiteSpaceEliminationStrategy::None,
        )
    });

pub static TRYBOX_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.rectpacking.trybox", false));

pub static CURRENT_POSITION_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.rectpacking.currentPosition", -1));

pub static DESIRED_POSITION_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.rectpacking.desiredPosition", -1));

pub static IN_NEW_ROW_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.rectpacking.inNewRow", false));

pub static ORDER_BY_SIZE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.rectpacking.orderBySize", false));

impl RectPackingOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.rectpacking";

    pub const WIDTH_APPROXIMATION_STRATEGY: &'static LazyLock<Property<WidthApproximationStrategy>> =
        &WIDTH_APPROXIMATION_STRATEGY_PROPERTY;
    pub const WIDTH_APPROXIMATION_TARGET_WIDTH: &'static LazyLock<Property<f64>> =
        &WIDTH_APPROXIMATION_TARGET_WIDTH_PROPERTY;
    pub const WIDTH_APPROXIMATION_OPTIMIZATION_GOAL: &'static LazyLock<Property<OptimizationGoal>> =
        &WIDTH_APPROXIMATION_OPTIMIZATION_GOAL_PROPERTY;
    pub const WIDTH_APPROXIMATION_LAST_PLACE_SHIFT: &'static LazyLock<Property<bool>> =
        &WIDTH_APPROXIMATION_LAST_PLACE_SHIFT_PROPERTY;

    pub const PACKING_STRATEGY: &'static LazyLock<Property<PackingStrategy>> = &PACKING_STRATEGY_PROPERTY;
    pub const PACKING_COMPACTION_ROW_HEIGHT_REEVALUATION: &'static LazyLock<Property<bool>> =
        &PACKING_COMPACTION_ROW_HEIGHT_REEVALUATION_PROPERTY;
    pub const PACKING_COMPACTION_ITERATIONS: &'static LazyLock<Property<i32>> =
        &PACKING_COMPACTION_ITERATIONS_PROPERTY;

    pub const WHITE_SPACE_ELIMINATION_STRATEGY: &'static LazyLock<Property<WhiteSpaceEliminationStrategy>> =
        &WHITE_SPACE_ELIMINATION_STRATEGY_PROPERTY;

    pub const TRYBOX: &'static LazyLock<Property<bool>> = &TRYBOX_PROPERTY;
    pub const CURRENT_POSITION: &'static LazyLock<Property<i32>> = &CURRENT_POSITION_PROPERTY;
    pub const DESIRED_POSITION: &'static LazyLock<Property<i32>> = &DESIRED_POSITION_PROPERTY;
    pub const IN_NEW_ROW: &'static LazyLock<Property<bool>> = &IN_NEW_ROW_PROPERTY;
    pub const ORDER_BY_SIZE: &'static LazyLock<Property<bool>> = &ORDER_BY_SIZE_PROPERTY;

    pub const PADDING: &'static LazyLock<Property<ElkPadding>> = CoreOptions::PADDING;
    pub const SPACING_NODE_NODE: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_NODE_NODE;
    pub const ASPECT_RATIO: &'static LazyLock<Property<f64>> = CoreOptions::ASPECT_RATIO;
    pub const CONTENT_ALIGNMENT: &'static LazyLock<Property<EnumSet<ContentAlignment>>> =
        CoreOptions::CONTENT_ALIGNMENT;
    pub const NODE_SIZE_CONSTRAINTS: &'static LazyLock<Property<EnumSet<SizeConstraint>>> =
        CoreOptions::NODE_SIZE_CONSTRAINTS;
    pub const NODE_SIZE_MINIMUM: &'static LazyLock<Property<org_eclipse_elk_core::org::eclipse::elk::core::math::KVector>> =
        CoreOptions::NODE_SIZE_MINIMUM;
    pub const NODE_SIZE_OPTIONS: &'static LazyLock<Property<EnumSet<SizeOptions>>> =
        CoreOptions::NODE_SIZE_OPTIONS;
    pub const NODE_LABELS_PLACEMENT: &'static LazyLock<Property<EnumSet<NodeLabelPlacement>>> =
        CoreOptions::NODE_LABELS_PLACEMENT;
    pub const OMIT_NODE_MICRO_LAYOUT: &'static LazyLock<Property<bool>> = CoreOptions::OMIT_NODE_MICRO_LAYOUT;
    pub const PORT_LABELS_PLACEMENT: &'static LazyLock<Property<EnumSet<PortLabelPlacement>>> =
        CoreOptions::PORT_LABELS_PLACEMENT;
    pub const NODE_SIZE_FIXED_GRAPH_SIZE: &'static LazyLock<Property<bool>> =
        CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE;
    pub const INTERACTIVE: &'static LazyLock<Property<bool>> = CoreOptions::INTERACTIVE;
    pub const INTERACTIVE_LAYOUT: &'static LazyLock<Property<bool>> = CoreOptions::INTERACTIVE_LAYOUT;
}
