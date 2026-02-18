use org_eclipse_elk_core::org::eclipse::elk::core::data::DeprecatedLayoutOptionReplacer;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, PortLabelPlacement, SizeOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, EnumSet};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

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
fn replacer_moves_next_to_port_option() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    with_node_properties_mut(&node, |props| {
        props.set_property(
            CoreOptions::PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE,
            Some(true),
        );
    });

    let mut replacer = DeprecatedLayoutOptionReplacer::new();
    ElkUtil::apply_visitors(&graph, &mut [&mut replacer]);

    let placement = with_node_properties_mut(&node, |props| {
        props.get_property(CoreOptions::PORT_LABELS_PLACEMENT)
    })
    .expect("expected placement to be set");
    assert!(placement.contains(&PortLabelPlacement::Outside));
    assert!(placement.contains(&PortLabelPlacement::NextToPortIfPossible));

    let has_old_flag = with_node_properties_mut(&node, |props| {
        props.has_property(CoreOptions::PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE)
    });
    assert!(!has_old_flag);
}

#[test]
fn replacer_moves_space_efficient_port_labels() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    let size_options = EnumSet::of(&[
        SizeOptions::SpaceEfficientPortLabels,
        SizeOptions::PortsOverhang,
    ]);
    with_node_properties_mut(&node, |props| {
        props.set_property(CoreOptions::NODE_SIZE_OPTIONS, Some(size_options));
    });

    let mut replacer = DeprecatedLayoutOptionReplacer::new();
    ElkUtil::apply_visitors(&graph, &mut [&mut replacer]);

    let updated_options = with_node_properties_mut(&node, |props| {
        props.get_property(CoreOptions::NODE_SIZE_OPTIONS)
    })
    .expect("expected size options to be set");
    assert!(!updated_options.contains(&SizeOptions::SpaceEfficientPortLabels));
    assert!(updated_options.contains(&SizeOptions::PortsOverhang));

    let placement = with_node_properties_mut(&node, |props| {
        props.get_property(CoreOptions::PORT_LABELS_PLACEMENT)
    })
    .expect("expected placement to be set");
    assert!(placement.contains(&PortLabelPlacement::SpaceEfficient));
}
