//! Unified performance benchmark for elk-rs.
//!
//! Supports two engines:
//!   - `rust_native`: Direct ElkNode construction + RecursiveGraphLayoutEngine
//!   - `rust_api`: JSON-based layout_api::layout_json() (same path as NAPI/WASM)
//!
//! Usage:
//!   perf_benchmark --engine rust_native --mode synthetic [options]
//!   perf_benchmark --engine rust_api    --mode synthetic [options]
//!   perf_benchmark --engine rust_api    --mode models   [options]
//!
//! Options:
//!   --engine ENGINE        rust_native or rust_api (default: rust_native)
//!   --mode MODE            synthetic or models (default: synthetic)
//!   --scenarios LIST       Comma-separated scenario names (synthetic mode)
//!   --iterations N         Iterations per scenario (default: 20)
//!   --warmup N             Warmup iterations (default: 3)
//!   --output PATH          CSV output path (default: stdout)
//!   --input-dir PATH       Model JSON directory (models mode)
//!   --manifest PATH        Java manifest TSV (models mode)
//!   --limit N              Max models (default: 50)

use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    CrossingMinimizationStrategy, GreedySwitchType, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::recursive_graph_layout_engine::RecursiveGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api;

const DEFAULT_SCENARIOS: &str =
    "layered_small,layered_medium,layered_large,layered_xlarge,\
     force_medium,force_large,force_xlarge,\
     stress_medium,stress_large,stress_xlarge,\
     mrtree_medium,mrtree_large,mrtree_xlarge,\
     radial_medium,radial_large,radial_xlarge,\
     rectpacking_medium,rectpacking_large,rectpacking_xlarge,\
     routing_polyline,routing_orthogonal,routing_splines,\
     crossmin_layer_sweep,crossmin_none,hierarchy_flat,hierarchy_nested";

// ---------------------------------------------------------------------------
// Arg parsing
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// LCG
// ---------------------------------------------------------------------------

fn lcg(state: u32) -> u32 {
    (state.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7fffffff
}

// ---------------------------------------------------------------------------
// CSV output
// ---------------------------------------------------------------------------

fn format_csv_line(
    engine: &str,
    scenario: &str,
    iterations: usize,
    warmup: usize,
    elapsed_nanos: u128,
    avg_ms: f64,
    ops_per_sec: f64,
) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    format!(
        "{timestamp},{engine},{scenario},{iterations},{warmup},{elapsed_nanos},{avg_ms:.6},{ops_per_sec:.2}"
    )
}

fn write_results(output_path: Option<&str>, lines: &[String]) {
    let header = "timestamp,engine,scenario,iterations,warmup,elapsed_nanos,avg_ms,ops_per_sec";
    let csv = format!("{}\n{}\n", header, lines.join("\n"));

    match output_path {
        Some(path) => {
            if let Some(parent) = Path::new(path).parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Err(e) = fs::write(path, &csv) {
                eprintln!("Failed to write CSV to {}: {}", path, e);
            } else {
                eprintln!("Results written to {}", path);
            }
        }
        None => {
            print!("{}", csv);
        }
    }
}

// ===========================================================================
// Native engine: direct ElkNode construction
// ===========================================================================

fn set_node_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    node.borrow_mut()
        .connectable()
        .shape()
        .set_dimensions(width, height);
}

fn set_node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
    value: T,
) {
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn build_dag(
    nodes: usize,
    edges: usize,
    seed: u32,
    algorithm: &str,
    direction: Direction,
    edge_routing: EdgeRouting,
) -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(&graph, CoreOptions::ALGORITHM, algorithm.to_string());
    set_node_property(&graph, CoreOptions::DIRECTION, direction);
    set_node_property(&graph, CoreOptions::EDGE_ROUTING, edge_routing);

    let node_refs: Vec<ElkNodeRef> = (0..nodes)
        .map(|_| {
            let node = ElkGraphUtil::create_node(Some(graph.clone()));
            set_node_dimensions(&node, 40.0, 30.0);
            node
        })
        .collect();

    let layer_of: Vec<usize> = (0..nodes).map(|i| i * 5 / nodes.max(1)).collect();

    let mut state = seed;
    let mut generated = 0usize;
    let mut attempts = 0usize;
    let max_attempts = edges * 100;

    while generated < edges && attempts < max_attempts {
        state = lcg(state);
        let src = (state as usize) % nodes;
        state = lcg(state);
        let tgt = (state as usize) % nodes;
        attempts += 1;

        if layer_of[src] < layer_of[tgt] {
            let _edge = ElkGraphUtil::create_simple_edge(
                ElkConnectableShapeRef::Node(node_refs[src].clone()),
                ElkConnectableShapeRef::Node(node_refs[tgt].clone()),
            );
            generated += 1;
        }
    }

    graph
}

