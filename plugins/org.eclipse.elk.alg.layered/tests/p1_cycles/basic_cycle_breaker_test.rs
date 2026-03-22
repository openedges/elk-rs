use std::collections::HashMap;
use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_importer::ElkGraphImporter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    CycleBreakingStrategy, LayeredMetaDataProvider, LayeredOptions, LayeringStrategy,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p1cycles::{
    BfsNodeOrderCycleBreaker, DepthFirstCycleBreaker, DfsNodeOrderCycleBreaker, GreedyCycleBreaker,
    ModelOrderCycleBreaker, ScConnectivityCycleBreaker, SccNodeTypeCycleBreaker,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};

#[test]
fn greedy_cycle_breaker_produces_acyclic_graph() {
    run_cycle_breaker_test(CycleBreakingStrategy::Greedy, build_cyclic_graph());
}

#[test]
fn greedy_cycle_breaker_type_name_matches_model_order_mode() {
    let greedy = GreedyCycleBreaker::new();
    assert_eq!(greedy.type_name(), "GreedyCycleBreaker");

    let model_order = GreedyCycleBreaker::new_with_model_order(true);
    assert_eq!(model_order.type_name(), "GreedyModelOrderCycleBreaker");
}

#[test]
fn depth_first_cycle_breaker_produces_acyclic_graph() {
    run_cycle_breaker_test(CycleBreakingStrategy::DepthFirst, build_cyclic_graph());
}

#[test]
fn greedy_cycle_breaker_handles_dense_cycle_graph() {
    run_cycle_breaker_test(CycleBreakingStrategy::Greedy, build_dense_cyclic_graph());
}

#[test]
fn depth_first_cycle_breaker_handles_dense_cycle_graph() {
    run_cycle_breaker_test(
        CycleBreakingStrategy::DepthFirst,
        build_dense_cyclic_graph(),
    );
}

#[test]
fn greedy_model_order_cycle_breaker_produces_acyclic_graph() {
    run_cycle_breaker_test(
        CycleBreakingStrategy::GreedyModelOrder,
        build_cyclic_graph(),
    );
}

#[test]
fn model_order_cycle_breaker_produces_acyclic_graph() {
    run_cycle_breaker_test(CycleBreakingStrategy::ModelOrder, build_cyclic_graph());
}

#[test]
fn bfs_node_order_cycle_breaker_produces_acyclic_graph() {
    run_cycle_breaker_test(CycleBreakingStrategy::BfsNodeOrder, build_cyclic_graph());
}

#[test]
fn dfs_node_order_cycle_breaker_produces_acyclic_graph() {
    run_cycle_breaker_test(CycleBreakingStrategy::DfsNodeOrder, build_cyclic_graph());
}

#[test]
fn scc_connectivity_cycle_breaker_produces_acyclic_graph() {
    run_cycle_breaker_test(CycleBreakingStrategy::SccConnectivity, build_cyclic_graph());
}

#[test]
fn scc_node_type_cycle_breaker_produces_acyclic_graph() {
    run_cycle_breaker_test(CycleBreakingStrategy::SccNodeType, build_cyclic_graph());
}

#[test]
fn model_order_cycle_breaker_handles_dense_cycle_graph() {
    run_cycle_breaker_test(
        CycleBreakingStrategy::ModelOrder,
        build_dense_cyclic_graph(),
    );
}

#[test]
fn bfs_node_order_cycle_breaker_handles_dense_cycle_graph() {
    run_cycle_breaker_test(
        CycleBreakingStrategy::BfsNodeOrder,
        build_dense_cyclic_graph(),
    );
}

#[test]
fn dfs_node_order_cycle_breaker_handles_dense_cycle_graph() {
    run_cycle_breaker_test(
        CycleBreakingStrategy::DfsNodeOrder,
        build_dense_cyclic_graph(),
    );
}

#[test]
fn scc_connectivity_cycle_breaker_handles_dense_cycle_graph() {
    run_cycle_breaker_test(
        CycleBreakingStrategy::SccConnectivity,
        build_dense_cyclic_graph(),
    );
}

#[test]
fn scc_node_type_cycle_breaker_handles_dense_cycle_graph() {
    run_cycle_breaker_test(
        CycleBreakingStrategy::SccNodeType,
        build_dense_cyclic_graph(),
    );
}

