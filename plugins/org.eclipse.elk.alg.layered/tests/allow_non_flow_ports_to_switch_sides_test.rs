use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, NullElkProgressMonitor};
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkNodeRef, ElkPortRef,
};

#[test]
fn allow_non_flow_ports_to_switch_sides_cases() {
    init_layered_options();

    let cases = [
        Case::new(PortConstraints::FixedSide, false, false, true),
        Case::new(PortConstraints::FixedPos, true, true, true),
        Case::new(PortConstraints::FixedSide, true, false, false),
        Case::new(PortConstraints::FixedSide, false, true, false),
        Case::new(PortConstraints::FixedSide, true, true, false),
        Case::new(PortConstraints::FixedOrder, true, true, false),
    ];

    for (index, case) in cases.iter().enumerate() {
        let (graph, edge1, edge2) =
            create_simple_graph(case.port_constraints, case.node1_switch, case.node2_switch);
        let mut layout = LayeredLayoutProvider::new();
        layout.layout(&graph, &mut NullElkProgressMonitor);

        let intersects = edges_intersect(&edge1, &edge2);
        assert_eq!(
            case.expect_intersect, intersects,
            "case {index} failed: {:?}",
            case
        );
    }
}

#[derive(Debug, Clone, Copy)]
struct Case {
    port_constraints: PortConstraints,
    node1_switch: bool,
    node2_switch: bool,
    expect_intersect: bool,
}

impl Case {
    fn new(
        port_constraints: PortConstraints,
        node1_switch: bool,
        node2_switch: bool,
        expect_intersect: bool,
    ) -> Self {
        Self {
            port_constraints,
            node1_switch,
            node2_switch,
            expect_intersect,
        }
    }
}

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn create_simple_graph(
    port_constraints: PortConstraints,
    node1_switch: bool,
    node2_switch: bool,
) -> (ElkNodeRef, ElkEdgeRef, ElkEdgeRef) {
    let graph = ElkGraphUtil::create_graph();

    let node1 = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_size(&node1, 30.0, 30.0);
    set_node_property(&node1, LayeredOptions::PORT_CONSTRAINTS, port_constraints);
    let port_n1 = ElkGraphUtil::create_port(Some(node1.clone()));
    set_port_location(&port_n1, 15.0, -1.0);
    set_port_property(&port_n1, LayeredOptions::PORT_SIDE, PortSide::North);
    set_port_property(
        &port_n1,
        LayeredOptions::ALLOW_NON_FLOW_PORTS_TO_SWITCH_SIDES,
        node1_switch,
    );
    let port_s1 = ElkGraphUtil::create_port(Some(node1.clone()));
    set_port_location(&port_s1, 15.0, 30.0);
    set_port_property(&port_s1, LayeredOptions::PORT_SIDE, PortSide::South);
    set_port_property(
        &port_s1,
        LayeredOptions::ALLOW_NON_FLOW_PORTS_TO_SWITCH_SIDES,
        node1_switch,
    );

    let node2 = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_size(&node2, 30.0, 30.0);
    set_node_property(&node2, LayeredOptions::PORT_CONSTRAINTS, port_constraints);
    let port_n2 = ElkGraphUtil::create_port(Some(node2.clone()));
    set_port_location(&port_n2, 15.0, -1.0);
    set_port_property(&port_n2, LayeredOptions::PORT_SIDE, PortSide::North);
    set_port_property(
        &port_n2,
        LayeredOptions::ALLOW_NON_FLOW_PORTS_TO_SWITCH_SIDES,
        node2_switch,
    );
    let port_s2 = ElkGraphUtil::create_port(Some(node2.clone()));
    set_port_location(&port_s2, 15.0, 30.0);
    set_port_property(&port_s2, LayeredOptions::PORT_SIDE, PortSide::South);
    set_port_property(
        &port_s2,
        LayeredOptions::ALLOW_NON_FLOW_PORTS_TO_SWITCH_SIDES,
        node2_switch,
    );

    let edge1 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(port_n1),
        ElkConnectableShapeRef::Port(port_s2),
    );
    let edge2 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(port_s1),
        ElkConnectableShapeRef::Port(port_n2),
    );

    (graph, edge1, edge2)
}

fn set_node_size(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
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

fn set_port_property<T: Clone + Send + Sync + 'static>(
    port: &ElkPortRef,
    property: &Property<T>,
    value: T,
) {
    let mut port_mut = port.borrow_mut();
    port_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn set_port_location(port: &ElkPortRef, x: f64, y: f64) {
    let mut port_mut = port.borrow_mut();
    let shape = port_mut.connectable().shape();
    shape.set_x(x);
    shape.set_y(y);
}

fn edges_intersect(edge1: &ElkEdgeRef, edge2: &ElkEdgeRef) -> bool {
    let segments1 = edge_segments(edge1);
    let segments2 = edge_segments(edge2);
    for (p1, q1) in &segments1 {
        for (p2, q2) in &segments2 {
            if segments_intersect(*p1, *q1, *p2, *q2) {
                return true;
            }
        }
    }
    false
}

fn edge_segments(edge: &ElkEdgeRef) -> Vec<(KVector, KVector)> {
    let sections: Vec<_> = edge.borrow_mut().sections().iter().cloned().collect();
    let mut segments = Vec::new();
    for section in sections {
        let chain = ElkUtil::create_vector_chain(&section);
        if chain.size() < 2 {
            continue;
        }
        let mut prev = chain.get(0);
        for idx in 1..chain.size() {
            let cur = chain.get(idx);
            segments.push((prev, cur));
            prev = cur;
        }
    }
    segments
}

fn segments_intersect(p1: KVector, q1: KVector, p2: KVector, q2: KVector) -> bool {
    let o1 = orientation(p1, q1, p2);
    let o2 = orientation(p1, q1, q2);
    let o3 = orientation(p2, q2, p1);
    let o4 = orientation(p2, q2, q1);

    if o1 != o2 && o3 != o4 {
        return true;
    }

    if o1 == 0 && on_segment(p1, p2, q1) {
        return true;
    }
    if o2 == 0 && on_segment(p1, q2, q1) {
        return true;
    }
    if o3 == 0 && on_segment(p2, p1, q2) {
        return true;
    }
    if o4 == 0 && on_segment(p2, q1, q2) {
        return true;
    }

    false
}

fn orientation(p: KVector, q: KVector, r: KVector) -> i32 {
    const EPSILON: f64 = 1e-9;
    let val = (q.y - p.y) * (r.x - q.x) - (q.x - p.x) * (r.y - q.y);
    if val.abs() <= EPSILON {
        0
    } else if val > 0.0 {
        1
    } else {
        2
    }
}

fn on_segment(p: KVector, q: KVector, r: KVector) -> bool {
    const EPSILON: f64 = 1e-9;
    q.x <= p.x.max(r.x) + EPSILON
        && q.x + EPSILON >= p.x.min(r.x)
        && q.y <= p.y.max(r.y) + EPSILON
        && q.y + EPSILON >= p.y.min(r.y)
}
