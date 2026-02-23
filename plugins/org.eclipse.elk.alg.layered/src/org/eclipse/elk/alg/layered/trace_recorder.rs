use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::options::PortSide;
use serde_json::{json, Value};

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LGraph, LNodeRef};

fn serialize_label(label: &Arc<std::sync::Mutex<super::graph::LLabel>>) -> Option<Value> {
    let mut guard = label.try_lock().ok()?;
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
    let mut guard = port.try_lock().ok()?;
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

fn side_rank(side: PortSide) -> i32 {
    match side {
        PortSide::North => 0,
        PortSide::East => 1,
        PortSide::South => 2,
        PortSide::West => 3,
        PortSide::Undefined => 4,
    }
}

fn compare_ports_for_trace(
    a: &Arc<std::sync::Mutex<super::graph::LPort>>,
    b: &Arc<std::sync::Mutex<super::graph::LPort>>,
) -> Ordering {
    let (side_a, pos_a, id_a) = if let Ok(mut guard) = a.try_lock() {
        (
            guard.side(),
            *guard.shape().position_ref(),
            guard.shape().graph_element().id,
        )
    } else {
        return Ordering::Equal;
    };
    let (side_b, pos_b, id_b) = if let Ok(mut guard) = b.try_lock() {
        (
            guard.side(),
            *guard.shape().position_ref(),
            guard.shape().graph_element().id,
        )
    } else {
        return Ordering::Equal;
    };

    let by_side = side_rank(side_a).cmp(&side_rank(side_b));
    if by_side != Ordering::Equal {
        return by_side;
    }

    let by_axis = match side_a {
        PortSide::North => pos_a.x.partial_cmp(&pos_b.x),
        PortSide::East => pos_a.y.partial_cmp(&pos_b.y),
        PortSide::South => pos_b.x.partial_cmp(&pos_a.x),
        PortSide::West => pos_b.y.partial_cmp(&pos_a.y),
        PortSide::Undefined => Some(Ordering::Equal),
    }
    .unwrap_or(Ordering::Equal);
    if by_axis != Ordering::Equal {
        return by_axis;
    }

    id_a.cmp(&id_b)
}

fn serialize_node(node: &LNodeRef, known_layer_index: Option<usize>) -> Option<Value> {
    let mut guard = node.try_lock().ok()?;
    let id = format!("N{}", guard.shape().graph_element().id);
    // Avoid calling designation() as it uses label.lock() which can deadlock
    // when a label mutex is already held in the current thread context.
    let name = guard
        .shape()
        .graph_element()
        .get_designation()
        .unwrap_or_else(|| id.clone());
    let node_type = guard.node_type();
    let pos_x = guard.shape().position_ref().x;
    let pos_y = guard.shape().position_ref().y;
    let size_x = guard.shape().size_ref().x;
    let size_y = guard.shape().size_ref().y;
    // Use the known layer index passed from the outer loop (avoids re-locking
    // the layer which is already held by the caller in serialize_lgraph_snapshot).
    let layer_index: Value = known_layer_index
        .map(|i| json!(i as i64))
        .or_else(|| {
            guard
                .layer()
                .and_then(|l| l.try_lock().ok().and_then(|lg| lg.index()))
                .map(|i| json!(i as i64))
        })
        .unwrap_or(json!(-1));

    let margin = guard.margin();
    let margin_json = json!({
        "top": margin.top, "bottom": margin.bottom,
        "left": margin.left, "right": margin.right,
    });

    let mut port_refs: Vec<_> = guard.ports().clone();
    let label_refs: Vec<_> = guard.labels().clone();
    drop(guard);

    port_refs.sort_by(compare_ports_for_trace);

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
        "layer": layer_index,
        "margin": margin_json,
        "ports": ports,
        "labels": labels,
    }))
}

fn serialize_edge(edge: &LEdgeRef) -> Option<Value> {
    let mut guard = edge.try_lock().ok()?;
    let id = format!("E{}", guard.graph_element().id);

    let source_id = guard
        .source()
        .and_then(|port| {
            port.try_lock().ok().and_then(|p| {
                p.node().and_then(|n| {
                    n.try_lock()
                        .ok()
                        .map(|mut ng| format!("N{}", ng.shape().graph_element().id))
                })
            })
        })
        .unwrap_or_default();

    let target_id = guard
        .target()
        .and_then(|port| {
            port.try_lock().ok().and_then(|p| {
                p.node().and_then(|n| {
                    n.try_lock()
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
        let nodes_json: Vec<Value> = if let Ok(layer_guard) = layer.try_lock() {
            layer_guard
                .nodes()
                .iter()
                .filter_map(|n| serialize_node(n, Some(layer_index)))
                .collect()
        } else {
            Vec::new()
        };
        layers_json.push(json!({
            "index": layer_index,
            "nodes": nodes_json,
        }));
    }

    // Layerless nodes go in a top-level "nodes" array (matching Java's format)
    let layerless_json: Vec<Value> = lgraph
        .layerless_nodes()
        .iter()
        .filter_map(|n| serialize_node(n, None))
        .collect();

    // Collect all edges from all nodes (outgoing only to avoid duplicates)
    let mut seen_edges: HashSet<usize> = HashSet::new();
    let mut edges_json: Vec<Value> = Vec::new();

    let mut collect_edges_from_nodes = |nodes: &[LNodeRef]| {
        for node in nodes {
            let ports = match node.try_lock() {
                Ok(guard) => guard.ports().clone(),
                Err(_) => continue,
            };
            for port in &ports {
                let outgoing = match port.try_lock() {
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
        if let Ok(layer_guard) = layer.try_lock() {
            collect_edges_from_nodes(layer_guard.nodes());
        }
    }

    let graph_size = lgraph.actual_size();
    let pad = lgraph.padding_ref();
    let offset = lgraph.offset_ref();
    let size = lgraph.size_ref();

    let snapshot = json!({
        "step": step,
        "processor": processor_name,
        "nodes": layerless_json,
        "layers": layers_json,
        "edges": edges_json,
        "graphSize": {"width": graph_size.x, "height": graph_size.y},
        "padding": {"top": pad.top, "bottom": pad.bottom, "left": pad.left, "right": pad.right},
        "offset": {"x": offset.x, "y": offset.y},
        "size": {"width": size.x, "height": size.y},
    });

    fs::create_dir_all(output_dir)?;
    // Extract short class name from full module path (e.g.
    // "org_eclipse_elk_alg_layered::...::EdgeAndLayerConstraintEdgeReverser" -> "EdgeAndLayerConstraintEdgeReverser")
    // to match Java's naming convention for batch comparison compatibility.
    let short_name = processor_name
        .rsplit("::")
        .next()
        .unwrap_or(processor_name);
    let safe_name = short_name.replace(['/', '\\', ' '], "_");
    let filename = format!("step_{step:03}_{safe_name}.json");
    let filepath = output_dir.join(filename);
    let serialized = serde_json::to_string_pretty(&snapshot)
        .map_err(std::io::Error::other)?;
    fs::write(filepath, serialized)
}
