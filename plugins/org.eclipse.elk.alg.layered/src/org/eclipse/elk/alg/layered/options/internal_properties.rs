use std::hash::{Hash, Hasher};
use std::sync::{Arc, LazyLock, Mutex};

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::nodespacing::LabelCell;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::SharedProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::label_side::LabelSide;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, Random};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

use crate::org::eclipse::elk::alg::layered::compound::CrossHierarchyMap;
use crate::org::eclipse::elk::alg::layered::graph::{
    LGraph, LEdgeRef, LGraphRef, LLabelRef, LNodeRef, LPortRef, NodeRefKey,
};
use crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfLoopHolderRef;
use crate::org::eclipse::elk::alg::layered::graph::transform::l_graph_adapters::LLabelAdapter;
use crate::org::eclipse::elk::alg::layered::p5edges::splines::SplineSegmentRef;
use crate::org::eclipse::elk::alg::layered::options::{
    EdgeConstraint, GraphProperties, InLayerConstraint, Spacings,
};

pub struct InternalProperties;

pub type OriginId = usize;

#[allow(clippy::mutable_key_type)]
#[derive(Clone)]
pub struct PortRefKey(pub LPortRef);

impl PartialEq for PortRefKey {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for PortRefKey {}

impl PartialOrd for PortRefKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PortRefKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_ptr = Arc::as_ptr(&self.0) as usize;
        let other_ptr = Arc::as_ptr(&other.0) as usize;
        self_ptr.cmp(&other_ptr)
    }
}

impl Hash for PortRefKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = Arc::as_ptr(&self.0) as usize;
        ptr.hash(state);
    }
}

#[derive(Clone)]
pub enum Origin {
    ElkNode(OriginId),
    ElkPort(OriginId),
    ElkEdge(OriginId),
    ElkLabel(OriginId),
    LGraph(LGraphRef),
    LNode(LNodeRef),
    LPort(LPortRef),
    LEdge(LEdgeRef),
    LLabel(LLabelRef),
}

pub type EndLabelCell = Arc<Mutex<LabelCell<LLabelAdapter, LLabelRef>>>;
pub type EndLabelMap = std::collections::HashMap<PortRefKey, EndLabelCell>;

pub static ORIGIN_PROPERTY: LazyLock<Property<Origin>> =
    LazyLock::new(|| Property::new("origin"));

pub static REPRESENTED_LABELS_PROPERTY: LazyLock<Property<Vec<LLabelRef>>> =
    LazyLock::new(|| Property::new("representedLabels"));

pub static END_LABELS_PROPERTY: LazyLock<Property<EndLabelMap>> =
    LazyLock::new(|| Property::new("endLabels"));

pub static END_LABEL_EDGE_PROPERTY: LazyLock<Property<LEdgeRef>> =
    LazyLock::new(|| Property::new("endLabel.origin"));

pub static REVERSED_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("reversed", false));

pub static INPUT_COLLECT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("inputCollect", false));

pub static OUTPUT_COLLECT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("outputCollect", false));

pub static INSIDE_CONNECTIONS_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("insideConnections", false));

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

pub static CROSS_HIERARCHY_MAP_PROPERTY: LazyLock<Property<CrossHierarchyMap>> =
    LazyLock::new(|| Property::new("crossHierarchyMap"));

pub static ORIGINAL_LABEL_EDGE_PROPERTY: LazyLock<Property<LEdgeRef>> =
    LazyLock::new(|| Property::new("originalLabelEdge"));

pub static PROCESSORS_PROPERTY: LazyLock<Property<Vec<SharedProcessor<LGraph>>>> =
    LazyLock::new(|| Property::new("processors"));

pub static RANDOM_PROPERTY: LazyLock<Property<Random>> =
    LazyLock::new(|| Property::new("random"));

pub static SPACINGS_PROPERTY: LazyLock<Property<Spacings>> =
    LazyLock::new(|| Property::new("spacings"));

pub static TARGET_OFFSET_PROPERTY: LazyLock<Property<KVector>> =
    LazyLock::new(|| Property::new("targetOffset"));

pub static COORDINATE_SYSTEM_ORIGIN_PROPERTY: LazyLock<Property<LGraphRef>> =
    LazyLock::new(|| Property::new("coordinateOrigin"));

