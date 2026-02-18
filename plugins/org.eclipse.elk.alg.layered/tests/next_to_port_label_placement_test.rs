mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{
    find_node_by_identifier, find_port_by_identifier, load_layered_graph_from_elkt,
};
use issue_support::{init_layered_options, run_layout};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::PortLabelPlacement;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkNodeRef, ElkPortRef};

const EPSILON: f64 = 1e-6;

#[test]
fn test_next_to_port_labels_inside() {
    run_next_to_port_case("next_to_port_if_possible_inside.elkt");
}

#[test]
fn test_next_to_port_labels_outside() {
    run_next_to_port_case("next_to_port_if_possible_outside.elkt");
}

fn run_next_to_port_case(resource: &str) {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/common_nodespacing/{resource}",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path)
        .unwrap_or_else(|err| panic!("resource {resource} should load: {err}"));
    run_layout(&graph);

    let parent = find_node_by_identifier(&graph, "parent").expect("parent node should exist");
    let external_port =
        find_port_by_identifier(&graph, "p_external").expect("external port should exist");
    let internal_port =
        find_port_by_identifier(&graph, "p_internal").expect("internal port should exist");

    let placement = parent
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
        .expect("port labels placement should be set");
    let inside_label_placement = placement.contains(&PortLabelPlacement::Inside);
    assert!(
        placement.contains(&PortLabelPlacement::NextToPortIfPossible),
        "next-to-port placement must be enabled in {resource}"
    );

    let external_should_be_next =
        should_label_be_placed_next_to_port(&external_port, &parent, inside_label_placement);
    let internal_should_be_next =
        should_label_be_placed_next_to_port(&internal_port, &parent, inside_label_placement);

    assert_ne!(
        external_should_be_next, internal_should_be_next,
        "resource {resource} should exercise both next-to-port outcomes"
    );

    assert_port_label_position(
        &external_port,
        external_should_be_next,
        resource,
        "p_external",
    );
    assert_port_label_position(
        &internal_port,
        internal_should_be_next,
        resource,
        "p_internal",
    );
}

fn assert_port_label_position(
    port: &ElkPortRef,
    should_be_next_to_port: bool,
    resource: &str,
    port_id: &str,
) {
    let (port_height, label) = {
        let mut port_mut = port.borrow_mut();
        let shape = port_mut.connectable().shape();
        let port_height = shape.height();
        let label = shape
            .graph_element()
            .labels()
            .get(0)
            .unwrap_or_else(|| panic!("port {port_id} should have one label"));
        (port_height, label)
    };

    let (label_y, label_height) = {
        let mut label_mut = label.borrow_mut();
        let label_shape = label_mut.shape();
        (label_shape.y(), label_shape.height())
    };

    if should_be_next_to_port {
        assert!(
            label_y + label_height >= -EPSILON && label_y <= port_height + EPSILON,
            "resource {resource}, port {port_id}: expected next-to-port placement, got y={label_y}, h={label_height}, port_h={port_height}"
        );
    } else {
        assert!(
            label_y + label_height <= EPSILON || label_y >= port_height - EPSILON,
            "resource {resource}, port {port_id}: expected above/below-port placement, got y={label_y}, h={label_height}, port_h={port_height}"
        );
    }
}

fn should_label_be_placed_next_to_port(
    port: &ElkPortRef,
    parent: &ElkNodeRef,
    inside_label_placement: bool,
) -> bool {
    let (incoming_edges, outgoing_edges) = {
        let mut port_mut = port.borrow_mut();
        let incoming_edges = port_mut
            .connectable()
            .incoming_edges()
            .iter()
            .collect::<Vec<_>>();
        let outgoing_edges = port_mut
            .connectable()
            .outgoing_edges()
            .iter()
            .collect::<Vec<_>>();
        (incoming_edges, outgoing_edges)
    };

    if incoming_edges.is_empty() && outgoing_edges.is_empty() {
        return true;
    }

    let mut edges_to_insides = false;
    let mut edges_to_somewhere_else = false;

    for out_edge in outgoing_edges {
        let target = {
            let edge = out_edge.borrow();
            edge.targets_ro().get(0)
        };
        if let Some(target_shape) = target {
            if let Some(target_node) = ElkGraphUtil::connectable_shape_to_node(&target_shape) {
                let inside_edge = ElkGraphUtil::is_descendant(&target_node, parent);
                edges_to_insides |= inside_edge;
                edges_to_somewhere_else |= !inside_edge;
            }
        }
    }

    for in_edge in incoming_edges {
        let source = {
            let edge = in_edge.borrow();
            edge.sources_ro().get(0)
        };
        if let Some(source_shape) = source {
            if let Some(source_node) = ElkGraphUtil::connectable_shape_to_node(&source_shape) {
                let inside_edge = ElkGraphUtil::is_descendant(&source_node, parent);
                edges_to_insides |= inside_edge;
                edges_to_somewhere_else |= !inside_edge;
            }
        }
    }

    (inside_label_placement && !edges_to_insides)
        || (!inside_label_placement && !edges_to_somewhere_else)
}
