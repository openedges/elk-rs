use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::networksimplex::{
    NEdge, NGraph, NNode, NetworkSimplex,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, Random};

#[test]
fn network_simplex_deltas() {
    let handle = std::thread::Builder::new()
        .name("network_simplex_deltas".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let mut random = Random::new(1);

            for _ in 0..5 {
                for _ in 0..5 {
                    let mut graph = generate_random_graph(&mut random);
                    assert!(graph.is_acyclic());

                    let mut simplex = NetworkSimplex::for_graph(&mut graph);
                    let mut monitor = BasicProgressMonitor::new();
                    simplex.execute_with_monitor(&mut monitor);

                    for node in &graph.nodes {
                        let outgoing = node
                            .lock_ok()
                            .map(|guard| guard.outgoing_edges().clone())
                            .unwrap_or_default();
                        for edge in outgoing {
                            let (source_layer, target_layer, delta) = {
                                let edge_guard = edge.lock();                                let source_layer = edge_guard
                                    .source
                                    .lock_ok()
                                    .map(|node_guard| node_guard.layer)
                                    .unwrap_or(0);
                                let target_layer = edge_guard
                                    .target
                                    .lock_ok()
                                    .map(|node_guard| node_guard.layer)
                                    .unwrap_or(0);
                                (source_layer, target_layer, edge_guard.delta)
                            };
                            assert!(target_layer - source_layer >= delta, "Valid delta");
                        }
                    }
                }
            }
        })
        .expect("spawn network_simplex_deltas");
    handle.join().expect("network_simplex_deltas join");
}

fn generate_random_graph(random: &mut Random) -> NGraph {
    let mut graph = NGraph::new();

    let n = 4000;
    let e = 8000;

    for i in 0..n {
        NNode::of().id(i as i32).create(&mut graph);
    }

    for _ in 0..e {
        let src = random.next_int(n as i32) as usize;
        let mut tgt = random.next_int(n as i32) as usize;
        while src == tgt {
            tgt = random.next_int(n as i32) as usize;
        }
        NEdge::of()
            .delta(random.next_int(50))
            .weight(random.next_double() * 50.0)
            .source(graph.nodes[src].clone())
            .target(graph.nodes[tgt].clone())
            .create();
    }

    for i in 0..(n - 1) {
        NEdge::of()
            .delta(random.next_int(50))
            .weight(random.next_double() * 50.0)
            .source(graph.nodes[i].clone())
            .target(graph.nodes[i + 1].clone())
            .create();
    }

    for node in &graph.nodes {
        let outgoing = node
            .lock_ok()
            .map(|guard| guard.outgoing_edges().clone())
            .unwrap_or_default();
        for edge in outgoing {
            let (source_id, target_id) = {
                let edge_guard = edge.lock();                let source_id = edge_guard
                    .source
                    .lock_ok()
                    .map(|node_guard| node_guard.id)
                    .unwrap_or(0);
                let target_id = edge_guard
                    .target
                    .lock_ok()
                    .map(|node_guard| node_guard.id)
                    .unwrap_or(0);
                (source_id, target_id)
            };
            if source_id > target_id {
                NEdge::reverse(&edge);
            }
        }
    }

    graph
}
