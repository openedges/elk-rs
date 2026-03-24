use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{LGraph, LNode, LPort};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::counting::in_north_south_east_west_order;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

fn add_port(
    node: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
    side: PortSide,
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LPortRef {
    let port = LPort::new();
    {
        let mut port_guard = port.lock();
        port_guard.set_side(side);
    }
    LPort::set_node(&port, Some(node.clone()));
    port
}

#[test]
fn in_north_south_east_west_order_respects_side_direction() {
    let graph = LGraph::new();
    let node = LNode::new(&graph);

    let north_a = add_port(&node, PortSide::North);
    let north_b = add_port(&node, PortSide::North);

    let east_a = add_port(&node, PortSide::East);
    let east_b = add_port(&node, PortSide::East);

    let south_a = add_port(&node, PortSide::South);
    let south_b = add_port(&node, PortSide::South);

    let west_a = add_port(&node, PortSide::West);
    let west_b = add_port(&node, PortSide::West);

    let north = in_north_south_east_west_order(&node, PortSide::North);
    assert_eq!(north.len(), 2);
    assert!(std::sync::Arc::ptr_eq(&north[0], &north_a));
    assert!(std::sync::Arc::ptr_eq(&north[1], &north_b));

    let east = in_north_south_east_west_order(&node, PortSide::East);
    assert_eq!(east.len(), 2);
    assert!(std::sync::Arc::ptr_eq(&east[0], &east_a));
    assert!(std::sync::Arc::ptr_eq(&east[1], &east_b));

    let south = in_north_south_east_west_order(&node, PortSide::South);
    assert_eq!(south.len(), 2);
    assert!(std::sync::Arc::ptr_eq(&south[0], &south_b));
    assert!(std::sync::Arc::ptr_eq(&south[1], &south_a));

    let west = in_north_south_east_west_order(&node, PortSide::West);
    assert_eq!(west.len(), 2);
    assert!(std::sync::Arc::ptr_eq(&west[0], &west_b));
    assert!(std::sync::Arc::ptr_eq(&west[1], &west_a));
}
