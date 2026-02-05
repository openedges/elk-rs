use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::alg::vertiflex::p2relative::OutlineNode;

pub static LEFT_OUTLINE_PROPERTY: LazyLock<Property<OutlineNode>> =
    LazyLock::new(|| Property::new("LEFT_OUTLINE"));
pub static RIGHT_OUTLINE_PROPERTY: LazyLock<Property<OutlineNode>> =
    LazyLock::new(|| Property::new("RIGHT_OUTLINE"));
pub static OUTLINE_MAX_DEPTH_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("OUTLINE_MAX_DEPTH"));
pub static MIN_X_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| Property::new("MIN_X"));
pub static MAX_X_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| Property::new("MAX_X"));
pub static MIN_Y_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| Property::new("MIN_Y"));
pub static MAX_Y_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| Property::new("MAX_Y"));
pub static ROOT_NODE_PROPERTY: LazyLock<Property<usize>> = LazyLock::new(|| Property::new("root"));
pub static EDGE_BEND_HEIGHT_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("EDGE_BEND_HEIGHT"));
pub static NODE_MODEL_ORDER_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::new("Node Model Order"));

pub struct InternalProperties;

impl InternalProperties {
    pub const LEFT_OUTLINE: &'static LazyLock<Property<OutlineNode>> = &LEFT_OUTLINE_PROPERTY;
    pub const RIGHT_OUTLINE: &'static LazyLock<Property<OutlineNode>> = &RIGHT_OUTLINE_PROPERTY;
    pub const OUTLINE_MAX_DEPTH: &'static LazyLock<Property<f64>> = &OUTLINE_MAX_DEPTH_PROPERTY;
    pub const MIN_X: &'static LazyLock<Property<f64>> = &MIN_X_PROPERTY;
    pub const MAX_X: &'static LazyLock<Property<f64>> = &MAX_X_PROPERTY;
    pub const MIN_Y: &'static LazyLock<Property<f64>> = &MIN_Y_PROPERTY;
    pub const MAX_Y: &'static LazyLock<Property<f64>> = &MAX_Y_PROPERTY;
    pub const ROOT_NODE: &'static LazyLock<Property<usize>> = &ROOT_NODE_PROPERTY;
    pub const EDGE_BEND_HEIGHT: &'static LazyLock<Property<f64>> = &EDGE_BEND_HEIGHT_PROPERTY;
    pub const NODE_MODEL_ORDER: &'static LazyLock<Property<i32>> = &NODE_MODEL_ORDER_PROPERTY;
}
