use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMargin, ElkPadding};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, LabelSide, PortConstraints, PortSide,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{
    ElkGraphAdapters, GraphAdapter, GraphElementAdapter, LabelAdapter, NodeAdapter, PortAdapter,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkGraphElementRef,
};

#[test]
fn elk_graph_adapter_lists_child_nodes() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    ElkGraphUtil::create_node(Some(graph.clone()));
    ElkGraphUtil::create_node(Some(graph.clone()));

    let adapter = ElkGraphAdapters::adapt(graph);
    let nodes = adapter.get_nodes();
    assert_eq!(nodes.len(), 2);
}

#[test]
fn node_adapter_sorts_ports_when_fixed_order() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph));

    let p1 = ElkGraphUtil::create_port(Some(node.clone()));
    let p2 = ElkGraphUtil::create_port(Some(node.clone()));
    let p3 = ElkGraphUtil::create_port(Some(node.clone()));

    {
        let mut node_mut = node.borrow_mut();
        node_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedOrder));
    }

    set_port_attrs(&p1, PortSide::West, 2);
    set_port_attrs(&p2, PortSide::East, 0);
    set_port_attrs(&p3, PortSide::West, 1);

    let adapter = ElkGraphAdapters::adapt_single_node(node.clone());
    adapter.sort_port_list();

    let ports: Vec<_> = {
        let mut node_mut = node.borrow_mut();
        node_mut.ports().iter().cloned().collect()
    };

    assert!(Rc::ptr_eq(&ports[0], &p2));
    assert!(Rc::ptr_eq(&ports[1], &p3));
    assert!(Rc::ptr_eq(&ports[2], &p1));
}

#[test]
fn node_adapter_lists_ports_and_labels() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph));
    ElkGraphUtil::create_port(Some(node.clone()));
    ElkGraphUtil::create_label_with_text(
        "L1",
        Some(ElkGraphElementRef::Node(node.clone())),
    );

    let adapter = ElkGraphAdapters::adapt_single_node(node);
    assert_eq!(adapter.get_ports().len(), 1);
    assert_eq!(adapter.get_labels().len(), 1);
}

#[test]
fn node_adapter_padding_and_margin_roundtrip() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph));
    let adapter = ElkGraphAdapters::adapt_single_node(node);

    let padding = ElkPadding::with_values(1.0, 2.0, 3.0, 4.0);
    adapter.set_padding(padding.clone());
    assert_eq!(adapter.get_padding(), padding);

    let margin = ElkMargin::with_values(5.0, 6.0, 7.0, 8.0);
    adapter.set_margin(margin.clone());
    assert_eq!(adapter.get_margin(), margin);
}

#[test]
fn port_adapter_margin_and_border_offset() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph));
    let port = ElkGraphUtil::create_port(Some(node));

    let adapter = ElkGraphAdapters::adapt_single_port(port);
    let margin = ElkMargin::with_values(1.0, 2.0, 3.0, 4.0);
    adapter.set_margin(margin.clone());
    assert_eq!(adapter.get_margin(), margin);

    let offset = adapter
        .get_property(CoreOptions::PORT_BORDER_OFFSET)
        .expect("port border offset");
    assert_eq!(offset, 0.0);
}

#[test]
fn port_adapter_detects_compound_connections() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let parent = ElkGraphUtil::create_node(Some(graph.clone()));
    let child = ElkGraphUtil::create_node(Some(parent.clone()));
    let port = ElkGraphUtil::create_port(Some(parent));

    ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(port.clone()),
        ElkConnectableShapeRef::Node(child),
    );

    let adapter = ElkGraphAdapters::adapt_single_port(port);
    assert!(adapter.has_compound_connections());
}

#[test]
fn port_adapter_reports_side() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph));
    let port = ElkGraphUtil::create_port(Some(node));

    {
        let mut port_mut = port.borrow_mut();
        port_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_SIDE, Some(PortSide::East));
    }

    let adapter = ElkGraphAdapters::adapt_single_port(port);
    assert_eq!(adapter.get_side(), PortSide::East);
}

#[test]
fn label_adapter_reads_text_and_side() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph));
    ElkGraphUtil::create_label_with_text(
        "Hello",
        Some(ElkGraphElementRef::Node(node.clone())),
    );

    let adapter = ElkGraphAdapters::adapt_single_node(node);
    let labels = adapter.get_labels();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].get_text(), "Hello");
    assert_eq!(labels[0].get_side(), LabelSide::Unknown);
}

#[test]
fn node_adapter_compound_detection_respects_inside_self_loops() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph));
    {
        let mut node_mut = node.borrow_mut();
        node_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE, Some(true));
    }

    let adapter = ElkGraphAdapters::adapt_single_node(node);
    assert!(adapter.is_compound_node());
}

fn set_port_attrs(port: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkPortRef, side: PortSide, index: i32) {
    let mut port_mut = port.borrow_mut();
    let props = port_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    props.set_property(CoreOptions::PORT_SIDE, Some(side));
    props.set_property(CoreOptions::PORT_INDEX, Some(index));
}