fn run_cycle_breaker_test(strategy: CycleBreakingStrategy, root: ElkNodeRef) {
    init_layered_options();

    set_node_property(&root, LayeredOptions::CYCLE_BREAKING_STRATEGY, strategy);
    set_node_property(
        &root,
        LayeredOptions::LAYERING_STRATEGY,
        LayeringStrategy::NetworkSimplex,
    );

    let lgraph = import_lgraph(&root);
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = lgraph.lock();    match strategy {
        CycleBreakingStrategy::Greedy => {
            let mut breaker = GreedyCycleBreaker::new();
            breaker.process(&mut graph_guard, &mut monitor);
        }
        CycleBreakingStrategy::DepthFirst => {
            let mut breaker = DepthFirstCycleBreaker::new();
            breaker.process(&mut graph_guard, &mut monitor);
        }
        CycleBreakingStrategy::GreedyModelOrder => {
            let mut breaker = GreedyCycleBreaker::new_with_model_order(true);
            breaker.process(&mut graph_guard, &mut monitor);
        }
        CycleBreakingStrategy::ModelOrder => {
            let mut breaker = ModelOrderCycleBreaker::new();
            breaker.process(&mut graph_guard, &mut monitor);
        }
        CycleBreakingStrategy::BfsNodeOrder => {
            let mut breaker = BfsNodeOrderCycleBreaker::new();
            breaker.process(&mut graph_guard, &mut monitor);
        }
        CycleBreakingStrategy::DfsNodeOrder => {
            let mut breaker = DfsNodeOrderCycleBreaker::new();
            breaker.process(&mut graph_guard, &mut monitor);
        }
        CycleBreakingStrategy::SccConnectivity => {
            let mut breaker = ScConnectivityCycleBreaker::new();
            breaker.process(&mut graph_guard, &mut monitor);
        }
        CycleBreakingStrategy::SccNodeType => {
            let mut breaker = SccNodeTypeCycleBreaker::new();
            breaker.process(&mut graph_guard, &mut monitor);
        }
        _ => panic!("unsupported cycle breaking strategy in this test"),
    }
    drop(graph_guard);

    assert_acyclic(&lgraph);
}

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn build_cyclic_graph() -> ElkNodeRef {
    let root = ElkGraphUtil::create_graph();
    let node_a = ElkGraphUtil::create_node(Some(root.clone()));
    let node_b = ElkGraphUtil::create_node(Some(root.clone()));
    let node_c = ElkGraphUtil::create_node(Some(root.clone()));

    set_dimensions(&node_a, 30.0, 30.0);
    set_dimensions(&node_b, 30.0, 30.0);
    set_dimensions(&node_c, 30.0, 30.0);

    let _e1 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node_a.clone()),
        ElkConnectableShapeRef::Node(node_b.clone()),
    );
    let _e2 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node_b.clone()),
        ElkConnectableShapeRef::Node(node_c.clone()),
    );
    let _e3 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node_c.clone()),
        ElkConnectableShapeRef::Node(node_a.clone()),
    );

    root
}

fn build_dense_cyclic_graph() -> ElkNodeRef {
    let root = ElkGraphUtil::create_graph();
    let node_a = ElkGraphUtil::create_node(Some(root.clone()));
    let node_b = ElkGraphUtil::create_node(Some(root.clone()));
    let node_c = ElkGraphUtil::create_node(Some(root.clone()));
    let node_d = ElkGraphUtil::create_node(Some(root.clone()));
    let node_e = ElkGraphUtil::create_node(Some(root.clone()));

    for node in [&node_a, &node_b, &node_c, &node_d, &node_e] {
        set_dimensions(node, 30.0, 30.0);
    }

    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node_a.clone()),
        ElkConnectableShapeRef::Node(node_b.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node_b.clone()),
        ElkConnectableShapeRef::Node(node_c.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node_c.clone()),
        ElkConnectableShapeRef::Node(node_d.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node_d.clone()),
        ElkConnectableShapeRef::Node(node_e.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node_e.clone()),
        ElkConnectableShapeRef::Node(node_a.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node_a.clone()),
        ElkConnectableShapeRef::Node(node_c.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node_b.clone()),
        ElkConnectableShapeRef::Node(node_d.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node_c.clone()),
        ElkConnectableShapeRef::Node(node_e.clone()),
    );

    root
}

fn import_lgraph(
    root: &ElkNodeRef,
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef {
    let mut origin_store = OriginStore::new();
    let mut importer = ElkGraphImporter::new(&mut origin_store);
    importer.import_graph(root)
}

fn assert_acyclic(
    lgraph: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef,
) {
    let nodes = collect_nodes(lgraph);
    let mut visit = HashMap::new();
    for node in &nodes {
        visit.insert(node_id(node), VisitState::Unvisited);
    }

    for node in nodes {
        if visit.get(&node_id(&node)) == Some(&VisitState::Unvisited) {
            dfs(&node, &mut visit);
        }
    }
}

fn collect_nodes(
    lgraph: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef,
) -> Vec<org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef> {
    let graph_guard = lgraph.lock();    let mut nodes = graph_guard.layerless_nodes().clone();
    for layer in graph_guard.layers() {
        {
            let layer_guard = layer.lock();
            nodes.extend(layer_guard.nodes().clone());
        }
    }
    nodes
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

fn dfs(
    node: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
    visit: &mut HashMap<usize, VisitState>,
) {
    let id = node_id(node);
    match visit.get(&id) {
        Some(VisitState::Visiting) => panic!("cycle detected"),
        Some(VisitState::Visited) => return,
        _ => {}
    }
    visit.insert(id, VisitState::Visiting);

    let outgoing = node
        .lock().outgoing_edges();
    for edge in outgoing {
        let target_node = edge
            .lock().target()
            .and_then(|port| port.lock().node());
        if let Some(target) = target_node {
            dfs(&target, visit);
        }
    }

    visit.insert(id, VisitState::Visited);
}

fn node_id(
    node: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
) -> usize {
    Arc::as_ptr(node) as usize
}

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

fn set_node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
    value: T,
) {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}