pub static SPLINE_LABEL_SIZE_PROPERTY: LazyLock<Property<KVector>> =
    LazyLock::new(|| Property::with_default("splineLabelSize", KVector::new()));

pub static ORIGINAL_PORT_CONSTRAINTS_PROPERTY: LazyLock<Property<PortConstraints>> =
    LazyLock::new(|| Property::new("originalPortConstraints"));

pub static SPLINE_SURVIVING_EDGE_PROPERTY: LazyLock<Property<LEdgeRef>> =
    LazyLock::new(|| Property::new("splines.survivingEdge"));

pub static SPLINE_ROUTE_START_PROPERTY: LazyLock<Property<Vec<SplineSegmentRef>>> =
    LazyLock::new(|| Property::new("splines.route.start"));

pub static SPLINE_EDGE_CHAIN_PROPERTY: LazyLock<Property<Vec<LEdgeRef>>> =
    LazyLock::new(|| Property::new("splines.edgeChain"));

pub static SPLINE_NS_PORT_Y_COORD_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("splines.nsPortY"));

pub static EXT_PORT_SIDE_PROPERTY: LazyLock<Property<PortSide>> =
    LazyLock::new(|| Property::with_default("externalPortSide", PortSide::Undefined));

pub static EXT_PORT_SIZE_PROPERTY: LazyLock<Property<KVector>> =
    LazyLock::new(|| Property::with_default("externalPortSize", KVector::new()));

pub static EXT_PORT_REPLACED_DUMMIES_PROPERTY: LazyLock<Property<Vec<LNodeRef>>> =
    LazyLock::new(|| Property::new("externalPortReplacedDummies"));

pub static EXT_PORT_REPLACED_DUMMY_PROPERTY: LazyLock<Property<LNodeRef>> =
    LazyLock::new(|| Property::new("externalPortReplacedDummy"));

pub static EXT_PORT_CONNECTIONS_PROPERTY: LazyLock<Property<EnumSet<PortSide>>> =
    LazyLock::new(|| {
        Property::with_default("externalPortConnections", EnumSet::none_of())
    });

pub static PORT_RATIO_OR_POSITION_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("portRatioOrPosition", 0.0));

pub static MODEL_ORDER_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::new("modelOrder"));

pub static ORIGINAL_DUMMY_NODE_POSITION_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("originalDummyNodePosition"));

pub static PORT_DUMMY_PROPERTY: LazyLock<Property<LNodeRef>> =
    LazyLock::new(|| Property::new("portDummy"));

pub static IN_LAYER_LAYOUT_UNIT_PROPERTY: LazyLock<Property<LNodeRef>> =
    LazyLock::new(|| Property::new("inLayerLayoutUnit"));

pub static IN_LAYER_SUCCESSOR_CONSTRAINTS_PROPERTY: LazyLock<Property<Vec<LNodeRef>>> =
    LazyLock::new(|| {
        ElkReflect::register_default_clone::<Vec<LNodeRef>>();
        Property::with_default("inLayerSuccessorConstraint", Vec::new())
    });

pub static IN_LAYER_SUCCESSOR_CONSTRAINTS_BETWEEN_NON_DUMMIES_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("inLayerSuccessorConstraintBetweenNonDummies", false));

pub static BARYCENTER_ASSOCIATES_PROPERTY: LazyLock<Property<Vec<LNodeRef>>> =
    LazyLock::new(|| Property::new("barycenterAssociates"));

pub static MAX_MODEL_ORDER_NODES_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("modelOrder.maximum", 0));

pub static CB_NUM_MODEL_ORDER_GROUPS_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("modelOrderGroups.cb.number", 0));

pub static WEIGHT_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("medianHeuristic.weight"));

pub static LONG_EDGE_SOURCE_PROPERTY: LazyLock<Property<LPortRef>> =
    LazyLock::new(|| Property::new("longEdgeSource"));

pub static LONG_EDGE_TARGET_PROPERTY: LazyLock<Property<LPortRef>> =
    LazyLock::new(|| Property::new("longEdgeTarget"));

pub static LONG_EDGE_HAS_LABEL_DUMMIES_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("longEdgeHasLabelDummies", false));

pub static LONG_EDGE_BEFORE_LABEL_DUMMY_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("longEdgeBeforeLabelDummy", false));

pub static LONG_EDGE_TARGET_NODE_PROPERTY: LazyLock<Property<LNodeRef>> =
    LazyLock::new(|| Property::new("longEdgeTargetNode"));

