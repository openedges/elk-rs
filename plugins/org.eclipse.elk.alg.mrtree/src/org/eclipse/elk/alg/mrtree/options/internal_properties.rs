use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::util::Random;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::alg::mrtree::graph::{TEdgeRef, TNodeRef};

pub type OriginId = usize;

#[derive(Clone)]
pub enum Origin {
    ElkNode(OriginId),
    ElkEdge(OriginId),
    ElkLabel(OriginId),
}

pub static ORIGIN_PROPERTY: LazyLock<Property<Origin>> =
    LazyLock::new(|| Property::new("origin"));
pub static RANDOM_PROPERTY: LazyLock<Property<Random>> =
    LazyLock::new(|| Property::new("random"));
pub static DEPTH_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("DEPTH", 0));
pub static FAN_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("FAN", 0));
pub static DESCENDANTS_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("DESCENDANTS", 0));
pub static ROOT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("ROOT", false));
pub static LEFTNEIGHBOR_PROPERTY: LazyLock<Property<Option<TNodeRef>>> =
    LazyLock::new(|| Property::new("LEFTNEIGHBOR"));
pub static RIGHTNEIGHBOR_PROPERTY: LazyLock<Property<Option<TNodeRef>>> =
    LazyLock::new(|| Property::new("RIGHTNEIGHBOR"));
pub static LEFTSIBLING_PROPERTY: LazyLock<Property<Option<TNodeRef>>> =
    LazyLock::new(|| Property::new("LEFTSIBLING"));
pub static RIGHTSIBLING_PROPERTY: LazyLock<Property<Option<TNodeRef>>> =
    LazyLock::new(|| Property::new("RIGHTSIBLING"));
pub static DUMMY_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("DUMMY", false));
pub static LEVEL_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("LEVEL", 0));
pub static REMOVABLE_EDGES_PROPERTY: LazyLock<Property<Vec<TEdgeRef>>> =
    LazyLock::new(|| Property::with_default("REMOVABLE_EDGES", Vec::new()));
pub static XCOOR_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("XCOOR", 0));
pub static YCOOR_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("YCOOR", 0));
pub static LEVELHEIGHT_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("LEVELHEIGHT", 0.0));
pub static LEVELMIN_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("LEVELMIN", 0.0));
pub static LEVELMAX_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("LEVELMAX", 0.0));
pub static GRAPH_XMIN_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("GRAPH_XMIN", 0.0));
pub static GRAPH_YMIN_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("GRAPH_YMIN", 0.0));
pub static GRAPH_XMAX_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("GRAPH_XMAX", 0.0));
pub static GRAPH_YMAX_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("GRAPH_YMAX", 0.0));
pub static COMPACT_LEVEL_ASCENSION_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("COMPACT_LEVEL_ASCENSION", false));
pub static COMPACT_CONSTRAINTS_PROPERTY: LazyLock<Property<Vec<TNodeRef>>> =
    LazyLock::new(|| Property::with_default("COMPACT_CONSTRAINTS", Vec::new()));
pub static ID_PROPERTY: LazyLock<Property<String>> =
    LazyLock::new(|| Property::with_default("ID", String::new()));
pub static POSITION_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("POSITION", 0));
pub static PRELIM_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("PRELIM", 0.0));
pub static MODIFIER_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("MODIFIER", 0.0));
pub static BB_UPLEFT_PROPERTY: LazyLock<Property<KVector>> =
    LazyLock::new(|| Property::new("boundingBox.upLeft"));
pub static BB_LOWRIGHT_PROPERTY: LazyLock<Property<KVector>> =
    LazyLock::new(|| Property::new("boundingBox.lowRight"));

pub struct InternalProperties;

impl InternalProperties {
    pub const ORIGIN: &'static LazyLock<Property<Origin>> = &ORIGIN_PROPERTY;
    pub const RANDOM: &'static LazyLock<Property<Random>> = &RANDOM_PROPERTY;
    pub const DEPTH: &'static LazyLock<Property<i32>> = &DEPTH_PROPERTY;
    pub const FAN: &'static LazyLock<Property<i32>> = &FAN_PROPERTY;
    pub const DESCENDANTS: &'static LazyLock<Property<i32>> = &DESCENDANTS_PROPERTY;
    pub const ROOT: &'static LazyLock<Property<bool>> = &ROOT_PROPERTY;
    pub const LEFTNEIGHBOR: &'static LazyLock<Property<Option<TNodeRef>>> = &LEFTNEIGHBOR_PROPERTY;
    pub const RIGHTNEIGHBOR: &'static LazyLock<Property<Option<TNodeRef>>> = &RIGHTNEIGHBOR_PROPERTY;
    pub const LEFTSIBLING: &'static LazyLock<Property<Option<TNodeRef>>> = &LEFTSIBLING_PROPERTY;
    pub const RIGHTSIBLING: &'static LazyLock<Property<Option<TNodeRef>>> = &RIGHTSIBLING_PROPERTY;
    pub const DUMMY: &'static LazyLock<Property<bool>> = &DUMMY_PROPERTY;
    pub const LEVEL: &'static LazyLock<Property<i32>> = &LEVEL_PROPERTY;
    pub const REMOVABLE_EDGES: &'static LazyLock<Property<Vec<TEdgeRef>>> = &REMOVABLE_EDGES_PROPERTY;
    pub const XCOOR: &'static LazyLock<Property<i32>> = &XCOOR_PROPERTY;
    pub const YCOOR: &'static LazyLock<Property<i32>> = &YCOOR_PROPERTY;
    pub const LEVELHEIGHT: &'static LazyLock<Property<f64>> = &LEVELHEIGHT_PROPERTY;
    pub const LEVELMIN: &'static LazyLock<Property<f64>> = &LEVELMIN_PROPERTY;
    pub const LEVELMAX: &'static LazyLock<Property<f64>> = &LEVELMAX_PROPERTY;
    pub const GRAPH_XMIN: &'static LazyLock<Property<f64>> = &GRAPH_XMIN_PROPERTY;
    pub const GRAPH_YMIN: &'static LazyLock<Property<f64>> = &GRAPH_YMIN_PROPERTY;
    pub const GRAPH_XMAX: &'static LazyLock<Property<f64>> = &GRAPH_XMAX_PROPERTY;
    pub const GRAPH_YMAX: &'static LazyLock<Property<f64>> = &GRAPH_YMAX_PROPERTY;
    pub const COMPACT_LEVEL_ASCENSION: &'static LazyLock<Property<bool>> =
        &COMPACT_LEVEL_ASCENSION_PROPERTY;
    pub const COMPACT_CONSTRAINTS: &'static LazyLock<Property<Vec<TNodeRef>>> =
        &COMPACT_CONSTRAINTS_PROPERTY;
    pub const ID: &'static LazyLock<Property<String>> = &ID_PROPERTY;
    pub const POSITION: &'static LazyLock<Property<i32>> = &POSITION_PROPERTY;
    pub const PRELIM: &'static LazyLock<Property<f64>> = &PRELIM_PROPERTY;
    pub const MODIFIER: &'static LazyLock<Property<f64>> = &MODIFIER_PROPERTY;
    pub const BB_UPLEFT: &'static LazyLock<Property<KVector>> = &BB_UPLEFT_PROPERTY;
    pub const BB_LOWRIGHT: &'static LazyLock<Property<KVector>> = &BB_LOWRIGHT_PROPERTY;
}
