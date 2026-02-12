mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::load_layered_graph_from_elkt;
use issue_support::{init_layered_options, node_bounds, run_layout};

#[test]
fn issue_433_self_loop_label_is_inside_graph_bounds() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_433_self_loop_label_bounds.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_433 resource should load");

    run_layout(&graph);

    let (_, _, graph_width, graph_height) = node_bounds(&graph);
    let label = graph
        .borrow_mut()
        .contained_edges()
        .iter()
        .flat_map(|edge| {
            let mut edge_mut = edge.borrow_mut();
            edge_mut.element().labels().iter().cloned().collect::<Vec<_>>()
        })
        .next()
        .expect("self-loop label should exist");
    let mut label_mut = label.borrow_mut();
    let shape = label_mut.shape();
    assert!(
        shape.x() >= 0.0
            && shape.y() >= 0.0
            && shape.x() + shape.width() <= graph_width + 0.5
            && shape.y() + shape.height() <= graph_height + 0.5,
        "label outside graph bounds: label=({}, {}, {}, {}), graph=({}, {})",
        shape.x(),
        shape.y(),
        shape.width(),
        shape.height(),
        graph_width,
        graph_height
    );
}
