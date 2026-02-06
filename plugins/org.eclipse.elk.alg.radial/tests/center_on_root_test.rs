use org_eclipse_elk_alg_radial::org::eclipse::elk::alg::radial::RadialLayoutProvider;
use org_eclipse_elk_alg_radial::org::eclipse::elk::alg::radial::options::{
    RadialMetaDataProvider, RadialOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};

fn init_radial_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&RadialMetaDataProvider);
}

#[test]
fn test_simple_centering() {
    init_radial_options();
    let parent = ElkGraphUtil::create_graph();
    let root = ElkGraphUtil::create_node(Some(parent.clone()));
    let n1 = ElkGraphUtil::create_node(Some(parent.clone()));
    let _e1 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(root.clone()),
        ElkConnectableShapeRef::Node(n1.clone()),
    );
    let n2 = ElkGraphUtil::create_node(Some(parent.clone()));
    let _e2 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(root.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );
    let n3 = ElkGraphUtil::create_node(Some(parent.clone()));
    let _e3 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(root.clone()),
        ElkConnectableShapeRef::Node(n3.clone()),
    );

    set_dimensions(&root, 30.0, 30.0);
    set_dimensions(&n1, 30.0, 30.0);
    set_dimensions(&n2, 30.0, 30.0);
    set_dimensions(&n3, 30.0, 30.0);

    set_node_property(&parent, CoreOptions::ALGORITHM, RadialOptions::ALGORITHM_ID.to_string());
    set_node_property(&parent, RadialOptions::CENTER_ON_ROOT, true);

    let mut layout_provider = RadialLayoutProvider::new();
    layout_provider.layout(&parent, &mut BasicProgressMonitor::new());

    let margins = node_property(&root, CoreOptions::MARGINS).unwrap_or_default();
    let (parent_w, parent_h) = node_size(&parent);
    let (root_x, root_y, root_w, root_h) = node_bounds(&root);

    assert!((parent_w / 2.0 - (root_x + margins.left + root_w / 2.0)).abs() < 0.1);
    assert!((parent_h / 2.0 - (root_y + margins.top + root_h / 2.0)).abs() < 0.1);
}

#[test]
fn test_larger_graph_centering() {
    init_radial_options();
    let parent = ElkGraphUtil::create_graph();
    let root = ElkGraphUtil::create_node(Some(parent.clone()));

    let n1 = ElkGraphUtil::create_node(Some(parent.clone()));
    let _e1 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(root.clone()),
        ElkConnectableShapeRef::Node(n1.clone()),
    );
    let n2 = ElkGraphUtil::create_node(Some(parent.clone()));
    let _e2 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(root.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );
    let n3 = ElkGraphUtil::create_node(Some(parent.clone()));
    let _e3 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(root.clone()),
        ElkConnectableShapeRef::Node(n3.clone()),
    );

    let n11 = ElkGraphUtil::create_node(Some(parent.clone()));
    let _e11 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n11.clone()),
    );
    let n12 = ElkGraphUtil::create_node(Some(parent.clone()));
    let _e12 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n12.clone()),
    );
    let n13 = ElkGraphUtil::create_node(Some(parent.clone()));
    let _e13 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n13.clone()),
    );

    for node in [&root, &n1, &n2, &n3, &n11, &n12, &n13] {
        set_dimensions(node, 30.0, 30.0);
    }

    set_node_property(&parent, CoreOptions::ALGORITHM, RadialOptions::ALGORITHM_ID.to_string());
    set_node_property(&parent, RadialOptions::CENTER_ON_ROOT, true);

    let mut layout_provider = RadialLayoutProvider::new();
    layout_provider.layout(&parent, &mut BasicProgressMonitor::new());

    let margins = node_property(&root, CoreOptions::MARGINS).unwrap_or_default();
    let (parent_w, parent_h) = node_size(&parent);
    let (root_x, root_y, root_w, root_h) = node_bounds(&root);

    assert!((parent_w / 2.0 - (root_x + margins.left + root_w / 2.0)).abs() < 0.1);
    assert!((parent_h / 2.0 - (root_y + margins.top + root_h / 2.0)).abs() < 0.1);
}

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    shape.set_dimensions(width, height);
}

fn node_bounds(node: &ElkNodeRef) -> (f64, f64, f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y(), shape.width(), shape.height())
}

fn node_size(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.width(), shape.height())
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
