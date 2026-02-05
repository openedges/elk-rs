use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;

use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::{
    ComponentsProcessor, ElkGraphImporter, FGraph, ForceOptions, IGraphImporter,
};

fn check_simple_graph(graph: &FGraph) {
    assert_eq!(graph.nodes().len(), 3);
    assert_eq!(graph.edges().len(), 2);
    assert_eq!(graph.labels().len(), 2);
}

fn create_elk_graph() -> org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef {
    let elk_graph = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(elk_graph.clone()));
    let n2 = ElkGraphUtil::create_node(Some(elk_graph.clone()));
    let n3 = ElkGraphUtil::create_node(Some(elk_graph.clone()));

    let edge = ElkGraphUtil::create_simple_edge(
        org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Node(n1.clone()),
        org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Node(n2.clone()),
    );
    ElkGraphUtil::create_label_with_text(
        "test",
        Some(org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef::Edge(edge)),
    );

    let edge2 = ElkGraphUtil::create_simple_edge(
        org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Node(n1.clone()),
        org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Node(n3.clone()),
    );
    ElkGraphUtil::create_label_with_text(
        "test2",
        Some(org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef::Edge(edge2)),
    );

    elk_graph
}

fn create_simple_graph() -> FGraph {
    LayoutMetaDataService::get_instance();
    let mut importer = ElkGraphImporter::new();
    importer.import_graph(&create_elk_graph()).expect("import graph")
}

fn create_two_components_graph() -> FGraph {
    LayoutMetaDataService::get_instance();
    let g1 = create_elk_graph();
    let g2 = create_elk_graph();
    let graph = ElkGraphUtil::create_graph();

    let children1: Vec<_> = {
        let mut g1_mut = g1.borrow_mut();
        g1_mut.children().iter().cloned().collect()
    };
    let children2: Vec<_> = {
        let mut g2_mut = g2.borrow_mut();
        g2_mut.children().iter().cloned().collect()
    };

    {
        let mut graph_mut = graph.borrow_mut();
        for child in children1 {
            graph_mut.children().add(child);
        }
        for child in children2 {
            graph_mut.children().add(child);
        }
    }

    let mut importer = ElkGraphImporter::new();
    importer.import_graph(&graph).expect("import graph")
}

#[test]
fn test_import() {
    let graph = create_simple_graph();
    check_simple_graph(&graph);
}

#[test]
fn test_separate_connected_components() {
    let mut graph = create_two_components_graph();
    graph.set_property(ForceOptions::SEPARATE_CONNECTED_COMPONENTS, Some(true));

    let cp = ComponentsProcessor::new();
    let graphs = cp.split(graph);
    assert_eq!(graphs.len(), 2);
    for graph in graphs {
        check_simple_graph(&graph);
    }
}

#[test]
fn test_do_not_separate_connected_components() {
    let mut graph = create_two_components_graph();
    graph.set_property(ForceOptions::SEPARATE_CONNECTED_COMPONENTS, Some(false));

    let cp = ComponentsProcessor::new();
    let graphs = cp.split(graph);
    assert_eq!(graphs.len(), 1);

    let graph = &graphs[0];
    assert_eq!(graph.nodes().len(), 6);
    assert_eq!(graph.edges().len(), 4);
    assert_eq!(graph.labels().len(), 4);
}
