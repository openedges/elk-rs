
use crate::common::elkt_test_loader::{
    find_edge_by_identifier, find_label_by_identifier, find_node_by_identifier,
    find_port_by_identifier, load_graph_from_elkt, load_layered_graph_from_elkt,
};
use crate::common::issue_support::{init_layered_options, run_layout};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    Direction, NodeLabelPlacement, PortLabelPlacement, PortSide,
};

#[test]
fn load_external_layered_example_and_layout() {
    init_layered_options();

    let path = format!(
        "{}/../../external/elk/plugins/org.eclipse.elk.alg.layered/images/example.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("layered example should load");

    run_layout(&graph);

    let child_count = graph.borrow_mut().children().len();
    let edge_count = graph.borrow_mut().contained_edges().len();
    assert!(child_count > 0, "expected nodes in loaded graph");
    assert!(edge_count > 0, "expected edges in loaded graph");
}

#[test]
fn load_external_box_example_reads_node_sizes() {
    let path = format!(
        "{}/../../external/elk/plugins/org.eclipse.elk.core/images/exampleBox.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_graph_from_elkt(&path, None).expect("box example should load");

    let node2 = find_node_by_identifier(&graph, "node2").expect("node2 should exist");
    let mut node_mut = node2.borrow_mut();
    let shape = node_mut.connectable().shape();
    assert_eq!(shape.width(), 30.0);
    assert_eq!(shape.height(), 30.0);
}

#[test]
fn load_advanced_elkt_features_for_labels_and_edge_points() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_advanced_features.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("advanced ELKT should load");

    let compound = find_node_by_identifier(&graph, "compound").expect("compound node should exist");
    let (direction, node_label_placement, port_label_placement) = {
        let mut compound_mut = compound.borrow_mut();
        let props = compound_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut();
        (
            props
                .get_property(CoreOptions::DIRECTION)
                .expect("direction should be set in node block"),
            props
                .get_property(LayeredOptions::NODE_LABELS_PLACEMENT)
                .expect("node labels placement should be set in node block"),
            props
                .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                .expect("port labels placement should be set in node block"),
        )
    };
    assert_eq!(direction, Direction::Right);
    assert!(node_label_placement.contains(&NodeLabelPlacement::Outside));
    assert!(node_label_placement.contains(&NodeLabelPlacement::VTop));
    assert!(node_label_placement.contains(&NodeLabelPlacement::HCenter));
    assert!(port_label_placement.contains(&PortLabelPlacement::Outside));
    assert!(port_label_placement.contains(&PortLabelPlacement::NextToPortIfPossible));

    let edge = find_edge_by_identifier(&graph, "compound", "child").expect("edge e1 should exist");
    let (start_x, start_y, end_x, end_y, bend_count) = {
        let section = edge
            .borrow_mut()
            .sections()
            .get(0)
            .expect("edge section should exist");
        let mut section_mut = section.borrow_mut();
        (
            section_mut.start_x(),
            section_mut.start_y(),
            section_mut.end_x(),
            section_mut.end_y(),
            section_mut.bend_points().len(),
        )
    };
    assert_eq!((start_x, start_y), (10.0, 20.0));
    assert_eq!((end_x, end_y), (80.0, 20.0));
    assert_eq!(bend_count, 3);

    let (junction_count, label_placement) = {
        let mut edge_mut = edge.borrow_mut();
        let props = edge_mut.element().properties_mut();
        let junctions = props
            .get_property(LayeredOptions::JUNCTION_POINTS)
            .expect("junction points should be set");
        let label = edge_mut
            .element()
            .labels()
            .get(0)
            .expect("edge label should exist");
        let label_placement = label
            .borrow_mut()
            .shape()
            .graph_element()
            .properties_mut()
            .get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
            .expect("edge label placement should be parsed");
        (junctions.len(), label_placement)
    };
    assert_eq!(junction_count, 2);
    assert_eq!(label_placement, EdgeLabelPlacement::Head);

    run_layout(&graph);
}

#[test]
fn load_nested_blocks_and_combined_layout_properties() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_nested_blocks.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("nested ELKT should load");

    let (direction, graph_port_labels) = {
        let mut graph_mut = graph.borrow_mut();
        let props = graph_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut();
        (
            props
                .get_property(CoreOptions::DIRECTION)
                .expect("graph direction should be set"),
            props
                .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                .expect("graph port labels placement should be set"),
        )
    };
    assert_eq!(direction, Direction::Down);
    assert!(graph_port_labels.contains(&PortLabelPlacement::Outside));
    assert!(graph_port_labels.contains(&PortLabelPlacement::NextToPortIfPossible));

    let root = find_node_by_identifier(&graph, "root").expect("root node should exist");
    let (root_x, root_y, root_w, root_h, child_ids) = {
        let mut root_mut = root.borrow_mut();
        let (x, y, w, h) = {
            let shape = root_mut.connectable().shape();
            (shape.x(), shape.y(), shape.width(), shape.height())
        };
        let child_ids = root_mut
            .children()
            .iter()
            .filter_map(|child| {
                child
                    .borrow_mut()
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(ToString::to_string)
            })
            .collect::<Vec<_>>();
        (x, y, w, h, child_ids)
    };
    assert_eq!((root_x, root_y), (10.0, 20.0));
    assert_eq!((root_w, root_h), (220.0, 140.0));
    assert!(child_ids.iter().any(|id| id == "child_a"));
    assert!(child_ids.iter().any(|id| id == "child_b"));

    let p_out = find_port_by_identifier(&graph, "p_out").expect("p_out should exist");
    let (p_out_x, p_out_y, p_out_side) = {
        let mut port_mut = p_out.borrow_mut();
        let (x, y) = {
            let shape = port_mut.connectable().shape();
            (shape.x(), shape.y())
        };
        let side = port_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .get_property(LayeredOptions::PORT_SIDE)
            .expect("p_out side should be parsed");
        (x, y, side)
    };
    assert_eq!((p_out_x, p_out_y), (50.0, 12.0));
    assert_eq!(p_out_side, PortSide::East);

    let edge = find_edge_by_identifier(&graph, "p_out", "p_in").expect("nested edge should exist");
    let (start_x, start_y, end_x, end_y, bend_count, label_placement) = {
        let mut edge_mut = edge.borrow_mut();
        let section = edge_mut
            .sections()
            .get(0)
            .expect("edge section should exist");
        let mut section_mut = section.borrow_mut();
        let label_placement = edge_mut
            .element()
            .properties_mut()
            .get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
            .expect("edge labels placement should be parsed");
        (
            section_mut.start_x(),
            section_mut.start_y(),
            section_mut.end_x(),
            section_mut.end_y(),
            section_mut.bend_points().len(),
            label_placement,
        )
    };
    assert_eq!((start_x, start_y), (80.0, 55.0));
    assert_eq!((end_x, end_y), (140.0, 55.0));
    assert_eq!(bend_count, 3);
    assert_eq!(label_placement, EdgeLabelPlacement::Center);

    run_layout(&graph);
}

