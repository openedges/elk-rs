use std::collections::HashMap;
use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::elk_layered::ElkLayered;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_importer::ElkGraphImporter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    CycleBreakingStrategy, LayeredMetaDataProvider, LayeredOptions, LayeringStrategy,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};

#[test]
fn cycle_breakers_produce_acyclic_graphs() {
    init_layered_options();

    for strategy in [CycleBreakingStrategy::Greedy, CycleBreakingStrategy::DepthFirst] {
        let root = build_cyclic_graph();
        set_node_property(&root, CoreOptions::ALGORITHM, "org.eclipse.elk.layered".to_string());
        set_node_property(&root, LayeredOptions::CYCLE_BREAKING_STRATEGY, strategy);
        set_node_property(
            &root,
            LayeredOptions::LAYERING_STRATEGY,
            LayeringStrategy::NetworkSimplex,
        );

        let lgraph = import_lgraph(&root);
        let mut layered = ElkLayered::new();
        layered.do_layout(&lgraph, None);

        assert_acyclic(&lgraph);
    }
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
    let graph_guard = lgraph.lock().expect("lgraph lock");
    let mut nodes = graph_guard.layerless_nodes().clone();
    for layer in graph_guard.layers() {
        if let Ok(layer_guard) = layer.lock() {
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
        .lock()
        .ok()
        .map(|node_guard| node_guard.outgoing_edges())
        .unwrap_or_default();
    for edge in outgoing {
        let target_node = edge
            .lock()
            .ok()
            .and_then(|edge_guard| edge_guard.target())
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
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
    node_mut
        .connectable()
        .shape()
        .set_dimensions(width, height);
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