pub static TARGET_NODE_MODEL_ORDER_PROPERTY: LazyLock<Property<std::collections::HashMap<NodeRefKey, i32>>> =
    LazyLock::new(|| Property::new("targetNode.modelOrder"));

pub static FIRST_TRY_WITH_INITIAL_ORDER_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("firstTryWithInitialOrder", false));

pub static SECOND_TRY_WITH_INITIAL_ORDER_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("secondTryWithInitialOrder", false));

pub static TARJAN_LOWLINK_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("tarjan.lowlink", i32::MAX));

pub static TARJAN_ID_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("tarjan.id", -1));

pub static TARJAN_ON_STACK_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("tarjan.onstack", false));

pub static IS_PART_OF_CYCLE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("partOfCycle", false));
pub static CYCLIC_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("cyclic", false));

pub static TOP_COMMENTS_PROPERTY: LazyLock<Property<Vec<LNodeRef>>> =
    LazyLock::new(|| Property::new("TopSideComments"));

pub static BOTTOM_COMMENTS_PROPERTY: LazyLock<Property<Vec<LNodeRef>>> =
    LazyLock::new(|| Property::new("BottomSideComments"));

pub static COMMENT_CONN_PORT_PROPERTY: LazyLock<Property<LPortRef>> =
    LazyLock::new(|| Property::new("CommentConnectionPort"));

pub static HIDDEN_NODES_PROPERTY: LazyLock<Property<Vec<LNodeRef>>> =
    LazyLock::new(|| {
        ElkReflect::register_default_clone::<Vec<LNodeRef>>();
        Property::new("layerConstraints.hiddenNodes")
    });

pub static ORIGINAL_OPPOSITE_PORT_PROPERTY: LazyLock<Property<LPortRef>> =
    LazyLock::new(|| Property::new("layerConstraints.opposidePort"));

pub static PARTITION_DUMMY_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("partitionDummy", false));

pub static LABEL_SIDE_PROPERTY: LazyLock<Property<LabelSide>> =
    LazyLock::new(|| Property::with_default("labelSide", LabelSide::Unknown));

pub static MAX_EDGE_THICKNESS_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("maxEdgeThickness", 0.0));

pub static SELF_LOOP_HOLDER_PROPERTY: LazyLock<Property<SelfLoopHolderRef>> =
    LazyLock::new(|| Property::new("selfLoopHolder"));

pub static COMPOUND_NODE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("compoundNode", false));