#[test]
fn load_hyperedge_and_nested_port_label_blocks() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_hyperedge_and_labels.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("hyperedge ELKT should load");

    let edge =
        find_edge_by_identifier(&graph, "b", "d").expect("hyperedge should match b->d lookup");
    let (source_count, target_count, start_x, start_y, end_x, end_y, bend_count) = {
        let mut edge_mut = edge.borrow_mut();
        let source_count = edge_mut.sources().len();
        let target_count = edge_mut.targets().len();
        let section = edge_mut
            .sections()
            .get(0)
            .expect("edge section should exist for edgePoint declaration");
        let mut section_mut = section.borrow_mut();
        (
            source_count,
            target_count,
            section_mut.start_x(),
            section_mut.start_y(),
            section_mut.end_x(),
            section_mut.end_y(),
            section_mut.bend_points().len(),
        )
    };
    assert_eq!(source_count, 2);
    assert_eq!(target_count, 2);
    assert_eq!((start_x, start_y), (10.0, 10.0));
    assert_eq!((end_x, end_y), (120.0, 20.0));
    assert_eq!(bend_count, 2);

    let port = find_port_by_identifier(&graph, "p_out").expect("p_out should exist");
    let (side, labels) = {
        let mut port_mut = port.borrow_mut();
        let side = port_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .get_property(LayeredOptions::PORT_SIDE)
            .expect("port side should be parsed in nested block");
        let labels = port_mut
            .connectable()
            .shape()
            .graph_element()
            .labels()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        (side, labels)
    };
    assert_eq!(side, PortSide::East);
    assert_eq!(labels.len(), 1);

    let (label_text, label_w, label_h) = {
        let mut label_mut = labels[0].borrow_mut();
        let text = label_mut.text().to_string();
        let (width, height) = {
            let shape = label_mut.shape();
            (shape.width(), shape.height())
        };
        (text, width, height)
    };
    assert_eq!(label_text, "out");
    assert_eq!((label_w, label_h), (12.0, 6.0));

    run_layout(&graph);
}