fn build_tree(nodes: usize, seed: u32, algorithm: &str) -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(&graph, CoreOptions::ALGORITHM, algorithm.to_string());

    let node_refs: Vec<ElkNodeRef> = (0..nodes)
        .map(|_| {
            let node = ElkGraphUtil::create_node(Some(graph.clone()));
            set_node_dimensions(&node, 40.0, 30.0);
            node
        })
        .collect();

    let mut state = seed;
    for i in 1..nodes {
        state = lcg(state);
        let parent = (state as usize) % i;
        let _edge = ElkGraphUtil::create_simple_edge(
            ElkConnectableShapeRef::Node(node_refs[parent].clone()),
            ElkConnectableShapeRef::Node(node_refs[i].clone()),
        );
    }

    graph
}

fn build_general_graph(nodes: usize, edges: usize, seed: u32, algorithm: &str) -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(&graph, CoreOptions::ALGORITHM, algorithm.to_string());

    let node_refs: Vec<ElkNodeRef> = (0..nodes)
        .map(|_| {
            let node = ElkGraphUtil::create_node(Some(graph.clone()));
            set_node_dimensions(&node, 40.0, 30.0);
            node
        })
        .collect();

    let mut state = seed;
    for _ in 0..edges {
        state = lcg(state);
        let src = (state as usize) % nodes;
        state = lcg(state);
        let tgt = (state as usize) % nodes;
        let _edge = ElkGraphUtil::create_simple_edge(
            ElkConnectableShapeRef::Node(node_refs[src].clone()),
            ElkConnectableShapeRef::Node(node_refs[tgt].clone()),
        );
    }

    graph
}

fn build_rectpacking(nodes: usize, seed: u32) -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(
        &graph,
        CoreOptions::ALGORITHM,
        "org.eclipse.elk.rectpacking".to_string(),
    );

    let mut state = seed;
    for _ in 0..nodes {
        let node = ElkGraphUtil::create_node(Some(graph.clone()));
        state = lcg(state);
        let w = 20.0 + (state as usize % 61) as f64;
        state = lcg(state);
        let h = 20.0 + (state as usize % 61) as f64;
        set_node_dimensions(&node, w, h);
    }

    graph
}

fn build_hierarchy_nested(seed: u32) -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(
        &graph,
        CoreOptions::ALGORITHM,
        "org.eclipse.elk.layered".to_string(),
    );
    set_node_property(&graph, CoreOptions::DIRECTION, Direction::Right);
    set_node_property(&graph, CoreOptions::EDGE_ROUTING, EdgeRouting::Orthogonal);
    set_node_property(
        &graph,
        CoreOptions::HIERARCHY_HANDLING,
        HierarchyHandling::IncludeChildren,
    );

    let compounds = 3usize;
    let leaves_per = 9usize;

    let mut state = seed;
    let mut compound_refs = Vec::new();
    let mut leaf_refs: Vec<Vec<ElkNodeRef>> = Vec::new();

    for _ in 0..compounds {
        let compound = ElkGraphUtil::create_node(Some(graph.clone()));
        set_node_dimensions(&compound, 0.0, 0.0);

        let mut leaves = Vec::new();
        for _ in 0..leaves_per {
            let leaf = ElkGraphUtil::create_node(Some(compound.clone()));
            set_node_dimensions(&leaf, 40.0, 30.0);
            leaves.push(leaf);
        }

        for i in 1..leaves_per {
            state = lcg(state);
            let parent = (state as usize) % i;
            let _edge = ElkGraphUtil::create_simple_edge(
                ElkConnectableShapeRef::Node(leaves[parent].clone()),
                ElkConnectableShapeRef::Node(leaves[i].clone()),
            );
        }

        compound_refs.push(compound);
        leaf_refs.push(leaves);
    }

    for c in 0..compounds - 1 {
        let _edge = ElkGraphUtil::create_simple_edge(
            ElkConnectableShapeRef::Node(leaf_refs[c][0].clone()),
            ElkConnectableShapeRef::Node(leaf_refs[c + 1][0].clone()),
        );
    }

    graph
}

