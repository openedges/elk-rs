use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{BoxLayouterOptions, CoreOptions, SizeConstraint};
use org_eclipse_elk_core::org::eclipse::elk::core::recursive_graph_layout_engine::RecursiveGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

const MIN_SIZE: f64 = 60.0;
const TOLERANCE: f64 = 1.0;

#[test]
fn issue_457_minimum_size_respected() {
    LayoutMetaDataService::get_instance();

    let root = ElkGraphUtil::create_graph();
    set_node_property(&root, CoreOptions::ALGORITHM, "org.eclipse.elk.box".to_string());

    let node1 = ElkGraphUtil::create_node(Some(root.clone()));
    let node2 = ElkGraphUtil::create_node(Some(root.clone()));

    for node in [&node1, &node2] {
        set_node_property(node, CoreOptions::ALGORITHM, "org.eclipse.elk.box".to_string());
        set_node_property(node, BoxLayouterOptions::NODE_SIZE_CONSTRAINTS, SizeConstraint::minimum_size());
        set_node_property(
            node,
            BoxLayouterOptions::NODE_SIZE_MINIMUM,
            KVector::with_values(MIN_SIZE, MIN_SIZE),
        );
        let child = ElkGraphUtil::create_node(Some(node.clone()));
        set_dimensions(&child, 5.0, 5.0);
    }

    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = NullElkProgressMonitor;
    engine.layout(&root, &mut monitor);

    for node in [&node1, &node2] {
        let (width, height) = node_dimensions(node);
        assert_close(MIN_SIZE, width);
        assert_close(MIN_SIZE, height);
    }
}

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .set_dimensions(width, height);
}

fn node_dimensions(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.width(), shape.height())
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

fn assert_close(expected: f64, actual: f64) {
    assert!(
        (expected - actual).abs() <= TOLERANCE,
        "Expected {expected}, got {actual}"
    );
}
