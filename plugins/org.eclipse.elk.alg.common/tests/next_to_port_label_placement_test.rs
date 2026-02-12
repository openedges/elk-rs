#[path = "../../org.eclipse.elk.alg.layered/tests/elkt_test_loader.rs"]
mod elkt_test_loader;

use elkt_test_loader::load_layered_graph_from_elkt;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::PortLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, ElkUtil};
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkLabelRef, ElkNodeRef, ElkPortRef};
use std::collections::VecDeque;
use std::path::PathBuf;

fn run_layout(graph: &ElkNodeRef) {
    let mut provider = LayeredLayoutProvider::new();
    provider.layout(graph, &mut BasicProgressMonitor::new());
}

const LABEL_CHAR_WIDTH: f64 = 6.0;
const LABEL_HEIGHT: f64 = 10.0;

fn apply_default_port_configuration(graph: &ElkNodeRef) {
    let children: Vec<ElkNodeRef> = graph.borrow_mut().children().iter().cloned().collect();
    let mut queue: VecDeque<ElkNodeRef> = VecDeque::from(children);
    while let Some(node) = queue.pop_front() {
        let (children, ports): (Vec<ElkNodeRef>, Vec<ElkPortRef>) = {
            let mut node_mut = node.borrow_mut();
            let children = node_mut.children().iter().cloned().collect();
            let ports = node_mut.ports().iter().cloned().collect();
            (children, ports)
        };
        for port in ports {
            ElkUtil::configure_with_default_values(&port);
        }
        queue.extend(children);
    }
}

fn ensure_label_sizes(graph: &ElkNodeRef) {
    let mut queue: VecDeque<ElkNodeRef> = VecDeque::from([graph.clone()]);
    while let Some(node) = queue.pop_front() {
        let (children, node_labels, ports): (Vec<ElkNodeRef>, Vec<ElkLabelRef>, Vec<ElkPortRef>) = {
            let mut node_mut = node.borrow_mut();
            let labels = node_mut
                .connectable()
                .shape()
                .graph_element()
                .labels()
                .iter()
                .cloned()
                .collect();
            let ports = node_mut.ports().iter().cloned().collect();
            let children = node_mut.children().iter().cloned().collect();
            (children, labels, ports)
        };

        for label in node_labels {
            set_label_size_if_missing(&label);
        }

        for port in ports {
            let port_labels: Vec<ElkLabelRef> = {
                let mut port_mut = port.borrow_mut();
                port_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect()
            };
            for label in port_labels {
                set_label_size_if_missing(&label);
            }
        }

        queue.extend(children);
    }
}

fn set_label_size_if_missing(label: &ElkLabelRef) {
    let (width, height, text) = {
        let mut label_mut = label.borrow_mut();
        let shape = label_mut.shape();
        (shape.width(), shape.height(), label_mut.text().to_string())
    };
    if width != 0.0 || height != 0.0 || text.is_empty() {
        return;
    }
    let width = (text.chars().count() as f64) * LABEL_CHAR_WIDTH;
    let height = LABEL_HEIGHT;
    label
        .borrow_mut()
        .shape()
        .set_dimensions(width.max(1.0), height);
}

fn label_for_port(port: &ElkPortRef) -> ElkLabelRef {
    let label = {
        let mut port_mut = port.borrow_mut();
        port_mut
            .connectable()
            .shape()
            .graph_element()
            .labels()
            .get(0)
    };
    label.expect("port should have one label")
}

fn should_label_be_placed_next_to_port(port: &ElkPortRef, inside_label_placement: bool) -> bool {
    let (incoming, outgoing, parent) = {
        let mut port_mut = port.borrow_mut();
        let incoming = port_mut
            .connectable()
            .incoming_edges()
            .iter()
            .collect::<Vec<_>>();
        let outgoing = port_mut
            .connectable()
            .outgoing_edges()
            .iter()
            .collect::<Vec<_>>();
        let parent = port_mut.parent();
        (incoming, outgoing, parent)
    };

    if incoming.is_empty() && outgoing.is_empty() {
        return true;
    }

    let parent = parent.expect("port should have a parent");
    let mut edges_to_insides = false;
    let mut edges_to_somewhere_else = false;

    for edge in outgoing {
        let target = edge.borrow().targets_ro().get(0);
        if let Some(shape) = target {
            if let Some(node) = ElkGraphUtil::connectable_shape_to_node(&shape) {
                let inside = ElkGraphUtil::is_descendant(&node, &parent);
                edges_to_insides |= inside;
                edges_to_somewhere_else |= !inside;
            }
        }
    }

    for edge in incoming {
        let source = edge.borrow().sources_ro().get(0);
        if let Some(shape) = source {
            if let Some(node) = ElkGraphUtil::connectable_shape_to_node(&shape) {
                let inside = ElkGraphUtil::is_descendant(&node, &parent);
                edges_to_insides |= inside;
                edges_to_somewhere_else |= !inside;
            }
        }
    }

    (inside_label_placement && !edges_to_insides)
        || (!inside_label_placement && !edges_to_somewhere_else)
}

fn test_port_label(port: &ElkPortRef, inside_label_placement: bool) {
    let label = label_for_port(port);
    let label_text = label.borrow().text().to_string();
    let (label_y, label_height) = {
        let mut label_mut = label.borrow_mut();
        let label_shape = label_mut.shape();
        (label_shape.y(), label_shape.height())
    };
    let (port_height, port_id) = {
        let mut port_mut = port.borrow_mut();
        let port_height = port_mut.connectable().shape().height();
        let port_id = port_mut
            .connectable()
            .shape()
            .graph_element()
            .identifier()
            .map(str::to_string)
            .unwrap_or_default();
        (port_height, port_id)
    };

    if should_label_be_placed_next_to_port(port, inside_label_placement) {
        assert!(
            label_y + label_height >= 0.0 && label_y <= port_height,
            "label should be next to port: port={port_id}, label={label_text}, y={label_y}, h={label_height}, port_h={port_height}"
        );
    } else {
        assert!(
            label_y + label_height <= 0.0 || label_y >= port_height,
            "label should be above or below port: port={port_id}, label={label_text}, y={label_y}, h={label_height}, port_h={port_height}"
        );
    }
}

#[test]
fn test_next_to_port_labels() {
    initialize_plain_java_layout();

    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../external/elk-models/tests/core/label_placement/port_labels");
    let resources = [
        "next_to_port_if_possible_inside.elkt",
        "next_to_port_if_possible_outside.elkt",
    ];

    for resource in resources {
        let path = base.join(resource);
        let path = path.to_string_lossy().into_owned();
        let graph = load_layered_graph_from_elkt(&path)
            .unwrap_or_else(|err| panic!("resource {resource} should load: {err}"));

        apply_default_port_configuration(&graph);
        ensure_label_sizes(&graph);
        run_layout(&graph);

        let mut queue: VecDeque<ElkNodeRef> =
            graph.borrow_mut().children().iter().cloned().collect();
        while let Some(node) = queue.pop_front() {
            let (children, ports, placement) = {
                let mut node_mut = node.borrow_mut();
                let children: Vec<_> = node_mut.children().iter().cloned().collect();
                let ports: Vec<_> = node_mut.ports().iter().cloned().collect();
                let placement = node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .get_property(CoreOptions::PORT_LABELS_PLACEMENT);
                (children, ports, placement)
            };

            let inside_label_placement = placement
                .map(|value| value.contains(&PortLabelPlacement::Inside))
                .unwrap_or(false);

            for port in ports {
                test_port_label(&port, inside_label_placement);
            }

            queue.extend(children);
        }
    }
}
