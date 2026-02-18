use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkPadding, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::node_label_placement::NodeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_label_placement::PortLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::topdown_node_types::TopdownNodeTypes;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, IndividualSpacings};

use super::{EdgeRoutingMode, GraphProperties, OrderWeighting, TreeifyingOrder};

pub struct MrTreeOptions;

pub static GRAPH_PROPERTIES_PROPERTY: LazyLock<Property<EnumSet<GraphProperties>>> =
    LazyLock::new(|| {
        Property::with_default("org.eclipse.elk.mrtree.graphProperties", EnumSet::none_of())
    });

pub static COMPACTION_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.mrtree.compaction", false));

pub static EDGE_END_TEXTURE_LENGTH_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.mrtree.edgeEndTextureLength", 7.0));

pub static TREE_LEVEL_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.mrtree.treeLevel", 0));

pub static POSITION_CONSTRAINT_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.mrtree.positionConstraint", -1));

pub static WEIGHTING_PROPERTY: LazyLock<Property<OrderWeighting>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.mrtree.weighting",
        OrderWeighting::ModelOrder,
    )
});

pub static EDGE_ROUTING_MODE_PROPERTY: LazyLock<Property<EdgeRoutingMode>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.mrtree.edgeRoutingMode",
        EdgeRoutingMode::AvoidOverlap,
    )
});

pub static SEARCH_ORDER_PROPERTY: LazyLock<Property<TreeifyingOrder>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.mrtree.searchOrder", TreeifyingOrder::Dfs)
});

impl MrTreeOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.mrtree";

    pub const GRAPH_PROPERTIES: &'static LazyLock<Property<EnumSet<GraphProperties>>> =
        &GRAPH_PROPERTIES_PROPERTY;
    pub const COMPACTION: &'static LazyLock<Property<bool>> = &COMPACTION_PROPERTY;
    pub const EDGE_END_TEXTURE_LENGTH: &'static LazyLock<Property<f64>> =
        &EDGE_END_TEXTURE_LENGTH_PROPERTY;
    pub const TREE_LEVEL: &'static LazyLock<Property<i32>> = &TREE_LEVEL_PROPERTY;
    pub const POSITION_CONSTRAINT: &'static LazyLock<Property<i32>> = &POSITION_CONSTRAINT_PROPERTY;
    pub const WEIGHTING: &'static LazyLock<Property<OrderWeighting>> = &WEIGHTING_PROPERTY;
    pub const EDGE_ROUTING_MODE: &'static LazyLock<Property<EdgeRoutingMode>> =
        &EDGE_ROUTING_MODE_PROPERTY;
    pub const SEARCH_ORDER: &'static LazyLock<Property<TreeifyingOrder>> = &SEARCH_ORDER_PROPERTY;

    pub const PADDING: &'static LazyLock<Property<ElkPadding>> = CoreOptions::PADDING;
    pub const SPACING_NODE_NODE: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_NODE_NODE;
    pub const SPACING_EDGE_NODE: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_EDGE_NODE;
    pub const SPACING_INDIVIDUAL: &'static LazyLock<Property<IndividualSpacings>> =
        CoreOptions::SPACING_INDIVIDUAL;
    pub const ASPECT_RATIO: &'static LazyLock<Property<f64>> = CoreOptions::ASPECT_RATIO;
    pub const PRIORITY: &'static LazyLock<Property<i32>> = CoreOptions::PRIORITY;
    pub const SEPARATE_CONNECTED_COMPONENTS: &'static LazyLock<Property<bool>> =
        CoreOptions::SEPARATE_CONNECTED_COMPONENTS;
    pub const DEBUG_MODE: &'static LazyLock<Property<bool>> = CoreOptions::DEBUG_MODE;
    pub const DIRECTION: &'static LazyLock<Property<Direction>> = CoreOptions::DIRECTION;
    pub const INTERACTIVE: &'static LazyLock<Property<bool>> = CoreOptions::INTERACTIVE;
    pub const INTERACTIVE_LAYOUT: &'static LazyLock<Property<bool>> =
        CoreOptions::INTERACTIVE_LAYOUT;
    pub const NODE_SIZE_CONSTRAINTS: &'static LazyLock<Property<EnumSet<SizeConstraint>>> =
        CoreOptions::NODE_SIZE_CONSTRAINTS;
    pub const NODE_SIZE_FIXED_GRAPH_SIZE: &'static LazyLock<Property<bool>> =
        CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE;
    pub const NODE_SIZE_MINIMUM: &'static LazyLock<Property<KVector>> =
        CoreOptions::NODE_SIZE_MINIMUM;
    pub const NODE_SIZE_OPTIONS: &'static LazyLock<Property<EnumSet<SizeOptions>>> =
        CoreOptions::NODE_SIZE_OPTIONS;
    pub const NODE_LABELS_PLACEMENT: &'static LazyLock<Property<EnumSet<NodeLabelPlacement>>> =
        CoreOptions::NODE_LABELS_PLACEMENT;
    pub const PORT_LABELS_PLACEMENT: &'static LazyLock<Property<EnumSet<PortLabelPlacement>>> =
        CoreOptions::PORT_LABELS_PLACEMENT;
    pub const OMIT_NODE_MICRO_LAYOUT: &'static LazyLock<Property<bool>> =
        CoreOptions::OMIT_NODE_MICRO_LAYOUT;
    pub const TOPDOWN_LAYOUT: &'static LazyLock<Property<bool>> = CoreOptions::TOPDOWN_LAYOUT;
    pub const TOPDOWN_SCALE_FACTOR: &'static LazyLock<Property<f64>> =
        CoreOptions::TOPDOWN_SCALE_FACTOR;
    pub const TOPDOWN_HIERARCHICAL_NODE_WIDTH: &'static LazyLock<Property<f64>> =
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH;
    pub const TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO: &'static LazyLock<Property<f64>> =
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO;
    pub const TOPDOWN_NODE_TYPE: &'static LazyLock<Property<TopdownNodeTypes>> =
        CoreOptions::TOPDOWN_NODE_TYPE;
}
