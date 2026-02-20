use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;

use crate::org::eclipse::elk::alg::force::graph::{FEdgeRef, FGraph, FNodeRef};
use crate::org::eclipse::elk::alg::force::options::StressOptions;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum Dimension {
    #[default]
    XY,
    X,
    Y,
}

#[derive(Debug, Clone, Copy)]
struct State {
    cost: f64,
    position: usize,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.position.cmp(&other.position))
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.position == other.position
    }
}

impl Eq for State {}

#[derive(Default)]
pub struct StressMajorization {
    apsp: Vec<Vec<f64>>,
    w: Vec<Vec<f64>>,
    desired_edge_length: f64,
    dim: Dimension,
    epsilon: f64,
    iteration_limit: i32,
    connected_edges: Vec<Vec<FEdgeRef>>,
}

impl StressMajorization {
    pub fn new() -> Self {
        StressMajorization::default()
    }

    pub fn initialize(&mut self, graph: &mut FGraph) {
        let n = graph.nodes().len();
        if n <= 1 {
            return;
        }

        self.dim = graph
            .get_property(StressOptions::DIMENSION)
            .unwrap_or(Dimension::XY);
        self.iteration_limit = graph
            .get_property(StressOptions::ITERATION_LIMIT)
            .unwrap_or(i32::MAX);
        self.epsilon = graph.get_property(StressOptions::EPSILON).unwrap_or(10e-4);
        self.desired_edge_length = graph
            .get_property(StressOptions::DESIRED_EDGE_LENGTH)
            .unwrap_or(100.0);

        self.connected_edges.clear();
        self.connected_edges.resize_with(n, Vec::new);
        for edge in graph.edges() {
            let (source_id, target_id) = {
                let edge_guard = edge.lock().ok();
                let Some(edge_guard) = edge_guard else {
                    continue;
                };
                let source_id = edge_guard
                    .source()
                    .and_then(|node| node.lock().ok().map(|n| n.id()));
                let target_id = edge_guard
                    .target()
                    .and_then(|node| node.lock().ok().map(|n| n.id()));
                match (source_id, target_id) {
                    (Some(source_id), Some(target_id)) => (source_id, target_id),
                    _ => continue,
                }
            };
            if source_id < n {
                self.connected_edges[source_id].push(edge.clone());
            }
            if target_id < n {
                self.connected_edges[target_id].push(edge.clone());
            }
        }

        self.apsp = vec![vec![0.0; n]; n];
        for node in graph.nodes() {
            let node_id = node.lock().ok().map(|n| n.id());
            let Some(node_id) = node_id else { continue };
            Self::dijkstra(
                &self.connected_edges,
                graph,
                self.desired_edge_length,
                node_id,
                &mut self.apsp[node_id],
            );
        }

        self.w = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                let dij = self.apsp[i][j];
                if dij != 0.0 {
                    self.w[i][j] = 1.0 / (dij * dij);
                }
            }
        }

        if std::env::var_os("ELK_TRACE_STRESS").is_some() {
            let edge_count = graph.edges().len();
            let apsp01 = if n > 1 { self.apsp[0][1] } else { 0.0 };
            let w01 = if n > 1 { self.w[0][1] } else { 0.0 };
            eprintln!(
                "stress-majorization: nodes={} edges={} desired_edge_length={} apsp01={} w01={}",
                n, edge_count, self.desired_edge_length, apsp01, w01
            );
        }
    }

    pub fn execute(&mut self, graph: &mut FGraph) {
        if graph.nodes().len() <= 1 {
            return;
        }

        let mut count = 0;
        let mut prev_stress = self.compute_stress(graph);
        let mut cur_stress = f64::INFINITY;

        loop {
            if count > 0 {
                prev_stress = cur_stress;
            }

            for node in graph.nodes() {
                let fixed = node
                    .lock()
                    .ok()
                    .and_then(|mut node_guard| node_guard.get_property(StressOptions::FIXED))
                    .unwrap_or(false);
                if fixed {
                    continue;
                }

                let new_pos = self.compute_new_position(graph, node);
                if let Ok(mut node_guard) = node.lock() {
                    node_guard.position().reset().add(&new_pos);
                }
            }

            cur_stress = self.compute_stress(graph);

            if self.done(count, prev_stress, cur_stress) {
                if std::env::var_os("ELK_TRACE_STRESS").is_some() {
                    eprintln!(
                        "stress-majorization: iterations={} prev_stress={} cur_stress={}",
                        count, prev_stress, cur_stress
                    );
                }
                break;
            }
            count += 1;
        }
    }

    fn dijkstra(
        connected_edges: &[Vec<FEdgeRef>],
        graph: &FGraph,
        desired_edge_length: f64,
        source_id: usize,
        dist: &mut [f64],
    ) {
        let n = graph.nodes().len();
        let mut heap = BinaryHeap::new();
        let mut visited = vec![false; n];

        for (i, d) in dist.iter_mut().enumerate().take(n) {
            *d = if i == source_id { 0.0 } else { f64::INFINITY };
            heap.push(State {
                cost: *d,
                position: i,
            });
        }

        while let Some(State { cost, position }) = heap.pop() {
            if visited[position] {
                continue;
            }
            visited[position] = true;
            if cost > dist[position] {
                continue;
            }

            for edge in &connected_edges[position] {
                let (other_id, edge_len) = {
                    let edge_guard = edge.lock().ok();
                    let Some(mut edge_guard) = edge_guard else {
                        continue;
                    };
                    let source_id = edge_guard
                        .source()
                        .and_then(|node| node.lock().ok().map(|n| n.id()));
                    let target_id = edge_guard
                        .target()
                        .and_then(|node| node.lock().ok().map(|n| n.id()));
                    let Some(source_id) = source_id else { continue };
                    let Some(target_id) = target_id else { continue };
                    let other_id = if source_id == position {
                        target_id
                    } else {
                        source_id
                    };
                    let edge_len = if edge_guard.has_property(StressOptions::DESIRED_EDGE_LENGTH) {
                        edge_guard
                            .get_property(StressOptions::DESIRED_EDGE_LENGTH)
                            .unwrap_or(desired_edge_length)
                    } else {
                        desired_edge_length
                    };
                    (other_id, edge_len)
                };

                if visited[other_id] {
                    continue;
                }
                let next = dist[position] + edge_len;
                if next < dist[other_id] {
                    dist[other_id] = next;
                    heap.push(State {
                        cost: next,
                        position: other_id,
                    });
                }
            }
        }
    }

    fn done(&self, count: i32, prev_stress: f64, cur_stress: f64) -> bool {
        prev_stress == 0.0
            || ((prev_stress - cur_stress) / prev_stress) < self.epsilon
            || count >= self.iteration_limit
    }

    fn compute_stress(&self, graph: &FGraph) -> f64 {
        let nodes = graph.nodes();
        let mut stress = 0.0;
        for i in 0..nodes.len() {
            let node_i = nodes.get(i).and_then(|node| node.lock().ok());
            let Some(node_i) = node_i else { continue };
            for j in (i + 1)..nodes.len() {
                let node_j = nodes.get(j).and_then(|node| node.lock().ok());
                let Some(node_j) = node_j else { continue };
                let euc_dist = node_i.position_ref().distance(node_j.position_ref());
                let euc_displacement = euc_dist - self.apsp[node_i.id()][node_j.id()];
                stress += self.w[node_i.id()][node_j.id()] * euc_displacement * euc_displacement;
            }
        }
        stress
    }

    fn compute_new_position(&self, graph: &FGraph, node: &FNodeRef) -> KVector {
        let nodes = graph.nodes();
        let mut weight_sum = 0.0;
        let mut x_disp = 0.0;
        let mut y_disp = 0.0;

        let Some(node_guard) = node.lock().ok() else {
            return KVector::new();
        };
        let u_id = node_guard.id();
        let u_pos = *node_guard.position_ref();

        for v in nodes {
            if Arc::ptr_eq(v, node) {
                continue;
            }
            let v_guard = v.lock().ok();
            let Some(v_guard) = v_guard else { continue };
            let v_id = v_guard.id();
            let v_pos = *v_guard.position_ref();

            let wij = self.w[u_id][v_id];
            weight_sum += wij;

            let euc_dist = u_pos.distance(&v_pos);
            if euc_dist > 0.0 && self.dim != Dimension::Y {
                x_disp += wij * (v_pos.x + self.apsp[u_id][v_id] * (u_pos.x - v_pos.x) / euc_dist);
            }

            if euc_dist > 0.0 && self.dim != Dimension::X {
                y_disp += wij * (v_pos.y + self.apsp[u_id][v_id] * (u_pos.y - v_pos.y) / euc_dist);
            }
        }

        match self.dim {
            Dimension::X => KVector::with_values(x_disp / weight_sum, u_pos.y),
            Dimension::Y => KVector::with_values(u_pos.x, y_disp / weight_sum),
            Dimension::XY => KVector::with_values(x_disp / weight_sum, y_disp / weight_sum),
        }
    }
}
