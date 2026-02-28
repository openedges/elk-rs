use std::env;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    CrossingMinimizationStrategy, CycleBreakingStrategy, GreedySwitchType, LayeredMetaDataProvider,
    LayeredOptions, OrderingStrategy,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    Direction, NodeLabelPlacement, PortLabelPlacement, PortSide,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkGraphElementRef, ElkLabelRef, ElkNodeRef, ElkPortRef,
};

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

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn set_node_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    node.borrow_mut()
        .connectable()
        .shape()
        .set_dimensions(width, height);
}

fn set_port_dimensions(port: &ElkPortRef, width: f64, height: f64) {
    port.borrow_mut()
        .connectable()
        .shape()
        .set_dimensions(width, height);
}

fn set_label_dimensions(label: &ElkLabelRef, width: f64, height: f64) {
    label.borrow_mut().shape().set_dimensions(width, height);
}

fn set_label_location(label: &ElkLabelRef, x: f64, y: f64) {
    label.borrow_mut().shape().set_location(x, y);
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

fn set_port_property<T: Clone + Send + Sync + 'static>(
    port: &ElkPortRef,
    property: &Property<T>,
    value: T,
) {
    port.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn set_label_property<T: Clone + Send + Sync + 'static>(
    label: &ElkLabelRef,
    property: &Property<T>,
    value: T,
) {
    label
        .borrow_mut()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn create_port_with_label(node: &ElkNodeRef, side: PortSide, text: &str) -> ElkPortRef {
    let port = ElkGraphUtil::create_port(Some(node.clone()));
    set_port_dimensions(&port, 10.0, 10.0);
    set_port_property(&port, LayeredOptions::PORT_SIDE, side);

    let label =
        ElkGraphUtil::create_label_with_text(text, Some(ElkGraphElementRef::Port(port.clone())));
    set_label_dimensions(&label, 20.0, 10.0);
    port
}

fn build_issue_405_scenario() -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(
        &graph,
        CoreOptions::ALGORITHM,
        LayeredOptions::ALGORITHM_ID.to_string(),
    );
    set_node_property(&graph, CoreOptions::DIRECTION, Direction::Right);
    set_node_property(&graph, CoreOptions::EDGE_ROUTING, EdgeRouting::Orthogonal);

    let reference_node = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_dimensions(&reference_node, 80.0, 60.0);
    set_node_property(
        &reference_node,
        LayeredOptions::PORT_CONSTRAINTS,
        PortConstraints::FixedSide,
    );

    let mut placement = PortLabelPlacement::outside();
    placement.insert(PortLabelPlacement::NextToPortIfPossible);
    set_node_property(
        &reference_node,
        CoreOptions::PORT_LABELS_PLACEMENT,
        placement,
    );

    let west = create_port_with_label(&reference_node, PortSide::West, "west");
    let east = create_port_with_label(&reference_node, PortSide::East, "east");
    let north = create_port_with_label(&reference_node, PortSide::North, "north");
    let south = create_port_with_label(&reference_node, PortSide::South, "south");

    let west_partner = ElkGraphUtil::create_node(Some(graph.clone()));
    let east_partner = ElkGraphUtil::create_node(Some(graph.clone()));
    let north_partner = ElkGraphUtil::create_node(Some(graph.clone()));
    let south_partner = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_dimensions(&west_partner, 30.0, 20.0);
    set_node_dimensions(&east_partner, 30.0, 20.0);
    set_node_dimensions(&north_partner, 30.0, 20.0);
    set_node_dimensions(&south_partner, 30.0, 20.0);

    let _edge_west = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(west_partner),
        ElkConnectableShapeRef::Port(west),
    );
    let _edge_east = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(east),
        ElkConnectableShapeRef::Node(east_partner),
    );
    let _edge_north = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(north),
        ElkConnectableShapeRef::Node(north_partner),
    );
    let _edge_south = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(south_partner),
        ElkConnectableShapeRef::Port(south),
    );

    graph
}

fn build_issue_603_scenario() -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(
        &graph,
        CoreOptions::ALGORITHM,
        LayeredOptions::ALGORITHM_ID.to_string(),
    );
    set_node_property(
        &graph,
        LayeredOptions::NODE_LABELS_PADDING,
        ElkPadding::with_values(24.0, 0.0, 0.0, 0.0),
    );

    let compound = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_dimensions(&compound, 120.0, 80.0);
    set_node_property(
        &compound,
        LayeredOptions::NODE_LABELS_PLACEMENT,
        NodeLabelPlacement::inside_top_center(),
    );
    set_node_property(
        &compound,
        LayeredOptions::NODE_LABELS_PADDING,
        ElkPadding::with_values(24.0, 0.0, 0.0, 0.0),
    );

    let label = ElkGraphUtil::create_label_with_text(
        "compound",
        Some(ElkGraphElementRef::Node(compound.clone())),
    );
    set_label_dimensions(&label, 40.0, 16.0);

    let child_a = ElkGraphUtil::create_node(Some(compound.clone()));
    let child_b = ElkGraphUtil::create_node(Some(compound.clone()));
    set_node_dimensions(&child_a, 30.0, 30.0);
    set_node_dimensions(&child_b, 30.0, 30.0);

    let _edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(child_a),
        ElkConnectableShapeRef::Node(child_b),
    );

    graph
}

