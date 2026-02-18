use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::topdown_node_types::TopdownNodeTypes;

use crate::org::eclipse::elk::alg::topdownpacking::node_arrangement_strategy::NodeArrangementStrategy;
use crate::org::eclipse::elk::alg::topdownpacking::whitespace_elimination_strategy::WhitespaceEliminationStrategy;

pub struct TopdownpackingOptions;

pub static NODE_ARRANGEMENT_STRATEGY_PROPERTY: LazyLock<Property<NodeArrangementStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.topdownpacking.nodeArrangement.strategy",
            NodeArrangementStrategy::LeftRightTopDownNodePlacer,
        )
    });

pub static WHITESPACE_ELIMINATION_STRATEGY_PROPERTY: LazyLock<
    Property<WhitespaceEliminationStrategy>,
> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.topdownpacking.whitespaceElimination.strategy",
        WhitespaceEliminationStrategy::BottomRowEqualWhitespaceEliminator,
    )
});

impl TopdownpackingOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.topdownpacking";

    pub const NODE_ARRANGEMENT_STRATEGY: &'static LazyLock<Property<NodeArrangementStrategy>> =
        &NODE_ARRANGEMENT_STRATEGY_PROPERTY;
    pub const WHITESPACE_ELIMINATION_STRATEGY: &'static LazyLock<
        Property<WhitespaceEliminationStrategy>,
    > = &WHITESPACE_ELIMINATION_STRATEGY_PROPERTY;

    pub const PADDING: &'static LazyLock<Property<ElkPadding>> = CoreOptions::PADDING;
    pub const SPACING_NODE_NODE: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_NODE_NODE;
    pub const TOPDOWN_HIERARCHICAL_NODE_WIDTH: &'static LazyLock<Property<f64>> =
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH;
    pub const TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO: &'static LazyLock<Property<f64>> =
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO;
    pub const TOPDOWN_LAYOUT: &'static LazyLock<Property<bool>> = CoreOptions::TOPDOWN_LAYOUT;
    pub const TOPDOWN_NODE_TYPE: &'static LazyLock<Property<TopdownNodeTypes>> =
        CoreOptions::TOPDOWN_NODE_TYPE;
}
