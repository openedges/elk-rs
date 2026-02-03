use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::options::{CoreOptions, PortConstraints, PortSide};
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{
    ElkGraphAdapters, GraphAdapter, NodeAdapter,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

#[test]
fn elk_graph_adapter_lists_child_nodes() {
    let graph = ElkGraphUtil::create_graph();
    ElkGraphUtil::create_node(Some(graph.clone()));
    ElkGraphUtil::create_node(Some(graph.clone()));

    let adapter = ElkGraphAdapters::adapt(graph);
    let nodes = adapter.get_nodes();
    assert_eq!(nodes.len(), 2);
}

#[test]
fn node_adapter_sorts_ports_when_fixed_order() {
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