impl InternalProperties {
    pub const ORIGIN: &'static LazyLock<Property<Origin>> = &ORIGIN_PROPERTY;
    pub const REPRESENTED_LABELS: &'static LazyLock<Property<Vec<LLabelRef>>> =
        &REPRESENTED_LABELS_PROPERTY;
    pub const END_LABELS: &'static LazyLock<Property<EndLabelMap>> = &END_LABELS_PROPERTY;
    pub const END_LABEL_EDGE: &'static LazyLock<Property<LEdgeRef>> =
        &END_LABEL_EDGE_PROPERTY;
    pub const REVERSED: &'static LazyLock<Property<bool>> = &REVERSED_PROPERTY;
    pub const INPUT_COLLECT: &'static LazyLock<Property<bool>> = &INPUT_COLLECT_PROPERTY;
    pub const OUTPUT_COLLECT: &'static LazyLock<Property<bool>> = &OUTPUT_COLLECT_PROPERTY;
    pub const INSIDE_CONNECTIONS: &'static LazyLock<Property<bool>> = &INSIDE_CONNECTIONS_PROPERTY;
    pub const EDGE_CONSTRAINT: &'static LazyLock<Property<EdgeConstraint>> = &EDGE_CONSTRAINT_PROPERTY;
    pub const IN_LAYER_CONSTRAINT: &'static LazyLock<Property<InLayerConstraint>> =
        &IN_LAYER_CONSTRAINT_PROPERTY;
    pub const GRAPH_PROPERTIES: &'static LazyLock<Property<EnumSet<GraphProperties>>> =
        &GRAPH_PROPERTIES_PROPERTY;
    pub const CROSS_HIERARCHY_MAP: &'static LazyLock<Property<CrossHierarchyMap>> =
        &CROSS_HIERARCHY_MAP_PROPERTY;
    pub const ORIGINAL_LABEL_EDGE: &'static LazyLock<Property<LEdgeRef>> =
        &ORIGINAL_LABEL_EDGE_PROPERTY;
    pub const PROCESSORS: &'static LazyLock<Property<Vec<SharedProcessor<LGraph>>>> =
        &PROCESSORS_PROPERTY;
    pub const RANDOM: &'static LazyLock<Property<Random>> = &RANDOM_PROPERTY;
    pub const SPACINGS: &'static LazyLock<Property<Spacings>> = &SPACINGS_PROPERTY;
    pub const TARGET_OFFSET: &'static LazyLock<Property<KVector>> = &TARGET_OFFSET_PROPERTY;
    pub const COORDINATE_SYSTEM_ORIGIN: &'static LazyLock<Property<LGraphRef>> =
        &COORDINATE_SYSTEM_ORIGIN_PROPERTY;
    pub const SPLINE_LABEL_SIZE: &'static LazyLock<Property<KVector>> = &SPLINE_LABEL_SIZE_PROPERTY;
    pub const ORIGINAL_PORT_CONSTRAINTS: &'static LazyLock<Property<PortConstraints>> =
        &ORIGINAL_PORT_CONSTRAINTS_PROPERTY;
    pub const SPLINE_SURVIVING_EDGE: &'static LazyLock<Property<LEdgeRef>> =
        &SPLINE_SURVIVING_EDGE_PROPERTY;
    pub const SPLINE_ROUTE_START: &'static LazyLock<Property<Vec<SplineSegmentRef>>> =
        &SPLINE_ROUTE_START_PROPERTY;
    pub const SPLINE_EDGE_CHAIN: &'static LazyLock<Property<Vec<LEdgeRef>>> =
        &SPLINE_EDGE_CHAIN_PROPERTY;
    pub const SPLINE_NS_PORT_Y_COORD: &'static LazyLock<Property<f64>> =
        &SPLINE_NS_PORT_Y_COORD_PROPERTY;
    pub const EXT_PORT_SIDE: &'static LazyLock<Property<PortSide>> = &EXT_PORT_SIDE_PROPERTY;
    pub const EXT_PORT_SIZE: &'static LazyLock<Property<KVector>> = &EXT_PORT_SIZE_PROPERTY;
    pub const EXT_PORT_REPLACED_DUMMIES: &'static LazyLock<Property<Vec<LNodeRef>>> =
        &EXT_PORT_REPLACED_DUMMIES_PROPERTY;
    pub const EXT_PORT_REPLACED_DUMMY: &'static LazyLock<Property<LNodeRef>> =
        &EXT_PORT_REPLACED_DUMMY_PROPERTY;
    pub const EXT_PORT_CONNECTIONS: &'static LazyLock<Property<EnumSet<PortSide>>> =
        &EXT_PORT_CONNECTIONS_PROPERTY;
    pub const PORT_RATIO_OR_POSITION: &'static LazyLock<Property<f64>> =
        &PORT_RATIO_OR_POSITION_PROPERTY;
    pub const MODEL_ORDER: &'static LazyLock<Property<i32>> = &MODEL_ORDER_PROPERTY;
    pub const ORIGINAL_DUMMY_NODE_POSITION: &'static LazyLock<Property<f64>> =
        &ORIGINAL_DUMMY_NODE_POSITION_PROPERTY;
    pub const PORT_DUMMY: &'static LazyLock<Property<LNodeRef>> = &PORT_DUMMY_PROPERTY;
    pub const IN_LAYER_LAYOUT_UNIT: &'static LazyLock<Property<LNodeRef>> =
        &IN_LAYER_LAYOUT_UNIT_PROPERTY;
    pub const IN_LAYER_SUCCESSOR_CONSTRAINTS: &'static LazyLock<Property<Vec<LNodeRef>>> =
        &IN_LAYER_SUCCESSOR_CONSTRAINTS_PROPERTY;
    pub const IN_LAYER_SUCCESSOR_CONSTRAINTS_BETWEEN_NON_DUMMIES: &'static LazyLock<Property<bool>> =
        &IN_LAYER_SUCCESSOR_CONSTRAINTS_BETWEEN_NON_DUMMIES_PROPERTY;
    pub const BARYCENTER_ASSOCIATES: &'static LazyLock<Property<Vec<LNodeRef>>> =
        &BARYCENTER_ASSOCIATES_PROPERTY;
    pub const MAX_MODEL_ORDER_NODES: &'static LazyLock<Property<i32>> =
        &MAX_MODEL_ORDER_NODES_PROPERTY;
    pub const CB_NUM_MODEL_ORDER_GROUPS: &'static LazyLock<Property<i32>> =
        &CB_NUM_MODEL_ORDER_GROUPS_PROPERTY;
    pub const WEIGHT: &'static LazyLock<Property<f64>> = &WEIGHT_PROPERTY;
    pub const LONG_EDGE_SOURCE: &'static LazyLock<Property<LPortRef>> = &LONG_EDGE_SOURCE_PROPERTY;
    pub const LONG_EDGE_TARGET: &'static LazyLock<Property<LPortRef>> = &LONG_EDGE_TARGET_PROPERTY;
    pub const LONG_EDGE_HAS_LABEL_DUMMIES: &'static LazyLock<Property<bool>> =
        &LONG_EDGE_HAS_LABEL_DUMMIES_PROPERTY;
    pub const LONG_EDGE_BEFORE_LABEL_DUMMY: &'static LazyLock<Property<bool>> =
        &LONG_EDGE_BEFORE_LABEL_DUMMY_PROPERTY;
    pub const LONG_EDGE_TARGET_NODE: &'static LazyLock<Property<LNodeRef>> =
        &LONG_EDGE_TARGET_NODE_PROPERTY;
    pub const TARGET_NODE_MODEL_ORDER: &'static LazyLock<Property<std::collections::HashMap<NodeRefKey, i32>>> =
        &TARGET_NODE_MODEL_ORDER_PROPERTY;
    pub const FIRST_TRY_WITH_INITIAL_ORDER: &'static LazyLock<Property<bool>> =
        &FIRST_TRY_WITH_INITIAL_ORDER_PROPERTY;
    pub const SECOND_TRY_WITH_INITIAL_ORDER: &'static LazyLock<Property<bool>> =
        &SECOND_TRY_WITH_INITIAL_ORDER_PROPERTY;
    pub const TARJAN_LOWLINK: &'static LazyLock<Property<i32>> = &TARJAN_LOWLINK_PROPERTY;
    pub const TARJAN_ID: &'static LazyLock<Property<i32>> = &TARJAN_ID_PROPERTY;
    pub const TARJAN_ON_STACK: &'static LazyLock<Property<bool>> = &TARJAN_ON_STACK_PROPERTY;
    pub const IS_PART_OF_CYCLE: &'static LazyLock<Property<bool>> = &IS_PART_OF_CYCLE_PROPERTY;
    pub const CYCLIC: &'static LazyLock<Property<bool>> = &CYCLIC_PROPERTY;
    pub const TOP_COMMENTS: &'static LazyLock<Property<Vec<LNodeRef>>> = &TOP_COMMENTS_PROPERTY;
    pub const BOTTOM_COMMENTS: &'static LazyLock<Property<Vec<LNodeRef>>> =
        &BOTTOM_COMMENTS_PROPERTY;
    pub const COMMENT_CONN_PORT: &'static LazyLock<Property<LPortRef>> = &COMMENT_CONN_PORT_PROPERTY;
    pub const HIDDEN_NODES: &'static LazyLock<Property<Vec<LNodeRef>>> = &HIDDEN_NODES_PROPERTY;
    pub const ORIGINAL_OPPOSITE_PORT: &'static LazyLock<Property<LPortRef>> =
        &ORIGINAL_OPPOSITE_PORT_PROPERTY;
    pub const PARTITION_DUMMY: &'static LazyLock<Property<bool>> = &PARTITION_DUMMY_PROPERTY;
    pub const LABEL_SIDE: &'static LazyLock<Property<LabelSide>> = &LABEL_SIDE_PROPERTY;
    pub const MAX_EDGE_THICKNESS: &'static LazyLock<Property<f64>> =
        &MAX_EDGE_THICKNESS_PROPERTY;
    pub const SELF_LOOP_HOLDER: &'static LazyLock<Property<SelfLoopHolderRef>> =
        &SELF_LOOP_HOLDER_PROPERTY;
    pub const COMPOUND_NODE: &'static LazyLock<Property<bool>> = &COMPOUND_NODE_PROPERTY;
}
