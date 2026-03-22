use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::components::{
    ComponentGroup, ModelOrderComponentGroup,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{LGraph, LGraphRef};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::InternalProperties;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::{
    PortSide, SIDES_EAST, SIDES_EAST_SOUTH, SIDES_EAST_SOUTH_WEST, SIDES_EAST_WEST, SIDES_NONE,
    SIDES_NORTH, SIDES_NORTH_EAST, SIDES_NORTH_EAST_SOUTH, SIDES_NORTH_EAST_SOUTH_WEST,
    SIDES_NORTH_EAST_WEST, SIDES_NORTH_SOUTH, SIDES_NORTH_SOUTH_WEST, SIDES_NORTH_WEST,
    SIDES_SOUTH, SIDES_SOUTH_WEST, SIDES_WEST,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;

#[test]
fn test_valid_constraints() {
    let mut group = ComponentGroup::new();
    assert!(group.add(generate_graph((*SIDES_NORTH).clone())));
    assert!(group.add(generate_graph((*SIDES_NORTH).clone())));
    assert!(group.add(generate_graph((*SIDES_SOUTH).clone())));
    assert!(group.add(generate_graph((*SIDES_SOUTH).clone())));
    assert!(group.add(generate_graph((*SIDES_WEST).clone())));
    assert!(group.add(generate_graph((*SIDES_WEST).clone())));
    assert!(group.add(generate_graph((*SIDES_EAST).clone())));
    assert!(group.add(generate_graph((*SIDES_EAST).clone())));
    assert!(group.add(generate_graph((*SIDES_NONE).clone())));
    assert!(group.add(generate_graph((*SIDES_NONE).clone())));

    group = ComponentGroup::new();
    assert!(group.add(generate_graph((*SIDES_EAST_WEST).clone())));
    assert!(group.add(generate_graph((*SIDES_EAST_WEST).clone())));
    assert!(group.add(generate_graph((*SIDES_WEST).clone())));
    assert!(group.add(generate_graph((*SIDES_WEST).clone())));
    assert!(group.add(generate_graph((*SIDES_EAST).clone())));
    assert!(group.add(generate_graph((*SIDES_EAST).clone())));
    assert!(group.add(generate_graph((*SIDES_NONE).clone())));
    assert!(group.add(generate_graph((*SIDES_NONE).clone())));

    group = ComponentGroup::new();
    assert!(group.add(generate_graph((*SIDES_NORTH_SOUTH).clone())));
    assert!(group.add(generate_graph((*SIDES_NORTH_SOUTH).clone())));
    assert!(group.add(generate_graph((*SIDES_NORTH).clone())));
    assert!(group.add(generate_graph((*SIDES_NORTH).clone())));
    assert!(group.add(generate_graph((*SIDES_SOUTH).clone())));
    assert!(group.add(generate_graph((*SIDES_SOUTH).clone())));
    assert!(group.add(generate_graph((*SIDES_NONE).clone())));
    assert!(group.add(generate_graph((*SIDES_NONE).clone())));

    group = ComponentGroup::new();
    assert!(group.add(generate_graph((*SIDES_NORTH_EAST_SOUTH_WEST).clone())));
    assert!(group.add(generate_graph((*SIDES_NORTH_WEST).clone())));
    assert!(group.add(generate_graph((*SIDES_NORTH_EAST).clone())));
    assert!(group.add(generate_graph((*SIDES_SOUTH_WEST).clone())));
    assert!(group.add(generate_graph((*SIDES_EAST_SOUTH).clone())));
}

#[test]
fn test_invalid_constraints() {
    let mut group = ComponentGroup::new();
    assert!(group.add(generate_graph((*SIDES_EAST_WEST).clone())));
    assert!(!group.add(generate_graph((*SIDES_NORTH_SOUTH).clone())));
    assert!(!group.add(generate_graph((*SIDES_NORTH_SOUTH_WEST).clone())));
    assert!(!group.add(generate_graph((*SIDES_NORTH_EAST_SOUTH).clone())));

    group = ComponentGroup::new();
    assert!(group.add(generate_graph((*SIDES_NORTH_SOUTH).clone())));
    assert!(!group.add(generate_graph((*SIDES_EAST_WEST).clone())));
    assert!(!group.add(generate_graph((*SIDES_NORTH_EAST_WEST).clone())));
    assert!(!group.add(generate_graph((*SIDES_EAST_SOUTH_WEST).clone())));

    group = ComponentGroup::new();
    assert!(group.add(generate_graph((*SIDES_NORTH_WEST).clone())));
    assert!(group.add(generate_graph((*SIDES_NORTH_EAST).clone())));
    assert!(group.add(generate_graph((*SIDES_SOUTH_WEST).clone())));
    assert!(group.add(generate_graph((*SIDES_EAST_SOUTH).clone())));
    assert!(!group.add(generate_graph((*SIDES_NORTH_EAST_SOUTH).clone())));
    assert!(!group.add(generate_graph((*SIDES_NORTH_EAST_WEST).clone())));
    assert!(!group.add(generate_graph((*SIDES_EAST_SOUTH_WEST).clone())));
    assert!(!group.add(generate_graph((*SIDES_NORTH_SOUTH_WEST).clone())));

    group = ComponentGroup::new();
    assert!(group.add(generate_graph((*SIDES_NORTH_EAST_SOUTH_WEST).clone())));
    assert!(!group.add(generate_graph((*SIDES_NONE).clone())));
    assert!(!group.add(generate_graph((*SIDES_NORTH_SOUTH).clone())));
    assert!(!group.add(generate_graph((*SIDES_EAST_WEST).clone())));
    assert!(!group.add(generate_graph((*SIDES_NORTH_EAST_SOUTH).clone())));
    assert!(!group.add(generate_graph((*SIDES_NORTH_EAST_WEST).clone())));
    assert!(!group.add(generate_graph((*SIDES_EAST_SOUTH_WEST).clone())));
    assert!(!group.add(generate_graph((*SIDES_NORTH_SOUTH_WEST).clone())));
    assert!(!group.add(generate_graph((*SIDES_NORTH_EAST_SOUTH_WEST).clone())));
}

#[test]
fn test_model_order_constraints() {
    let mut group = ModelOrderComponentGroup::new();
    assert!(group.add(generate_graph((*SIDES_NONE).clone())));
    assert!(!group.add(generate_graph((*SIDES_NORTH).clone())));

    group = ModelOrderComponentGroup::new();
    assert!(group.add(generate_graph((*SIDES_EAST).clone())));
    assert!(!group.add(generate_graph((*SIDES_EAST_WEST).clone())));
}

fn generate_graph(connections: EnumSet<PortSide>) -> LGraphRef {
    let graph = LGraph::new();
    {
        let mut graph_guard = graph.lock();
        graph_guard.set_property(InternalProperties::EXT_PORT_CONNECTIONS, Some(connections));
    }
    graph
}
