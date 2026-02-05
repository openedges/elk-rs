use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkEdgeRef, ElkNodeRef};

const EPSILON: f64 = 1.0e-4;

#[test]
fn node_coordinates_positive() {
    let graph = layout_simple_graph();
    let nodes: Vec<ElkNodeRef> = graph
        .borrow_mut()
        .children()
        .iter()
        .cloned()
        .collect();

    for node in nodes {
        let mut node_mut = node.borrow_mut();
        let shape = node_mut.connectable().shape();
        assert!(shape.x() > 0.0, "node x is not positive");
        assert!(shape.y() > 0.0, "node y is not positive");
    }
}

#[test]
fn edge_coordinates_positive() {
    let graph = layout_simple_graph();
    let edges: Vec<ElkEdgeRef> = graph
        .borrow_mut()
        .contained_edges()
        .iter()
        .cloned()
        .collect();

    for edge in edges {
        let sections: Vec<_> = edge
            .borrow_mut()
            .sections()
            .iter()
            .cloned()
            .collect();
        for section in sections {
            let section_ref = section.borrow();
            assert!(section_ref.start_x() > 0.0, "section start_x not positive");
            assert!(section_ref.start_y() > 0.0, "section start_y not positive");
            assert!(section_ref.end_x() > 0.0, "section end_x not positive");
            assert!(section_ref.end_y() > 0.0, "section end_y not positive");
        }
    }
}

#[test]
fn graph_size_positive() {
    let graph = layout_simple_graph();
    let mut graph_mut = graph.borrow_mut();
    let shape = graph_mut.connectable().shape();
    assert!(shape.width() > 0.0, "graph width not positive");
    assert!(shape.height() > 0.0, "graph height not positive");
}

#[test]
fn edge_orthogonality() {
    let graph = layout_simple_graph();
    let edges: Vec<ElkEdgeRef> = graph
        .borrow_mut()
        .contained_edges()
        .iter()
        .cloned()
        .collect();

    for edge in edges {
        let sections: Vec<_> = edge
            .borrow_mut()
            .sections()
            .iter()
            .cloned()
            .collect();
        for section in sections {
            let (start_x, start_y, end_x, end_y, bend_points) = {
                let mut section_mut = section.borrow_mut();
                (
                    section_mut.start_x(),
                    section_mut.start_y(),
                    section_mut.end_x(),
                    section_mut.end_y(),
                    section_mut.bend_points().iter().cloned().collect::<Vec<_>>(),
                )
            };

            let mut points = Vec::new();
            points.push((start_x, start_y));
            for bend_point in bend_points {
                let bend_point_ref = bend_point.borrow();
                points.push((bend_point_ref.x(), bend_point_ref.y()));
            }
            points.push((end_x, end_y));

            if points.len() < 2 {
                continue;
            }

            for pair in points.windows(2) {
                let (prev_x, prev_y) = pair[0];
                let (curr_x, curr_y) = pair[1];
                assert!(
                    (prev_x - curr_x).abs() <= EPSILON || (prev_y - curr_y).abs() <= EPSILON,
                    "edge segment not orthogonal: ({prev_x},{prev_y}) -> ({curr_x},{curr_y})"
                );
            }
        }
    }
}

fn layout_simple_graph() -> ElkNodeRef {
    init_layered_options();
    let graph = create_simple_graph();
    set_node_property(&graph, CoreOptions::EDGE_ROUTING, EdgeRouting::Orthogonal);

    let mut provider = LayeredLayoutProvider::new();
    provider.layout(&graph, &mut BasicProgressMonitor::new());
    graph
}

fn create_simple_graph() -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();

    let node1 = ElkGraphUtil::create_node(Some(graph.clone()));
    set_dimensions(&node1, 30.0, 30.0);
    set_node_property(&node1, LayeredOptions::NODE_SIZE_CONSTRAINTS, SizeConstraint::fixed());

    let node2 = ElkGraphUtil::create_node(Some(graph.clone()));
    set_dimensions(&node2, 30.0, 30.0);
    set_node_property(&node2, LayeredOptions::NODE_SIZE_CONSTRAINTS, SizeConstraint::fixed());

    let _edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node1),
        ElkConnectableShapeRef::Node(node2),
    );

    graph
}

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .set_dimensions(width, height);
}

fn set_node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
    value: T,
) {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}
