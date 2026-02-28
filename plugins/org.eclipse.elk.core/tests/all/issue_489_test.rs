use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{BoxLayouterOptions, CoreOptions};
use org_eclipse_elk_core::org::eclipse::elk::core::recursive_graph_layout_engine::RecursiveGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

const PADDING: f64 = 5.0;
const TOLERANCE: f64 = 0.1;

#[test]
fn issue_489_padding_and_min_size() {
    LayoutMetaDataService::get_instance();

    let root = ElkGraphUtil::create_graph();
    set_node_property(
        &root,
        CoreOptions::ALGORITHM,
        "org.eclipse.elk.box".to_string(),
    );

    let node1 = ElkGraphUtil::create_node(Some(root.clone()));
    let node2 = ElkGraphUtil::create_node(Some(root.clone()));

    for node in [&node1, &node2] {
        set_node_property(
            node,
            CoreOptions::ALGORITHM,
            "org.eclipse.elk.box".to_string(),
        );
        set_node_property(
            node,
            BoxLayouterOptions::PADDING,
            ElkPadding::with_any(PADDING),
        );
        let child = ElkGraphUtil::create_node(Some(node.clone()));
        set_dimensions(&child, 20.0, 20.0);
    }

    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = NullElkProgressMonitor;
    engine.layout(&root, &mut monitor);

    for node in [&node1, &node2] {
        let children: Vec<ElkNodeRef> = {
            let mut node_mut = node.borrow_mut();
            node_mut.children().iter().cloned().collect()
        };
        assert_eq!(1, children.len());
        let child = &children[0];

        let (child_x, child_y) = node_position(child);
        let (child_w, child_h) = node_dimensions(child);
        let (parent_w, parent_h) = node_dimensions(node);

        assert_close(PADDING, child_x);
        assert_close(PADDING, child_y);
        assert_close(parent_w - 2.0 * PADDING, child_w);
        assert_close(parent_h - 2.0 * PADDING, child_h);
    }
}

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

fn node_dimensions(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.width(), shape.height())
}

fn node_position(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y())
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
