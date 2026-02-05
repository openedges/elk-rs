use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;

use crate::org::eclipse::elk::alg::disco::graph::DCGraph;
use crate::org::eclipse::elk::alg::disco::options::CompactionStrategy;
use crate::org::eclipse::elk::alg::disco::structures::DCPolyomino;

pub struct DisCoOptions;

pub static COMPONENT_COMPACTION_STRATEGY_PROPERTY: LazyLock<Property<CompactionStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.disco.componentCompaction.strategy",
            CompactionStrategy::Polyomino,
        )
    });

pub static COMPONENT_COMPACTION_COMPONENT_LAYOUT_ALGORITHM_PROPERTY: LazyLock<Property<String>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.disco.componentCompaction.componentLayoutAlgorithm"));

pub static DEBUG_DISCO_GRAPH_PROPERTY: LazyLock<Property<DCGraph>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.disco.debug.discoGraph"));

pub static DEBUG_DISCO_POLYS_PROPERTY: LazyLock<Property<Vec<DCPolyomino>>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.disco.debug.discoPolys"));

impl DisCoOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.disco";

    pub const COMPONENT_COMPACTION_STRATEGY: &'static LazyLock<Property<CompactionStrategy>> =
        &COMPONENT_COMPACTION_STRATEGY_PROPERTY;
    pub const COMPONENT_COMPACTION_COMPONENT_LAYOUT_ALGORITHM: &'static LazyLock<Property<String>> =
        &COMPONENT_COMPACTION_COMPONENT_LAYOUT_ALGORITHM_PROPERTY;
    pub const DEBUG_DISCO_GRAPH: &'static LazyLock<Property<DCGraph>> = &DEBUG_DISCO_GRAPH_PROPERTY;
    pub const DEBUG_DISCO_POLYS: &'static LazyLock<Property<Vec<DCPolyomino>>> =
        &DEBUG_DISCO_POLYS_PROPERTY;

    pub const EDGE_THICKNESS: &'static LazyLock<Property<f64>> = CoreOptions::EDGE_THICKNESS;
    pub const SPACING_COMPONENT_COMPONENT: &'static LazyLock<Property<f64>> =
        CoreOptions::SPACING_COMPONENT_COMPONENT;
    pub const ASPECT_RATIO: &'static LazyLock<Property<f64>> = CoreOptions::ASPECT_RATIO;
    pub const PADDING: &'static LazyLock<Property<ElkPadding>> = CoreOptions::PADDING;
}