fn build_native_scenario(name: &str) -> Option<ElkNodeRef> {
    match name {
        "layered_small" => Some(build_dag(
            10, 15, 42, "org.eclipse.elk.layered", Direction::Right, EdgeRouting::Orthogonal,
        )),
        "layered_medium" => Some(build_dag(
            50, 100, 42, "org.eclipse.elk.layered", Direction::Right, EdgeRouting::Orthogonal,
        )),
        "layered_large" => Some(build_dag(
            200, 500, 42, "org.eclipse.elk.layered", Direction::Right, EdgeRouting::Orthogonal,
        )),
        "layered_xlarge" => Some(build_dag(
            1000, 3000, 42, "org.eclipse.elk.layered", Direction::Right, EdgeRouting::Orthogonal,
        )),
        "force_medium" => Some(build_general_graph(50, 80, 100, "org.eclipse.elk.force")),
        "force_large" => Some(build_general_graph(200, 400, 100, "org.eclipse.elk.force")),
        "force_xlarge" => Some(build_general_graph(500, 1200, 100, "org.eclipse.elk.force")),
        "stress_medium" => Some(build_general_graph(50, 80, 100, "org.eclipse.elk.stress")),
        "stress_large" => Some(build_general_graph(200, 400, 100, "org.eclipse.elk.stress")),
        "stress_xlarge" => Some(build_general_graph(500, 1200, 100, "org.eclipse.elk.stress")),
        "mrtree_medium" => Some(build_tree(50, 200, "org.eclipse.elk.mrtree")),
        "mrtree_large" => Some(build_tree(200, 200, "org.eclipse.elk.mrtree")),
        "mrtree_xlarge" => Some(build_tree(1000, 200, "org.eclipse.elk.mrtree")),
        "radial_medium" => Some(build_tree(50, 200, "org.eclipse.elk.radial")),
        "radial_large" => Some(build_tree(200, 200, "org.eclipse.elk.radial")),
        "radial_xlarge" => Some(build_tree(1000, 200, "org.eclipse.elk.radial")),
        "rectpacking_medium" => Some(build_rectpacking(50, 100)),
        "rectpacking_large" => Some(build_rectpacking(200, 100)),
        "rectpacking_xlarge" => Some(build_rectpacking(1000, 100)),
        "routing_polyline" => Some(build_dag(
            50, 100, 42, "org.eclipse.elk.layered", Direction::Right, EdgeRouting::Polyline,
        )),
        "routing_orthogonal" => Some(build_dag(
            50, 100, 42, "org.eclipse.elk.layered", Direction::Right, EdgeRouting::Orthogonal,
        )),
        "routing_splines" => Some(build_dag(
            50, 100, 42, "org.eclipse.elk.layered", Direction::Right, EdgeRouting::Splines,
        )),
        "crossmin_layer_sweep" => {
            let graph = build_dag(
                50, 100, 42, "org.eclipse.elk.layered", Direction::Right, EdgeRouting::Orthogonal,
            );
            set_node_property(
                &graph,
                LayeredOptions::CROSSING_MINIMIZATION_STRATEGY,
                CrossingMinimizationStrategy::LayerSweep,
            );
            set_node_property(
                &graph,
                LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE,
                GreedySwitchType::TwoSided,
            );
            Some(graph)
        }
        "crossmin_none" => {
            let graph = build_dag(
                50, 100, 42, "org.eclipse.elk.layered", Direction::Right, EdgeRouting::Orthogonal,
            );
            set_node_property(
                &graph,
                LayeredOptions::CROSSING_MINIMIZATION_STRATEGY,
                CrossingMinimizationStrategy::None,
            );
            set_node_property(
                &graph,
                LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE,
                GreedySwitchType::Off,
            );
            Some(graph)
        }
        "hierarchy_flat" => Some(build_dag(
            30, 50, 300, "org.eclipse.elk.layered", Direction::Right, EdgeRouting::Orthogonal,
        )),
        "hierarchy_nested" => Some(build_hierarchy_nested(300)),
        _ => None,
    }
}

