
use std::env;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;

use crate::common::elkt_test_loader::load_layered_graph_from_elk_text;
use crate::common::issue_support::{init_layered_options, run_layout, set_node_property};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredOptions, NodePlacementStrategy,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::PortSide;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkLabelRef, ElkNodeRef, ElkPortRef};

const MIN_EXTERNAL_PORT_LABEL_CHECKED: usize = 5;

fn external_port_label_resources() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../external/elk-models");
    [
        "tests/core/label_placement/port_labels/variants.elkt",
        "tests/core/label_placement/port_labels/next_to_port_if_possible_inside.elkt",
        "tests/core/label_placement/port_labels/next_to_port_if_possible_outside.elkt",
        "tests/core/label_placement/port_labels/treat_as_group_inside.elkt",
        "tests/core/label_placement/port_labels/treat_as_group_outside.elkt",
        "tickets/core/056_portLabelPlacement.elkt",
        "tickets/layered/405_differentPortLabelPositionsNSWEOutside.elkt",
        "tickets/layered/405_differentPortLabelPositionsNSWEOutsideNextToPort.elkt",
        "tickets/layered/405_differentPortLabelPositionsNSWEInside.elkt",
        "tickets/layered/405_differentPortLabelPositionsTwoWestOutside.elkt",
        "tickets/layered/405_differentPortLabelPositionsTwoWestOutsideNextToPort.elkt",
        "tickets/layered/596_outsideHierarchicalPortLabels.elkt",
        "tickets/layered/701_portLabels.elkt",
    ]
    .into_iter()
    .map(|relative| root.join(relative))
    .collect()
}

fn collect_ports(root: &ElkNodeRef) -> Vec<ElkPortRef> {
    let mut queue = vec![root.clone()];
    let mut ports = Vec::new();
    while let Some(node) = queue.pop() {
        let (children, node_ports): (Vec<ElkNodeRef>, Vec<ElkPortRef>) = {
            let mut node_mut = node.borrow_mut();
            let children = node_mut.children().iter().cloned().collect();
            let ports = node_mut.ports().iter().cloned().collect();
            (children, ports)
        };
        queue.extend(children);
        ports.extend(node_ports);
    }
    ports
}

fn port_label_count_and_finite_geometry(graph: &ElkNodeRef, resource: &str) -> usize {
    let mut label_count = 0usize;
    for port in collect_ports(graph) {
        let (side, labels): (PortSide, Vec<ElkLabelRef>) = {
            let mut port_mut = port.borrow_mut();
            let shape = port_mut.connectable().shape();
            let graph_element = shape.graph_element();
            let side = graph_element
                .properties_mut()
                .get_property(CoreOptions::PORT_SIDE)
                .unwrap_or(PortSide::Undefined);
            let labels = graph_element.labels().iter().cloned().collect();
            (side, labels)
        };

        if !labels.is_empty() {
            assert!(
                side != PortSide::Undefined,
                "port side must be resolved for labeled port in {resource}"
            );
        }

        for label in labels {
            label_count += 1;
            let mut label_mut = label.borrow_mut();
            let shape = label_mut.shape();
            for value in [shape.x(), shape.y(), shape.width(), shape.height()] {
                assert!(
                    value.is_finite(),
                    "non-finite label geometry in {resource}: {value}"
                );
            }
            assert!(
                shape.width() >= 0.0 && shape.height() >= 0.0,
                "negative label size in {resource}: ({}, {})",
                shape.width(),
                shape.height()
            );
        }
    }
    label_count
}

fn apply_uniform_port_side(graph: &ElkNodeRef, side: PortSide) {
    for port in collect_ports(graph) {
        let mut port_mut = port.borrow_mut();
        port_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_SIDE, Some(side));
    }
}

#[test]
fn external_port_label_resources_keep_finite_geometry_if_available() {
    init_layered_options();

    let mut checked = 0usize;
    let mut parse_failures = Vec::new();
    let mut layout_failures = Vec::new();
    let mut no_label_resources = Vec::new();

    for resource in external_port_label_resources() {
        if !resource.exists() {
            continue;
        }
        let path = resource.to_string_lossy().into_owned();
        let graph = match load_layered_graph_from_elk_text(&path) {
            Ok(graph) => graph,
            Err(err) => {
                parse_failures.push(format!("{path}: {err}"));
                continue;
            }
        };

        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            run_layout(&graph);
            port_label_count_and_finite_geometry(&graph, &path)
        }));
        match result {
            Ok(label_count) => {
                if label_count == 0 {
                    no_label_resources.push(path);
                    continue;
                }
                checked += 1;
            }
            Err(_) => layout_failures.push(path),
        }
    }

    assert!(
        checked >= MIN_EXTERNAL_PORT_LABEL_CHECKED,
        "expected at least {MIN_EXTERNAL_PORT_LABEL_CHECKED} validated external port-label resources, got checked={checked}, parse_failures={}, layout_failures={}, no_label_resources={}",
        parse_failures.len(),
        layout_failures.len(),
        no_label_resources.len()
    );
}

#[test]
fn external_port_label_variants_support_configurator_side_sweep_if_available() {
    init_layered_options();

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../external/elk-models/tests/core/label_placement/port_labels/variants.elkt");
    if !path.exists() {
        eprintln!("port-label configurator sweep: variants.elkt not found");
        return;
    }
    let path = path.to_string_lossy().into_owned();

    for side in [
        PortSide::East,
        PortSide::West,
        PortSide::North,
        PortSide::South,
    ] {
        let graph = load_layered_graph_from_elk_text(&path)
            .unwrap_or_else(|err| panic!("failed to load variants resource {path}: {err}"));
        apply_uniform_port_side(&graph, side);
        run_layout(&graph);

        let label_count = port_label_count_and_finite_geometry(&graph, &path);
        assert!(
            label_count > 0,
            "expected at least one port label for configurator side sweep ({side:?})"
        );
    }
}

#[test]
fn ticket_701_port_labels_runs_without_panic_if_available() {
    init_layered_options();

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../external/elk-models/tickets/layered/701_portLabels.elkt");
    if !path.exists() {
        eprintln!("port-label regression: 701_portLabels.elkt not found");
        return;
    }
    let path = path.to_string_lossy().into_owned();

    let graph = load_layered_graph_from_elk_text(&path)
        .unwrap_or_else(|err| panic!("failed to load 701_portLabels resource {path}: {err}"));
    set_node_property(
        &graph,
        LayeredOptions::NODE_PLACEMENT_STRATEGY,
        NodePlacementStrategy::BrandesKoepf,
    );
    run_layout(&graph);

    let label_count = port_label_count_and_finite_geometry(&graph, &path);
    assert!(
        label_count > 0,
        "expected at least one port label in 701_portLabels resource"
    );
}