#[test]
fn load_section_links_and_label_identifiers() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_sections_and_label_ids.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("section/link ELKT should load");

    let parent_label =
        find_label_by_identifier(&graph, "l_parent").expect("parent label should exist");
    let child_label =
        find_label_by_identifier(&graph, "l_child").expect("child label should exist");
    let (parent_text, parent_w, parent_h) = {
        let mut label_mut = parent_label.borrow_mut();
        let text = label_mut.text().to_string();
        let shape = label_mut.shape();
        (text, shape.width(), shape.height())
    };
    let (child_text, child_w, child_h) = {
        let mut label_mut = child_label.borrow_mut();
        let text = label_mut.text().to_string();
        let shape = label_mut.shape();
        (text, shape.width(), shape.height())
    };
    assert_eq!(parent_text, "parent");
    assert_eq!((parent_w, parent_h), (12.0, 6.0));
    assert_eq!(child_text, "child");
    assert_eq!((child_w, child_h), (7.0, 3.0));

    let edge = find_edge_by_identifier(&graph, "p_source", "p_target")
        .expect("edge with sections should exist");
    let (section_count, s0_target_count, s1_incoming_count, s0_outgoing_shape, s1_incoming_shape) = {
        let mut edge_mut = edge.borrow_mut();
        let sections: Vec<_> = edge_mut.sections().iter().cloned().collect();
        let section_count = sections.len();

        let mut s0 = None;
        let mut s1 = None;
        for section in sections {
            let id = section.borrow().identifier().map(ToString::to_string);
            if id.as_deref() == Some("s0") {
                s0 = Some(section.clone());
            } else if id.as_deref() == Some("s1") {
                s1 = Some(section.clone());
            }
        }
        let s0 = s0.expect("section s0 should exist");
        let s1 = s1.expect("section s1 should exist");

        let s0_target_count = s0.borrow().outgoing_sections().len();
        let s1_incoming_count = s1.borrow().incoming_sections().len();
        let s0_outgoing_shape = s0
            .borrow()
            .outgoing_shape()
            .expect("s0 outgoing shape should exist");
        let s1_incoming_shape = s1
            .borrow()
            .incoming_shape()
            .expect("s1 incoming shape should exist");

        let s0_outgoing_shape = match s0_outgoing_shape {
            org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Node(node) => {
                node.borrow_mut()
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(ToString::to_string)
                    .unwrap_or_default()
            }
            org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Port(port) => {
                port.borrow_mut()
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(ToString::to_string)
                    .unwrap_or_default()
            }
        };
        let s1_incoming_shape = match s1_incoming_shape {
            org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Node(node) => {
                node.borrow_mut()
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(ToString::to_string)
                    .unwrap_or_default()
            }
            org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Port(port) => {
                port.borrow_mut()
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(ToString::to_string)
                    .unwrap_or_default()
            }
        };

        (
            section_count,
            s0_target_count,
            s1_incoming_count,
            s0_outgoing_shape,
            s1_incoming_shape,
        )
    };
    assert_eq!(section_count, 2);
    assert_eq!(s0_target_count, 1);
    assert_eq!(s1_incoming_count, 1);
    assert_eq!(s0_outgoing_shape, "p_source");
    assert_eq!(s1_incoming_shape, "p_target");

    run_layout(&graph);
}

#[test]
fn load_quoted_label_and_section_identifiers_with_escaped_text() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_quoted_identifiers.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("quoted identifier ELKT should load");

    let parent_label =
        find_label_by_identifier(&graph, "l parent").expect("parent label should exist");
    let child_label =
        find_label_by_identifier(&graph, "l child").expect("child label should exist");
    let parent_text = parent_label.borrow_mut().text().to_string();
    let child_text = child_label.borrow_mut().text().to_string();
    assert_eq!(parent_text, "parent label");
    assert_eq!(child_text, "child \"quoted\" text");

    let edge = find_edge_by_identifier(&graph, "p0", "p1")
        .expect("edge with quoted sections should exist");
    let (section_count, s0_outgoing_ids, s1_incoming_ids) = {
        let mut edge_mut = edge.borrow_mut();
        let sections: Vec<_> = edge_mut.sections().iter().cloned().collect();
        let section_count = sections.len();

        let mut s0_outgoing_ids = Vec::new();
        let mut s1_incoming_ids = Vec::new();
        for section in sections {
            let id = section.borrow().identifier().map(ToString::to_string);
            if id.as_deref() == Some("s 0") {
                s0_outgoing_ids = section
                    .borrow()
                    .outgoing_sections()
                    .iter()
                    .filter_map(|target| target.borrow().identifier().map(ToString::to_string))
                    .collect::<Vec<_>>();
            } else if id.as_deref() == Some("s 1") {
                s1_incoming_ids = section
                    .borrow()
                    .incoming_sections()
                    .iter()
                    .filter_map(|source| source.borrow().identifier().map(ToString::to_string))
                    .collect::<Vec<_>>();
            }
        }
        (section_count, s0_outgoing_ids, s1_incoming_ids)
    };
    assert_eq!(section_count, 2);
    assert_eq!(s0_outgoing_ids, vec!["s 1".to_string()]);
    assert_eq!(s1_incoming_ids, vec!["s 0".to_string()]);

    run_layout(&graph);
}

#[test]
fn load_quoted_node_port_edge_identifiers() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_quoted_node_port_edge.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph =
        load_layered_graph_from_elkt(&path).expect("quoted node/port/edge ELKT should load");

    let node_a =
        find_node_by_identifier(&graph, "node a").expect("quoted node identifier should parse");
    let node_b =
        find_node_by_identifier(&graph, "node b").expect("second quoted node should parse");
    assert_eq!(
        node_a
            .borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .identifier(),
        Some("node a")
    );
    assert_eq!(
        node_b
            .borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .identifier(),
        Some("node b")
    );

    let source_port = find_port_by_identifier(&graph, "port, out")
        .expect("quoted port id with comma should parse");
    let target_port =
        find_port_by_identifier(&graph, "port in").expect("quoted port id should parse");
    let source_side = source_port
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(LayeredOptions::PORT_SIDE)
        .expect("source side should be set");
    let target_side = target_port
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(LayeredOptions::PORT_SIDE)
        .expect("target side should be set");
    assert_eq!(source_side, PortSide::East);
    assert_eq!(target_side, PortSide::West);

    let edge = find_edge_by_identifier(&graph, "port, out", "port in")
        .expect("quoted edge source/target identifiers should parse");
    let (start_x, start_y, end_x, end_y, bend_count) = {
        let mut edge_mut = edge.borrow_mut();
        let section = edge_mut
            .sections()
            .get(0)
            .expect("edge section should exist");
        let mut section_mut = section.borrow_mut();
        (
            section_mut.start_x(),
            section_mut.start_y(),
            section_mut.end_x(),
            section_mut.end_y(),
            section_mut.bend_points().len(),
        )
    };
    assert_eq!((start_x, start_y), (10.0, 10.0));
    assert_eq!((end_x, end_y), (70.0, 12.0));
    assert_eq!(bend_count, 1);

    run_layout(&graph);
}

