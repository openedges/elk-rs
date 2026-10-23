//! Resize boundary cases shared by the public and layered graph models.
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphUtil, LNode, LPort,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, PortConstraints, PortSide,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

fn check_ports(
    old: (f64, f64),
    new: (f64, f64),
    fixed: bool,
    move_ports: bool,
    expected: [(f64, f64); 4],
) {
    let constraints = if fixed {
        PortConstraints::FixedPos
    } else {
        PortConstraints::Free
    };
    let root = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(root));
    {
        let mut n = node.borrow_mut();
        n.connectable().shape().set_dimensions(old.0, old.1);
        n.connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_CONSTRAINTS, Some(constraints));
    }
    let graph = LGraph::new();
    let layered = LNode::new(&graph);
    {
        let mut n = layered.lock();
        *n.shape().size() = KVector::with_values(old.0, old.1);
        n.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(constraints));
    }
    let positions = [(3.0, -2.0), (old.0, 4.0), (5.0, old.1), (-2.0, 6.0)];
    let mut public_ports = Vec::new();
    let mut layered_ports = Vec::new();
    for (side, (x, y)) in [
        PortSide::North,
        PortSide::East,
        PortSide::South,
        PortSide::West,
    ]
    .into_iter()
    .zip(positions)
    {
        let port = ElkGraphUtil::create_port(Some(node.clone()));
        {
            let mut p = port.borrow_mut();
            p.connectable().shape().set_location(x, y);
            p.connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .set_property(CoreOptions::PORT_SIDE, Some(side));
        }
        public_ports.push(port);
        let port = LPort::new();
        LPort::set_node(&port, Some(layered.clone()));
        {
            let mut p = port.lock();
            p.set_side(side);
            *p.shape().position() = KVector::with_values(x, y);
        }
        layered_ports.push(port);
    }
    let ratio = ElkUtil::resize_node_with(&node, new.0, new.1, move_ports, true);
    assert!(ratio.x.is_finite() && ratio.y.is_finite());
    LGraphUtil::resize_node(
        &layered,
        &KVector::with_values(new.0, new.1),
        move_ports,
        true,
    );
    for ((public, internal), expected) in public_ports.iter().zip(&layered_ports).zip(expected) {
        let mut p = public.borrow_mut();
        let shape = p.connectable().shape();
        assert_eq!((shape.x(), shape.y()), expected);
        let pos = *internal.lock().shape().position_ref();
        assert_eq!((pos.x, pos.y), expected);
    }
}

#[test]
fn resize_from_zero_preserves_tangential_coordinates() {
    check_ports(
        (0.0, 0.0),
        (40.0, 30.0),
        false,
        true,
        [(3.0, -2.0), (40.0, 4.0), (5.0, 30.0), (-2.0, 6.0)],
    );
}
#[test]
fn resize_zero_to_zero_stays_finite() {
    check_ports(
        (0.0, 0.0),
        (0.0, 0.0),
        false,
        true,
        [(3.0, -2.0), (0.0, 4.0), (5.0, 0.0), (-2.0, 6.0)],
    );
}
#[test]
fn resize_one_collapsed_axis_scales_only_the_other() {
    check_ports(
        (0.0, 10.0),
        (40.0, 30.0),
        false,
        true,
        [(3.0, -2.0), (40.0, 12.0), (5.0, 30.0), (-2.0, 18.0)],
    );
    check_ports(
        (20.0, 0.0),
        (40.0, 30.0),
        false,
        true,
        [(6.0, -2.0), (40.0, 4.0), (10.0, 30.0), (-2.0, 6.0)],
    );
}
#[test]
fn ordinary_resize_keeps_proportional_placement() {
    check_ports(
        (20.0, 10.0),
        (40.0, 30.0),
        false,
        true,
        [(6.0, -2.0), (40.0, 12.0), (10.0, 30.0), (-2.0, 18.0)],
    );
}
#[test]
fn fixed_positions_and_disabled_movement_are_preserved() {
    check_ports(
        (20.0, 10.0),
        (40.0, 30.0),
        true,
        true,
        [(3.0, -2.0), (40.0, 4.0), (5.0, 30.0), (-2.0, 6.0)],
    );
    check_ports(
        (0.0, 0.0),
        (40.0, 30.0),
        false,
        false,
        [(3.0, -2.0), (0.0, 4.0), (5.0, 0.0), (-2.0, 6.0)],
    );
}

#[test]
fn unsupported_cross_algorithm_edges_return_an_error() {
    let input = r#"{"id":"root","layoutOptions":{"elk.algorithm":"layered","elk.hierarchyHandling":"INCLUDE_CHILDREN"},"children":[{"id":"source"},{"id":"outside","children":[{"id":"other","layoutOptions":{"elk.algorithm":"stress"},"children":[{"id":"target"}]}]}],"edges":[{"id":"invalid","sources":["source"],"targets":["target"]}]}"#;
    let error =
        org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api::layout_json(
            input, "{}",
        )
        .unwrap_err();
    assert!(
        error.contains("org.eclipse.elk.core.UnsupportedGraphException"),
        "{error}"
    );
    assert!(error.contains("invalid"), "{error}");
}

#[test]
fn labels_on_collapsed_rectangles_keep_their_coordinates() {
    use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LLabel;
    use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef;
    use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;
    use std::sync::Arc;
    for (old, expected) in [
        ((0.0, 0.0), (18.0, 3.0)),
        ((0.0, 10.0), (18.0, 3.0)),
        ((20.0, 0.0), (18.0, 3.0)),
        ((20.0, 10.0), (38.0, 11.0)),
    ] {
        let node = ElkGraphUtil::create_graph();
        node.borrow_mut()
            .connectable()
            .shape()
            .set_dimensions(old.0, old.1);
        let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Node(node.clone())));
        {
            let mut l = label.borrow_mut();
            l.shape().set_location(18.0, 3.0);
            l.shape().set_dimensions(2.0, 2.0);
        }
        ElkUtil::resize_node_with(&node, 40.0, 30.0, true, true);
        {
            let mut l = label.borrow_mut();
            assert_eq!((l.shape().x(), l.shape().y()), expected);
        }
        let graph = LGraph::new();
        let node = LNode::new(&graph);
        *node.lock().shape().size() = KVector::with_values(old.0, old.1);
        let label = Arc::new(Mutex::new(LLabel::new()));
        {
            let mut l = label.lock();
            *l.shape().position() = KVector::with_values(18.0, 3.0);
            *l.shape().size() = KVector::with_values(2.0, 2.0);
        }
        node.lock().labels_mut().push(label.clone());
        LGraphUtil::resize_node(&node, &KVector::with_values(40.0, 30.0), true, true);
        let p = *label.lock().shape().position_ref();
        assert_eq!((p.x, p.y), expected);
    }
}

#[test]
fn repeated_resize_returns_finite_ratios_and_preserves_disabled_labels() {
    use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef;
    let node = ElkGraphUtil::create_graph();
    let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Node(node.clone())));
    label.borrow_mut().shape().set_location(18.0, 3.0);
    let first = ElkUtil::resize_node_with(&node, 40.0, 30.0, true, false);
    assert_eq!((first.x, first.y), (1.0, 1.0));
    let second = ElkUtil::resize_node_with(&node, 80.0, 60.0, true, false);
    assert_eq!((second.x, second.y), (2.0, 2.0));
    let mut label = label.borrow_mut();
    assert_eq!((label.shape().x(), label.shape().y()), (18.0, 3.0));
}