fn run_native_scenario(name: &str, iterations: usize, warmup: usize) -> Option<(u128, f64, f64)> {
    let mut engine = RecursiveGraphLayoutEngine::new();

    for _ in 0..warmup {
        let graph = build_native_scenario(name)?;
        engine.layout(&graph, &mut BasicProgressMonitor::new());
    }

    let start = Instant::now();
    for _ in 0..iterations {
        let graph = build_native_scenario(name)?;
        engine.layout(&graph, &mut BasicProgressMonitor::new());
    }
    let elapsed = start.elapsed();
    let nanos = elapsed.as_nanos().max(1);
    let avg_ms = nanos as f64 / iterations.max(1) as f64 / 1_000_000.0;
    let ops_per_sec = iterations.max(1) as f64 / (nanos as f64 / 1_000_000_000.0);

    Some((nanos, avg_ms, ops_per_sec))
}

// ===========================================================================
// API engine: JSON-based layout_api::layout_json()
// ===========================================================================

fn generate_dag_json(nodes: usize, edges: usize, seed: u32, layout_options: &str) -> String {
    let layer_of: Vec<usize> = (0..nodes).map(|i| i * 5 / nodes.max(1)).collect();

    let children: String = (0..nodes)
        .map(|i| format!(r#"    {{"id":"n{i}","width":40,"height":30}}"#))
        .collect::<Vec<_>>()
        .join(",\n");

    let mut edge_lines = Vec::new();
    let mut state = seed;
    let mut attempts = 0usize;
    while edge_lines.len() < edges && attempts < edges * 20 {
        attempts += 1;
        state = lcg(state);
        let src = (state as usize) % nodes;
        state = lcg(state);
        let tgt = (state as usize) % nodes;
        if layer_of[src] < layer_of[tgt] {
            let eid = edge_lines.len();
            edge_lines.push(format!(
                r#"    {{"id":"e{eid}","sources":["n{src}"],"targets":["n{tgt}"]}}"#
            ));
        }
    }
    let edges_json = edge_lines.join(",\n");

    let extra = if layout_options.is_empty() {
        String::new()
    } else {
        format!(",\n    {}", layout_options)
    };

    format!(
        r#"{{
  "id":"root",
  "layoutOptions":{{
    "org.eclipse.elk.algorithm":"org.eclipse.elk.layered",
    "org.eclipse.elk.direction":"RIGHT",
    "org.eclipse.elk.edgeRouting":"ORTHOGONAL"{extra}
  }},
  "children":[
{children}
  ],
  "edges":[
{edges_json}
  ]
}}"#
    )
}

fn generate_tree_json(nodes: usize, seed: u32, algorithm: &str) -> String {
    let children: String = (0..nodes)
        .map(|i| format!(r#"    {{"id":"n{i}","width":40,"height":30}}"#))
        .collect::<Vec<_>>()
        .join(",\n");

    let mut edge_lines = Vec::new();
    let mut state = seed;
    for i in 1..nodes {
        state = lcg(state);
        let parent = (state as usize) % i;
        edge_lines.push(format!(
            r#"    {{"id":"e{i}","sources":["n{parent}"],"targets":["n{i}"]}}"#
        ));
    }
    let edges_json = edge_lines.join(",\n");

    format!(
        r#"{{
  "id":"root",
  "layoutOptions":{{
    "org.eclipse.elk.algorithm":"{algorithm}"
  }},
  "children":[
{children}
  ],
  "edges":[
{edges_json}
  ]
}}"#
    )
}

fn generate_general_graph_json(nodes: usize, edges: usize, seed: u32, algorithm: &str) -> String {
    let children: String = (0..nodes)
        .map(|i| format!(r#"    {{"id":"n{i}","width":40,"height":30}}"#))
        .collect::<Vec<_>>()
        .join(",\n");

    let mut edge_lines = Vec::new();
    let mut state = seed;
    for eid in 0..edges {
        state = lcg(state);
        let src = (state as usize) % nodes;
        state = lcg(state);
        let tgt = (state as usize) % nodes;
        edge_lines.push(format!(
            r#"    {{"id":"e{eid}","sources":["n{src}"],"targets":["n{tgt}"]}}"#
        ));
    }
    let edges_json = edge_lines.join(",\n");

    format!(
        r#"{{
  "id":"root",
  "layoutOptions":{{
    "org.eclipse.elk.algorithm":"{algorithm}"
  }},
  "children":[
{children}
  ],
  "edges":[
{edges_json}
  ]
}}"#
    )
}

