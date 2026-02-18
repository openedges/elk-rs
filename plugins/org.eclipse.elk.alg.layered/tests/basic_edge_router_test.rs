use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredMetaDataProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_util::ElkUtil;
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkNodeRef,
};

const TOLERANCE: f64 = 0.05;

#[test]
fn orthogonal_edges_are_axis_aligned() {
    init_layered_options();
    let graph = create_simple_graph();
    set_graph_property(&graph, CoreOptions::EDGE_ROUTING, EdgeRouting::Orthogonal);

    let mut provider = LayeredLayoutProvider::new();
    provider.layout(&graph, &mut BasicProgressMonitor::new());

    let edges: Vec<ElkEdgeRef> = graph
        .borrow_mut()
        .contained_edges()
        .iter()
        .cloned()
        .collect();

    for (index, edge) in edges.iter().enumerate() {
        let sections: Vec<_> = edge.borrow_mut().sections().iter().cloned().collect();
        for section in sections {
            let route = ElkUtil::create_vector_chain(&section);
            check_edge_route_is_orthogonal(index, &route);
        }
    }
}

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn create_simple_graph() -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    let node1 = ElkGraphUtil::create_node(Some(graph.clone()));
    let node2 = ElkGraphUtil::create_node(Some(graph.clone()));
    let node3 = ElkGraphUtil::create_node(Some(graph.clone()));

    set_dimensions(&node1, 30.0, 30.0);
    set_dimensions(&node2, 30.0, 30.0);
    set_dimensions(&node3, 30.0, 30.0);

    let _edge1 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node1),
        ElkConnectableShapeRef::Node(node2.clone()),
    );
    let _edge2 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node2),
        ElkConnectableShapeRef::Node(node3),
    );
    graph
}

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

fn set_graph_property<T: Clone + Send + Sync + 'static>(
    graph: &ElkNodeRef,
    property: &Property<T>,
    value: T,
) {
    let mut graph_mut = graph.borrow_mut();
    graph_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn check_edge_route_is_orthogonal(edge_index: usize, route: &KVectorChain) {
    if route.size() < 2 {
        panic!("edge {edge_index}: route with less than 2 points");
    }

    let mut horizontal = is_horizontal(route.get(0), route.get(1));
    let mut prev = route.get(0);

    for idx in 1..route.size() {
        let curr = route.get(idx);
        if horizontal {
            assert!(
                (prev.y - curr.y).abs() <= TOLERANCE,
                "edge {edge_index}: expected horizontal segment, prev={prev:?}, curr={curr:?}"
            );
        } else {
            assert!(
                (prev.x - curr.x).abs() <= TOLERANCE,
                "edge {edge_index}: expected vertical segment, prev={prev:?}, curr={curr:?}"
            );
        }
        horizontal = !horizontal;
        prev = curr;
    }
}

fn is_horizontal(p1: KVector, p2: KVector) -> bool {
    (p1.y - p2.y).abs() < TOLERANCE
}
