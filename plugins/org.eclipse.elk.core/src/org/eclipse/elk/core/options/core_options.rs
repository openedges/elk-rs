use std::sync::{Arc, LazyLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::core::data::LayoutAlgorithmData;
use crate::org::eclipse::elk::core::labels::ILabelManager;
use crate::org::eclipse::elk::core::math::{ElkMargin, ElkPadding, KVector, KVectorChain};
use crate::org::eclipse::elk::core::options::{
    Alignment, ContentAlignment, Direction, EdgeCoords, EdgeLabelPlacement, EdgeRouting, EdgeType,
    HierarchyHandling, ITopdownSizeApproximator, NodeLabelPlacement, PackingMode, PortAlignment,
    PortConstraints, PortLabelPlacement, PortSide, ShapeCoords, SizeConstraint, SizeOptions,
    TopdownNodeTypes,
};
use crate::org::eclipse::elk::core::util::{EnumSet, IndividualSpacings};

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

pub static CONTENT_ALIGNMENT_PROPERTY: LazyLock<Property<EnumSet<ContentAlignment>>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.contentAlignment",
            ContentAlignment::top_left(),
        )
    });

pub static DEBUG_MODE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.debugMode", false));

pub static DIRECTION_PROPERTY: LazyLock<Property<Direction>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.direction", Direction::Undefined));

pub static EDGE_ROUTING_PROPERTY: LazyLock<Property<EdgeRouting>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.edgeRouting", EdgeRouting::Undefined));

pub static EXPAND_NODES_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.expandNodes", false));

pub static HIERARCHY_HANDLING_PROPERTY: LazyLock<Property<HierarchyHandling>> =
    LazyLock::new(|| {
        Property::with_default("org.eclipse.elk.hierarchyHandling", HierarchyHandling::Inherit)
    });

pub static PADDING_PROPERTY: LazyLock<Property<ElkPadding>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.padding", ElkPadding::with_any(12.0)));

pub static INTERACTIVE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.interactive", false));

pub static INTERACTIVE_LAYOUT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.interactiveLayout", false));

pub static OMIT_NODE_MICRO_LAYOUT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.omitNodeMicroLayout", false));

pub static BOX_PACKING_MODE_PROPERTY: LazyLock<Property<PackingMode>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.box.packingMode", PackingMode::Simple));

pub static JSON_SHAPE_COORDS_PROPERTY: LazyLock<Property<ShapeCoords>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.json.shapeCoords", ShapeCoords::Inherit));

pub static JSON_EDGE_COORDS_PROPERTY: LazyLock<Property<EdgeCoords>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.json.edgeCoords", EdgeCoords::Inherit));

pub static SPACING_COMMENT_COMMENT_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.spacing.commentComment", 10.0));

pub static SPACING_COMMENT_NODE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.spacing.commentNode", 10.0));

pub static SPACING_COMPONENT_COMPONENT_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.spacing.componentComponent", 20.0));

pub static SPACING_EDGE_EDGE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.spacing.edgeEdge", 10.0));

pub static SPACING_EDGE_LABEL_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.spacing.edgeLabel", 2.0));

pub static SPACING_EDGE_NODE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.spacing.edgeNode", 10.0));

pub static SPACING_LABEL_LABEL_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.spacing.labelLabel", 0.0));

pub static SPACING_LABEL_NODE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.spacing.labelNode", 5.0));

pub static SPACING_LABEL_PORT_HORIZONTAL_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.spacing.labelPortHorizontal", 1.0));

pub static SPACING_LABEL_PORT_VERTICAL_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.spacing.labelPortVertical", 1.0));

pub static SPACING_NODE_NODE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.spacing.nodeNode", 20.0));

pub static SPACING_NODE_SELF_LOOP_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.spacing.nodeSelfLoop", 10.0));

pub static SPACING_PORT_PORT_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.spacing.portPort", 10.0));

pub static SPACING_INDIVIDUAL_PROPERTY: LazyLock<Property<IndividualSpacings>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.spacing.individual"));

pub static SPACING_PORTS_SURROUNDING_PROPERTY: LazyLock<Property<ElkMargin>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.spacing.portsSurrounding",
            ElkMargin::new(),
        )
    });

