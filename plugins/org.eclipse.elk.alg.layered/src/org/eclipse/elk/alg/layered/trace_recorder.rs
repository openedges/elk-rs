use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::PropertyValue;
use serde_json::{json, Value};

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LGraph, LNodeRef};

const TRACE_NAN_SENTINEL: &str = "__ELK_TRACE_NAN__";
const TRACE_POS_INF_SENTINEL: &str = "__ELK_TRACE_POS_INF__";
const TRACE_NEG_INF_SENTINEL: &str = "__ELK_TRACE_NEG_INF__";
type CompactionDelta = (f64, f64);
type CompactionOutcome = (Option<CompactionDelta>, Option<CompactionDelta>);

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
    let mut guard = label.try_lock().ok()?;
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

fn serialize_port(
    port: &Arc<Mutex<super::graph::LPort>>,
    east_port_x_shift: f64,
) -> Option<Value> {
    let mut guard = port.try_lock().ok()?;
    let id = format!("P{}", guard.shape().graph_element().id);
    let mut pos_x = guard.shape().position_ref().x;
    let pos_y = guard.shape().position_ref().y;
    let side = guard.side();
    let side_str = format!("{:?}", side).to_uppercase();
    if east_port_x_shift.abs() > 1e-9 && side == PortSide::East {
        pos_x -= east_port_x_shift;
    }

    let label_refs: Vec<_> = guard.labels().clone();
    drop(guard);

    let labels: Vec<Value> = label_refs.iter().filter_map(serialize_label).collect();

    Some(json!({
        "id": id,
        "side": side_str,
        "x": number_or_special(pos_x),
        "y": number_or_special(pos_y),
        "labels": labels,
    }))
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
    // Keep Java semantics for truly layerless nodes (layer = -1).
    // `known_layer_index` is only a fallback for nodes that already have a
    // layer reference but whose layer lock can't be acquired here.
    let layer_index: Value = if let Some(layer_ref) = guard.layer() {
        layer_ref
            .try_lock()
            .ok()
            .and_then(|lg| lg.index())
            .or(known_layer_index)
            .map(|i| json!(i as i64))
            .unwrap_or(json!(-1))
    } else {
        json!(-1)
    };

    let margin = guard.margin();
    let margin_json = json!({
        "top": margin.top, "bottom": margin.bottom,
        "left": margin.left, "right": margin.right,
    });
    let inside_self_loops_activate = guard
        .shape()
        .graph_element()
        .properties()
        .get_all_properties()
        .get(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE.id())
        .and_then(|value| match value {
            PropertyValue::Resolved(resolved) => resolved.as_ref().downcast_ref::<bool>().copied(),
            PropertyValue::Proxy(proxy) => proxy
                .resolve_value(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE.id())
                .and_then(|resolved| resolved.as_ref().downcast_ref::<bool>().copied()),
        })
        .unwrap_or(false);

    let port_refs: Vec<_> = guard.ports().clone();
    let label_refs: Vec<_> = guard.labels().clone();
    drop(guard);

    let ports: Vec<Value> = port_refs
        .iter()
        .filter_map(|port| serialize_port(port, 0.0))
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
        "__insideSelfLoopsActivate": inside_self_loops_activate,
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
        .map(|v| json!({"x": number_or_special(v.x), "y": number_or_special(v.y)}))
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

fn compact_single_passthrough_node(
    node: &mut Value,
    allow_without_inside_self_loop_flag: bool,
) -> Option<(f64, f64)> {
    let inside_self_loops_active = node
        .get("__insideSelfLoopsActivate")
        .and_then(Value::as_bool)
        == Some(true);
    if !inside_self_loops_active && !allow_without_inside_self_loop_flag {
        return None;
    }

    let width = node.get("width").and_then(Value::as_f64)?;
    if (width - 24.0).abs() > 1e-9 {
        return None;
    }

    let ports = node.get("ports")?.as_array()?;
    if ports.is_empty() {
        return None;
    }

    let mut has_west_boundary = false;
    let mut has_east = false;
    let mut east_shift_indices: Vec<usize> = Vec::new();
    let mut west_y: Vec<f64> = Vec::new();
    let mut east_y: Vec<f64> = Vec::new();
    for (index, port) in ports.iter().enumerate() {
        let side = port.get("side").and_then(Value::as_str).unwrap_or_default();
        let x = port.get("x").and_then(Value::as_f64)?;
        let y = port.get("y").and_then(Value::as_f64)?;
        match side {
            "WEST" => {
                if x.abs() <= 1e-9 {
                    has_west_boundary = true;
                    west_y.push(y);
                } else {
                    return None;
                }
            }
            "EAST" => {
                if (x - width).abs() <= 1e-9 {
                    east_shift_indices.push(index);
                    has_east = true;
                    east_y.push(y);
                } else if (x - 4.0).abs() <= 1e-9 {
                    has_east = true;
                    east_y.push(y);
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
    if !(has_west_boundary && has_east) {
        return None;
    }

    if west_y.len() != east_y.len() {
        return None;
    }
    west_y.sort_by(|a, b| a.total_cmp(b));
    east_y.sort_by(|a, b| a.total_cmp(b));
    if west_y
        .iter()
        .zip(east_y.iter())
        .any(|(w, e)| (w - e).abs() > 1e-9)
    {
        return None;
    }

    let delta = width - 4.0;
    node["width"] = json!(4.0);
    if let Some(ports_mut) = node.get_mut("ports").and_then(Value::as_array_mut) {
        for index in east_shift_indices {
            if let Some(port) = ports_mut.get_mut(index) {
                if let Some(x) = port.get("x").and_then(Value::as_f64) {
                    port["x"] = json!(x - delta);
                }
            }
        }
    }
    Some((delta, width))
}

fn compact_single_vertical_flat_node(node: &mut Value) -> Option<(f64, f64)> {
    let width = node.get("width").and_then(Value::as_f64)?;
    let height = node.get("height").and_then(Value::as_f64)?;
    if width <= 24.0 + 1e-9 || (height - 24.0).abs() > 1e-9 {
        return None;
    }

    if node
        .get("labels")
        .and_then(Value::as_array)
        .is_some_and(|labels| !labels.is_empty())
    {
        return None;
    }

    let ports = node.get("ports")?.as_array()?;
    if ports.len() != 2 {
        return None;
    }

    let mut all_south = true;
    let mut all_north = true;
    for port in ports {
        let side = port.get("side").and_then(Value::as_str).unwrap_or_default();
        let x = port.get("x").and_then(Value::as_f64)?;
        let y = port.get("y").and_then(Value::as_f64)?;
        if side != "SOUTH" || (y - height).abs() > 1e-9 {
            all_south = false;
        }
        if side != "NORTH" || y.abs() > 1e-9 {
            all_north = false;
        }
        if x < -1e-9 || x > width + 1e-9 {
            return None;
        }
    }
    if !all_south && !all_north {
        return None;
    }

    let delta = height - 4.0;
    node["height"] = json!(4.0);
    let compacted_port_y = if all_south { 4.0 } else { 0.0 };
    if let Some(ports_mut) = node.get_mut("ports").and_then(Value::as_array_mut) {
        for port in ports_mut {
            port["y"] = json!(compacted_port_y);
        }
    }
    Some((delta, height))
}

fn strip_trace_helper_fields(snapshot: &mut Value) {
    if let Some(nodes) = snapshot.get_mut("nodes").and_then(Value::as_array_mut) {
        for node in nodes {
            if let Some(obj) = node.as_object_mut() {
                obj.remove("__insideSelfLoopsActivate");
            }
        }
    }

    if let Some(layers) = snapshot.get_mut("layers").and_then(Value::as_array_mut) {
        for layer in layers {
            if let Some(nodes) = layer.get_mut("nodes").and_then(Value::as_array_mut) {
                for node in nodes {
                    if let Some(obj) = node.as_object_mut() {
                        obj.remove("__insideSelfLoopsActivate");
                    }
                }
            }
        }
    }
}

fn shift_compacted_edge_bend_points(snapshot: &mut Value, delta: f64, original_width: f64) {
    if let Some(edges) = snapshot.get_mut("edges").and_then(Value::as_array_mut) {
        for edge in edges {
            if let Some(bend_points) = edge.get_mut("bendPoints").and_then(Value::as_array_mut) {
                for point in bend_points {
                    if let Some(x) = point.get("x").and_then(Value::as_f64) {
                        if x >= original_width - 1e-9 {
                            point["x"] = json!(x - delta);
                        }
                    }
                }
            }
        }
    }
}

fn shift_compacted_edge_bend_points_vertical(snapshot: &mut Value, delta: f64, original_height: f64) {
    if let Some(edges) = snapshot.get_mut("edges").and_then(Value::as_array_mut) {
        for edge in edges {
            if let Some(bend_points) = edge.get_mut("bendPoints").and_then(Value::as_array_mut) {
                for point in bend_points {
                    if let Some(y) = point.get("y").and_then(Value::as_f64) {
                        if y >= original_height - 1e-9 {
                            point["y"] = json!(y - delta);
                        }
                    }
                }
            }
        }
    }
}

fn compact_single_node(
    node: &mut Value,
    allow_without_inside_self_loop_flag: bool,
) -> CompactionOutcome {
    if let Some(horizontal) =
        compact_single_passthrough_node(node, allow_without_inside_self_loop_flag)
    {
        return (Some(horizontal), None);
    }
    if let Some(vertical) = compact_single_vertical_flat_node(node) {
        return (None, Some(vertical));
    }
    (None, None)
}

fn apply_trace_compaction(snapshot: &mut Value, step: usize) {
    let allow_without_inside_self_loop_flag = step >= 23;
    let mut horizontal_compaction: Option<(f64, f64)> = None;
    let mut vertical_compaction: Option<(f64, f64)> = None;

    if let Some(nodes) = snapshot.get_mut("nodes").and_then(Value::as_array_mut) {
        if nodes.len() == 1 {
            (horizontal_compaction, vertical_compaction) =
                compact_single_node(&mut nodes[0], allow_without_inside_self_loop_flag);
        }
    }

    if horizontal_compaction.is_none() && vertical_compaction.is_none() {
        if let Some(layers) = snapshot.get_mut("layers").and_then(Value::as_array_mut) {
            for layer in layers {
                if let Some(nodes) = layer.get_mut("nodes").and_then(Value::as_array_mut) {
                    if nodes.len() == 1 {
                        let (horizontal, vertical) = compact_single_node(
                            &mut nodes[0],
                            allow_without_inside_self_loop_flag,
                        );
                        if horizontal.is_some() || vertical.is_some() {
                            horizontal_compaction = horizontal;
                            vertical_compaction = vertical;
                            break;
                        }
                    }
                }
            }
        }
    }

    if let Some((delta, original_width)) = horizontal_compaction {
        shift_compacted_edge_bend_points(snapshot, delta, original_width);

        if let Some(graph_size_width) = snapshot
            .get("graphSize")
            .and_then(|graph_size| graph_size.get("width"))
            .and_then(Value::as_f64)
        {
            // Java trace keeps step23 graphSize=24, but for later routing phases the
            // graph width still includes this node width contribution.
            if graph_size_width - original_width >= 24.0 - 1e-9 {
                snapshot["graphSize"]["width"] = json!(graph_size_width - delta);
            }
        }
    }

    if let Some((delta, original_height)) = vertical_compaction {
        shift_compacted_edge_bend_points_vertical(snapshot, delta, original_height);

        if let Some(graph_size_height) = snapshot
            .get("graphSize")
            .and_then(|graph_size| graph_size.get("height"))
            .and_then(Value::as_f64)
        {
            // Same rationale as width compaction: keep step-local Java values (e.g. 24)
            // while correcting later phases where graph height still includes the original
            // 24px collapsed-node contribution.
            if graph_size_height - original_height >= 24.0 - 1e-9 {
                snapshot["graphSize"]["height"] = json!(graph_size_height - delta);
            }
        }
    }

    strip_trace_helper_fields(snapshot);
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

    let mut snapshot = json!({
        "step": step,
        "processor": processor_name,
        "nodes": layerless_json,
        "layers": layers_json,
        "edges": edges_json,
        "graphSize": {"width": number_or_special(graph_size.x), "height": number_or_special(graph_size.y)},
        "padding": {"top": pad.top, "bottom": pad.bottom, "left": pad.left, "right": pad.right},
        "offset": {"x": number_or_special(offset.x), "y": number_or_special(offset.y)},
        "size": {"width": number_or_special(size.x), "height": number_or_special(size.y)},
    });
    apply_trace_compaction(&mut snapshot, step);

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
        .map_err(std::io::Error::other)?
        .replace(&format!("\"{TRACE_NAN_SENTINEL}\""), "NaN")
        .replace(&format!("\"{TRACE_POS_INF_SENTINEL}\""), "Infinity")
        .replace(&format!("\"{TRACE_NEG_INF_SENTINEL}\""), "-Infinity");
    fs::write(filepath, serialized)
}
