use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::node_label_placement::NodeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_label_placement::PortLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;

use crate::org::eclipse::elk::alg::radial::options::{
    AnnulusWedgeCriteria, CompactionStrategy, RadialTranslationStrategy, SortingStrategy,
};

pub struct RadialOptions;

pub static CENTER_ON_ROOT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.radial.centerOnRoot", false));

pub static ORDER_ID_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.radial.orderId", 0));

pub static RADIUS_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.radial.radius", 0.0));

pub static ROTATE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.radial.rotate", false));

pub static ROTATION_TARGET_ANGLE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.radial.rotation.targetAngle", 0.0));

pub static ROTATION_COMPUTE_ADDITIONAL_WEDGE_SPACE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.radial.rotation.computeAdditionalWedgeSpace",
            false,
        )
    });

pub static ROTATION_OUTGOING_EDGE_ANGLES_PROPERTY: LazyLock<Property<bool>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.radial.rotation.outgoingEdgeAngles", false)
});

pub static COMPACTOR_PROPERTY: LazyLock<Property<CompactionStrategy>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.radial.compactor", CompactionStrategy::None)
});

pub static COMPACTION_STEP_SIZE_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.radial.compactionStepSize", 1));

pub static SORTER_PROPERTY: LazyLock<Property<SortingStrategy>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.radial.sorter", SortingStrategy::None)
});

pub static WEDGE_CRITERIA_PROPERTY: LazyLock<Property<AnnulusWedgeCriteria>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.radial.wedgeCriteria",
            AnnulusWedgeCriteria::NodeSize,
        )
    });

pub static OPTIMIZATION_CRITERIA_PROPERTY: LazyLock<Property<RadialTranslationStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.radial.optimizationCriteria",
            RadialTranslationStrategy::None,
        )
    });

impl RadialOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.radial";

    pub const CENTER_ON_ROOT: &'static LazyLock<Property<bool>> = &CENTER_ON_ROOT_PROPERTY;
    pub const ORDER_ID: &'static LazyLock<Property<i32>> = &ORDER_ID_PROPERTY;
    pub const RADIUS: &'static LazyLock<Property<f64>> = &RADIUS_PROPERTY;

    pub const ROTATE: &'static LazyLock<Property<bool>> = &ROTATE_PROPERTY;
    pub const ROTATION_TARGET_ANGLE: &'static LazyLock<Property<f64>> =
        &ROTATION_TARGET_ANGLE_PROPERTY;
    pub const ROTATION_COMPUTE_ADDITIONAL_WEDGE_SPACE: &'static LazyLock<Property<bool>> =
        &ROTATION_COMPUTE_ADDITIONAL_WEDGE_SPACE_PROPERTY;
    pub const ROTATION_OUTGOING_EDGE_ANGLES: &'static LazyLock<Property<bool>> =
        &ROTATION_OUTGOING_EDGE_ANGLES_PROPERTY;

    pub const COMPACTOR: &'static LazyLock<Property<CompactionStrategy>> = &COMPACTOR_PROPERTY;
    pub const COMPACTION_STEP_SIZE: &'static LazyLock<Property<i32>> =
        &COMPACTION_STEP_SIZE_PROPERTY;

    pub const SORTER: &'static LazyLock<Property<SortingStrategy>> = &SORTER_PROPERTY;
    pub const WEDGE_CRITERIA: &'static LazyLock<Property<AnnulusWedgeCriteria>> =
        &WEDGE_CRITERIA_PROPERTY;
    pub const OPTIMIZATION_CRITERIA: &'static LazyLock<Property<RadialTranslationStrategy>> =
        &OPTIMIZATION_CRITERIA_PROPERTY;

    pub const POSITION: &'static LazyLock<
        Property<org_eclipse_elk_core::org::eclipse::elk::core::math::KVector>,
    > = CoreOptions::POSITION;
    pub const SPACING_NODE_NODE: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_NODE_NODE;
    pub const NODE_SIZE_CONSTRAINTS: &'static LazyLock<Property<EnumSet<SizeConstraint>>> =
        CoreOptions::NODE_SIZE_CONSTRAINTS;
    pub const NODE_SIZE_MINIMUM: &'static LazyLock<
        Property<org_eclipse_elk_core::org::eclipse::elk::core::math::KVector>,
    > = CoreOptions::NODE_SIZE_MINIMUM;
    pub const NODE_SIZE_OPTIONS: &'static LazyLock<Property<EnumSet<SizeOptions>>> =
        CoreOptions::NODE_SIZE_OPTIONS;
    pub const NODE_LABELS_PLACEMENT: &'static LazyLock<Property<EnumSet<NodeLabelPlacement>>> =
        CoreOptions::NODE_LABELS_PLACEMENT;
    pub const OMIT_NODE_MICRO_LAYOUT: &'static LazyLock<Property<bool>> =
        CoreOptions::OMIT_NODE_MICRO_LAYOUT;
    pub const PORT_LABELS_PLACEMENT: &'static LazyLock<Property<EnumSet<PortLabelPlacement>>> =
        CoreOptions::PORT_LABELS_PLACEMENT;
    pub const NODE_SIZE_FIXED_GRAPH_SIZE: &'static LazyLock<Property<bool>> =
        CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE;
    pub const PADDING: &'static LazyLock<
        Property<org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding>,
    > = CoreOptions::PADDING;
    pub const MARGINS: &'static LazyLock<
        Property<org_eclipse_elk_core::org::eclipse::elk::core::math::ElkMargin>,
    > = CoreOptions::MARGINS;
    pub const CHILD_AREA_WIDTH: &'static LazyLock<Property<f64>> = CoreOptions::CHILD_AREA_WIDTH;
    pub const CHILD_AREA_HEIGHT: &'static LazyLock<Property<f64>> = CoreOptions::CHILD_AREA_HEIGHT;
}
