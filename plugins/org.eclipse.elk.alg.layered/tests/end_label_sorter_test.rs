use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphRef, LLabel, LNode, LNodeRef, Layer, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::EndLabelSorter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::InternalProperties;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn graph_with_single_layer() -> (LGraphRef, Arc<Mutex<Layer>>) {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());
    (graph, layer)
}

fn add_label_dummy(graph: &LGraphRef, layer: &Arc<Mutex<Layer>>) -> LNodeRef {
    let node = LNode::new(graph);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.set_node_type(NodeType::Label);
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

#[test]
fn test_correct_order() {
    let (graph, layer) = graph_with_single_layer();
    let label_dummy = add_label_dummy(&graph, &layer);

    let label_c = Arc::new(Mutex::new(LLabel::with_text("c")));
    let label_a = Arc::new(Mutex::new(LLabel::with_text("a")));
    let label_b = Arc::new(Mutex::new(LLabel::with_text("b")));

    label_dummy.lock().expect("label dummy lock").set_property(
        InternalProperties::REPRESENTED_LABELS,
        Some(vec![label_c, label_a, label_b]),
    );

    let mut sorter = EndLabelSorter;
    let mut monitor = NullElkProgressMonitor;
    sorter.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let ordered = label_dummy
        .lock()
        .expect("label dummy lock")
        .get_property(InternalProperties::REPRESENTED_LABELS)
        .unwrap_or_default()
        .iter()
        .map(|label| label.lock().expect("label lock").text().to_owned())
        .collect::<Vec<_>>();

    assert_eq!(
        ordered,
        vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
    );
}