fn build_issue_680_scenario() -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(
        &graph,
        CoreOptions::ALGORITHM,
        LayeredOptions::ALGORITHM_ID.to_string(),
    );
    set_node_property(&graph, CoreOptions::DIRECTION, Direction::Down);
    set_node_property(&graph, CoreOptions::EDGE_ROUTING, EdgeRouting::Orthogonal);

    let parent = ElkGraphUtil::create_node(Some(graph.clone()));
    let child = ElkGraphUtil::create_node(Some(parent.clone()));
    set_node_dimensions(&parent, 180.0, 110.0);
    set_node_dimensions(&child, 100.0, 60.0);

    let p1 = ElkGraphUtil::create_port(Some(parent.clone()));
    let p2 = ElkGraphUtil::create_port(Some(parent.clone()));
    set_port_dimensions(&p1, 10.0, 10.0);
    set_port_dimensions(&p2, 10.0, 10.0);
    set_port_property(&p1, LayeredOptions::PORT_SIDE, PortSide::West);
    set_port_property(&p2, LayeredOptions::PORT_SIDE, PortSide::East);
    set_port_property(&p1, LayeredOptions::PORT_BORDER_OFFSET, -20.0);
    set_port_property(&p2, LayeredOptions::PORT_BORDER_OFFSET, -22.0);

    let c1 = ElkGraphUtil::create_port(Some(child.clone()));
    let c2 = ElkGraphUtil::create_port(Some(child.clone()));
    set_port_dimensions(&c1, 10.0, 10.0);
    set_port_dimensions(&c2, 10.0, 10.0);
    set_port_property(&c1, LayeredOptions::PORT_SIDE, PortSide::West);
    set_port_property(&c2, LayeredOptions::PORT_SIDE, PortSide::East);
    set_port_property(&c1, LayeredOptions::PORT_BORDER_OFFSET, -8.0);
    set_port_property(&c2, LayeredOptions::PORT_BORDER_OFFSET, -8.0);

    let _edge1 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(p1),
        ElkConnectableShapeRef::Port(c1),
    );
    let _edge2 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(c2),
        ElkConnectableShapeRef::Port(p2),
    );

    graph
}

fn build_issue_871_base_scenario(model_order_feedback_mode: bool) -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(
        &graph,
        CoreOptions::ALGORITHM,
        LayeredOptions::ALGORITHM_ID.to_string(),
    );
    set_node_property(&graph, CoreOptions::DIRECTION, Direction::Right);
    if model_order_feedback_mode {
        set_node_property(
            &graph,
            LayeredOptions::CYCLE_BREAKING_STRATEGY,
            CycleBreakingStrategy::ModelOrder,
        );
        set_node_property(
            &graph,
            LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY,
            OrderingStrategy::PreferEdges,
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
        set_node_property(&graph, LayeredOptions::FEEDBACK_EDGES, true);
    }

    let n1 = ElkGraphUtil::create_node(Some(graph.clone()));
    let n2 = ElkGraphUtil::create_node(Some(graph.clone()));
    let n3 = ElkGraphUtil::create_node(Some(graph.clone()));
    let n4 = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_dimensions(&n1, 30.0, 30.0);
    set_node_dimensions(&n2, 30.0, 30.0);
    set_node_dimensions(&n3, 30.0, 30.0);
    set_node_dimensions(&n4, 30.0, 30.0);

    let _e1 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );
    let _e2 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1),
        ElkConnectableShapeRef::Node(n3.clone()),
    );
    let _e3 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n2),
        ElkConnectableShapeRef::Node(n4.clone()),
    );
    let _e4 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n4),
        ElkConnectableShapeRef::Node(n3),
    );

    graph
}

fn build_issue_871_scenario() -> ElkNodeRef {
    build_issue_871_base_scenario(true)
}

fn build_issue_871_plain_scenario() -> ElkNodeRef {
    build_issue_871_base_scenario(false)
}

fn build_issue_905_scenario() -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(
        &graph,
        CoreOptions::ALGORITHM,
        LayeredOptions::ALGORITHM_ID.to_string(),
    );
    set_node_property(&graph, CoreOptions::DIRECTION, Direction::Right);

    let source = ElkGraphUtil::create_node(Some(graph.clone()));
    let target = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_dimensions(&source, 30.0, 30.0);
    set_node_dimensions(&target, 30.0, 30.0);

    let edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(source),
        ElkConnectableShapeRef::Node(target),
    );

    let tail =
        ElkGraphUtil::create_label_with_text("tail", Some(ElkGraphElementRef::Edge(edge.clone())));
    set_label_dimensions(&tail, 16.0, 10.0);
    set_label_property(
        &tail,
        LayeredOptions::EDGE_LABELS_PLACEMENT,
        EdgeLabelPlacement::Tail,
    );
    set_label_location(&tail, 5.0, 10.0);

    let center = ElkGraphUtil::create_label_with_text(
        "center",
        Some(ElkGraphElementRef::Edge(edge.clone())),
    );
    set_label_dimensions(&center, 20.0, 10.0);
    set_label_property(
        &center,
        LayeredOptions::EDGE_LABELS_PLACEMENT,
        EdgeLabelPlacement::Center,
    );
    set_label_location(&center, 20.0, 80.0);

    let head = ElkGraphUtil::create_label_with_text("head", Some(ElkGraphElementRef::Edge(edge)));
    set_label_dimensions(&head, 16.0, 10.0);
    set_label_property(
        &head,
        LayeredOptions::EDGE_LABELS_PLACEMENT,
        EdgeLabelPlacement::Head,
    );
    set_label_location(&head, 35.0, 150.0);

    graph
}

