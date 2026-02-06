mod elkt_test_loader;
mod issue_support;

use std::collections::VecDeque;

use elkt_test_loader::load_layered_graph_from_elkt;
use issue_support::{init_layered_options, run_recursive_layout};
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkRectangle;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

const MAX_VERTICAL_OVERLAP: f64 = 6.0;

#[test]
fn issue_603_compound_labels_stay_in_top_band() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_603_compound_label_top_band.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_603 resource should load");

    run_recursive_layout(&graph);

    let mut queue: VecDeque<ElkNodeRef> = VecDeque::new();
    let top_children: Vec<_> = graph.borrow_mut().children().iter().cloned().collect();
    queue.extend(top_children);

    while let Some(node) = queue.pop_front() {
        let has_labels = !node
            .borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .labels()
            .is_empty();
        let children: Vec<_> = node.borrow_mut().children().iter().cloned().collect();

        if has_labels && !children.is_empty() {
            assert_label_child_top_band_separation(&node);
        }

        queue.extend(children);
    }
}

fn assert_label_child_top_band_separation(node: &ElkNodeRef) {
    let labels: Vec<_> = node
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .labels()
        .iter()
        .cloned()
        .collect();
    let mut label_rects = Vec::new();
    for label in labels {
        let mut label_mut = label.borrow_mut();
        let shape = label_mut.shape();
        label_rects.push(ElkRectangle::with_values(
            shape.x(),
            shape.y(),
            shape.width(),
            shape.height(),
        ));
    }

    let children: Vec<_> = node.borrow_mut().children().iter().cloned().collect();
    let mut child_rects = Vec::new();
    for child in children {
        let mut child_mut = child.borrow_mut();
        let shape = child_mut.connectable().shape();
        child_rects.push(ElkRectangle::with_values(
            shape.x(),
            shape.y(),
            shape.width(),
            shape.height(),
        ));
    }

    for label_rect in label_rects {
        for child_rect in &child_rects {
            let vertical_overlap = overlap_on_axis(
                label_rect.y,
                label_rect.y + label_rect.height,
                child_rect.y,
                child_rect.y + child_rect.height,
            );

            assert!(
                vertical_overlap <= MAX_VERTICAL_OVERLAP,
                "label/child vertical overlap too large ({} > {}): label={} child={}",
                vertical_overlap,
                MAX_VERTICAL_OVERLAP,
                label_rect,
                child_rect
            );
        }
    }
}

fn overlap_on_axis(first_min: f64, first_max: f64, second_min: f64, second_max: f64) -> f64 {
    (first_max.min(second_max) - first_min.max(second_min)).max(0.0)
}
