use std::env;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, IGraphElementVisitor};
use org_eclipse_elk_core::org::eclipse::elk::core::validation::{
    GraphValidator, LayoutOptionValidator,
};
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

fn build_graph(node_count: usize, edge_count: usize) -> ElkNodeRef {
    let root = ElkGraphUtil::create_graph();
    let mut nodes = Vec::with_capacity(node_count);
    for _ in 0..node_count {
        let node = ElkGraphUtil::create_node(Some(root.clone()));
        {
            let mut node_mut = node.borrow_mut();
            let props = node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            props.set_property(CoreOptions::SPACING_NODE_NODE, Some(10.0));
        }
        nodes.push(node);
    }

    {
        let mut root_mut = root.borrow_mut();
        let props = root_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut();
        props.set_property(CoreOptions::SPACING_NODE_NODE, Some(10.0));
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

fn run_validation(mode: &str, root: &ElkNodeRef, iterations: usize, warmup: usize) -> (u128, f64) {
    for _ in 0..warmup {
        run_once(mode, root);
    }

    let start = Instant::now();
    for _ in 0..iterations {
        run_once(mode, root);
    }
    let elapsed = start.elapsed();
    let nanos = elapsed.as_nanos().max(1);
    (nanos, nanos as f64 / iterations.max(1) as f64)
}

fn run_once(mode: &str, root: &ElkNodeRef) {
    match mode {
        "graph" => {
            let mut validator = GraphValidator::new();
            let mut visitors: Vec<&mut dyn IGraphElementVisitor> = vec![&mut validator];
            ElkUtil::apply_visitors(root, &mut visitors);
            let _ = validator.issues();
        }
        "options" => {
            let mut validator = LayoutOptionValidator::new();
            let mut visitors: Vec<&mut dyn IGraphElementVisitor> = vec![&mut validator];
            ElkUtil::apply_visitors(root, &mut visitors);
            let _ = validator.issues();
        }
        _ => {
            let mut graph_validator = GraphValidator::new();
            let mut option_validator = LayoutOptionValidator::new();
            let mut visitors: Vec<&mut dyn IGraphElementVisitor> =
                vec![&mut graph_validator, &mut option_validator];
            ElkUtil::apply_visitors(root, &mut visitors);
            let _ = graph_validator.issues();
            let _ = option_validator.issues();
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let nodes = parse_arg(&args, "--nodes", 1_000).max(1);
    let edges = parse_arg(&args, "--edges", 2_000);
    let iterations = parse_arg(&args, "--iterations", 5).max(1);
    let warmup = parse_arg(&args, "--warmup", 1);
    let mode = parse_arg_str(&args, "--mode", "both");
    let output_path = parse_arg_str_opt(&args, "--output");

    let root = build_graph(nodes, edges);
    let total_elements = nodes + edges + 1;

    let modes: Vec<&str> = match mode.as_str() {
        "graph" => vec!["graph"],
        "options" => vec!["options"],
        "both" => vec!["both"],
        _ => vec!["both"],
    };

    for mode in modes {
        let (elapsed_nanos, avg_nanos) = run_validation(mode, &root, iterations, warmup);
        let ops = total_elements.saturating_mul(iterations) as f64;
        let ops_per_sec = ops / (elapsed_nanos as f64 / 1_000_000_000.0);
        let avg_ms = avg_nanos / 1_000_000.0;

        println!(
            "Graph validation ({mode}): nodes={nodes} edges={edges} iterations={iterations} warmup={warmup} -> {elapsed_nanos} ns"
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
                "{timestamp},{mode},{nodes},{edges},{iterations},{warmup},{elapsed_nanos},{avg_ms:.6},{ops_per_sec:.2}\n"
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
}
