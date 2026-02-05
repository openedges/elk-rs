use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

pub static ROOT_NODE_PROPERTY: LazyLock<Property<usize>> = LazyLock::new(|| Property::new("root"));

pub struct InternalProperties;

impl InternalProperties {
    pub const ROOT_NODE: &'static LazyLock<Property<usize>> = &ROOT_NODE_PROPERTY;
}
