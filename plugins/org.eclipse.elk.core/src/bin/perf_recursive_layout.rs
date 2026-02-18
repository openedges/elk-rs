use std::env;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    BoxLayouterOptions, CoreOptions, FixedLayouterOptions, RandomLayouterOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::recursive_graph_layout_engine::RecursiveGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};

fn parse_arg(args: &[String], flag: &str, default: usize) -> usize {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

fn parse_arg_str(args: &[String], flag: &str, default: &str) -> String {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .map(|value| value.to_string())
        .unwrap_or_else(|| default.to_string())
}

fn parse_arg_str_opt(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .map(|value| value.to_string())
}

fn parse_arg_bool(args: &[String], flag: &str, default: bool) -> bool {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .map(|value| match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "y" => true,
            "0" | "false" | "no" | "n" => false,
            _ => default,
        })
        .unwrap_or(default)
}

fn algorithm_id(name: &str) -> &str {
    match name {
        "fixed" => FixedLayouterOptions::ALGORITHM_ID,
        "random" => RandomLayouterOptions::ALGORITHM_ID,
        "box" => BoxLayouterOptions::ALGORITHM_ID,
        "layered" => "org.eclipse.elk.layered",
        _ => name,
    }
}

fn build_graph(
    node_count: usize,
    edge_count: usize,
    algorithm: &str,
    validate_graph: bool,
    validate_options: bool,
) -> ElkNodeRef {
    let root = ElkGraphUtil::create_graph();
    {
        let mut root_mut = root.borrow_mut();
        let props = root_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut();
        props.set_property(
            CoreOptions::ALGORITHM,
            Some(algorithm_id(algorithm).to_string()),
        );
        props.set_property(CoreOptions::VALIDATE_GRAPH, Some(validate_graph));
        props.set_property(CoreOptions::VALIDATE_OPTIONS, Some(validate_options));
    }

    let mut nodes = Vec::with_capacity(node_count);
    for _ in 0..node_count {
        let node = ElkGraphUtil::create_node(Some(root.clone()));
        {
            let mut node_mut = node.borrow_mut();
            node_mut.connectable().shape().set_dimensions(30.0, 20.0);
        }
        nodes.push(node);
    }

    let node_len = nodes.len().max(1);
    for index in 0..edge_count {
        let source = nodes[index % node_len].clone();
        let target = nodes[(index + 1) % node_len].clone();
        let _edge = ElkGraphUtil::create_simple_edge(
            ElkConnectableShapeRef::Node(source),
            ElkConnectableShapeRef::Node(target),
        );
    }

    root
}

fn run_layout(root: &ElkNodeRef, iterations: usize, warmup: usize) -> (u128, f64) {
    let mut engine = RecursiveGraphLayoutEngine::new();
    for _ in 0..warmup {
        let mut monitor = NullElkProgressMonitor;
        engine.layout(root, &mut monitor);
    }

    let start = Instant::now();
    for _ in 0..iterations {
        let mut monitor = NullElkProgressMonitor;
        engine.layout(root, &mut monitor);
    }
    let elapsed = start.elapsed();
    let nanos = elapsed.as_nanos().max(1);
    (nanos, nanos as f64 / iterations.max(1) as f64)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let nodes = parse_arg(&args, "--nodes", 500).max(1);
    let edges = parse_arg(&args, "--edges", 1_000);
    let iterations = parse_arg(&args, "--iterations", 5).max(1);
    let warmup = parse_arg(&args, "--warmup", 1);
    let algorithm = parse_arg_str(&args, "--algorithm", "fixed");
    let validate_graph = parse_arg_bool(&args, "--validate-graph", false);
    let validate_options = parse_arg_bool(&args, "--validate-options", false);
    let output_path = parse_arg_str_opt(&args, "--output");

    let root = build_graph(nodes, edges, &algorithm, validate_graph, validate_options);
    let total_elements = nodes + edges + 1;

    let (elapsed_nanos, avg_nanos) = run_layout(&root, iterations, warmup);
    let ops = total_elements.saturating_mul(iterations) as f64;
    let ops_per_sec = ops / (elapsed_nanos as f64 / 1_000_000_000.0);
    let avg_ms = avg_nanos / 1_000_000.0;

    println!(
        "Recursive layout: algorithm={} nodes={} edges={} iterations={} warmup={} -> {} ns",
        algorithm, nodes, edges, iterations, warmup, elapsed_nanos
    );
    println!(
        "Average per iteration: {:.2} ms, throughput: {:.2} elems/s",
        avg_ms, ops_per_sec
    );

    if let Some(path) = output_path.as_ref() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let line = format!(
            "{timestamp},{},{nodes},{edges},{iterations},{warmup},{elapsed_nanos},{avg_ms:.6},{ops_per_sec:.2},{},{}\n",
            algorithm, validate_graph, validate_options
        );
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            use std::io::Write;
            let _ = file.write_all(line.as_bytes());
        }
    }
}
