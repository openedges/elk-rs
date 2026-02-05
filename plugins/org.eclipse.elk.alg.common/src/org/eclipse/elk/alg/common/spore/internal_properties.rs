use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

pub struct InternalProperties;

pub static DEBUG_SVG_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("debugSVG", false));

pub static OVERLAPS_EXISTED_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("overlapsExisted", true));

impl InternalProperties {
    pub const FUZZINESS: f64 = 0.0001;

    pub const DEBUG_SVG: &'static LazyLock<Property<bool>> = &DEBUG_SVG_PROPERTY;
    pub const OVERLAPS_EXISTED: &'static LazyLock<Property<bool>> = &OVERLAPS_EXISTED_PROPERTY;
}