#[test]
fn load_quoted_property_and_parent_of_references() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_quoted_property_parent_refs.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path)
        .expect("quoted parent/of and property references should load");

    let root =
        find_node_by_identifier(&graph, "root parent").expect("root parent node should exist");
    let child_ids = root
        .borrow_mut()
        .children()
        .iter()
        .filter_map(|child| {
            child
                .borrow_mut()
                .connectable()
                .shape()
                .graph_element()
                .identifier()
                .map(ToString::to_string)
        })
        .collect::<Vec<_>>();
    assert!(child_ids.iter().any(|id| id == "child node"));
    assert!(child_ids.iter().any(|id| id == "target node"));

    let child = find_node_by_identifier(&graph, "child node").expect("child node should exist");
    let child_spacing = child
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(CoreOptions::SPACING_NODE_NODE)
        .expect("quoted nodeProperty reference should apply");
    assert_eq!(child_spacing, 42.0);

    let source_port =
        find_port_by_identifier(&graph, "port out").expect("source port should exist");
    let target_port = find_port_by_identifier(&graph, "port in").expect("target port should exist");

    let (source_side, source_anchor) = {
        let mut source_port_mut = source_port.borrow_mut();
        let props = source_port_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut();
        let side = props
            .get_property(LayeredOptions::PORT_SIDE)
            .expect("quoted portProperty side should apply");
        let anchor = props
            .get_property(LayeredOptions::PORT_ANCHOR)
            .expect("quoted portProperty anchor should apply");
        (side, anchor)
    };
    assert_eq!(source_side, PortSide::East);
    assert_eq!((source_anchor.x, source_anchor.y), (1.5, 2.5));

    let target_side = target_port
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(LayeredOptions::PORT_SIDE)
        .expect("target side should parse through of: reference");
    assert_eq!(target_side, PortSide::West);

    let child_port_ids = child
        .borrow_mut()
        .ports()
        .iter()
        .filter_map(|port| {
            port.borrow_mut()
                .connectable()
                .shape()
                .graph_element()
                .identifier()
                .map(ToString::to_string)
        })
        .collect::<Vec<_>>();
    assert!(child_port_ids.iter().any(|id| id == "port out"));

    let target_node =
        find_node_by_identifier(&graph, "target node").expect("target node should exist");
    let target_port_ids = target_node
        .borrow_mut()
        .ports()
        .iter()
        .filter_map(|port| {
            port.borrow_mut()
                .connectable()
                .shape()
                .graph_element()
                .identifier()
                .map(ToString::to_string)
        })
        .collect::<Vec<_>>();
    assert!(target_port_ids.iter().any(|id| id == "port in"));

    let edge = find_edge_by_identifier(&graph, "port out", "port in")
        .expect("edge between quoted-referenced ports should exist");
    let placement = edge
        .borrow_mut()
        .element()
        .properties_mut()
        .get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
        .expect("quoted edgeProperty reference should apply");
    assert_eq!(placement, EdgeLabelPlacement::Head);

    run_layout(&graph);
}

