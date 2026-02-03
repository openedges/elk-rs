use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

use crate::org::eclipse::elk::alg::layered::graph::LLabelRef;
use crate::org::eclipse::elk::alg::layered::options::{
    EdgeConstraint, GraphProperties, InLayerConstraint,
};

pub struct InternalProperties;

pub static REPRESENTED_LABELS_PROPERTY: LazyLock<Property<Vec<LLabelRef>>> =
    LazyLock::new(|| Property::new("representedLabels"));

pub static REVERSED_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("reversed", false));

pub static INPUT_COLLECT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("inputCollect", false));

pub static OUTPUT_COLLECT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("outputCollect", false));

pub static EDGE_CONSTRAINT_PROPERTY: LazyLock<Property<EdgeConstraint>> =
    LazyLock::new(|| Property::with_default("edgeConstraint", EdgeConstraint::None));

pub static IN_LAYER_CONSTRAINT_PROPERTY: LazyLock<Property<InLayerConstraint>> =
    LazyLock::new(|| Property::with_default("inLayerConstraint", InLayerConstraint::None));

pub static GRAPH_PROPERTIES_PROPERTY: LazyLock<Property<EnumSet<GraphProperties>>> =
    LazyLock::new(|| {
        ElkReflect::register(
            Some(EnumSet::<GraphProperties>::none_of),
            Some(|v: &EnumSet<GraphProperties>| v.clone()),
        );
        Property::with_default("graphProperties", EnumSet::none_of())
    });

pub static EXT_PORT_SIDE_PROPERTY: LazyLock<Property<PortSide>> =
    LazyLock::new(|| Property::with_default("externalPortSide", PortSide::Undefined));

pub static EXT_PORT_SIZE_PROPERTY: LazyLock<Property<KVector>> =
    LazyLock::new(|| Property::with_default("externalPortSize", KVector::new()));

pub static PORT_RATIO_OR_POSITION_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("portRatioOrPosition", 0.0));

pub static MODEL_ORDER_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::new("modelOrder"));

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
    pub const EDGE_CONSTRAINT: &'static LazyLock<Property<EdgeConstraint>> = &EDGE_CONSTRAINT_PROPERTY;
    pub const IN_LAYER_CONSTRAINT: &'static LazyLock<Property<InLayerConstraint>> =
        &IN_LAYER_CONSTRAINT_PROPERTY;
    pub const GRAPH_PROPERTIES: &'static LazyLock<Property<EnumSet<GraphProperties>>> =
        &GRAPH_PROPERTIES_PROPERTY;
    pub const EXT_PORT_SIDE: &'static LazyLock<Property<PortSide>> = &EXT_PORT_SIDE_PROPERTY;
    pub const EXT_PORT_SIZE: &'static LazyLock<Property<KVector>> = &EXT_PORT_SIZE_PROPERTY;
    pub const PORT_RATIO_OR_POSITION: &'static LazyLock<Property<f64>> =
        &PORT_RATIO_OR_POSITION_PROPERTY;
    pub const MODEL_ORDER: &'static LazyLock<Property<i32>> = &MODEL_ORDER_PROPERTY;
    pub const TARJAN_LOWLINK: &'static LazyLock<Property<i32>> = &TARJAN_LOWLINK_PROPERTY;
    pub const TARJAN_ID: &'static LazyLock<Property<i32>> = &TARJAN_ID_PROPERTY;
    pub const TARJAN_ON_STACK: &'static LazyLock<Property<bool>> = &TARJAN_ON_STACK_PROPERTY;
    pub const IS_PART_OF_CYCLE: &'static LazyLock<Property<bool>> = &IS_PART_OF_CYCLE_PROPERTY;
}
