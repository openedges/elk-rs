use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::{
    IGraphLayoutEngine, RecursiveGraphLayoutEngine,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

#[test]
fn test_unresolved_graph() {
    let graph = TestGraph::new();
    set_node_property(
        &graph.root,
        CoreOptions::ALGORITHM,
        "org.eclipse.elk.box".to_string(),
    );

    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = BasicProgressMonitor::new();
    engine.layout(&graph.root, &mut monitor);

    let (width, height) = node_dimensions(&graph.root);
    assert!(width > 0.0);
    assert!(height > 0.0);
}

#[test]
fn test_resolved_graph() {
    let graph = TestGraph::new();
    let algorithm = LayoutMetaDataService::get_instance()
        .get_algorithm_data("org.eclipse.elk.box")
        .expect("Expected box algorithm data");
    set_node_property(&graph.root, CoreOptions::RESOLVED_ALGORITHM, algorithm);

    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = BasicProgressMonitor::new();
    engine.layout(&graph.root, &mut monitor);

    let (width, height) = node_dimensions(&graph.root);
    assert!(width > 0.0);
    assert!(height > 0.0);
}

#[test]
#[should_panic]
fn test_unknown_algorithm_id() {
    let graph = TestGraph::new();
    set_node_property(&graph.root, CoreOptions::ALGORITHM, "foo.Bar".to_string());

    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = BasicProgressMonitor::new();
    engine.layout(&graph.root, &mut monitor);
}

#[test]
fn test_empty_algorithm_id() {
    let graph = TestGraph::new();

    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = BasicProgressMonitor::new();
    engine.layout(&graph.root, &mut monitor);

    let resolved = node_property(&graph.root, CoreOptions::RESOLVED_ALGORITHM)
        .expect("Expected resolved algorithm to be set");
    assert_eq!("org.eclipse.elk.layered", resolved.id());
}

struct TestGraph {
    root: ElkNodeRef,
    _n1: ElkNodeRef,
    _n2: ElkNodeRef,
}

impl TestGraph {
    fn new() -> Self {
        LayoutMetaDataService::get_instance();
        let root = ElkGraphUtil::create_graph();
        let n1 = ElkGraphUtil::create_node(Some(root.clone()));
        let n2 = ElkGraphUtil::create_node(Some(root.clone()));
        set_dimensions(&n1, 10.0, 10.0);
        set_dimensions(&n2, 10.0, 10.0);
        TestGraph {
            root,
            _n1: n1,
            _n2: n2,
        }
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

fn node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
) -> Option<T> {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(property)
}
