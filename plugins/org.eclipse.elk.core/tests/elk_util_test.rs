use org_eclipse_elk_core::org::eclipse::elk::core::math::{KVector, KVectorChain};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    ContentAlignment, CoreOptions, PortSide,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, EnumSet};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdge, ElkGraphElementRef, ElkGraphFactory, ElkNodeRef,
};

fn create_content_alignment_test_graph(
    content_alignment: Option<EnumSet<ContentAlignment>>,
) -> ElkNodeRef {
    let parent = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(parent.clone()));
    if let Some(alignment) = content_alignment {
        node.borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::CONTENT_ALIGNMENT, Some(alignment));
    }
    let inner = ElkGraphUtil::create_node(Some(node.clone()));
    node.borrow_mut()
        .connectable()
        .shape()
        .set_dimensions(100.0, 100.0);
    inner
        .borrow_mut()
        .connectable()
        .shape()
        .set_dimensions(80.0, 80.0);
    parent
}

fn get_child(parent: &ElkNodeRef, index: usize) -> ElkNodeRef {
    let mut parent_mut = parent.borrow_mut();
    parent_mut.children().get(index).expect("child not found")
}

fn get_location(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y())
}

fn assert_translation(
    alignment: Option<EnumSet<ContentAlignment>>,
    expected_x: f64,
    expected_y: f64,
) {
    let parent = create_content_alignment_test_graph(alignment);
    let node = get_child(&parent, 0);
    let inner = get_child(&node, 0);
    ElkUtil::translate((
        &node,
        &KVector::with_values(120.0, 120.0),
        &KVector::with_values(100.0, 100.0),
    ));
    let (x, y) = get_location(&inner);
    assert!((x - expected_x).abs() <= 1.0);
    assert!((y - expected_y).abs() <= 1.0);
}

#[test]
fn translate_with_content_alignment_test() {
    assert_translation(None, 0.0, 0.0);
    assert_translation(Some(ContentAlignment::top_left()), 0.0, 0.0);
    assert_translation(Some(ContentAlignment::top_center()), 10.0, 0.0);
    assert_translation(
        Some(EnumSet::of(&[
            ContentAlignment::VTop,
            ContentAlignment::HRight,
        ])),
        20.0,
        0.0,
    );
    assert_translation(
        Some(EnumSet::of(&[
            ContentAlignment::VCenter,
            ContentAlignment::HLeft,
        ])),
        0.0,
        10.0,
    );
    assert_translation(Some(ContentAlignment::center_center()), 10.0, 10.0);
    assert_translation(
        Some(EnumSet::of(&[
            ContentAlignment::VCenter,
            ContentAlignment::HRight,
        ])),
        20.0,
        10.0,
    );
    assert_translation(
        Some(EnumSet::of(&[
            ContentAlignment::VBottom,
            ContentAlignment::HLeft,
        ])),
        0.0,
        20.0,
    );
    assert_translation(
        Some(EnumSet::of(&[
            ContentAlignment::VBottom,
            ContentAlignment::HCenter,
        ])),
        10.0,
        20.0,
    );
    assert_translation(Some(ContentAlignment::bottom_right()), 20.0, 20.0);
}

#[test]
fn absolute_position_test() {
    let root = ElkGraphUtil::create_graph();
    {
        let mut root_mut = root.borrow_mut();
        let shape = root_mut.connectable().shape();
        shape.set_location(10.0, 20.0);
    }

    let child = ElkGraphUtil::create_node(Some(root.clone()));
    {
        let mut child_mut = child.borrow_mut();
        let shape = child_mut.connectable().shape();
        shape.set_location(5.0, 7.0);
    }

    let port = ElkGraphUtil::create_port(Some(child.clone()));
    {
        let mut port_mut = port.borrow_mut();
        let shape = port_mut.connectable().shape();
        shape.set_location(2.0, 3.0);
    }

    let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Port(port.clone())));
    {
        let mut label_mut = label.borrow_mut();
        let shape = label_mut.shape();
        shape.set_location(1.0, 1.0);
    }

    let root_pos =
        ElkUtil::absolute_position(&ElkGraphElementRef::Node(root.clone())).expect("root position");
    assert_eq!(root_pos, KVector::with_values(10.0, 20.0));

    let child_pos = ElkUtil::absolute_position(&ElkGraphElementRef::Node(child.clone()))
        .expect("child position");
    assert_eq!(child_pos, KVector::with_values(15.0, 27.0));

    let port_pos =
        ElkUtil::absolute_position(&ElkGraphElementRef::Port(port.clone())).expect("port position");
    assert_eq!(port_pos, KVector::with_values(17.0, 30.0));

    let label_pos = ElkUtil::absolute_position(&ElkGraphElementRef::Label(label.clone()))
        .expect("label position");
    assert_eq!(label_pos, KVector::with_values(18.0, 31.0));
}

