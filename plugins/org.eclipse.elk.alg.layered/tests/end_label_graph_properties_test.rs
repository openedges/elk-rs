use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_importer::ElkGraphImporter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{GraphProperties, InternalProperties};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkGraphElementRef,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

#[test]
fn graph_properties_include_end_labels() {
    initialize_plain_java_layout();
    let root = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(root.clone()));
    let n2 = ElkGraphUtil::create_node(Some(root.clone()));
    let edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1),
        ElkConnectableShapeRef::Node(n2),
    );
    let label = ElkGraphUtil::create_label_with_text("tail", Some(ElkGraphElementRef::Edge(edge)));
    {
        let mut label_mut = label.borrow_mut();
        label_mut
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::EDGE_LABELS_PLACEMENT, Some(EdgeLabelPlacement::Tail));
    }
    LayoutMetaDataService::get_instance();
    let mut origin_store = OriginStore::new();
    let mut importer = ElkGraphImporter::new(&mut origin_store);
    let lgraph = importer.import_graph(&root);

    let graph_props = lgraph
        .lock()
        .ok()
        .and_then(|mut guard| guard.get_property(InternalProperties::GRAPH_PROPERTIES))
        .unwrap_or_else(EnumSet::none_of);

    assert!(graph_props.contains(&GraphProperties::EndLabels));
}