fn lcg(state: u32) -> u32 {
    (state.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7fffffff
}

fn build_layered_dag_scenario(nodes: usize, edges: usize, seed: u32) -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(
        &graph,
        CoreOptions::ALGORITHM,
        LayeredOptions::ALGORITHM_ID.to_string(),
    );
    set_node_property(&graph, CoreOptions::DIRECTION, Direction::Right);
    set_node_property(&graph, CoreOptions::EDGE_ROUTING, EdgeRouting::Orthogonal);

    let node_refs: Vec<ElkNodeRef> = (0..nodes)
        .map(|_| {
            let node = ElkGraphUtil::create_node(Some(graph.clone()));
            set_node_dimensions(&node, 40.0, 30.0);
            node
        })
        .collect();

    let layer_of: Vec<usize> = (0..nodes).map(|i| i * 5 / nodes).collect();

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

fn build_issue_scenario(name: &str) -> Option<ElkNodeRef> {
    match name {
        "issue_405" => Some(build_issue_405_scenario()),
        "issue_603" => Some(build_issue_603_scenario()),
        "issue_680" => Some(build_issue_680_scenario()),
        "issue_871" => Some(build_issue_871_scenario()),
        "issue_871_plain" => Some(build_issue_871_plain_scenario()),
        "issue_905" => Some(build_issue_905_scenario()),
        "layered_small" => Some(build_layered_dag_scenario(10, 15, 42)),
        "layered_medium" => Some(build_layered_dag_scenario(50, 100, 42)),
        "layered_large" => Some(build_layered_dag_scenario(200, 500, 42)),
        "layered_xlarge" => Some(build_layered_dag_scenario(1000, 3000, 42)),
        _ => None,
    }
}

fn run_scenario(name: &str, iterations: usize, warmup: usize) -> Option<(u128, f64, f64)> {
    let mut provider = LayeredLayoutProvider::new();

    for _ in 0..warmup {
        let graph = build_issue_scenario(name)?;
        provider.layout(&graph, &mut BasicProgressMonitor::new());
    }

    let start = Instant::now();
    for _ in 0..iterations {
        let graph = build_issue_scenario(name)?;
        provider.layout(&graph, &mut BasicProgressMonitor::new());
    }
    let elapsed = start.elapsed();
    let nanos = elapsed.as_nanos().max(1);
    let avg_ms = nanos as f64 / iterations.max(1) as f64 / 1_000_000.0;
    let scenarios_per_sec = iterations.max(1) as f64 / (nanos as f64 / 1_000_000_000.0);
    Some((nanos, avg_ms, scenarios_per_sec))
}

fn append_result(
    path: &str,
    scenario: &str,
    iterations: usize,
    warmup: usize,
    elapsed_nanos: u128,
    avg_ms: f64,
    scenarios_per_sec: f64,
) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);

    let line = format!(
        "{timestamp},{scenario},{iterations},{warmup},{elapsed_nanos},{avg_ms:.6},{scenarios_per_sec:.2}\n"
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

fn main() {
    init_layered_options();

    let args: Vec<String> = env::args().collect();
    let scenarios_arg = parse_arg_str(
        &args,
        "--scenarios",
        "layered_small,layered_medium,layered_large,layered_xlarge",
    );
    let iterations = parse_arg(&args, "--iterations", 20).max(1);
    let warmup = parse_arg(&args, "--warmup", 3);
    let output_path = parse_arg_str_opt(&args, "--output");

    let scenarios = scenarios_arg
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();

    for scenario in scenarios {
        match run_scenario(scenario, iterations, warmup) {
            Some((elapsed_nanos, avg_ms, scenarios_per_sec)) => {
                println!(
                    "Layered issue perf: scenario={} iterations={} warmup={} -> {} ns",
                    scenario, iterations, warmup, elapsed_nanos
                );
                println!(
                    "Average per iteration: {:.4} ms, throughput: {:.2} scenarios/s",
                    avg_ms, scenarios_per_sec
                );

                if let Some(path) = output_path.as_ref() {
                    append_result(
                        path,
                        scenario,
                        iterations,
                        warmup,
                        elapsed_nanos,
                        avg_ms,
                        scenarios_per_sec,
                    );
                }
            }
            None => {
                eprintln!("Skipping unknown scenario: {}", scenario);
            }
        }
    }
}
