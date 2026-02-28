use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::comments::{
    ElkGraphBoundsProvider, ElkGraphDataProvider, IBoundsProvider, IDataProvider,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkGraphElementRef, ElkNodeRef};

fn set_comment_box(node: &ElkNodeRef, value: bool) {
    with_node_properties_mut(node, |props| {
        props.set_property(CoreOptions::COMMENT_BOX, Some(value));
    });
}

fn init_metadata() {
    LayoutMetaDataService::get_instance();
}

fn with_node_properties_mut<R>(
    node: &ElkNodeRef,
    f: impl FnOnce(&mut MapPropertyHolder) -> R,
) -> R {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    f(props)
}

#[test]
fn data_provider_separates_comments_and_targets() {
    init_metadata();
    let root = ElkGraphUtil::create_graph();
    let comment = ElkGraphUtil::create_node(Some(root.clone()));
    let target = ElkGraphUtil::create_node(Some(root.clone()));
    set_comment_box(&comment, true);

    let provider = ElkGraphDataProvider::new(root);
    let comments = provider.provide_comments();
    let targets = provider.provide_targets();

    assert_eq!(comments.len(), 1);
    assert!(Rc::ptr_eq(&comments[0], &comment));
    assert_eq!(targets.len(), 1);
    assert!(Rc::ptr_eq(&targets[0], &target));
}

#[test]
fn data_provider_returns_sub_hierarchies() {
    init_metadata();
    let root = ElkGraphUtil::create_graph();
    let container = ElkGraphUtil::create_node(Some(root.clone()));
    let _child = ElkGraphUtil::create_node(Some(container.clone()));

    let provider = ElkGraphDataProvider::new(root);
    let subs = provider.provide_sub_hierarchies();

    assert_eq!(subs.len(), 1);
}

#[test]
fn bounds_provider_reads_node_and_label_bounds() {
    init_metadata();
    let root = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(root));
    {
        let mut node_mut = node.borrow_mut();
        let shape = node_mut.connectable().shape();
        shape.set_location(1.0, 2.0);
        shape.set_dimensions(3.0, 4.0);
    }

    let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Node(node.clone())));
    {
        let mut label_mut = label.borrow_mut();
        let shape = label_mut.shape();
        shape.set_location(5.0, 6.0);
        shape.set_dimensions(7.0, 8.0);
    }

    let provider = ElkGraphBoundsProvider;
    let node_bounds = provider.bounds_for_comment(&node).unwrap();
    assert_eq!(node_bounds.x, 1.0);
    assert_eq!(node_bounds.y, 2.0);
    assert_eq!(node_bounds.width, 3.0);
    assert_eq!(node_bounds.height, 4.0);

    let label_bounds = provider.bounds_for_comment(&label).unwrap();
    assert_eq!(label_bounds.x, 5.0);
    assert_eq!(label_bounds.y, 6.0);
    assert_eq!(label_bounds.width, 7.0);
    assert_eq!(label_bounds.height, 8.0);
}
