#[path = "../../org.eclipse.elk.alg.layered/tests/elkt_test_loader.rs"]
mod elkt_test_loader;

use elkt_test_loader::load_graph_from_elkt;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::FixedLayouterOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    FixedLayoutProvider, NullElkProgressMonitor,
};
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;
use std::path::PathBuf;

const SIZE: f64 = 20.0;

#[test]
fn test_compound_node_size() {
    let fixed_size_graph = load_issue_475_graph();
    let resizable_graph = load_issue_475_graph();

    configure_fixed_graph_size(&fixed_size_graph, true);
    configure_fixed_graph_size(&resizable_graph, false);

    run_fixed_layout(&fixed_size_graph);
    run_fixed_layout(&resizable_graph);

    assert_graph_size_behavior(&fixed_size_graph);
    assert_graph_size_behavior(&resizable_graph);
}

fn load_issue_475_graph() -> ElkNodeRef {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../external/elk-models/tickets/core/475_fixedLayoutWithLongLabels.elkt");
    let path = path.to_string_lossy().into_owned();
    load_graph_from_elkt(&path, Some(FixedLayouterOptions::ALGORITHM_ID))
        .unwrap_or_else(|err| panic!("issue 475 graph should load: {err}"))
}

fn configure_fixed_graph_size(graph: &ElkNodeRef, fixed_graph_size: bool) {
    let children: Vec<ElkNodeRef> = graph.borrow_mut().children().iter().cloned().collect();
    for child in children {
        set_node_property(
            &child,
            FixedLayouterOptions::NODE_SIZE_FIXED_GRAPH_SIZE,
            fixed_graph_size,
        );
        if fixed_graph_size {
            set_node_dimensions(&child, SIZE, SIZE);
        }
    }
}

fn run_fixed_layout(graph: &ElkNodeRef) {
    LayoutMetaDataService::get_instance();
    let children: Vec<ElkNodeRef> = graph.borrow_mut().children().iter().cloned().collect();
    if children.is_empty() {
        let mut provider = FixedLayoutProvider::new();
        let mut monitor = NullElkProgressMonitor;
        provider.layout(graph, &mut monitor);
    } else {
        for child in children {
            let mut provider = FixedLayoutProvider::new();
            let mut monitor = NullElkProgressMonitor;
            provider.layout(&child, &mut monitor);
        }
    }
}

fn assert_graph_size_behavior(graph: &ElkNodeRef) {
    let children: Vec<ElkNodeRef> = graph.borrow_mut().children().iter().cloned().collect();
    for node in children {
        do_test_compound_node_size(&node);
    }
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
