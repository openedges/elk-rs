use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::FixedLayouterOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    FixedLayoutProvider, NullElkProgressMonitor,
};
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

const SIZE: f64 = 20.0;

#[test]
fn test_compound_node_size() {
    let fixed_size_graph = create_compound_graph(true);
    let resizable_graph = create_compound_graph(false);

    run_fixed_layout(&fixed_size_graph);
    run_fixed_layout(&resizable_graph);

    do_test_compound_node_size(&fixed_size_graph);
    do_test_compound_node_size(&resizable_graph);
}

fn create_compound_graph(fixed_graph_size: bool) -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    set_node_dimensions(&graph, SIZE, SIZE);
    set_node_property(
        &graph,
        FixedLayouterOptions::NODE_SIZE_FIXED_GRAPH_SIZE,
        fixed_graph_size,
    );

    let child_a = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_dimensions(&child_a, 10.0, 8.0);
    set_node_property(
        &child_a,
        FixedLayouterOptions::POSITION,
        KVector::with_values(6.0, 3.0),
    );

    let child_b = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_dimensions(&child_b, 12.0, 9.0);
    set_node_property(
        &child_b,
        FixedLayouterOptions::POSITION,
        KVector::with_values(26.0, 4.0),
    );

    let child_c = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_dimensions(&child_c, 7.0, 12.0);
    set_node_property(
        &child_c,
        FixedLayouterOptions::POSITION,
        KVector::with_values(2.0, 22.0),
    );

    graph
}

fn run_fixed_layout(graph: &ElkNodeRef) {
    LayoutMetaDataService::get_instance();
    let mut provider = FixedLayoutProvider::new();
    let mut monitor = NullElkProgressMonitor;
    provider.layout(graph, &mut monitor);
}

fn do_test_compound_node_size(node: &ElkNodeRef) {
    let fixed_graph_size = node
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(FixedLayouterOptions::NODE_SIZE_FIXED_GRAPH_SIZE)
        .unwrap_or(false);

    let (node_width, node_height) = node_size(node);
    if fixed_graph_size {
        assert!((node_width - SIZE).abs() < 1e-9);
        assert!((node_height - SIZE).abs() < 1e-9);
    } else {
        let children: Vec<ElkNodeRef> = node.borrow_mut().children().iter().cloned().collect();
        let mut max_x: f64 = 0.0;
        let mut max_y: f64 = 0.0;

        for child in children {
            let mut child_mut = child.borrow_mut();
            let shape = child_mut.connectable().shape();
            max_x = max_x.max(shape.x() + shape.width());
            max_y = max_y.max(shape.y() + shape.height());
        }

        assert!(node_width >= max_x);
        assert!(node_height >= max_y);
    }
}

fn set_node_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    node.borrow_mut()
        .connectable()
        .shape()
        .set_dimensions(width, height);
}

fn node_size(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.width(), shape.height())
}

fn set_node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
    value: T,
) {
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}