pub static PARTITIONING_PARTITION_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.partitioning.partition"));

pub static PARTITIONING_ACTIVATE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.partitioning.activate", false));

pub static NODE_LABELS_PADDING_PROPERTY: LazyLock<Property<ElkPadding>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.nodeLabels.padding", ElkPadding::with_any(5.0)));

pub static NODE_LABELS_PLACEMENT_PROPERTY: LazyLock<Property<EnumSet<NodeLabelPlacement>>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.nodeLabels.placement",
            NodeLabelPlacement::fixed(),
        )
    });

pub static PORT_ALIGNMENT_DEFAULT_PROPERTY: LazyLock<Property<PortAlignment>> =
    LazyLock::new(|| {
        Property::with_default("org.eclipse.elk.portAlignment.default", PortAlignment::Distributed)
    });

pub static PORT_ALIGNMENT_NORTH_PROPERTY: LazyLock<Property<PortAlignment>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.portAlignment.north"));

pub static PORT_ALIGNMENT_SOUTH_PROPERTY: LazyLock<Property<PortAlignment>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.portAlignment.south"));

pub static PORT_ALIGNMENT_WEST_PROPERTY: LazyLock<Property<PortAlignment>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.portAlignment.west"));

pub static PORT_ALIGNMENT_EAST_PROPERTY: LazyLock<Property<PortAlignment>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.portAlignment.east"));

pub static PORT_CONSTRAINTS_PROPERTY: LazyLock<Property<PortConstraints>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.portConstraints", PortConstraints::Undefined));

pub static POSITION_PROPERTY: LazyLock<Property<KVector>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.position"));

pub static PRIORITY_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.priority"));

pub static RANDOM_SEED_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.randomSeed"));

pub static SEPARATE_CONNECTED_COMPONENTS_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.separateConnectedComponents"));

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

pub static NODE_SIZE_FIXED_GRAPH_SIZE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.nodeSize.fixedGraphSize", false));

pub static JUNCTION_POINTS_PROPERTY: LazyLock<Property<KVectorChain>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.junctionPoints", KVectorChain::new()));

pub static COMMENT_BOX_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.commentBox", false));

pub static EDGE_LABELS_PLACEMENT_PROPERTY: LazyLock<Property<EdgeLabelPlacement>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.edgeLabels.placement", EdgeLabelPlacement::Center));

pub static EDGE_LABELS_INLINE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.edgeLabels.inline", false));

pub static FONT_NAME_PROPERTY: LazyLock<Property<String>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.font.name"));

pub static FONT_SIZE_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.font.size"));

pub static HYPERNODE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.hypernode", false));

pub static LABEL_MANAGER_PROPERTY: LazyLock<Property<Arc<dyn ILabelManager>>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.labelManager"));

pub static SOFTWRAPPING_FUZZINESS_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.softwrappingFuzziness", 0.0));

pub static MARGINS_PROPERTY: LazyLock<Property<ElkMargin>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.margins", ElkMargin::new()));

pub static NO_LAYOUT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.noLayout", false));

pub static PORT_ANCHOR_PROPERTY: LazyLock<Property<KVector>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.port.anchor"));

pub static PORT_INDEX_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.port.index"));

pub static PORT_SIDE_PROPERTY: LazyLock<Property<PortSide>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.port.side", PortSide::Undefined));

pub static PORT_BORDER_OFFSET_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.port.borderOffset"));

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

pub static SCALE_FACTOR_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.scaleFactor", 1.0));

pub static CHILD_AREA_WIDTH_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.childAreaWidth"));

pub static CHILD_AREA_HEIGHT_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.childAreaHeight"));

pub static TOPDOWN_LAYOUT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.topdownLayout", false));

pub static TOPDOWN_SIZE_CATEGORIES_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.topdown.sizeCategories", 3));

pub static TOPDOWN_SIZE_CATEGORIES_HIERARCHICAL_NODE_WEIGHT_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.topdown.sizeCategoriesHierarchicalNodeWeight",
            4,
        )
    });

pub static TOPDOWN_SCALE_FACTOR_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.topdown.scaleFactor", 1.0));

