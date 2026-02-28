
use crate::common::elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use crate::common::issue_support::{init_layered_options, run_layout, set_node_property};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{Direction, PortSide};
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkLabelRef, ElkNodeRef};

const POSITION_EPSILON: f64 = 1.0e-4;
type AxisSignatureList = Vec<(PortSide, i64)>;

#[test]
fn issue_405_2_axis_based_port_and_label_positions_match_across_directions() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_405_2_axis_port_labels.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let directions = [
        Direction::Right,
        Direction::Down,
        Direction::Left,
        Direction::Up,
    ];

    let mut baseline_port_axis: Option<Vec<(PortSide, i64)>> = None;
    let mut baseline_label_axis: Option<Vec<(PortSide, i64)>> = None;

    for direction in directions {
        let graph = load_layered_graph_from_elkt(&path).expect("issue_405_2 resource should load");
        set_node_property(&graph, CoreOptions::DIRECTION, direction);

        run_layout(&graph);

        let reference_node =
            find_node_by_identifier(&graph, "reference").expect("reference node should exist");
        let (port_axis, label_axis) = collect_axis_signatures(&reference_node);

        assert_eq!(
            port_axis.len(),
            4,
            "expected four ports for {:?}",
            direction
        );
        assert_eq!(
            label_axis.len(),
            4,
            "expected one label per port for {:?}",
            direction
        );

        if let (Some(ref_port_axis), Some(ref_label_axis)) =
            (&baseline_port_axis, &baseline_label_axis)
        {
            assert_eq!(
                ref_port_axis, &port_axis,
                "port axis mismatch for {:?}",
                direction
            );
            assert_eq!(
                ref_label_axis, &label_axis,
                "label axis mismatch for {:?}",
                direction
            );
        } else {
            baseline_port_axis = Some(port_axis);
            baseline_label_axis = Some(label_axis);
        }
    }
}

fn collect_axis_signatures(node: &ElkNodeRef) -> (AxisSignatureList, AxisSignatureList) {
    let ports: Vec<_> = node.borrow_mut().ports().iter().cloned().collect();
    let mut port_axis = Vec::new();
    let mut label_axis = Vec::new();

    for port in ports {
        let (side, port_x, port_y, labels) = {
            let mut port_mut = port.borrow_mut();
            let shape = port_mut.connectable().shape();
            let side = shape
                .graph_element()
                .properties_mut()
                .get_property(LayeredOptions::PORT_SIDE)
                .unwrap_or(PortSide::Undefined);
            let labels: Vec<ElkLabelRef> = shape.graph_element().labels().iter().cloned().collect();
            (side, shape.x(), shape.y(), labels)
        };

        port_axis.push((side, quantize(axis_value(side, port_x, port_y))));

        for label in labels {
            let mut label_mut = label.borrow_mut();
            let label_shape = label_mut.shape();
            label_axis.push((
                side,
                quantize(axis_value(side, label_shape.x(), label_shape.y())),
            ));
        }
    }

    port_axis.sort_unstable();
    label_axis.sort_unstable();
    (port_axis, label_axis)
}

fn axis_value(side: PortSide, x: f64, y: f64) -> f64 {
    if matches!(side, PortSide::East | PortSide::West) {
        x
    } else {
        y
    }
}

fn quantize(value: f64) -> i64 {
    (value / POSITION_EPSILON).round() as i64
}