#[test]
fn load_quoted_section_and_edge_point_references_with_aliases() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_quoted_section_edge_refs.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path)
        .expect("quoted section/edgePoint references should load");

    let section_edge = find_edge_by_identifier(&graph, "p out", "p in")
        .expect("section edge should resolve by quoted port references");
    let (
        section_count,
        sec_a_target_ids,
        sec_b_source_ids,
        sec_a_outgoing_shape,
        sec_b_incoming_shape,
    ) = {
        let mut edge_mut = section_edge.borrow_mut();
        let sections: Vec<_> = edge_mut.sections().iter().cloned().collect();

        let mut sec_a = None;
        let mut sec_b = None;
        for section in &sections {
            let id = section.borrow().identifier().map(ToString::to_string);
            if id.as_deref() == Some("sec a") {
                sec_a = Some(section.clone());
            } else if id.as_deref() == Some("sec b") {
                sec_b = Some(section.clone());
            }
        }

        let sec_a = sec_a.expect("sec a should exist");
        let sec_b = sec_b.expect("sec b should exist");
        let sec_a_target_ids = sec_a
            .borrow()
            .outgoing_sections()
            .iter()
            .filter_map(|section| section.borrow().identifier().map(ToString::to_string))
            .collect::<Vec<_>>();
        let sec_b_source_ids = sec_b
            .borrow()
            .incoming_sections()
            .iter()
            .filter_map(|section| section.borrow().identifier().map(ToString::to_string))
            .collect::<Vec<_>>();

        let sec_a_outgoing_shape = sec_a.borrow().outgoing_shape().map(|shape| match shape {
            org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Node(node) => {
                node.borrow_mut()
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(ToString::to_string)
                    .unwrap_or_default()
            }
            org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Port(port) => {
                port.borrow_mut()
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(ToString::to_string)
                    .unwrap_or_default()
            }
        });
        let sec_b_incoming_shape = sec_b.borrow().incoming_shape().map(|shape| match shape {
            org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Node(node) => {
                node.borrow_mut()
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(ToString::to_string)
                    .unwrap_or_default()
            }
            org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef::Port(port) => {
                port.borrow_mut()
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(ToString::to_string)
                    .unwrap_or_default()
            }
        });

        (
            sections.len(),
            sec_a_target_ids,
            sec_b_source_ids,
            sec_a_outgoing_shape,
            sec_b_incoming_shape,
        )
    };
    assert_eq!(section_count, 2);
    assert_eq!(sec_a_target_ids, vec!["sec b".to_string()]);
    assert_eq!(sec_b_source_ids, vec!["sec a".to_string()]);
    assert_eq!(sec_a_outgoing_shape, Some("p out".to_string()));
    assert_eq!(sec_b_incoming_shape, Some("p in".to_string()));

    let point_edge = find_edge_by_identifier(&graph, "p out 2", "p in 2")
        .expect("edgePoint edge should resolve by quoted port references");
    let (start_x, start_y, end_x, end_y, bend_count) = {
        let mut edge_mut = point_edge.borrow_mut();
        let section = edge_mut
            .sections()
            .get(0)
            .expect("edge section should exist for edgeSection/edgePoint declarations");
        let mut section_mut = section.borrow_mut();
        (
            section_mut.start_x(),
            section_mut.start_y(),
            section_mut.end_x(),
            section_mut.end_y(),
            section_mut.bend_points().len(),
        )
    };
    assert_eq!((start_x, start_y), (14.0, 12.0));
    assert_eq!((end_x, end_y), (86.0, 12.0));
    assert_eq!(bend_count, 3);

    run_layout(&graph);
}

#[test]
fn fail_when_section_link_target_is_missing() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_invalid_section_link.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let err = load_layered_graph_from_elkt(&path)
        .err()
        .expect("missing section target should fail");
    assert!(
        err.contains("line 10:"),
        "expected line context in error: {err}"
    );
    assert!(
        err.contains("section s0 -> s_missing"),
        "expected source line snippet in error: {err}"
    );
    assert!(err.contains("unknown target section 's_missing'"));
}

#[test]
fn fail_on_duplicate_label_identifier() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_duplicate_label_identifier.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let err = load_layered_graph_from_elkt(&path)
        .err()
        .expect("duplicate label identifier should fail");
    assert!(
        err.contains("line 4:"),
        "expected line context in error: {err}"
    );
    assert!(
        err.contains("label l_dup: \"child\""),
        "expected source line snippet in error: {err}"
    );
    assert!(err.contains("duplicate label identifier: l_dup"));
}

#[test]
fn load_section_chain_and_aliases() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_section_chain_and_aliases.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("section chain ELKT should load");

    let edge = find_edge_by_identifier(&graph, "p_source", "p_target")
        .expect("edge with chained sections should exist");

    let (s0_geometry, s0_outgoing, s0_incoming, s1_outgoing, s2_outgoing, s3_outgoing) = {
        let mut edge_mut = edge.borrow_mut();
        let sections: Vec<_> = edge_mut.sections().iter().cloned().collect();

        let mut s0 = None;
        let mut s1 = None;
        let mut s2 = None;
        let mut s3 = None;
        for section in sections {
            let id = section.borrow().identifier().map(ToString::to_string);
            match id.as_deref() {
                Some("s0") => s0 = Some(section.clone()),
                Some("s1") => s1 = Some(section.clone()),
                Some("s2") => s2 = Some(section.clone()),
                Some("s3") => s3 = Some(section.clone()),
                _ => {}
            }
        }

        let s0 = s0.expect("section s0 should exist");
        let s1 = s1.expect("section s1 should exist");
        let s2 = s2.expect("section s2 should exist");
        let s3 = s3.expect("section s3 should exist");

        let s0_geometry = {
            let mut s0_mut = s0.borrow_mut();
            (
                s0_mut.start_x(),
                s0_mut.start_y(),
                s0_mut.end_x(),
                s0_mut.end_y(),
                s0_mut.bend_points().len(),
            )
        };

        let to_ids =
            |refs: Vec<org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeSectionRef>| {
                refs.into_iter()
                    .filter_map(|section| section.borrow().identifier().map(ToString::to_string))
                    .collect::<Vec<_>>()
            };

        let s0_outgoing = to_ids(s0.borrow().outgoing_sections().clone());
        let s0_incoming = to_ids(s0.borrow().incoming_sections().clone());
        let s1_outgoing = to_ids(s1.borrow().outgoing_sections().clone());
        let s2_outgoing = to_ids(s2.borrow().outgoing_sections().clone());
        let s3_outgoing = to_ids(s3.borrow().outgoing_sections().clone());

        (
            s0_geometry,
            s0_outgoing,
            s0_incoming,
            s1_outgoing,
            s2_outgoing,
            s3_outgoing,
        )
    };

    assert_eq!(s0_geometry, (10.0, 10.0, 40.0, 20.0, 2));
    assert!(s0_outgoing.iter().any(|id| id == "s1"));
    assert!(s0_outgoing.iter().any(|id| id == "s2"));
    assert!(s0_incoming.iter().any(|id| id == "s3"));
    assert_eq!(s1_outgoing, vec!["s3".to_string()]);
    assert_eq!(s2_outgoing, vec!["s3".to_string()]);
    assert_eq!(s3_outgoing, vec!["s0".to_string()]);

    run_layout(&graph);
}

