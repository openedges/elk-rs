use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::core::math::{ElkPadding, KVector, KVectorChain};
use crate::org::eclipse::elk::core::options::CoreOptions;
use crate::org::eclipse::elk::core::options::SizeConstraint;
use crate::org::eclipse::elk::core::util::EnumSet;

pub struct FixedLayouterOptions;

impl FixedLayouterOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.fixed";

    pub const POSITION: &'static LazyLock<Property<KVector>> = CoreOptions::POSITION;
    pub const BEND_POINTS: &'static LazyLock<Property<KVectorChain>> = CoreOptions::BEND_POINTS;
    pub const NODE_SIZE_CONSTRAINTS: &'static LazyLock<Property<EnumSet<SizeConstraint>>> =
        CoreOptions::NODE_SIZE_CONSTRAINTS;
    pub const NODE_SIZE_MINIMUM: &'static LazyLock<Property<KVector>> =
        CoreOptions::NODE_SIZE_MINIMUM;
    pub const NODE_SIZE_FIXED_GRAPH_SIZE: &'static LazyLock<Property<bool>> =
        CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE;
    pub const PADDING: &'static LazyLock<Property<ElkPadding>> = CoreOptions::PADDING;
}
