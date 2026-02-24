use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{LGraph, LNode, Layer};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::trace_recorder::serialize_lgraph_snapshot;

fn temp_trace_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("elk-rs-trace-recorder-test-{nanos}"))
}

#[test]
fn trace_recorder_keeps_layerless_nodes_at_minus_one_even_in_layer_list() {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());

    // Mimics comment post-processing behavior: node is appended into a layer
    // list without assigning node.layer.
    let node = LNode::new(&graph);
    layer
        .lock()
        .expect("layer lock")
        .nodes_mut()
        .push(node);

    let trace_dir = temp_trace_dir();
    {
        let graph_guard = graph.lock().expect("graph lock");
        serialize_lgraph_snapshot(&graph_guard, 0, "CommentPostprocessor", &trace_dir)
            .expect("serialize snapshot");
    }

    let snapshot_path = trace_dir.join("step_000_CommentPostprocessor.json");
    let snapshot_bytes = fs::read(&snapshot_path).expect("read snapshot");
    let snapshot: serde_json::Value =
        serde_json::from_slice(&snapshot_bytes).expect("parse snapshot json");
    let layer_value = snapshot["layers"][0]["nodes"][0]["layer"].as_i64();
    assert_eq!(layer_value, Some(-1));

    let _ = fs::remove_dir_all(&trace_dir);
}