#[test]
fn load_edge_layout_and_edge_section_aliases() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_edge_aliases.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("edge alias ELKT should load");

    let (graph_spacing_node_node, graph_spacing_edge_edge) = {
        let mut graph_mut = graph.borrow_mut();
        let props = graph_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut();
        (
            props
                .get_property(CoreOptions::SPACING_NODE_NODE)
                .expect("spacingNodeNode alias should be parsed"),
            props
                .get_property(CoreOptions::SPACING_EDGE_EDGE)
                .expect("spacingEdgeEdge alias should be parsed"),
        )
    };
    assert_eq!(graph_spacing_node_node, 17.0);
    assert_eq!(graph_spacing_edge_edge, 9.0);

    let layout_source = find_node_by_identifier(&graph, "n_layout_source")
        .expect("n_layout_source should exist for nodeProperty alias check");
    let node_spacing = layout_source
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(CoreOptions::SPACING_NODE_NODE)
        .expect("nodeProperty spacingNodeNode alias should be parsed");
    assert_eq!(node_spacing, 21.0);

    let layout_edge = find_edge_by_identifier(&graph, "p_layout_source", "p_layout_target")
        .expect("layout edge exists");
    let (layout_start_x, layout_start_y, layout_end_x, layout_end_y, layout_bends) = {
        let section = layout_edge
            .borrow_mut()
            .sections()
            .get(0)
            .expect("layout section should exist");
        let mut section_mut = section.borrow_mut();
        (
            section_mut.start_x(),
            section_mut.start_y(),
            section_mut.end_x(),
            section_mut.end_y(),
            section_mut.bend_points().len(),
        )
    };
    assert_eq!((layout_start_x, layout_start_y), (5.0, 6.0));
    assert_eq!((layout_end_x, layout_end_y), (35.0, 16.0));
    assert_eq!(layout_bends, 2);

    let section_edge = find_edge_by_identifier(&graph, "p_section_source", "p_section_target")
        .expect("edgeSection alias edge exists");
    let (section_start_x, section_start_y, section_end_x, section_end_y, section_bends) = {
        let section = section_edge
            .borrow_mut()
            .sections()
            .get(0)
            .expect("edgeSection should exist");
        let mut section_mut = section.borrow_mut();
        (
            section_mut.start_x(),
            section_mut.start_y(),
            section_mut.end_x(),
            section_mut.end_y(),
            section_mut.bend_points().len(),
        )
    };
    assert_eq!((section_start_x, section_start_y), (40.0, 30.0));
    assert_eq!((section_end_x, section_end_y), (90.0, 35.0));
    assert_eq!(section_bends, 2);

    run_layout(&graph);
}

#[test]
fn load_layout_parentheses_and_trailing_section_links() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_layout_variants.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("layout variant ELKT should load");

    let n0 = find_node_by_identifier(&graph, "n0").expect("n0 should exist");
    let (n0_x, n0_y, n0_w, n0_h) = {
        let mut node_mut = n0.borrow_mut();
        let shape = node_mut.connectable().shape();
        (shape.x(), shape.y(), shape.width(), shape.height())
    };
    assert_eq!((n0_x, n0_y), (5.0, 6.0));
    assert_eq!((n0_w, n0_h), (30.0, 20.0));

    let p0 = find_port_by_identifier(&graph, "p0").expect("p0 should exist");
    let (p0_x, p0_y, p0_side) = {
        let mut port_mut = p0.borrow_mut();
        let (x, y) = {
            let shape = port_mut.connectable().shape();
            (shape.x(), shape.y())
        };
        let side = port_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .get_property(LayeredOptions::PORT_SIDE)
            .expect("p0 side should be parsed");
        (x, y, side)
    };
    assert_eq!((p0_x, p0_y), (30.0, 10.0));
    assert_eq!(p0_side, PortSide::East);

    let edge_paren = find_edge_by_identifier(&graph, "p0", "p1").expect("edge p0->p1 should exist");
    let (start_x, start_y, end_x, end_y, bend_count) = {
        let section = edge_paren
            .borrow_mut()
            .sections()
            .get(0)
            .expect("edge section should exist");
        let mut section_mut = section.borrow_mut();
        (
            section_mut.start_x(),
            section_mut.start_y(),
            section_mut.end_x(),
            section_mut.end_y(),
            section_mut.bend_points().len(),
        )
    };
    assert_eq!((start_x, start_y), (35.0, 16.0));
    assert_eq!((end_x, end_y), (80.0, 16.0));
    assert_eq!(bend_count, 2);

    let edge_trailing = find_edge_by_identifier(&graph, "p2", "p3")
        .expect("edge with trailing section links should exist");
    let (section_count, s1_targets) = {
        let mut edge_mut = edge_trailing.borrow_mut();
        let sections: Vec<_> = edge_mut.sections().iter().cloned().collect();
        let section_count = sections.len();
        let mut s1_targets = Vec::new();
        for section in sections {
            let id = section.borrow().identifier().map(ToString::to_string);
            if id.as_deref() == Some("s1") {
                s1_targets = section
                    .borrow()
                    .outgoing_sections()
                    .iter()
                    .filter_map(|target| target.borrow().identifier().map(ToString::to_string))
                    .collect::<Vec<_>>();
            }
        }
        (section_count, s1_targets)
    };
    assert_eq!(section_count, 2);
    assert_eq!(s1_targets, vec!["s0".to_string()]);

    run_layout(&graph);
}

