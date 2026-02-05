use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_core::org::eclipse::elk::core::math::elk_padding::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::node_label_placement::NodeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_label_placement::PortLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::topdown_node_types::TopdownNodeTypes;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;

use super::ForceModelStrategy;

pub struct ForceOptions;

pub static MODEL_PROPERTY: LazyLock<Property<ForceModelStrategy>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.force.model", ForceModelStrategy::FruchtermanReingold)
});

pub static ITERATIONS_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.force.iterations", 300));

pub static REPULSIVE_POWER_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.force.repulsivePower", 0));

pub static TEMPERATURE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.force.temperature", 0.001));

pub static REPULSION_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.force.repulsion", 5.0));

impl ForceOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.force";

    pub const MODEL: &'static LazyLock<Property<ForceModelStrategy>> = &MODEL_PROPERTY;
    pub const ITERATIONS: &'static LazyLock<Property<i32>> = &ITERATIONS_PROPERTY;
    pub const REPULSIVE_POWER: &'static LazyLock<Property<i32>> = &REPULSIVE_POWER_PROPERTY;
    pub const TEMPERATURE: &'static LazyLock<Property<f64>> = &TEMPERATURE_PROPERTY;
    pub const REPULSION: &'static LazyLock<Property<f64>> = &REPULSION_PROPERTY;

    pub const PRIORITY: &'static LazyLock<Property<i32>> = CoreOptions::PRIORITY;
    pub const SPACING_NODE_NODE: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_NODE_NODE;
    pub const SPACING_EDGE_LABEL: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_EDGE_LABEL;
    pub const ASPECT_RATIO: &'static LazyLock<Property<f64>> = CoreOptions::ASPECT_RATIO;
    pub const RANDOM_SEED: &'static LazyLock<Property<i32>> = CoreOptions::RANDOM_SEED;
    pub const SEPARATE_CONNECTED_COMPONENTS: &'static LazyLock<Property<bool>> =
        CoreOptions::SEPARATE_CONNECTED_COMPONENTS;
    pub const PADDING: &'static LazyLock<Property<ElkPadding>> = CoreOptions::PADDING;
    pub const INTERACTIVE: &'static LazyLock<Property<bool>> = CoreOptions::INTERACTIVE;
    pub const PORT_CONSTRAINTS: &'static LazyLock<Property<PortConstraints>> = CoreOptions::PORT_CONSTRAINTS;
    pub const EDGE_LABELS_INLINE: &'static LazyLock<Property<bool>> = CoreOptions::EDGE_LABELS_INLINE;
    pub const OMIT_NODE_MICRO_LAYOUT: &'static LazyLock<Property<bool>> = CoreOptions::OMIT_NODE_MICRO_LAYOUT;
    pub const NODE_SIZE_FIXED_GRAPH_SIZE: &'static LazyLock<Property<bool>> =
        CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE;
    pub const NODE_SIZE_OPTIONS: &'static LazyLock<Property<EnumSet<SizeOptions>>> =
        CoreOptions::NODE_SIZE_OPTIONS;
    pub const NODE_SIZE_CONSTRAINTS: &'static LazyLock<Property<EnumSet<SizeConstraint>>> =
        CoreOptions::NODE_SIZE_CONSTRAINTS;
    pub const NODE_LABELS_PLACEMENT: &'static LazyLock<Property<EnumSet<NodeLabelPlacement>>> =
        CoreOptions::NODE_LABELS_PLACEMENT;
    pub const PORT_LABELS_PLACEMENT: &'static LazyLock<Property<EnumSet<PortLabelPlacement>>> =
        CoreOptions::PORT_LABELS_PLACEMENT;
    pub const TOPDOWN_LAYOUT: &'static LazyLock<Property<bool>> = CoreOptions::TOPDOWN_LAYOUT;
    pub const TOPDOWN_SCALE_FACTOR: &'static LazyLock<Property<f64>> = CoreOptions::TOPDOWN_SCALE_FACTOR;
    pub const TOPDOWN_HIERARCHICAL_NODE_WIDTH: &'static LazyLock<Property<f64>> =
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH;
    pub const TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO: &'static LazyLock<Property<f64>> =
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO;
    pub const TOPDOWN_NODE_TYPE: &'static LazyLock<Property<TopdownNodeTypes>> =
        CoreOptions::TOPDOWN_NODE_TYPE;
    pub const NODE_SIZE_MINIMUM: &'static LazyLock<Property<KVector>> = CoreOptions::NODE_SIZE_MINIMUM;
}
