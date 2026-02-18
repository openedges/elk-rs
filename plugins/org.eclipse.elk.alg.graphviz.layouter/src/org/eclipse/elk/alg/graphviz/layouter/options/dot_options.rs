use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use super::graphviz_options::GraphvizOptions;

pub struct DotOptions;

impl DotOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.graphviz.dot";

    pub const ADAPT_PORT_POSITIONS: &'static LazyLock<Property<bool>> =
        GraphvizOptions::ADAPT_PORT_POSITIONS;
    pub const CONCENTRATE: &'static LazyLock<Property<bool>> = GraphvizOptions::CONCENTRATE;
    pub const EPSILON: &'static LazyLock<Property<f64>> = GraphvizOptions::EPSILON;
    pub const ITERATIONS_FACTOR: &'static LazyLock<Property<f64>> =
        GraphvizOptions::ITERATIONS_FACTOR;
    pub const LABEL_ANGLE: &'static LazyLock<Property<f64>> = GraphvizOptions::LABEL_ANGLE;
    pub const LABEL_DISTANCE: &'static LazyLock<Property<f64>> = GraphvizOptions::LABEL_DISTANCE;
    pub const LAYER_SPACING_FACTOR: &'static LazyLock<Property<f64>> =
        GraphvizOptions::LAYER_SPACING_FACTOR;
    pub const MAXITER: &'static LazyLock<Property<i32>> = GraphvizOptions::MAXITER;
    pub const NEATO_MODEL: &'static LazyLock<Property<org_eclipse_elk_alg_graphviz_dot::org::eclipse::elk::alg::graphviz::dot::transform::NeatoModel>> =
        GraphvizOptions::NEATO_MODEL;
    pub const OVERLAP_MODE: &'static LazyLock<Property<org_eclipse_elk_alg_graphviz_dot::org::eclipse::elk::alg::graphviz::dot::transform::OverlapMode>> =
        GraphvizOptions::OVERLAP_MODE;

    pub const PADDING: &'static LazyLock<
        Property<org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding>,
    > = GraphvizOptions::PADDING;
    pub const DIRECTION: &'static LazyLock<
        Property<org_eclipse_elk_core::org::eclipse::elk::core::options::Direction>,
    > = GraphvizOptions::DIRECTION;
    pub const SPACING_NODE_NODE: &'static LazyLock<Property<f64>> =
        GraphvizOptions::SPACING_NODE_NODE;
    pub const SPACING_EDGE_LABEL: &'static LazyLock<Property<f64>> =
        GraphvizOptions::SPACING_EDGE_LABEL;
    pub const NODE_SIZE_CONSTRAINTS: &'static LazyLock<
        Property<
            org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet<
                org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint,
            >,
        >,
    > = GraphvizOptions::NODE_SIZE_CONSTRAINTS;
    pub const NODE_SIZE_OPTIONS: &'static LazyLock<
        Property<
            org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet<
                org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions,
            >,
        >,
    > = GraphvizOptions::NODE_SIZE_OPTIONS;
    pub const EDGE_ROUTING: &'static LazyLock<
        Property<org_eclipse_elk_core::org::eclipse::elk::core::options::EdgeRouting>,
    > = GraphvizOptions::EDGE_ROUTING;
    pub const DEBUG_MODE: &'static LazyLock<Property<bool>> = GraphvizOptions::DEBUG_MODE;
    pub const HIERARCHY_HANDLING: &'static LazyLock<
        Property<org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling>,
    > = GraphvizOptions::HIERARCHY_HANDLING;
}