#[test]
fn load_colon_and_equals_key_value_variants() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_key_value_variants.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("key/value variant ELKT should load");

    let (direction, spacing_node_node, spacing_edge_edge) = {
        let mut graph_mut = graph.borrow_mut();
        let props = graph_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut();
        (
            props
                .get_property(CoreOptions::DIRECTION)
                .expect("direction should be parsed with '='"),
            props
                .get_property(CoreOptions::SPACING_NODE_NODE)
                .expect("node spacing should be parsed with '='"),
            props
                .get_property(CoreOptions::SPACING_EDGE_EDGE)
                .expect("edge spacing should be parsed with '='"),
        )
    };
    assert_eq!(direction, Direction::Down);
    assert_eq!(spacing_node_node, 11.0);
    assert_eq!(spacing_edge_edge, 4.0);

    let node = find_node_by_identifier(&graph, "n_eq").expect("n_eq should exist");
    let constraints = node
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(LayeredOptions::PORT_CONSTRAINTS)
        .expect("portConstraints should be parsed with '='");
    assert_eq!(constraints, PortConstraints::FixedSide);

    let port = find_port_by_identifier(&graph, "p_eq").expect("p_eq should exist");
    let (port_side, border_offset, anchor, x, y) = {
        let mut port_mut = port.borrow_mut();
        let (x, y) = {
            let shape = port_mut.connectable().shape();
            (shape.x(), shape.y())
        };
        let props = port_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut();
        (
            props
                .get_property(LayeredOptions::PORT_SIDE)
                .expect("side should be parsed with '='"),
            props
                .get_property(LayeredOptions::PORT_BORDER_OFFSET)
                .expect("portBorderOffset should be parsed with '='"),
            props
                .get_property(LayeredOptions::PORT_ANCHOR)
                .expect("portAnchor should be parsed with '='"),
            x,
            y,
        )
    };
    assert_eq!(port_side, PortSide::East);
    assert_eq!(border_offset, 1.5);
    assert_eq!((anchor.x, anchor.y), (2.0, 3.0));
    assert_eq!((x, y), (30.0, 10.0));

    let edge = find_edge_by_identifier(&graph, "p_eq", "p_tgt").expect("e_eq should exist");
    let (edge_placement, label_placement, start_x, start_y, end_x, end_y, bend_count) = {
        let mut edge_mut = edge.borrow_mut();
        let edge_placement = edge_mut
            .element()
            .properties_mut()
            .get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
            .expect("edge property should be parsed with '='");
        let label_placement = edge_mut
            .element()
            .labels()
            .get(0)
            .expect("edge label should exist")
            .borrow_mut()
            .shape()
            .graph_element()
            .properties_mut()
            .get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
            .expect("edge label inline placement should be parsed with '='");
        let section = edge_mut
            .sections()
            .get(0)
            .expect("edge section should exist");
        let mut section_mut = section.borrow_mut();
        (
            edge_placement,
            label_placement,
            section_mut.start_x(),
            section_mut.start_y(),
            section_mut.end_x(),
            section_mut.end_y(),
            section_mut.bend_points().len(),
        )
    };
    assert_eq!(edge_placement, EdgeLabelPlacement::Tail);
    assert_eq!(label_placement, EdgeLabelPlacement::Head);
    assert_eq!((start_x, start_y), (33.0, 14.0));
    assert_eq!((end_x, end_y), (60.0, 14.0));
    assert_eq!(bend_count, 2);

    run_layout(&graph);
}

