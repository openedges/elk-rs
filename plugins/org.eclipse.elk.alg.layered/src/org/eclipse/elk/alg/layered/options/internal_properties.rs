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

pub static TARJAN_LOWLINK_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("tarjan.lowlink", i32::MAX));

pub static TARJAN_ID_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("tarjan.id", -1));

pub static TARJAN_ON_STACK_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("tarjan.onstack", false));

pub static IS_PART_OF_CYCLE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("partOfCycle", false));

impl InternalProperties {
    pub const REPRESENTED_LABELS: &'static LazyLock<Property<Vec<LLabelRef>>> =
        &REPRESENTED_LABELS_PROPERTY;
    pub const REVERSED: &'static LazyLock<Property<bool>> = &REVERSED_PROPERTY;
    pub const INPUT_COLLECT: &'static LazyLock<Property<bool>> = &INPUT_COLLECT_PROPERTY;
    pub const OUTPUT_COLLECT: &'static LazyLock<Property<bool>> = &OUTPUT_COLLECT_PROPERTY;
    pub const TARJAN_LOWLINK: &'static LazyLock<Property<i32>> = &TARJAN_LOWLINK_PROPERTY;
    pub const TARJAN_ID: &'static LazyLock<Property<i32>> = &TARJAN_ID_PROPERTY;
    pub const TARJAN_ON_STACK: &'static LazyLock<Property<bool>> = &TARJAN_ON_STACK_PROPERTY;
    pub const IS_PART_OF_CYCLE: &'static LazyLock<Property<bool>> = &IS_PART_OF_CYCLE_PROPERTY;
}