fn generate_rectpacking_json(nodes: usize, seed: u32) -> String {
    let mut state = seed;
    let children: String = (0..nodes)
        .map(|i| {
            state = lcg(state);
            let w = 20 + (state as usize) % 61;
            state = lcg(state);
            let h = 20 + (state as usize) % 61;
            format!(r#"    {{"id":"n{i}","width":{w},"height":{h}}}"#)
        })
        .collect::<Vec<_>>()
        .join(",\n");

    format!(
        r#"{{
  "id":"root",
  "layoutOptions":{{
    "org.eclipse.elk.algorithm":"org.eclipse.elk.rectpacking"
  }},
  "children":[
{children}
  ],
  "edges":[]
}}"#
    )
}

fn generate_hierarchy_nested_json(seed: u32) -> String {
    let mut state = seed;
    let compounds = 3usize;
    let leaves_per = 9usize;

    let mut compound_blocks = Vec::new();
    let mut cross_edges = Vec::new();
    let mut cross_eid = 0usize;

    for c in 0..compounds {
        let mut leaf_nodes = Vec::new();
        for l in 0..leaves_per {
            leaf_nodes.push(format!(r#"      {{"id":"c{c}_l{l}","width":40,"height":30}}"#));
        }

        let mut internal_edges = Vec::new();
        for l in 1..leaves_per {
            state = lcg(state);
            let src = (state as usize) % l;
            internal_edges.push(format!(
                r#"      {{"id":"c{c}_ie{l}","sources":["c{c}_l{src}"],"targets":["c{c}_l{l}"]}}"#
            ));
        }

        compound_blocks.push(format!(
            r#"    {{
      "id":"compound{c}","width":0,"height":0,
      "children":[
{}
      ],
      "edges":[
{}
      ]
    }}"#,
            leaf_nodes.join(",\n"),
            internal_edges.join(",\n")
        ));

        if c + 1 < compounds {
            let nc = c + 1;
            cross_edges.push(format!(
                r#"    {{"id":"xe{cross_eid}","sources":["c{c}_l0"],"targets":["c{nc}_l0"]}}"#
            ));
            cross_eid += 1;
        }
    }

    format!(
        r#"{{
  "id":"root",
  "layoutOptions":{{
    "org.eclipse.elk.algorithm":"org.eclipse.elk.layered",
    "org.eclipse.elk.direction":"RIGHT",
    "org.eclipse.elk.edgeRouting":"ORTHOGONAL",
    "org.eclipse.elk.hierarchyHandling":"INCLUDE_CHILDREN"
  }},
  "children":[
{}
  ],
  "edges":[
{}
  ]
}}"#,
        compound_blocks.join(",\n"),
        cross_edges.join(",\n")
    )
}

