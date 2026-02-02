use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::core::math::ElkPadding;
use crate::org::eclipse::elk::core::options::CoreOptions;

pub struct RandomLayouterOptions;

impl RandomLayouterOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.random";

    pub const PADDING: &'static LazyLock<Property<ElkPadding>> = CoreOptions::PADDING;
    pub const SPACING_NODE_NODE: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_NODE_NODE;
    pub const RANDOM_SEED: &'static LazyLock<Property<i32>> = CoreOptions::RANDOM_SEED;
    pub const ASPECT_RATIO: &'static LazyLock<Property<f64>> = CoreOptions::ASPECT_RATIO;
}
