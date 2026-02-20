use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use serde_json::{json, Value};

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LGraph, LNodeRef};

fn serialize_label(label: &Arc<std::sync::Mutex<super::graph::LLabel>>) -> Option<Value> {
    let mut guard = label.lock().ok()?;
    let text = guard.text().to_string();
    let pos_x = guard.shape().position_ref().x;
    let pos_y = guard.shape().position_ref().y;
    let size_x = guard.shape().size_ref().x;
    let size_y = guard.shape().size_ref().y;
    Some(json!({
        "text": text,
        "x": pos_x,
        "y": pos_y,
        "width": size_x,
        "height": size_y,
    }))
}

fn serialize_port(port: &Arc<std::sync::Mutex<super::graph::LPort>>) -> Option<Value> {
    let mut guard = port.lock().ok()?;
    let id = format!("P{}", guard.shape().graph_element().id);
    let pos_x = guard.shape().position_ref().x;
    let pos_y = guard.shape().position_ref().y;
    let side = guard.side();
    let side_str = format!("{:?}", side).to_uppercase();

    let label_refs: Vec<_> = guard.labels().clone();
    drop(guard);

    let labels: Vec<Value> = label_refs.iter().filter_map(serialize_label).collect();

    Some(json!({
        "id": id,
        "side": side_str,
        "x": pos_x,
        "y": pos_y,
        "labels": labels,
    }))
}

fn serialize_node(node: &LNodeRef) -> Option<Value> {
    let mut guard = node.lock().ok()?;
    let id = format!("N{}", guard.shape().graph_element().id);
    let name = guard.designation();
    let node_type = guard.node_type();
    let pos_x = guard.shape().position_ref().x;
    let pos_y = guard.shape().position_ref().y;
    let size_x = guard.shape().size_ref().x;
    let size_y = guard.shape().size_ref().y;

    let port_refs: Vec<_> = guard.ports().clone();
    let label_refs: Vec<_> = guard.labels().clone();
    drop(guard);

    let ports: Vec<Value> = port_refs.iter().filter_map(serialize_port).collect();
    let labels: Vec<Value> = label_refs.iter().filter_map(serialize_label).collect();

    Some(json!({
        "id": id,
        "name": name,
        "type": node_type.name(),
        "x": pos_x,
        "y": pos_y,
        "width": size_x,
        "height": size_y,
        "ports": ports,
        "labels": labels,
    }))
}

fn serialize_edge(edge: &LEdgeRef) -> Option<Value> {
    let mut guard = edge.lock().ok()?;
    let id = format!("E{}", guard.graph_element().id);

    let source_id = guard
        .source()
        .and_then(|port| {
            port.lock().ok().and_then(|p| {
                p.node().and_then(|n| {
                    n.lock()
                        .ok()
                        .map(|mut ng| format!("N{}", ng.shape().graph_element().id))
                })
            })
        })
        .unwrap_or_default();

    let target_id = guard
        .target()
        .and_then(|port| {
            port.lock().ok().and_then(|p| {
                p.node().and_then(|n| {
                    n.lock()
                        .ok()
                        .map(|mut ng| format!("N{}", ng.shape().graph_element().id))
                })
            })
        })
        .unwrap_or_default();

    let bend_points: Vec<Value> = guard
        .bend_points_ref()
        .iter()
        .map(|v| json!({"x": v.x, "y": v.y}))
        .collect();

    let label_refs: Vec<_> = guard.labels().clone();
    drop(guard);

    let labels: Vec<Value> = label_refs.iter().filter_map(serialize_label).collect();

    Some(json!({
        "id": id,
        "source": source_id,
        "target": target_id,
        "bendPoints": bend_points,
        "labels": labels,
    }))
}

/// Serialize the current state of an LGraph to a JSON snapshot file.
///
/// The output file is written to `{output_dir}/step_{step:02}_{processor_name}.json`.
/// This is intended to be called after each processor step to enable parity
/// comparison with Java ELK trace output.
pub fn serialize_lgraph_snapshot(
    lgraph: &LGraph,
    step: usize,
    processor_name: &str,
    output_dir: &Path,
) -> std::io::Result<()> {
    // Build layers array
    let mut layers_json: Vec<Value> = Vec::new();
    for (layer_index, layer) in lgraph.layers().iter().enumerate() {
        let nodes_json: Vec<Value> = if let Ok(layer_guard) = layer.lock() {
            layer_guard
                .nodes()
                .iter()
                .filter_map(serialize_node)
                .collect()
        } else {
            Vec::new()
        };
        layers_json.push(json!({
            "index": layer_index,
            "nodes": nodes_json,
        }));
    }

    // Also include layerless nodes as a special layer with index -1
    let layerless: Vec<Value> = lgraph
        .layerless_nodes()
        .iter()
        .filter_map(serialize_node)
        .collect();
    if !layerless.is_empty() {
        layers_json.insert(
            0,
            json!({
                "index": -1,
                "nodes": layerless,
            }),
        );
    }

    // Collect all edges from all nodes (outgoing only to avoid duplicates)
    let mut seen_edges: HashSet<usize> = HashSet::new();
    let mut edges_json: Vec<Value> = Vec::new();

    let mut collect_edges_from_nodes = |nodes: &[LNodeRef]| {
        for node in nodes {
            let ports = match node.lock() {
                Ok(guard) => guard.ports().clone(),
                Err(_) => continue,
            };
            for port in &ports {
                let outgoing = match port.lock() {
                    Ok(guard) => guard.outgoing_edges().clone(),
                    Err(_) => continue,
                };
                for edge in &outgoing {
                    let edge_ptr = Arc::as_ptr(edge) as usize;
                    if seen_edges.insert(edge_ptr) {
                        if let Some(edge_val) = serialize_edge(edge) {
                            edges_json.push(edge_val);
                        }
                    }
                }
            }
        }
    };

    collect_edges_from_nodes(lgraph.layerless_nodes());
    for layer in lgraph.layers() {
        if let Ok(layer_guard) = layer.lock() {
            collect_edges_from_nodes(layer_guard.nodes());
        }
    }

    let snapshot = json!({
        "step": step,
        "processor": processor_name,
        "layers": layers_json,
        "edges": edges_json,
    });

    fs::create_dir_all(output_dir)?;
    let safe_name = processor_name.replace(['/', '\\', ' '], "_");
    let filename = format!("step_{step:02}_{safe_name}.json");
    let filepath = output_dir.join(filename);
    let serialized = serde_json::to_string_pretty(&snapshot)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(filepath, serialized)
}