#[test]
fn vector_chain_round_trip_test() {
    let factory = ElkGraphFactory::instance();
    let section = factory.create_elk_edge_section();

    {
        let mut section_mut = section.borrow_mut();
        section_mut.set_start_x(0.0);
        section_mut.set_start_y(0.0);
        section_mut.set_end_x(10.0);
        section_mut.set_end_y(0.0);

        let bend1 = factory.create_elk_bend_point();
        {
            let mut bend_mut = bend1.borrow_mut();
            bend_mut.set_x(2.0);
            bend_mut.set_y(0.0);
        }
        let bend2 = factory.create_elk_bend_point();
        {
            let mut bend_mut = bend2.borrow_mut();
            bend_mut.set_x(5.0);
            bend_mut.set_y(5.0);
        }
        section_mut.bend_points().push(bend1);
        section_mut.bend_points().push(bend2);
    }

    let chain = ElkUtil::create_vector_chain(&section);
    let expected = KVectorChain::from_vectors(&[
        KVector::with_values(0.0, 0.0),
        KVector::with_values(2.0, 0.0),
        KVector::with_values(5.0, 5.0),
        KVector::with_values(10.0, 0.0),
    ]);
    assert_eq!(expected, chain);

    {
        let mut section_mut = section.borrow_mut();
        section_mut
            .bend_points()
            .push(factory.create_elk_bend_point());
    }

    let new_chain = KVectorChain::from_vectors(&[
        KVector::with_values(0.0, 0.0),
        KVector::with_values(1.0, 1.0),
        KVector::with_values(2.0, 2.0),
        KVector::with_values(3.0, 3.0),
    ]);
    ElkUtil::apply_vector_chain(&new_chain, &section);

    let mut section_mut = section.borrow_mut();
    assert_eq!(section_mut.start_x(), 0.0);
    assert_eq!(section_mut.start_y(), 0.0);
    assert_eq!(section_mut.end_x(), 3.0);
    assert_eq!(section_mut.end_y(), 3.0);

    let bend_points = section_mut.bend_points();
    assert_eq!(bend_points.len(), 2);
    let bend0 = bend_points[0].borrow();
    assert_eq!(bend0.x(), 1.0);
    assert_eq!(bend0.y(), 1.0);
    let bend1 = bend_points[1].borrow();
    assert_eq!(bend1.x(), 2.0);
    assert_eq!(bend1.y(), 2.0);
}

#[test]
fn compute_inside_part_test() {
    let label_pos = KVector::with_values(0.0, 5.0);
    let label_size = KVector::with_values(10.0, 10.0);
    let port_size = KVector::with_values(10.0, 10.0);
    let inside =
        ElkUtil::compute_inside_part(&label_pos, &label_size, &port_size, 0.0, PortSide::North);
    assert_eq!(inside, 5.0);
}

#[test]
fn determine_junction_points_empty_for_single_edge() {
    let graph = ElkGraphUtil::create_graph();
    let source_node = ElkGraphUtil::create_node(Some(graph.clone()));
    let source_port = ElkGraphUtil::create_port(Some(source_node.clone()));
    let target_node = ElkGraphUtil::create_node(Some(graph.clone()));
    let target_port = ElkGraphUtil::create_port(Some(target_node.clone()));

    let edge = ElkGraphUtil::create_edge(Some(graph));
    ElkEdge::add_source(&edge, ElkConnectableShapeRef::Port(source_port));
    ElkEdge::add_target(&edge, ElkConnectableShapeRef::Port(target_port));

    let section = ElkGraphFactory::instance().create_elk_edge_section();
    {
        let mut section_mut = section.borrow_mut();
        section_mut.set_start_x(0.0);
        section_mut.set_start_y(0.0);
        section_mut.set_end_x(10.0);
        section_mut.set_end_y(0.0);
    }
    edge.borrow_mut().sections().add(section);

    let junction_points = ElkUtil::determine_junction_points(&edge);
    assert!(junction_points.is_empty());
}
