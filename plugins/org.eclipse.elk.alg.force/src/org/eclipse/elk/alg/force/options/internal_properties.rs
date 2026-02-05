use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::util::Random;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

pub type OriginId = usize;

#[derive(Clone, Debug)]
pub enum Origin {
    ElkNode(OriginId),
    ElkEdge(OriginId),
    ElkLabel(OriginId),
}

pub static ORIGIN_PROPERTY: LazyLock<Property<Origin>> = LazyLock::new(|| Property::new("origin"));

pub static RANDOM_PROPERTY: LazyLock<Property<Random>> = LazyLock::new(|| Property::new("random"));

pub static BB_UPLEFT_PROPERTY: LazyLock<Property<KVector>> =
    LazyLock::new(|| Property::new("boundingBox.upLeft"));

pub static BB_LOWRIGHT_PROPERTY: LazyLock<Property<KVector>> =
    LazyLock::new(|| Property::new("boundingBox.lowRight"));

pub struct InternalProperties;

impl InternalProperties {
    pub const ORIGIN: &'static LazyLock<Property<Origin>> = &ORIGIN_PROPERTY;
    pub const RANDOM: &'static LazyLock<Property<Random>> = &RANDOM_PROPERTY;
    pub const BB_UPLEFT: &'static LazyLock<Property<KVector>> = &BB_UPLEFT_PROPERTY;
    pub const BB_LOWRIGHT: &'static LazyLock<Property<KVector>> = &BB_LOWRIGHT_PROPERTY;
}
