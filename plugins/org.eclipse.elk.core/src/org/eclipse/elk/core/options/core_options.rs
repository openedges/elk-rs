use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::core::data::LayoutAlgorithmData;
use crate::org::eclipse::elk::core::math::{ElkMargin, ElkPadding, KVector, KVectorChain};
use crate::org::eclipse::elk::core::options::{
    Alignment, ContentAlignment, Direction, EdgeLabelPlacement, EdgeRouting, NodeLabelPlacement,
    PortConstraints, PortLabelPlacement, PortSide, SizeConstraint, SizeOptions,
};
use crate::org::eclipse::elk::core::util::EnumSet;

pub static ALGORITHM_PROPERTY: LazyLock<Property<String>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.algorithm"));

pub static RESOLVED_ALGORITHM_PROPERTY: LazyLock<Property<LayoutAlgorithmData>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.resolvedAlgorithm"));

pub static ALIGNMENT_PROPERTY: LazyLock<Property<Alignment>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alignment", Alignment::Automatic));

pub static ASPECT_RATIO_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.aspectRatio"));

pub static BEND_POINTS_PROPERTY: LazyLock<Property<KVectorChain>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.bendPoints"));

pub static POSITION_PROPERTY: LazyLock<Property<KVector>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.position"));

pub static PRIORITY_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.priority"));

pub static RANDOM_SEED_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.randomSeed", 0));

pub static SEPARATE_CONNECTED_COMPONENTS_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.separateConnectedComponents", false));

pub static PADDING_PROPERTY: LazyLock<Property<ElkPadding>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.padding", ElkPadding::with_any(12.0))
});

pub static CONTENT_ALIGNMENT_PROPERTY: LazyLock<Property<EnumSet<ContentAlignment>>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.contentAlignment",
            ContentAlignment::top_left(),
        )
    });

pub static PORT_CONSTRAINTS_PROPERTY: LazyLock<Property<PortConstraints>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.portConstraints", PortConstraints::Undefined));

pub static DIRECTION_PROPERTY: LazyLock<Property<Direction>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.direction", Direction::Undefined));

pub static EDGE_ROUTING_PROPERTY: LazyLock<Property<EdgeRouting>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.edgeRouting", EdgeRouting::Undefined));

pub static PORT_SIDE_PROPERTY: LazyLock<Property<PortSide>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.port.side", PortSide::Undefined));

pub static NODE_SIZE_CONSTRAINTS_PROPERTY: LazyLock<Property<EnumSet<SizeConstraint>>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.nodeSize.constraints", EnumSet::none_of()));

pub static NODE_SIZE_OPTIONS_PROPERTY: LazyLock<Property<EnumSet<SizeOptions>>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.nodeSize.options",
            EnumSet::of(&[SizeOptions::DefaultMinimumSize]),
        )
    });

pub static NODE_SIZE_MINIMUM_PROPERTY: LazyLock<Property<KVector>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.nodeSize.minimum", KVector::new()));

pub static SCALE_FACTOR_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.scaleFactor", 1.0));

pub static PORT_ANCHOR_PROPERTY: LazyLock<Property<KVector>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.port.anchor"));

pub static MARGINS_PROPERTY: LazyLock<Property<ElkMargin>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.margins", ElkMargin::new()));

pub static CHILD_AREA_WIDTH_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.childAreaWidth"));

pub static CHILD_AREA_HEIGHT_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.childAreaHeight"));

pub static JUNCTION_POINTS_PROPERTY: LazyLock<Property<KVectorChain>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.junctionPoints", KVectorChain::new()));

pub static NODE_LABELS_PLACEMENT_PROPERTY: LazyLock<Property<EnumSet<NodeLabelPlacement>>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.nodeLabels.placement",
            NodeLabelPlacement::fixed(),
        )
    });

pub static EDGE_LABELS_PLACEMENT_PROPERTY: LazyLock<Property<EdgeLabelPlacement>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.edgeLabels.placement", EdgeLabelPlacement::Center));

pub static PORT_LABELS_PLACEMENT_PROPERTY: LazyLock<Property<EnumSet<PortLabelPlacement>>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.portLabels.placement",
            PortLabelPlacement::outside(),
        )
    });

pub static PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.portLabels.nextToPortIfPossible",
            false,
        )
    });

pub static PORT_LABELS_TREAT_AS_GROUP_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.portLabels.treatAsGroup", true));

