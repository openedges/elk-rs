use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::alg::layered::graph::LLabelRef;

pub struct InternalProperties;

pub static REPRESENTED_LABELS_PROPERTY: LazyLock<Property<Vec<LLabelRef>>> =
    LazyLock::new(|| Property::new("representedLabels"));

pub static REVERSED_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("reversed", false));

pub static INPUT_COLLECT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("inputCollect", false));

pub static OUTPUT_COLLECT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("outputCollect", false));

impl InternalProperties {
    pub const REPRESENTED_LABELS: &'static LazyLock<Property<Vec<LLabelRef>>> =
        &REPRESENTED_LABELS_PROPERTY;
    pub const REVERSED: &'static LazyLock<Property<bool>> = &REVERSED_PROPERTY;
    pub const INPUT_COLLECT: &'static LazyLock<Property<bool>> = &INPUT_COLLECT_PROPERTY;
    pub const OUTPUT_COLLECT: &'static LazyLock<Property<bool>> = &OUTPUT_COLLECT_PROPERTY;
}
