use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use serde_json::{Value, json};

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LGraph, LNodeRef};

const TRACE_NAN_SENTINEL: &str = "__ELK_TRACE_NAN__";
const TRACE_POS_INF_SENTINEL: &str = "__ELK_TRACE_POS_INF__";
const TRACE_NEG_INF_SENTINEL: &str = "__ELK_TRACE_NEG_INF__";

// IDs belong to the snapshot, not the processors' reusable scratch ID fields.
struct TraceIds {
    external_nodes: Vec<LNodeRef>,
    nodes: HashMap<usize, String>,
    ports: HashMap<usize, String>,
}

impl TraceIds {
    fn new(graph: &LGraph) -> Self {
        let mut nodes = Vec::new();
        for layer in graph.layers() {
            nodes.extend(layer.lock().nodes().iter().cloned());
        }
        nodes.extend(graph.layerless_nodes().iter().cloned());
        let mut seen: HashSet<usize> = nodes.iter().map(|n| Arc::as_ptr(n) as usize).collect();
        let mut external_nodes = Vec::new();
        for node in &nodes {
            let ports = node.lock().ports().clone();
            for port in ports {
                let edges = port.lock().outgoing_edges().clone();
                for edge in edges {
                    if let Some(target) = edge.lock().target() {
                        if let Some(node) = target.lock().node() {
                            if seen.insert(Arc::as_ptr(&node) as usize) {
                                external_nodes.push(node);
                            }
                        }
                    }
                }
            }
        }
        nodes.extend(external_nodes.iter().cloned());
        let mut ids = Self {
            external_nodes,
            nodes: HashMap::new(),
            ports: HashMap::new(),
        };
        for (index, node) in nodes.iter().enumerate() {
            ids.nodes
                .insert(Arc::as_ptr(node) as usize, format!("N{index}"));
            for port in node.lock().ports() {
                let index = ids.ports.len();
                ids.ports
                    .insert(Arc::as_ptr(port) as usize, format!("P{index}"));
            }
        }
        ids
    }
}

fn number_or_special(value: f64) -> Value {
    if value.is_nan() {
        Value::String(TRACE_NAN_SENTINEL.to_owned())
    } else if value.is_infinite() && value.is_sign_positive() {
        Value::String(TRACE_POS_INF_SENTINEL.to_owned())
    } else if value.is_infinite() && value.is_sign_negative() {
        Value::String(TRACE_NEG_INF_SENTINEL.to_owned())
    } else {
        json!(value)
    }
}

fn serialize_label(label: &Arc<Mutex<super::graph::LLabel>>) -> Option<Value> {
    let mut guard = label.try_lock()?;
    let text = guard.text().to_string();
    let pos_x = guard.shape().position_ref().x;
    let pos_y = guard.shape().position_ref().y;
    let size_x = guard.shape().size_ref().x;
    let size_y = guard.shape().size_ref().y;
    Some(json!({
        "text": text,
        "x": number_or_special(pos_x),
        "y": number_or_special(pos_y),
        "width": number_or_special(size_x),
        "height": number_or_special(size_y),
    }))
}

fn serialize_port(port: &Arc<Mutex<super::graph::LPort>>, ids: &TraceIds) -> Option<Value> {
    let mut guard = port.try_lock()?;
    let id = &ids.ports[&(Arc::as_ptr(port) as usize)];
    let pos_x = guard.shape().position_ref().x;
    let pos_y = guard.shape().position_ref().y;
    let side = guard.side();
    let side_str = format!("{:?}", side).to_uppercase();
    let size = *guard.shape().size_ref();
    let anchor = *guard.anchor_ref();
    let margin = guard.margin().clone();

    let label_refs: Vec<_> = guard.labels().clone();
    drop(guard);

    let labels: Vec<Value> = label_refs.iter().filter_map(serialize_label).collect();

    Some(json!({
        "id": id,
        "side": side_str,
        "width": number_or_special(size.x),
        "height": number_or_special(size.y),
        "anchor": {"x": number_or_special(anchor.x), "y": number_or_special(anchor.y)},
        "margin": {"top": number_or_special(margin.top), "bottom": number_or_special(margin.bottom),
                   "left": number_or_special(margin.left), "right": number_or_special(margin.right)},
        "x": number_or_special(pos_x),
        "y": number_or_special(pos_y),
        "labels": labels,
    }))
}

fn serialize_node(
    node: &LNodeRef,
    known_layer_index: Option<usize>,
    ids: &TraceIds,
) -> Option<Value> {
    let mut guard = node.try_lock()?;
    let id = &ids.nodes[&(Arc::as_ptr(node) as usize)];
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
    // Keep Java semantics for truly layerless nodes (layer = -1).
    // `known_layer_index` is only a fallback for nodes that already have a
    // layer reference but whose layer lock can't be acquired here.
    let layer_index: Value = if let Some(layer_ref) = guard.layer() {
        layer_ref
            .try_lock()
            .and_then(|lg| lg.index())
            .or(known_layer_index)
            .map(|i| json!(i as i64))
            .unwrap_or(json!(-1))
    } else {
        json!(-1)
    };

    let margin = guard.margin();
    let margin_json = json!({
        "top": number_or_special(margin.top), "bottom": number_or_special(margin.bottom),
        "left": number_or_special(margin.left), "right": number_or_special(margin.right),
    });
    let pad = guard.padding();
    let padding_json = json!({
        "top": number_or_special(pad.top), "bottom": number_or_special(pad.bottom),
        "left": number_or_special(pad.left), "right": number_or_special(pad.right),
    });

    let port_refs: Vec<_> = guard.ports().clone();
    let label_refs: Vec<_> = guard.labels().clone();
    drop(guard);

    let ports: Vec<Value> = port_refs
        .iter()
        .filter_map(|port| serialize_port(port, ids))
        .collect();
    let labels: Vec<Value> = label_refs.iter().filter_map(serialize_label).collect();

    Some(json!({
        "id": id,
        "name": name,
        "type": node_type.name(),
        "x": number_or_special(pos_x),
        "y": number_or_special(pos_y),
        "width": number_or_special(size_x),
        "height": number_or_special(size_y),
        "layer": layer_index,
        "margin": margin_json,
        "ports": ports,
        "labels": labels,
        "padding": padding_json,
    }))
}