fn synthetic_scenarios_json() -> Vec<(&'static str, String)> {
    vec![
        ("layered_small", generate_dag_json(10, 15, 42, "")),
        ("layered_medium", generate_dag_json(50, 100, 42, "")),
        ("layered_large", generate_dag_json(200, 500, 42, "")),
        ("layered_xlarge", generate_dag_json(1000, 3000, 42, "")),
        ("force_medium", generate_general_graph_json(50, 80, 100, "org.eclipse.elk.force")),
        ("force_large", generate_general_graph_json(200, 400, 100, "org.eclipse.elk.force")),
        ("force_xlarge", generate_general_graph_json(500, 1200, 100, "org.eclipse.elk.force")),
        ("stress_medium", generate_general_graph_json(50, 80, 100, "org.eclipse.elk.stress")),
        ("stress_large", generate_general_graph_json(200, 400, 100, "org.eclipse.elk.stress")),
        ("stress_xlarge", generate_general_graph_json(500, 1200, 100, "org.eclipse.elk.stress")),
        ("mrtree_medium", generate_tree_json(50, 200, "org.eclipse.elk.mrtree")),
        ("mrtree_large", generate_tree_json(200, 200, "org.eclipse.elk.mrtree")),
        ("mrtree_xlarge", generate_tree_json(1000, 200, "org.eclipse.elk.mrtree")),
        ("radial_medium", generate_tree_json(50, 200, "org.eclipse.elk.radial")),
        ("radial_large", generate_tree_json(200, 200, "org.eclipse.elk.radial")),
        ("radial_xlarge", generate_tree_json(1000, 200, "org.eclipse.elk.radial")),
        ("rectpacking_medium", generate_rectpacking_json(50, 100)),
        ("rectpacking_large", generate_rectpacking_json(200, 100)),
        ("rectpacking_xlarge", generate_rectpacking_json(1000, 100)),
        ("routing_polyline", generate_dag_json(50, 100, 42, r#""org.eclipse.elk.edgeRouting": "POLYLINE""#)),
        ("routing_orthogonal", generate_dag_json(50, 100, 42, r#""org.eclipse.elk.edgeRouting": "ORTHOGONAL""#)),
        ("routing_splines", generate_dag_json(50, 100, 42, r#""org.eclipse.elk.edgeRouting": "SPLINES""#)),
        ("crossmin_layer_sweep", generate_dag_json(50, 100, 42, r#""org.eclipse.elk.layered.crossingMinimization.strategy": "LAYER_SWEEP", "org.eclipse.elk.layered.crossingMinimization.greedySwitch.type": "TWO_SIDED""#)),
        ("crossmin_none", generate_dag_json(50, 100, 42, r#""org.eclipse.elk.layered.crossingMinimization.strategy": "NONE", "org.eclipse.elk.layered.crossingMinimization.greedySwitch.type": "OFF""#)),
        ("hierarchy_flat", generate_dag_json(30, 50, 300, "")),
        ("hierarchy_nested", generate_hierarchy_nested_json(300)),
    ]
}

fn run_api_benchmark(
    _name: &str,
    json: &str,
    iterations: usize,
    warmup: usize,
) -> Result<(u128, f64, f64), String> {
    for _ in 0..warmup {
        layout_api::layout_json(json, "{}")?;
    }

    let start = Instant::now();
    for _ in 0..iterations {
        layout_api::layout_json(json, "{}")?;
    }
    let elapsed = start.elapsed();
    let nanos = elapsed.as_nanos().max(1);
    let avg_ms = nanos as f64 / iterations.max(1) as f64 / 1_000_000.0;
    let ops_per_sec = iterations.max(1) as f64 / (nanos as f64 / 1_000_000_000.0);

    Ok((nanos, avg_ms, ops_per_sec))
}

// ===========================================================================
// Model loading (models mode, rust_api only)
// ===========================================================================

struct ModelEntry {
    name: String,
    json: String,
}

fn load_models_from_manifest(manifest_path: &str, limit: usize) -> Vec<ModelEntry> {
    let file = match fs::File::open(manifest_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Cannot open manifest {}: {}", manifest_path, e);
            return Vec::new();
        }
    };

    let reader = BufReader::new(file);
    let mut models = Vec::new();
    let mut first_line = true;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if first_line {
            first_line = false;
            let trimmed = line.trim_start_matches('\u{feff}');
            if trimmed.starts_with("model_rel_path") {
                continue;
            }
        }

        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 5 {
            continue;
        }

        let model_rel = cols[0];
        let input_json_path = cols[1];
        let status = cols[3];

        if status != "ok" {
            continue;
        }

        let json_path = Path::new(input_json_path);
        if !json_path.exists() {
            continue;
        }

        match fs::read_to_string(json_path) {
            Ok(json) => {
                let name = model_rel
                    .trim_end_matches(".elkg")
                    .trim_end_matches(".elkt")
                    .replace(['/', '\\'], "_");
                models.push(ModelEntry { name, json });
            }
            Err(_) => continue,
        }

        if limit > 0 && models.len() >= limit {
            break;
        }
    }

    models
}

fn load_models_from_dir(dir: &str, limit: usize) -> Vec<ModelEntry> {
    let mut models = Vec::new();
    let mut files = Vec::new();
    collect_json_files(Path::new(dir), &mut files);
    files.sort();

    for file in files {
        match fs::read_to_string(&file) {
            Ok(json) => {
                let name = file
                    .strip_prefix(dir)
                    .unwrap_or(&file)
                    .trim_start_matches('/')
                    .trim_end_matches(".json")
                    .replace(['/', '\\'], "_");
                models.push(ModelEntry { name, json });
            }
            Err(_) => continue,
        }
        if limit > 0 && models.len() >= limit {
            break;
        }
    }

    models
}

fn collect_json_files(dir: &Path, out: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(s) = path.to_str() {
                out.push(s.to_string());
            }
        }
    }
}

// ===========================================================================
// Main
// ===========================================================================

fn main() {
    layout_api::ensure_initialized();

    let args: Vec<String> = env::args().collect();
    let engine = parse_arg_str(&args, "--engine", "rust_native");
    let mode = parse_arg_str(&args, "--mode", "synthetic");
    let scenarios_arg = parse_arg_str(&args, "--scenarios", DEFAULT_SCENARIOS);
    let iterations = parse_arg(&args, "--iterations", 20).max(1);
    let warmup = parse_arg(&args, "--warmup", 3);
    let output_path = parse_arg_str_opt(&args, "--output");
    let input_dir = parse_arg_str_opt(&args, "--input-dir");
    let manifest = parse_arg_str_opt(&args, "--manifest");
    let limit = parse_arg(&args, "--limit", 50);

    eprintln!("Rust Performance Benchmark");
    eprintln!("  Engine: {engine}, Mode: {mode}, Iterations: {iterations}, Warmup: {warmup}");
    eprintln!();

    let mut csv_lines = Vec::new();

    match engine.as_str() {
        "rust_native" => {
            let scenarios: Vec<&str> = scenarios_arg
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();

            for scenario in &scenarios {
                match run_native_scenario(scenario, iterations, warmup) {
                    Some((elapsed_nanos, avg_ms, ops_per_sec)) => {
                        eprintln!(
                            "  {scenario}: {avg_ms:.4} ms/op, {ops_per_sec:.0} ops/s ({elapsed_nanos} ns total)"
                        );
                        csv_lines.push(format_csv_line(
                            &engine,
                            scenario,
                            iterations,
                            warmup,
                            elapsed_nanos,
                            avg_ms,
                            ops_per_sec,
                        ));
                    }
                    None => {
                        eprintln!("  Skipping unknown scenario: {scenario}");
                    }
                }
            }
        }

        "rust_api" => match mode.as_str() {
            "synthetic" => {
                for (name, json) in synthetic_scenarios_json() {
                    match run_api_benchmark(name, json.as_str(), iterations, warmup) {
                        Ok((nanos, avg_ms, ops_per_sec)) => {
                            eprintln!(
                                "  {name}: {avg_ms:.4} ms/op, {ops_per_sec:.0} ops/s ({nanos} ns total)"
                            );
                            csv_lines.push(format_csv_line(
                                &engine, name, iterations, warmup, nanos, avg_ms, ops_per_sec,
                            ));
                        }
                        Err(e) => {
                            eprintln!("  {name}: ERROR — {e}");
                        }
                    }
                }
            }

            "models" => {
                let models = if let Some(ref m) = manifest {
                    eprintln!("Loading models from manifest: {m}");
                    load_models_from_manifest(m, limit)
                } else if let Some(ref d) = input_dir {
                    eprintln!("Loading models from directory: {d}");
                    load_models_from_dir(d, limit)
                } else {
                    let default_manifest = "parity/model_parity/java/java_manifest.tsv";
                    if Path::new(default_manifest).exists() {
                        eprintln!("Loading models from manifest: {default_manifest}");
                        load_models_from_manifest(default_manifest, limit)
                    } else {
                        eprintln!(
                            "No manifest or input directory specified. Use --manifest or --input-dir."
                        );
                        return;
                    }
                };

                eprintln!("Loaded {} models", models.len());
                eprintln!();

                let mut ok = 0usize;
                let mut errors = 0usize;
                for model in &models {
                    match run_api_benchmark(&model.name, &model.json, iterations, warmup) {
                        Ok((nanos, avg_ms, ops_per_sec)) => {
                            ok += 1;
                            if ok <= 5 || ok.is_multiple_of(10) {
                                eprintln!("  {}: {:.4} ms/op", model.name, avg_ms);
                            }
                            csv_lines.push(format_csv_line(
                                &engine,
                                &model.name,
                                iterations,
                                warmup,
                                nanos,
                                avg_ms,
                                ops_per_sec,
                            ));
                        }
                        Err(e) => {
                            errors += 1;
                            if errors <= 3 {
                                eprintln!("  {}: ERROR — {}", model.name, e);
                            }
                        }
                    }
                }
                eprintln!();
                eprintln!("Done: {ok} ok, {errors} errors");
            }

            _ => {
                eprintln!("Unknown mode: {mode}. Use 'synthetic' or 'models'.");
            }
        },

        _ => {
            eprintln!("Unknown engine: {engine}. Use 'rust_native' or 'rust_api'.");
        }
    }

    eprintln!();
    write_results(output_path.as_deref(), &csv_lines);
}