pub static TOPDOWN_SIZE_APPROXIMATOR_PROPERTY: LazyLock<Property<Arc<dyn ITopdownSizeApproximator>>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.topdown.sizeApproximator"));

pub static TOPDOWN_HIERARCHICAL_NODE_WIDTH_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.topdown.hierarchicalNodeWidth", 150.0));

pub static TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.topdown.hierarchicalNodeAspectRatio",
            1.414,
        )
    });

pub static TOPDOWN_NODE_TYPE_PROPERTY: LazyLock<Property<TopdownNodeTypes>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.topdown.nodeType"));

pub static TOPDOWN_SCALE_CAP_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.topdown.scaleCap", 1.0));

pub static INSIDE_SELF_LOOPS_ACTIVATE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.insideSelfLoops.activate", false));

pub static INSIDE_SELF_LOOPS_YO_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.insideSelfLoops.yo", false));

pub static EDGE_THICKNESS_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.edge.thickness", 1.0));

pub static EDGE_TYPE_PROPERTY: LazyLock<Property<EdgeType>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.edge.type", EdgeType::None));

pub static ANIMATE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.animate", true));

pub static ANIM_TIME_FACTOR_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.animTimeFactor", 100));

pub static LAYOUT_ANCESTORS_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.layoutAncestors", false));

pub static MAX_ANIM_TIME_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.maxAnimTime", 4000));

pub static MIN_ANIM_TIME_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.minAnimTime", 400));

pub static PROGRESS_BAR_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.progressBar", false));

pub static VALIDATE_GRAPH_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.validateGraph", false));

pub static VALIDATE_OPTIONS_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.validateOptions", true));

pub static ZOOM_TO_FIT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.zoomToFit", false));

pub struct CoreOptions;