fn serialize_edge(edge: &LEdgeRef, ids: &TraceIds) -> Option<Value> {
    let mut guard = edge.try_lock()?;
    let id = format!("E{}", guard.graph_element().id);

    let source = guard.source()?;
    let target = guard.target()?;
    let source_node = source.lock().node()?;
    let target_node = target.lock().node()?;
    let source_id = &ids.nodes[&(Arc::as_ptr(&source_node) as usize)];
    let target_id = &ids.nodes[&(Arc::as_ptr(&target_node) as usize)];
    let source_port = &ids.ports[&(Arc::as_ptr(&source) as usize)];
    let target_port = &ids.ports[&(Arc::as_ptr(&target) as usize)];

    let bend_points: Vec<Value> = guard
        .bend_points_ref()
        .iter()
        .map(|v| json!({"x": number_or_special(v.x), "y": number_or_special(v.y)}))
        .collect();

    let label_refs: Vec<_> = guard.labels().clone();
    drop(guard);

    let labels: Vec<Value> = label_refs.iter().filter_map(serialize_label).collect();

    Some(json!({
        "id": id,
        "sourceNode": source_id,
        "sourcePort": source_port,
        "targetNode": target_id,
        "targetPort": target_port,
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
    let ids = TraceIds::new(lgraph);
    // Build layers array
    let mut layers_json: Vec<Value> = Vec::new();
    for (layer_index, layer) in lgraph.layers().iter().enumerate() {
        let nodes_json: Vec<Value> = if let Some(layer_guard) = layer.try_lock() {
            layer_guard
                .nodes()
                .iter()
                .filter_map(|n| serialize_node(n, Some(layer_index), &ids))
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
        .filter_map(|n| serialize_node(n, None, &ids))
        .collect();

    // Collect all edges from all nodes (outgoing only to avoid duplicates)
    let mut seen_edges: HashSet<usize> = HashSet::new();
    let mut edges_json: Vec<Value> = Vec::new();

    let mut collect_edges_from_nodes = |nodes: &[LNodeRef]| {
        for node in nodes {
            let ports = match node.try_lock() {
                Some(guard) => guard.ports().clone(),
                None => continue,
            };
            for port in &ports {
                let outgoing = match port.try_lock() {
                    Some(guard) => guard.outgoing_edges().clone(),
                    None => continue,
                };
                for edge in &outgoing {
                    let edge_ptr = Arc::as_ptr(edge) as usize;
                    if seen_edges.insert(edge_ptr) {
                        if let Some(edge_val) = serialize_edge(edge, &ids) {
                            edges_json.push(edge_val);
                        }
                    }
                }
            }
        }
    };

    collect_edges_from_nodes(lgraph.layerless_nodes());
    for layer in lgraph.layers() {
        if let Some(layer_guard) = layer.try_lock() {
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
        "externalNodes": ids.external_nodes.iter().filter_map(|node| serialize_node(node, None, &ids)).collect::<Vec<_>>(),
        "graphSize": {"width": number_or_special(graph_size.x), "height": number_or_special(graph_size.y)},
        "padding": {"top": number_or_special(pad.top), "bottom": number_or_special(pad.bottom), "left": number_or_special(pad.left), "right": number_or_special(pad.right)},
        "offset": {"x": number_or_special(offset.x), "y": number_or_special(offset.y)},
        "size": {"width": number_or_special(size.x), "height": number_or_special(size.y)},
    });

    fs::create_dir_all(output_dir)?;
    // Extract short class name from full module path (e.g.
    // "org_eclipse_elk_alg_layered::...::EdgeAndLayerConstraintEdgeReverser" -> "EdgeAndLayerConstraintEdgeReverser")
    // to match Java's naming convention for batch comparison compatibility.
    let short_name = processor_name.rsplit("::").next().unwrap_or(processor_name);
    let safe_name = short_name.replace(['/', '\\', ' '], "_");
    let filename = format!("step_{step:03}_{safe_name}.json");
    let filepath = output_dir.join(filename);
    let serialized = serde_json::to_string_pretty(&snapshot)
        .map_err(std::io::Error::other)?
        .replace(&format!("\"{TRACE_NAN_SENTINEL}\""), "NaN")
        .replace(&format!("\"{TRACE_POS_INF_SENTINEL}\""), "Infinity")
        .replace(&format!("\"{TRACE_NEG_INF_SENTINEL}\""), "-Infinity");
    fs::write(filepath, serialized)
}
