use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{CoreOptions, HierarchyHandling};
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::{
    IGraphLayoutEngine, RecursiveGraphLayoutEngine, TestController,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

#[test]
fn non_target_test_controller_does_not_install() {
    LayoutMetaDataService::get_instance();

    let root = create_graph("org.eclipse.elk.box");
    let mut controller = TestController::new("org.eclipse.elk.layered");
    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = BasicProgressMonitor::new();
    engine.layout_with_test_controller(&root, controller.as_mut(), &mut monitor);

    let (width, height) = node_dimensions(&root);
    assert!(width > 0.0);
    assert!(height > 0.0);
}

#[test]
#[should_panic(expected = "Test controllers can only be installed on white-box testable layout algorithms")]
fn target_test_controller_attempts_install() {
    LayoutMetaDataService::get_instance();

    let root = create_graph("org.eclipse.elk.layered");
    let mut controller = TestController::new("org.eclipse.elk.layered");
    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = BasicProgressMonitor::new();
    engine.layout_with_test_controller(&root, controller.as_mut(), &mut monitor);
}

#[test]
#[should_panic(expected = "Topdown layout cannot be used together with hierarchy handling.")]
fn include_children_conflicts_with_topdown_layout() {
    LayoutMetaDataService::get_instance();

    let root = create_graph("org.eclipse.elk.layered");
    set_node_property(
        &root,
        CoreOptions::HIERARCHY_HANDLING,
        HierarchyHandling::IncludeChildren,
    );
    set_node_property(&root, CoreOptions::TOPDOWN_LAYOUT, true);

    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = BasicProgressMonitor::new();
    engine.layout(&root, &mut monitor);
}

#[test]
fn include_children_inherits_hierarchy_handling_for_descendants() {
    LayoutMetaDataService::get_instance();

    let root = create_graph("org.eclipse.elk.layered");
    set_node_property(
        &root,
        CoreOptions::HIERARCHY_HANDLING,
        HierarchyHandling::IncludeChildren,
    );

    let children = child_nodes(&root);
    let child = children
        .first()
        .expect("Expected a child node")
        .clone();
    let grandchild = ElkGraphUtil::create_node(Some(child.clone()));
    set_dimensions(&grandchild, 8.0, 8.0);

    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = BasicProgressMonitor::new();
    engine.layout(&root, &mut monitor);

    let child_handling = node_property(&child, CoreOptions::HIERARCHY_HANDLING)
        .expect("Expected hierarchy handling after inheritance evaluation");
    assert_eq!(HierarchyHandling::IncludeChildren, child_handling);
}

#[test]
fn include_children_switching_algorithm_forces_separate_children() {
    LayoutMetaDataService::get_instance();

    let root = create_graph("org.eclipse.elk.layered");
    set_node_property(
        &root,
        CoreOptions::HIERARCHY_HANDLING,
        HierarchyHandling::IncludeChildren,
    );

    let children = child_nodes(&root);
    let child = children
        .first()
        .expect("Expected a child node")
        .clone();
    set_node_property(&child, CoreOptions::ALGORITHM, "org.eclipse.elk.box".to_string());
    let grandchild = ElkGraphUtil::create_node(Some(child.clone()));
    set_dimensions(&grandchild, 8.0, 8.0);

    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = BasicProgressMonitor::new();
    engine.layout(&root, &mut monitor);

    let child_handling = node_property(&child, CoreOptions::HIERARCHY_HANDLING)
        .expect("Expected hierarchy handling after algorithm switch");
    assert_eq!(HierarchyHandling::SeparateChildren, child_handling);
}

#[test]
fn hierarchy_handling_inherit_defaults_to_separate_children() {
    LayoutMetaDataService::get_instance();

    let root = create_graph("org.eclipse.elk.layered");
    let children = child_nodes(&root);
    let child = children
        .first()
        .expect("Expected a child node")
        .clone();
    let grandchild = ElkGraphUtil::create_node(Some(child.clone()));
    set_dimensions(&grandchild, 8.0, 8.0);

    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = BasicProgressMonitor::new();
    engine.layout(&root, &mut monitor);

    let root_handling = node_property(&root, CoreOptions::HIERARCHY_HANDLING)
        .expect("Expected root hierarchy handling after inheritance evaluation");
    assert_eq!(HierarchyHandling::SeparateChildren, root_handling);

    let child_handling = node_property(&child, CoreOptions::HIERARCHY_HANDLING)
        .expect("Expected child hierarchy handling after inheritance evaluation");
    assert_eq!(HierarchyHandling::SeparateChildren, child_handling);
}

#[test]
fn include_children_nested_algorithm_switch_forces_separate_children() {
    LayoutMetaDataService::get_instance();

    let root = create_graph("org.eclipse.elk.layered");
    set_node_property(
        &root,
        CoreOptions::HIERARCHY_HANDLING,
        HierarchyHandling::IncludeChildren,
    );

    let children = child_nodes(&root);
    let child = children
        .first()
        .expect("Expected a child node")
        .clone();
    let grandchild = ElkGraphUtil::create_node(Some(child.clone()));
    set_dimensions(&grandchild, 8.0, 8.0);
    set_node_property(
        &grandchild,
        CoreOptions::ALGORITHM,
        "org.eclipse.elk.box".to_string(),
    );

    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = BasicProgressMonitor::new();
    engine.layout(&root, &mut monitor);

    let child_handling = node_property(&child, CoreOptions::HIERARCHY_HANDLING)
        .expect("Expected hierarchy handling for child after inheritance evaluation");
    assert_eq!(HierarchyHandling::IncludeChildren, child_handling);

    let grandchild_handling = node_property(&grandchild, CoreOptions::HIERARCHY_HANDLING)
        .expect("Expected hierarchy handling for switched descendant");
    assert_eq!(HierarchyHandling::SeparateChildren, grandchild_handling);
}

#[test]
#[should_panic(expected = "Test controllers can only be installed on white-box testable layout algorithms")]
fn target_test_controller_attempts_install_on_target_descendant() {
    LayoutMetaDataService::get_instance();

    let root = create_graph("org.eclipse.elk.box");
    let children = child_nodes(&root);
    let child = children
        .first()
        .expect("Expected a child node")
        .clone();
    set_node_property(&child, CoreOptions::ALGORITHM, "org.eclipse.elk.layered".to_string());
    let grandchild = ElkGraphUtil::create_node(Some(child.clone()));
    set_dimensions(&grandchild, 8.0, 8.0);

    let mut controller = TestController::new("org.eclipse.elk.layered");
    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = BasicProgressMonitor::new();
    engine.layout_with_test_controller(&root, controller.as_mut(), &mut monitor);
}

fn create_graph(algorithm_id: &str) -> ElkNodeRef {
    let root = ElkGraphUtil::create_graph();
    set_node_property(&root, CoreOptions::ALGORITHM, algorithm_id.to_string());

    let left = ElkGraphUtil::create_node(Some(root.clone()));
    let right = ElkGraphUtil::create_node(Some(root.clone()));
    set_dimensions(&left, 10.0, 10.0);
    set_dimensions(&right, 10.0, 10.0);

    root
}

fn child_nodes(node: &ElkNodeRef) -> Vec<ElkNodeRef> {
    let mut node_mut = node.borrow_mut();
    node_mut.children().iter().cloned().collect()
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