#[test]
fn load_qualified_option_ids_and_apply_properties() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_qualified_option_ids.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("qualified option id ELKT should load");

    let (direction, spacing_node_node, spacing_edge_edge, port_labels_placement) = {
        let mut graph_mut = graph.borrow_mut();
        let props = graph_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut();
        (
            props
                .get_property(CoreOptions::DIRECTION)
                .expect("qualified direction should be parsed"),
            props
                .get_property(CoreOptions::SPACING_NODE_NODE)
                .expect("qualified node-node spacing should be parsed"),
            props
                .get_property(CoreOptions::SPACING_EDGE_EDGE)
                .expect("qualified edge-edge spacing should be parsed"),
            props
                .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                .expect("qualified port labels placement should be parsed"),
        )
    };
    assert_eq!(direction, Direction::Down);
    assert_eq!(spacing_node_node, 13.0);
    assert_eq!(spacing_edge_edge, 7.0);
    assert!(port_labels_placement.contains(&PortLabelPlacement::Outside));
    assert!(port_labels_placement.contains(&PortLabelPlacement::NextToPortIfPossible));

    let node = find_node_by_identifier(&graph, "n_q1").expect("n_q1 should exist");
    let node_constraints = node
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(LayeredOptions::PORT_CONSTRAINTS)
        .expect("qualified portConstraints should be parsed");
    assert_eq!(node_constraints, PortConstraints::FixedSide);

    let port = find_port_by_identifier(&graph, "p_q1").expect("p_q1 should exist");
    let (port_side, port_border_offset, port_anchor) = {
        let mut port_mut = port.borrow_mut();
        let props = port_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut();
        (
            props
                .get_property(LayeredOptions::PORT_SIDE)
                .expect("qualified portSide should be parsed"),
            props
                .get_property(LayeredOptions::PORT_BORDER_OFFSET)
                .expect("qualified portBorderOffset should be parsed"),
            props
                .get_property(LayeredOptions::PORT_ANCHOR)
                .expect("qualified portAnchor should be parsed"),
        )
    };
    assert_eq!(port_side, PortSide::East);
    assert_eq!(port_border_offset, 2.0);
    assert_eq!((port_anchor.x, port_anchor.y), (1.0, 2.0));

    let edge = find_edge_by_identifier(&graph, "p_q1", "p_q2").expect("e_q should exist");
    let (
        edge_placement,
        label_placement,
        junction_count,
        start_x,
        start_y,
        end_x,
        end_y,
        bend_count,
    ) = {
        let mut edge_mut = edge.borrow_mut();
        let edge_placement = edge_mut
            .element()
            .properties_mut()
            .get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
            .expect("qualified edge label placement should be parsed");
        let label_placement = edge_mut
            .element()
            .labels()
            .get(0)
            .expect("edge label should exist")
            .borrow_mut()
            .shape()
            .graph_element()
            .properties_mut()
            .get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
            .expect("qualified inline edge label placement should be parsed");
        let junctions = edge_mut
            .element()
            .properties_mut()
            .get_property(LayeredOptions::JUNCTION_POINTS)
            .expect("qualified junction points should be parsed");
        let section = edge_mut
            .sections()
            .get(0)
            .expect("edge section should exist");
        let mut section_mut = section.borrow_mut();
        (
            edge_placement,
            label_placement,
            junctions.len(),
            section_mut.start_x(),
            section_mut.start_y(),
            section_mut.end_x(),
            section_mut.end_y(),
            section_mut.bend_points().len(),
        )
    };
    assert_eq!(edge_placement, EdgeLabelPlacement::Head);
    assert_eq!(label_placement, EdgeLabelPlacement::Tail);
    assert_eq!(junction_count, 2);
    assert_eq!((start_x, start_y), (3.0, 4.0));
    assert_eq!((end_x, end_y), (30.0, 40.0));
    assert_eq!(bend_count, 2);

    run_layout(&graph);
}

#[test]
fn fail_on_unknown_edge_reference_in_edge_section() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_unknown_edge_reference.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let err = load_layered_graph_from_elkt(&path)
        .err()
        .expect("unknown edge in edgeSection should fail");
    assert!(
        err.contains("line 2:"),
        "expected line context in error: {err}"
    );
    assert!(
        err.contains("edgeSection references unknown edge 'e_missing'"),
        "expected unknown-edge message: {err}"
    );
}

#[test]
fn fail_on_label_declaration_without_parent_context() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_label_without_parent.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let err = load_layered_graph_from_elkt(&path)
        .err()
        .expect("label declaration without parent context should fail");
    assert!(
        err.contains("line 1:"),
        "expected line context in error: {err}"
    );
    assert!(
        err.contains("label declaration must be inside node/port/edge/label block"),
        "expected parent-context message: {err}"
    );
}

#[test]
fn fail_when_section_shape_reference_is_missing() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_invalid_section_shape.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let err = load_layered_graph_from_elkt(&path)
        .err()
        .expect("missing section shape reference should fail");
    assert!(
        err.contains("line 10:"),
        "expected line context in error: {err}"
    );
    assert!(
        err.contains("outgoingShape:p_missing"),
        "expected source line snippet in error: {err}"
    );
    assert!(err.contains("references unknown outgoing shape 'p_missing'"));
}

#[test]
fn fail_when_section_link_target_is_empty() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/elkt_parser_invalid_section_link_empty_target.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let err = load_layered_graph_from_elkt(&path)
        .err()
        .expect("empty section link target should fail");
    assert!(
        err.contains("line 5:"),
        "expected line context in error: {err}"
    );
    assert!(
        err.contains("section s0 -> [ outgoing:n0 incoming:n1 start:0,0 end:10,0 ]"),
        "expected source line snippet in error: {err}"
    );
    assert!(err.contains("section link has empty target section list"));
}
