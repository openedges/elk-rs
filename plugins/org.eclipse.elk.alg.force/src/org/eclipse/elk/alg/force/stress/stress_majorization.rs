use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::LazyLock;

static TRACE_STRESS: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_STRESS").is_some());




use crate::org::eclipse::elk::alg::force::graph::{FEdgeRef, FGraph};
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

        // Pre-extract edge connectivity into flat adjacency list for lock-free Dijkstra
        let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        for edge in graph.edges() {
            let (source_id, target_id, edge_len) = {
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
                let edge_len = if edge_guard.has_property(StressOptions::DESIRED_EDGE_LENGTH) {
                    edge_guard
                        .get_property(StressOptions::DESIRED_EDGE_LENGTH)
                        .unwrap_or(self.desired_edge_length)
                } else {
                    self.desired_edge_length
                };
                match (source_id, target_id) {
                    (Some(source_id), Some(target_id)) => (source_id, target_id, edge_len),
                    _ => continue,
                }
            };
            if source_id < n {
                adj[source_id].push((target_id, edge_len));
            }
            if target_id < n {
                adj[target_id].push((source_id, edge_len));
            }
        }

        // Also build connected_edges for backward compatibility (unused in optimized path)
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

        // APSP via Dijkstra using flat adjacency (zero locks)
        self.apsp = vec![vec![0.0; n]; n];
        for source_id in 0..n {
            Self::dijkstra_flat(&adj, n, source_id, &mut self.apsp[source_id]);
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

        if *TRACE_STRESS {
            let edge_count = graph.edges().len();
            let apsp01 = if n > 1 { self.apsp[0][1] } else { 0.0 };
            let w01 = if n > 1 { self.w[0][1] } else { 0.0 };
            eprintln!(
                "stress-majorization: nodes={} edges={} desired_edge_length={} apsp01={} w01={}",
                n, edge_count, self.desired_edge_length, apsp01, w01
            );
        }
    }

    /// SoA-optimized execute: pre-extracts node positions/ids/fixed flags into flat arrays,
    /// runs the O(n²) stress iteration with zero locks.
    pub fn execute(&mut self, graph: &mut FGraph) {
        let n = graph.nodes().len();
        if n <= 1 {
            return;
        }

        // Pre-extract node data into flat arrays (one lock per node)
        let nodes = graph.nodes();
        let mut pos_x = vec![0.0f64; n];
        let mut pos_y = vec![0.0f64; n];
        let mut node_ids = vec![0usize; n];
        let mut fixed = vec![false; n];

        for (i, node) in nodes.iter().enumerate() {
            if let Ok(mut node_guard) = node.lock() {
                pos_x[i] = node_guard.position_ref().x;
                pos_y[i] = node_guard.position_ref().y;
                node_ids[i] = node_guard.id();
                fixed[i] = node_guard
                    .get_property(StressOptions::FIXED)
                    .unwrap_or(false);
            }
        }

        let mut count = 0;
        let mut prev_stress = self.compute_stress_soa(&pos_x, &pos_y, &node_ids, n);
        let mut cur_stress = f64::INFINITY;

        loop {
            if count > 0 {
                prev_stress = cur_stress;
            }

            for i in 0..n {
                if fixed[i] {
                    continue;
                }
                let (nx, ny) = self.compute_new_position_soa(
                    &pos_x, &pos_y, &node_ids, i, n,
                );
                pos_x[i] = nx;
                pos_y[i] = ny;
            }

            cur_stress = self.compute_stress_soa(&pos_x, &pos_y, &node_ids, n);

            if self.done(count, prev_stress, cur_stress) {
                if *TRACE_STRESS {
                    eprintln!(
                        "stress-majorization: iterations={} prev_stress={} cur_stress={}",
                        count, prev_stress, cur_stress
                    );
                }
                break;
            }
            count += 1;
        }

        // Write back positions to nodes
        for (i, node) in graph.nodes().iter().enumerate() {
            if let Ok(mut node_guard) = node.lock() {
                let pos = node_guard.position();
                pos.x = pos_x[i];
                pos.y = pos_y[i];
            }
        }
    }

    /// Lock-free Dijkstra using pre-computed flat adjacency list
    fn dijkstra_flat(
        adj: &[Vec<(usize, f64)>],
        n: usize,
        source_id: usize,
        dist: &mut [f64],
    ) {
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

            for &(other_id, edge_len) in &adj[position] {
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

    /// SoA compute_stress: uses flat position arrays, zero locks
    fn compute_stress_soa(
        &self,
        pos_x: &[f64],
        pos_y: &[f64],
        node_ids: &[usize],
        n: usize,
    ) -> f64 {
        let mut stress = 0.0;
        for i in 0..n {
            let ui = node_ids[i];
            for j in (i + 1)..n {
                let uj = node_ids[j];
                let dx = pos_x[i] - pos_x[j];
                let dy = pos_y[i] - pos_y[j];
                let euc_dist = (dx * dx + dy * dy).sqrt();
                let euc_displacement = euc_dist - self.apsp[ui][uj];
                stress += self.w[ui][uj] * euc_displacement * euc_displacement;
            }
        }
        stress
    }

    /// SoA compute_new_position: uses flat position arrays, zero locks
    fn compute_new_position_soa(
        &self,
        pos_x: &[f64],
        pos_y: &[f64],
        node_ids: &[usize],
        node_idx: usize,
        n: usize,
    ) -> (f64, f64) {
        let mut weight_sum = 0.0;
        let mut x_disp = 0.0;
        let mut y_disp = 0.0;
        let u_id = node_ids[node_idx];
        let ux = pos_x[node_idx];
        let uy = pos_y[node_idx];

        for j in 0..n {
            if j == node_idx {
                continue;
            }
            let v_id = node_ids[j];
            let vx = pos_x[j];
            let vy = pos_y[j];

            let wij = self.w[u_id][v_id];
            weight_sum += wij;

            let dx = ux - vx;
            let dy = uy - vy;
            let euc_dist = (dx * dx + dy * dy).sqrt();

            if euc_dist > 0.0 && self.dim != Dimension::Y {
                x_disp += wij * (vx + self.apsp[u_id][v_id] * dx / euc_dist);
            }

            if euc_dist > 0.0 && self.dim != Dimension::X {
                y_disp += wij * (vy + self.apsp[u_id][v_id] * dy / euc_dist);
            }
        }

        match self.dim {
            Dimension::X => (x_disp / weight_sum, uy),
            Dimension::Y => (ux, y_disp / weight_sum),
            Dimension::XY => (x_disp / weight_sum, y_disp / weight_sum),
        }
    }
}
