mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_layout, set_node_property};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::Direction;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkLabelRef, ElkNodeRef};

const POSITION_EPSILON: f64 = 1.0e-4;
type PositionList = Vec<(f64, f64)>;

#[test]
fn issue_405_port_and_label_positions_match_across_directions() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_405_port_label_positions.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let directions = [
        Direction::Right,
        Direction::Down,
        Direction::Left,
        Direction::Up,
    ];

    let mut baseline_ports: Option<Vec<(f64, f64)>> = None;
    let mut baseline_labels: Option<Vec<(f64, f64)>> = None;

    for direction in directions {
        let graph = load_layered_graph_from_elkt(&path).expect("issue_405 resource should load");
        set_node_property(&graph, CoreOptions::DIRECTION, direction);

        run_layout(&graph);

        let reference_node =
            find_node_by_identifier(&graph, "reference").expect("reference node should exist");
        let (port_positions, label_positions) = collect_port_and_label_positions(&reference_node);

        assert_eq!(
            port_positions.len(),
            4,
            "expected four ports for direction {:?}",
            direction
        );
        assert_eq!(
            label_positions.len(),
            4,
            "expected one label per port for direction {:?}",
            direction
        );

        for (x, y) in port_positions.iter().chain(label_positions.iter()) {
            assert!(
                x.is_finite() && y.is_finite(),
                "non-finite coordinates for direction {:?}",
                direction
            );
        }

        if let (Some(reference_ports), Some(reference_labels)) = (&baseline_ports, &baseline_labels) {
            assert_positions_match(reference_ports, &port_positions, "port", direction);
            assert_positions_match(reference_labels, &label_positions, "port label", direction);
        } else {
            baseline_ports = Some(port_positions);
            baseline_labels = Some(label_positions);
        }
    }
}

fn collect_port_and_label_positions(node: &ElkNodeRef) -> (PositionList, PositionList) {
    let ports: Vec<_> = node.borrow_mut().ports().iter().cloned().collect();

    let mut port_positions = Vec::new();
    let mut label_positions = Vec::new();
    for port in ports {
        let (port_pos, labels) = {
            let mut port_mut = port.borrow_mut();
            let shape = port_mut.connectable().shape();
            let labels: Vec<ElkLabelRef> = shape.graph_element().labels().iter().cloned().collect();
            ((shape.x(), shape.y()), labels)
        };
        port_positions.push(port_pos);

        for label in labels {
            let mut label_mut = label.borrow_mut();
            let shape = label_mut.shape();
            label_positions.push((shape.x(), shape.y()));
        }
    }

    (port_positions, label_positions)
}

fn assert_positions_match(reference: &[(f64, f64)], current: &[(f64, f64)], kind: &str, direction: Direction) {
    assert_eq!(
        reference.len(),
        current.len(),
        "{kind} count mismatch for direction {:?}",
        direction
    );

    let mut reference_norm = normalize_positions(reference);
    let mut current_norm = normalize_positions(current);
    reference_norm.sort_unstable();
    current_norm.sort_unstable();

    assert_eq!(
        reference_norm, current_norm,
        "{kind} positions differ for direction {:?}",
        direction
    );
}

fn normalize_positions(positions: &[(f64, f64)]) -> Vec<(i64, i64)> {
    positions
        .iter()
        .map(|(x, y)| {
            (
                (x / POSITION_EPSILON).round() as i64,
                (y / POSITION_EPSILON).round() as i64,
            )
        })
        .collect()
}
