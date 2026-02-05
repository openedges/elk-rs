use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::node_label_placement::NodeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_label_placement::PortLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;

use crate::org::eclipse::elk::alg::force::stress::Dimension;

pub struct StressOptions;

pub static FIXED_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.stress.fixed", false));

pub static DESIRED_EDGE_LENGTH_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.stress.desiredEdgeLength", 100.0));

pub static DIMENSION_PROPERTY: LazyLock<Property<Dimension>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.stress.dimension", Dimension::XY));

pub static EPSILON_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.stress.epsilon", 10e-4));

pub static ITERATION_LIMIT_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.stress.iterationLimit", i32::MAX));

impl StressOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.stress";

    pub const FIXED: &'static LazyLock<Property<bool>> = &FIXED_PROPERTY;
    pub const DESIRED_EDGE_LENGTH: &'static LazyLock<Property<f64>> = &DESIRED_EDGE_LENGTH_PROPERTY;
    pub const DIMENSION: &'static LazyLock<Property<Dimension>> = &DIMENSION_PROPERTY;
    pub const EPSILON: &'static LazyLock<Property<f64>> = &EPSILON_PROPERTY;
    pub const ITERATION_LIMIT: &'static LazyLock<Property<i32>> = &ITERATION_LIMIT_PROPERTY;

    pub const INTERACTIVE: &'static LazyLock<Property<bool>> = CoreOptions::INTERACTIVE;
    pub const EDGE_LABELS_INLINE: &'static LazyLock<Property<bool>> = CoreOptions::EDGE_LABELS_INLINE;
    pub const OMIT_NODE_MICRO_LAYOUT: &'static LazyLock<Property<bool>> = CoreOptions::OMIT_NODE_MICRO_LAYOUT;
    pub const NODE_SIZE_CONSTRAINTS: &'static LazyLock<Property<EnumSet<SizeConstraint>>> =
        CoreOptions::NODE_SIZE_CONSTRAINTS;
    pub const NODE_SIZE_MINIMUM: &'static LazyLock<Property<KVector>> = CoreOptions::NODE_SIZE_MINIMUM;
    pub const NODE_SIZE_OPTIONS: &'static LazyLock<Property<EnumSet<SizeOptions>>> =
        CoreOptions::NODE_SIZE_OPTIONS;
    pub const NODE_LABELS_PLACEMENT: &'static LazyLock<Property<EnumSet<NodeLabelPlacement>>> =
        CoreOptions::NODE_LABELS_PLACEMENT;
    pub const PORT_LABELS_PLACEMENT: &'static LazyLock<Property<EnumSet<PortLabelPlacement>>> =
        CoreOptions::PORT_LABELS_PLACEMENT;
}