impl CoreOptions {
    pub const ALGORITHM: &'static LazyLock<Property<String>> = &ALGORITHM_PROPERTY;
    pub const RESOLVED_ALGORITHM: &'static LazyLock<Property<LayoutAlgorithmData>> =
        &RESOLVED_ALGORITHM_PROPERTY;
    pub const ALIGNMENT: &'static LazyLock<Property<Alignment>> = &ALIGNMENT_PROPERTY;
    pub const ASPECT_RATIO: &'static LazyLock<Property<f64>> = &ASPECT_RATIO_PROPERTY;
    pub const BEND_POINTS: &'static LazyLock<Property<KVectorChain>> = &BEND_POINTS_PROPERTY;
    pub const CONTENT_ALIGNMENT: &'static LazyLock<Property<EnumSet<ContentAlignment>>> =
        &CONTENT_ALIGNMENT_PROPERTY;
    pub const DEBUG_MODE: &'static LazyLock<Property<bool>> = &DEBUG_MODE_PROPERTY;
    pub const DIRECTION: &'static LazyLock<Property<Direction>> = &DIRECTION_PROPERTY;
    pub const EDGE_ROUTING: &'static LazyLock<Property<EdgeRouting>> = &EDGE_ROUTING_PROPERTY;
    pub const EXPAND_NODES: &'static LazyLock<Property<bool>> = &EXPAND_NODES_PROPERTY;
    pub const HIERARCHY_HANDLING: &'static LazyLock<Property<HierarchyHandling>> =
        &HIERARCHY_HANDLING_PROPERTY;
    pub const PADDING: &'static LazyLock<Property<ElkPadding>> = &PADDING_PROPERTY;
    pub const INTERACTIVE: &'static LazyLock<Property<bool>> = &INTERACTIVE_PROPERTY;
    pub const INTERACTIVE_LAYOUT: &'static LazyLock<Property<bool>> = &INTERACTIVE_LAYOUT_PROPERTY;
    pub const OMIT_NODE_MICRO_LAYOUT: &'static LazyLock<Property<bool>> =
        &OMIT_NODE_MICRO_LAYOUT_PROPERTY;
    pub const BOX_PACKING_MODE: &'static LazyLock<Property<PackingMode>> =
        &BOX_PACKING_MODE_PROPERTY;
    pub const JSON_SHAPE_COORDS: &'static LazyLock<Property<ShapeCoords>> =
        &JSON_SHAPE_COORDS_PROPERTY;
    pub const JSON_EDGE_COORDS: &'static LazyLock<Property<EdgeCoords>> =
        &JSON_EDGE_COORDS_PROPERTY;
    pub const SPACING_COMMENT_COMMENT: &'static LazyLock<Property<f64>> =
        &SPACING_COMMENT_COMMENT_PROPERTY;
    pub const SPACING_COMMENT_NODE: &'static LazyLock<Property<f64>> =
        &SPACING_COMMENT_NODE_PROPERTY;
    pub const SPACING_COMPONENT_COMPONENT: &'static LazyLock<Property<f64>> =
        &SPACING_COMPONENT_COMPONENT_PROPERTY;
    pub const SPACING_EDGE_EDGE: &'static LazyLock<Property<f64>> = &SPACING_EDGE_EDGE_PROPERTY;
    pub const SPACING_EDGE_LABEL: &'static LazyLock<Property<f64>> =
        &SPACING_EDGE_LABEL_PROPERTY;
    pub const SPACING_EDGE_NODE: &'static LazyLock<Property<f64>> = &SPACING_EDGE_NODE_PROPERTY;
    pub const SPACING_LABEL_LABEL: &'static LazyLock<Property<f64>> =
        &SPACING_LABEL_LABEL_PROPERTY;
    pub const SPACING_LABEL_NODE: &'static LazyLock<Property<f64>> =
        &SPACING_LABEL_NODE_PROPERTY;
    pub const SPACING_LABEL_PORT_HORIZONTAL: &'static LazyLock<Property<f64>> =
        &SPACING_LABEL_PORT_HORIZONTAL_PROPERTY;
    pub const SPACING_LABEL_PORT_VERTICAL: &'static LazyLock<Property<f64>> =
        &SPACING_LABEL_PORT_VERTICAL_PROPERTY;
    pub const SPACING_NODE_NODE: &'static LazyLock<Property<f64>> = &SPACING_NODE_NODE_PROPERTY;
    pub const SPACING_NODE_SELF_LOOP: &'static LazyLock<Property<f64>> =
        &SPACING_NODE_SELF_LOOP_PROPERTY;
    pub const SPACING_PORT_PORT: &'static LazyLock<Property<f64>> = &SPACING_PORT_PORT_PROPERTY;
    pub const SPACING_INDIVIDUAL: &'static LazyLock<Property<IndividualSpacings>> =
        &SPACING_INDIVIDUAL_PROPERTY;
    pub const SPACING_PORTS_SURROUNDING: &'static LazyLock<Property<ElkMargin>> =
        &SPACING_PORTS_SURROUNDING_PROPERTY;
    pub const PARTITIONING_PARTITION: &'static LazyLock<Property<i32>> =
        &PARTITIONING_PARTITION_PROPERTY;
    pub const PARTITIONING_ACTIVATE: &'static LazyLock<Property<bool>> =
        &PARTITIONING_ACTIVATE_PROPERTY;
    pub const NODE_LABELS_PADDING: &'static LazyLock<Property<ElkPadding>> =
        &NODE_LABELS_PADDING_PROPERTY;
    pub const NODE_LABELS_PLACEMENT: &'static LazyLock<Property<EnumSet<NodeLabelPlacement>>> =
        &NODE_LABELS_PLACEMENT_PROPERTY;
    pub const PORT_ALIGNMENT_DEFAULT: &'static LazyLock<Property<PortAlignment>> =
        &PORT_ALIGNMENT_DEFAULT_PROPERTY;
    pub const PORT_ALIGNMENT_NORTH: &'static LazyLock<Property<PortAlignment>> =
        &PORT_ALIGNMENT_NORTH_PROPERTY;
    pub const PORT_ALIGNMENT_SOUTH: &'static LazyLock<Property<PortAlignment>> =
        &PORT_ALIGNMENT_SOUTH_PROPERTY;
    pub const PORT_ALIGNMENT_WEST: &'static LazyLock<Property<PortAlignment>> =
        &PORT_ALIGNMENT_WEST_PROPERTY;
    pub const PORT_ALIGNMENT_EAST: &'static LazyLock<Property<PortAlignment>> =
        &PORT_ALIGNMENT_EAST_PROPERTY;
    pub const PORT_CONSTRAINTS: &'static LazyLock<Property<PortConstraints>> =
        &PORT_CONSTRAINTS_PROPERTY;
    pub const POSITION: &'static LazyLock<Property<KVector>> = &POSITION_PROPERTY;
    pub const PRIORITY: &'static LazyLock<Property<i32>> = &PRIORITY_PROPERTY;
    pub const RANDOM_SEED: &'static LazyLock<Property<i32>> = &RANDOM_SEED_PROPERTY;
    pub const SEPARATE_CONNECTED_COMPONENTS: &'static LazyLock<Property<bool>> =
        &SEPARATE_CONNECTED_COMPONENTS_PROPERTY;
    pub const NODE_SIZE_CONSTRAINTS: &'static LazyLock<Property<EnumSet<SizeConstraint>>> =
        &NODE_SIZE_CONSTRAINTS_PROPERTY;
    pub const NODE_SIZE_OPTIONS: &'static LazyLock<Property<EnumSet<SizeOptions>>> =
        &NODE_SIZE_OPTIONS_PROPERTY;
    pub const NODE_SIZE_MINIMUM: &'static LazyLock<Property<KVector>> =
        &NODE_SIZE_MINIMUM_PROPERTY;
    pub const NODE_SIZE_FIXED_GRAPH_SIZE: &'static LazyLock<Property<bool>> =
        &NODE_SIZE_FIXED_GRAPH_SIZE_PROPERTY;
    pub const JUNCTION_POINTS: &'static LazyLock<Property<KVectorChain>> =
        &JUNCTION_POINTS_PROPERTY;
    pub const COMMENT_BOX: &'static LazyLock<Property<bool>> = &COMMENT_BOX_PROPERTY;
    pub const EDGE_LABELS_PLACEMENT: &'static LazyLock<Property<EdgeLabelPlacement>> =
        &EDGE_LABELS_PLACEMENT_PROPERTY;
    pub const EDGE_LABELS_INLINE: &'static LazyLock<Property<bool>> =
        &EDGE_LABELS_INLINE_PROPERTY;
    pub const FONT_NAME: &'static LazyLock<Property<String>> = &FONT_NAME_PROPERTY;
    pub const FONT_SIZE: &'static LazyLock<Property<i32>> = &FONT_SIZE_PROPERTY;
    pub const HYPERNODE: &'static LazyLock<Property<bool>> = &HYPERNODE_PROPERTY;
    pub const LABEL_MANAGER: &'static LazyLock<Property<Arc<dyn ILabelManager>>> =
        &LABEL_MANAGER_PROPERTY;
    pub const SOFTWRAPPING_FUZZINESS: &'static LazyLock<Property<f64>> =
        &SOFTWRAPPING_FUZZINESS_PROPERTY;
    pub const MARGINS: &'static LazyLock<Property<ElkMargin>> = &MARGINS_PROPERTY;
    pub const NO_LAYOUT: &'static LazyLock<Property<bool>> = &NO_LAYOUT_PROPERTY;
    pub const PORT_ANCHOR: &'static LazyLock<Property<KVector>> = &PORT_ANCHOR_PROPERTY;
    pub const PORT_INDEX: &'static LazyLock<Property<i32>> = &PORT_INDEX_PROPERTY;
    pub const PORT_SIDE: &'static LazyLock<Property<PortSide>> = &PORT_SIDE_PROPERTY;
    pub const PORT_BORDER_OFFSET: &'static LazyLock<Property<f64>> =
        &PORT_BORDER_OFFSET_PROPERTY;
    pub const PORT_LABELS_PLACEMENT: &'static LazyLock<Property<EnumSet<PortLabelPlacement>>> =
        &PORT_LABELS_PLACEMENT_PROPERTY;
    pub const PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE: &'static LazyLock<Property<bool>> =
        &PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE_PROPERTY;
    pub const PORT_LABELS_TREAT_AS_GROUP: &'static LazyLock<Property<bool>> =
        &PORT_LABELS_TREAT_AS_GROUP_PROPERTY;
    pub const SCALE_FACTOR: &'static LazyLock<Property<f64>> = &SCALE_FACTOR_PROPERTY;
    pub const CHILD_AREA_WIDTH: &'static LazyLock<Property<f64>> = &CHILD_AREA_WIDTH_PROPERTY;
    pub const CHILD_AREA_HEIGHT: &'static LazyLock<Property<f64>> = &CHILD_AREA_HEIGHT_PROPERTY;
    pub const TOPDOWN_LAYOUT: &'static LazyLock<Property<bool>> = &TOPDOWN_LAYOUT_PROPERTY;
    pub const TOPDOWN_SIZE_CATEGORIES: &'static LazyLock<Property<i32>> =
        &TOPDOWN_SIZE_CATEGORIES_PROPERTY;
    pub const TOPDOWN_SIZE_CATEGORIES_HIERARCHICAL_NODE_WEIGHT: &'static LazyLock<Property<i32>> =
        &TOPDOWN_SIZE_CATEGORIES_HIERARCHICAL_NODE_WEIGHT_PROPERTY;
    pub const TOPDOWN_SCALE_FACTOR: &'static LazyLock<Property<f64>> =
        &TOPDOWN_SCALE_FACTOR_PROPERTY;
    pub const TOPDOWN_SIZE_APPROXIMATOR: &'static LazyLock<Property<Arc<dyn ITopdownSizeApproximator>>> =
        &TOPDOWN_SIZE_APPROXIMATOR_PROPERTY;
    pub const TOPDOWN_HIERARCHICAL_NODE_WIDTH: &'static LazyLock<Property<f64>> =
        &TOPDOWN_HIERARCHICAL_NODE_WIDTH_PROPERTY;
    pub const TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO: &'static LazyLock<Property<f64>> =
        &TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO_PROPERTY;
    pub const TOPDOWN_NODE_TYPE: &'static LazyLock<Property<TopdownNodeTypes>> =
        &TOPDOWN_NODE_TYPE_PROPERTY;
    pub const TOPDOWN_SCALE_CAP: &'static LazyLock<Property<f64>> = &TOPDOWN_SCALE_CAP_PROPERTY;
    pub const INSIDE_SELF_LOOPS_ACTIVATE: &'static LazyLock<Property<bool>> =
        &INSIDE_SELF_LOOPS_ACTIVATE_PROPERTY;
    pub const INSIDE_SELF_LOOPS_YO: &'static LazyLock<Property<bool>> = &INSIDE_SELF_LOOPS_YO_PROPERTY;
    pub const EDGE_THICKNESS: &'static LazyLock<Property<f64>> = &EDGE_THICKNESS_PROPERTY;
    pub const EDGE_TYPE: &'static LazyLock<Property<EdgeType>> = &EDGE_TYPE_PROPERTY;
    pub const ANIMATE: &'static LazyLock<Property<bool>> = &ANIMATE_PROPERTY;
    pub const ANIM_TIME_FACTOR: &'static LazyLock<Property<i32>> = &ANIM_TIME_FACTOR_PROPERTY;
    pub const LAYOUT_ANCESTORS: &'static LazyLock<Property<bool>> = &LAYOUT_ANCESTORS_PROPERTY;
    pub const MAX_ANIM_TIME: &'static LazyLock<Property<i32>> = &MAX_ANIM_TIME_PROPERTY;
    pub const MIN_ANIM_TIME: &'static LazyLock<Property<i32>> = &MIN_ANIM_TIME_PROPERTY;
    pub const PROGRESS_BAR: &'static LazyLock<Property<bool>> = &PROGRESS_BAR_PROPERTY;
    pub const VALIDATE_GRAPH: &'static LazyLock<Property<bool>> = &VALIDATE_GRAPH_PROPERTY;
    pub const VALIDATE_OPTIONS: &'static LazyLock<Property<bool>> = &VALIDATE_OPTIONS_PROPERTY;
    pub const ZOOM_TO_FIT: &'static LazyLock<Property<bool>> = &ZOOM_TO_FIT_PROPERTY;
}
