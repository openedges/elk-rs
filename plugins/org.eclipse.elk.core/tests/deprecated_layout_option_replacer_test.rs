use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    DeprecatedLayoutOptionReplacer, LayoutMetaDataService,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, PortLabelPlacement, SizeOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, IGraphElementVisitor};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkGraphElementRef, ElkNodeRef};

fn with_node_properties_mut<R>(node: &ElkNodeRef, f: impl FnOnce(&mut MapPropertyHolder) -> R) -> R {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    f(props)
}

#[test]
fn deprecated_layout_option_replacer_next_to_port() {
    LayoutMetaDataService::get_instance();
    let node = ElkGraphUtil::create_graph();
    with_node_properties_mut(&node, |props| {
        props.set_property(CoreOptions::PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE, Some(true));
    });

    let mut replacer = DeprecatedLayoutOptionReplacer::new();
    replacer.visit(&ElkGraphElementRef::Node(node.clone()));

    let has_deprecated = with_node_properties_mut(&node, |props| {
        props.has_property(CoreOptions::PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE)
    });
    assert!(!has_deprecated);

    let placement = with_node_properties_mut(&node, |props| {
        props.get_property(CoreOptions::PORT_LABELS_PLACEMENT)
    })
    .expect("Expected port label placement to be set.");
    assert!(placement.contains(&PortLabelPlacement::NextToPortIfPossible));
}

#[test]
fn deprecated_layout_option_replacer_moves_space_efficient() {
    LayoutMetaDataService::get_instance();
    let node = ElkGraphUtil::create_graph();
    let options = EnumSet::of(&[SizeOptions::SpaceEfficientPortLabels]);
    with_node_properties_mut(&node, |props| {
        props.set_property(CoreOptions::NODE_SIZE_OPTIONS, Some(options));
    });

    let mut replacer = DeprecatedLayoutOptionReplacer::new();
    replacer.visit(&ElkGraphElementRef::Node(node.clone()));

    let size_options = with_node_properties_mut(&node, |props| {
        props.get_property(CoreOptions::NODE_SIZE_OPTIONS)
    })
    .expect("Expected node size options to exist.");
    assert!(!size_options.contains(&SizeOptions::SpaceEfficientPortLabels));

    let placement = with_node_properties_mut(&node, |props| {
        props.get_property(CoreOptions::PORT_LABELS_PLACEMENT)
    })
    .expect("Expected port label placement to be set.");
    assert!(placement.contains(&PortLabelPlacement::SpaceEfficient));
}
