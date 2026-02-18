mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{
    find_edge_by_identifier, find_port_by_identifier, load_layered_graph_from_elkt,
};
use issue_support::{init_layered_options, run_layout};
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkPortRef;

const PORT_ANCHOR: f64 = 5.0;
const COORDINATE_FUZZINESS: f64 = 0.5;

#[test]
fn issue_447_edges_connect_to_port_anchors() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_447_port_anchor.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_447 resource should load");

    run_layout(&graph);

    let edge_infos = [
        (
            find_edge_by_identifier(&graph, "left_out", "right_in")
                .expect("edge left_out->right_in should exist"),
            find_port_by_identifier(&graph, "left_out").expect("left_out port should exist"),
            find_port_by_identifier(&graph, "right_in").expect("right_in port should exist"),
        ),
        (
            find_edge_by_identifier(&graph, "right_out", "left_in")
                .expect("edge right_out->left_in should exist"),
            find_port_by_identifier(&graph, "right_out").expect("right_out port should exist"),
            find_port_by_identifier(&graph, "left_in").expect("left_in port should exist"),
        ),
    ];

    let mut checked_edges = 0usize;
    for (edge, source_port, target_port) in edge_infos {
        let containing_node = edge.borrow().containing_node();
        let sections = edge
            .borrow_mut()
            .sections()
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        assert!(
            !sections.is_empty(),
            "edge should contain at least one section after layout"
        );

        let source_anchor_abs = port_anchor_absolute(&source_port);
        let target_anchor_abs = port_anchor_absolute(&target_port);
        let mut source_matched = false;
        let mut target_matched = false;

        for section in sections {
            let section_guard = section.borrow();
            let start_abs = ElkUtil::to_absolute(
                KVector::with_values(section_guard.start_x(), section_guard.start_y()),
                containing_node.clone(),
            );
            let end_abs = ElkUtil::to_absolute(
                KVector::with_values(section_guard.end_x(), section_guard.end_y()),
                containing_node.clone(),
            );
            drop(section_guard);

            if matches_anchor(&start_abs, &source_anchor_abs)
                || matches_anchor(&end_abs, &source_anchor_abs)
            {
                source_matched = true;
            }

            if matches_anchor(&start_abs, &target_anchor_abs)
                || matches_anchor(&end_abs, &target_anchor_abs)
            {
                target_matched = true;
            }
        }

        assert!(
            source_matched,
            "edge source endpoint should match source port anchor"
        );
        assert!(
            target_matched,
            "edge target endpoint should match target port anchor"
        );
        checked_edges += 1;
    }

    assert!(checked_edges > 0, "no edges were checked");
}

fn port_anchor_absolute(port: &ElkPortRef) -> KVector {
    let anchor = {
        let mut port_mut = port.borrow_mut();
        let shape = port_mut.connectable().shape();
        KVector::with_values(shape.x() + PORT_ANCHOR, shape.y() + PORT_ANCHOR)
    };

    ElkUtil::to_absolute(anchor, port.borrow().parent())
}

fn matches_anchor(endpoint_abs: &KVector, anchor_abs: &KVector) -> bool {
    (anchor_abs.x - endpoint_abs.x).abs() <= COORDINATE_FUZZINESS
        && (anchor_abs.y - endpoint_abs.y).abs() <= COORDINATE_FUZZINESS
}