pub struct CoreOptions;

impl CoreOptions {
    pub const ALGORITHM: &'static LazyLock<Property<String>> = &ALGORITHM_PROPERTY;
    pub const RESOLVED_ALGORITHM: &'static LazyLock<Property<LayoutAlgorithmData>> =
        &RESOLVED_ALGORITHM_PROPERTY;
    pub const ALIGNMENT: &'static LazyLock<Property<Alignment>> = &ALIGNMENT_PROPERTY;
    pub const ASPECT_RATIO: &'static LazyLock<Property<f64>> = &ASPECT_RATIO_PROPERTY;
    pub const BEND_POINTS: &'static LazyLock<Property<KVectorChain>> = &BEND_POINTS_PROPERTY;
    pub const POSITION: &'static LazyLock<Property<KVector>> = &POSITION_PROPERTY;
    pub const PRIORITY: &'static LazyLock<Property<i32>> = &PRIORITY_PROPERTY;
    pub const RANDOM_SEED: &'static LazyLock<Property<i32>> = &RANDOM_SEED_PROPERTY;
    pub const SEPARATE_CONNECTED_COMPONENTS: &'static LazyLock<Property<bool>> =
        &SEPARATE_CONNECTED_COMPONENTS_PROPERTY;
    pub const PADDING: &'static LazyLock<Property<ElkPadding>> = &PADDING_PROPERTY;
    pub const CONTENT_ALIGNMENT: &'static LazyLock<Property<EnumSet<ContentAlignment>>> =
        &CONTENT_ALIGNMENT_PROPERTY;
    pub const PORT_CONSTRAINTS: &'static LazyLock<Property<PortConstraints>> =
        &PORT_CONSTRAINTS_PROPERTY;
    pub const DIRECTION: &'static LazyLock<Property<Direction>> = &DIRECTION_PROPERTY;
    pub const EDGE_ROUTING: &'static LazyLock<Property<EdgeRouting>> = &EDGE_ROUTING_PROPERTY;
    pub const PORT_SIDE: &'static LazyLock<Property<PortSide>> = &PORT_SIDE_PROPERTY;
    pub const NODE_SIZE_CONSTRAINTS: &'static LazyLock<Property<EnumSet<SizeConstraint>>> =
        &NODE_SIZE_CONSTRAINTS_PROPERTY;
    pub const NODE_SIZE_OPTIONS: &'static LazyLock<Property<EnumSet<SizeOptions>>> =
        &NODE_SIZE_OPTIONS_PROPERTY;
    pub const NODE_SIZE_MINIMUM: &'static LazyLock<Property<KVector>> =
        &NODE_SIZE_MINIMUM_PROPERTY;
    pub const SCALE_FACTOR: &'static LazyLock<Property<f64>> = &SCALE_FACTOR_PROPERTY;
    pub const PORT_ANCHOR: &'static LazyLock<Property<KVector>> = &PORT_ANCHOR_PROPERTY;
    pub const MARGINS: &'static LazyLock<Property<ElkMargin>> = &MARGINS_PROPERTY;
    pub const CHILD_AREA_WIDTH: &'static LazyLock<Property<f64>> = &CHILD_AREA_WIDTH_PROPERTY;
    pub const CHILD_AREA_HEIGHT: &'static LazyLock<Property<f64>> = &CHILD_AREA_HEIGHT_PROPERTY;
    pub const JUNCTION_POINTS: &'static LazyLock<Property<KVectorChain>> =
        &JUNCTION_POINTS_PROPERTY;
    pub const NODE_LABELS_PLACEMENT: &'static LazyLock<Property<EnumSet<NodeLabelPlacement>>> =
        &NODE_LABELS_PLACEMENT_PROPERTY;
    pub const EDGE_LABELS_PLACEMENT: &'static LazyLock<Property<EdgeLabelPlacement>> =
        &EDGE_LABELS_PLACEMENT_PROPERTY;
    pub const PORT_LABELS_PLACEMENT: &'static LazyLock<Property<EnumSet<PortLabelPlacement>>> =
        &PORT_LABELS_PLACEMENT_PROPERTY;
    pub const PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE: &'static LazyLock<Property<bool>> =
        &PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE_PROPERTY;
    pub const PORT_LABELS_TREAT_AS_GROUP: &'static LazyLock<Property<bool>> =
        &PORT_LABELS_TREAT_AS_GROUP_PROPERTY;
}
