use std::collections::HashMap;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::{TGraphRef, TNodeRef};
use crate::org::eclipse::elk::alg::mrtree::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::mrtree::options::{MrTreeOptions, InternalProperties};
use crate::org::eclipse::elk::alg::mrtree::tree_layout_phases::TreeLayoutPhases;

/// SoA (Struct-of-Arrays) representation of the tree for lock-free Reingold-Tilford.
/// All node data is pre-extracted into flat arrays indexed by BFS order.
struct TreeSoA {
    nodes: Vec<TNodeRef>,
    children: Vec<Vec<usize>>,
    parent: Vec<Option<usize>>,
    left_sibling: Vec<Option<usize>>,
    left_neighbor: Vec<Option<usize>>,
    right_sibling: Vec<Option<usize>>,
    is_leaf: Vec<bool>,
    /// Half of node width (or height for horizontal layouts) used in mean_node_width.
    half_width: Vec<f64>,
    /// Half of node size for centering offset (inlined NodePositionProcessor).
    half_size_x: Vec<f64>,
    half_size_y: Vec<f64>,
    levelheight: Vec<f64>,
    prelim: Vec<f64>,
    modifier: Vec<f64>,
}

impl TreeSoA {
    /// Build SoA from the tree rooted at `root`.
    /// Single-pass BFS: extracts sizes and children in one lock per node.
    /// Computes level heights and neighbor/sibling links from BFS level structure (zero locks).
    /// Replaces LevelHeightProcessor and NeighborsProcessor entirely.
    fn build(root: &TNodeRef, is_vertical: bool) -> Self {
        let mut nodes: Vec<TNodeRef> = Vec::new();
        let mut index_map: HashMap<usize, usize> = HashMap::new();
        let mut children_vec: Vec<Vec<usize>> = Vec::new();
        let mut parent_vec: Vec<Option<usize>> = Vec::new();
        let mut is_leaf_vec: Vec<bool> = Vec::new();
        let mut half_width_vec: Vec<f64> = Vec::new();
        let mut half_size_x_vec: Vec<f64> = Vec::new();
        let mut half_size_y_vec: Vec<f64> = Vec::new();
        let mut full_dim_vec: Vec<f64> = Vec::new(); // for level height computation

        // Seed root
        nodes.push(root.clone());
        index_map.insert(std::sync::Arc::as_ptr(root) as usize, 0);
        children_vec.push(Vec::new());
        parent_vec.push(None);
        is_leaf_vec.push(true);
        half_width_vec.push(0.0);
        half_size_x_vec.push(0.0);
        half_size_y_vec.push(0.0);
        full_dim_vec.push(0.0);

        // BFS with level tracking — one lock per node for children + size extraction
        let mut level_ranges: Vec<(usize, usize)> = Vec::new();
        let mut level_start = 0;
        let mut level_end = 1;

        while level_start < level_end {
            level_ranges.push((level_start, level_end));

            for idx in level_start..level_end {
                let node = nodes[idx].clone();
                let lock_result = node.lock();
                let (hw, hsx, hsy, fd, child_refs) = if let Ok(guard) = lock_result {
                    let size = guard.size_ref();
                    let hw = if is_vertical {
                        size.x / 2.0
                    } else {
                        size.y / 2.0
                    };
                    let fd = if is_vertical { size.y } else { size.x };
                    (hw, size.x / 2.0, size.y / 2.0, fd, guard.children())
                } else {
                    continue;
                };
                // guard dropped here — safe to push to vectors
                half_width_vec[idx] = hw;
                half_size_x_vec[idx] = hsx;
                half_size_y_vec[idx] = hsy;
                full_dim_vec[idx] = fd;
                is_leaf_vec[idx] = child_refs.is_empty();

                let mut child_indices = Vec::with_capacity(child_refs.len());
                for child in child_refs {
                    let key = std::sync::Arc::as_ptr(&child) as usize;
                    if let std::collections::hash_map::Entry::Vacant(e) =
                        index_map.entry(key)
                    {
                        let ci = nodes.len();
                        e.insert(ci);
                        nodes.push(child);
                        children_vec.push(Vec::new());
                        parent_vec.push(Some(idx));
                        is_leaf_vec.push(true);
                        half_width_vec.push(0.0);
                        half_size_x_vec.push(0.0);
                        half_size_y_vec.push(0.0);
                        full_dim_vec.push(0.0);
                        child_indices.push(ci);
                    }
                }
                children_vec[idx] = child_indices;
            }

            level_start = level_end;
            level_end = nodes.len();
        }

        let n = nodes.len();

        // Compute level heights and neighbor/sibling links from BFS structure (zero locks)
        let mut levelheight = vec![0.0; n];
        let mut left_sibling: Vec<Option<usize>> = vec![None; n];
        let mut left_neighbor: Vec<Option<usize>> = vec![None; n];
        let mut right_sibling: Vec<Option<usize>> = vec![None; n];

        for &(start, end) in &level_ranges {
            // Level height = max node dimension across the level
            let height = (start..end)
                .map(|i| full_dim_vec[i])
                .fold(0.0f64, f64::max);

            let mut prev: Option<usize> = None;
            for idx in start..end {
                levelheight[idx] = height;
                if let Some(p) = prev {
                    // Left neighbor = previous node at same level (always)
                    left_neighbor[idx] = Some(p);
                    // Siblings = consecutive nodes with same parent
                    if parent_vec[p] == parent_vec[idx] {
                        left_sibling[idx] = Some(p);
                        right_sibling[p] = Some(idx);
                    }
                }
                prev = Some(idx);
            }
        }

        TreeSoA {
            nodes,
            children: children_vec,
            parent: parent_vec,
            left_sibling,
            left_neighbor,
            right_sibling,
            is_leaf: is_leaf_vec,
            half_width: half_width_vec,
            half_size_x: half_size_x_vec,
            half_size_y: half_size_y_vec,
            levelheight,
            prelim: vec![0.0; n],
            modifier: vec![0.0; n],
        }
    }

    fn mean_node_width(&self, left: usize, right: usize) -> f64 {
        self.half_width[left] + self.half_width[right]
    }

    /// Get the leftmost descendant at a given depth from `start_children`.
    fn get_left_most(&self, start_children: &[usize], depth: i32) -> Option<usize> {
        if start_children.is_empty() {
            return None;
        }

        if depth > 1 {
            let mut next_level: Vec<usize> = Vec::new();
            for &idx in start_children {
                next_level.extend_from_slice(&self.children[idx]);
            }
            return self.get_left_most(&next_level, depth - 1);
        }

        if depth < 0 {
            let mut next_level: Vec<usize> = Vec::new();
            for &idx in start_children {
                next_level.extend_from_slice(&self.children[idx]);
            }
            if !next_level.is_empty() {
                return self.get_left_most(&next_level, depth);
            }
        }

        start_children.first().copied()
    }
}

pub struct NodePlacer {
    spacing: f64,
    x_top_adjustment: f64,
    y_top_adjustment: f64,
    direction: Direction,
}

impl Default for NodePlacer {
    fn default() -> Self {
        Self {
            spacing: 0.0,
            x_top_adjustment: 0.0,
            y_top_adjustment: 0.0,
            direction: Direction::Down,
        }
    }
}

impl ILayoutPhase<TreeLayoutPhases, TGraphRef> for NodePlacer {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Processor order nodes", 2.0);

        let (spacing, direction, root) = {
            let mut graph_guard = match graph.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    progress_monitor.done();
                    return;
                }
            };
            let spacing = graph_guard
                .get_property(MrTreeOptions::SPACING_NODE_NODE)
                .unwrap_or(0.0);
            let mut direction = graph_guard
                .get_property(MrTreeOptions::DIRECTION)
                .unwrap_or(Direction::Undefined);
            if direction == Direction::Undefined {
                direction = Direction::Down;
                graph_guard.set_property(MrTreeOptions::DIRECTION, Some(direction));
            }

            let root = graph_guard
                .nodes()
                .iter()
                .find(|node| {
                    node.lock()
                        .ok()
                        .and_then(|mut node_guard| {
                            node_guard.get_property(InternalProperties::ROOT)
                        })
                        .unwrap_or(false)
                })
                .cloned();
            (spacing, direction, root)
        };

        self.spacing = spacing;
        self.direction = direction;

        if let Some(root) = root {
            // Build SoA — single lock per node
            let mut soa = TreeSoA::build(&root, direction.is_vertical());

            // first_walk on flat arrays (zero locks)
            self.first_walk_soa(&mut soa, 0);
            progress_monitor.worked(1.0);

            // second_walk: computes final positions directly (inlines Direction + NodePosition)
            let level_height = soa.levelheight[0];
            self.second_walk_soa(
                &soa,
                0,
                self.y_top_adjustment - (level_height / 2.0),
                self.x_top_adjustment,
            );
            progress_monitor.worked(1.0);
        }

        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &TGraphRef,
    ) -> Option<LayoutProcessorConfiguration<TreeLayoutPhases, TGraphRef>> {
        let mut config = LayoutProcessorConfiguration::create();
        config
            .before(TreeLayoutPhases::P2NodeOrdering)
            .add(std::sync::Arc::new(IntermediateProcessorStrategy::RootProc));
            // LevelHeight, NeighborsProc — computed in TreeSoA::build()
            // DirectionProc, NodePositionProc — inlined into second_walk_soa()
        Some(config)
    }
}

impl NodePlacer {
    /// SoA first_walk: operates entirely on flat arrays, zero locks.
    fn first_walk_soa(&self, soa: &mut TreeSoA, idx: usize) {
        soa.modifier[idx] = 0.0;

        if soa.is_leaf[idx] {
            let prelim = if let Some(ls) = soa.left_sibling[idx] {
                soa.prelim[ls] + self.spacing + soa.mean_node_width(ls, idx)
            } else {
                0.0
            };
            soa.prelim[idx] = prelim;
            return;
        }

        let children = soa.children[idx].clone();
        for &child in &children {
            self.first_walk_soa(soa, child);
        }

        let first_child = children[0];
        let last_child = children[children.len() - 1];
        let mid_point = (soa.prelim[last_child] + soa.prelim[first_child]) / 2.0;

        if let Some(ls) = soa.left_sibling[idx] {
            let p = soa.prelim[ls] + self.spacing + soa.mean_node_width(ls, idx);
            soa.prelim[idx] = p;
            soa.modifier[idx] = p - mid_point;
            self.apportion_soa(soa, idx);
        } else {
            soa.prelim[idx] = mid_point;
        }
    }

    /// SoA apportion: operates entirely on flat arrays, zero locks.
    fn apportion_soa(&self, soa: &mut TreeSoA, node_idx: usize) {
        let children = soa.children[node_idx].clone();
        let mut leftmost = children.first().copied();
        let mut neighbor = leftmost.and_then(|lm| soa.left_neighbor[lm]);

        let mut compare_depth = 1;
        let mut apportion_iterations = 0usize;

        while let (Some(lm_idx), Some(nb_idx)) = (leftmost, neighbor) {
            apportion_iterations += 1;
            if apportion_iterations > 256 {
                return;
            }

            let mut left_mod_sum = 0.0;
            let mut right_mod_sum = 0.0;
            let mut ancestor_leftmost = lm_idx;
            let mut ancestor_neighbor = nb_idx;

            for _ in 0..compare_depth {
                let Some(alp) = soa.parent[ancestor_leftmost] else {
                    return;
                };
                let Some(anp) = soa.parent[ancestor_neighbor] else {
                    return;
                };
                ancestor_leftmost = alp;
                ancestor_neighbor = anp;
                right_mod_sum += soa.modifier[ancestor_leftmost];
                left_mod_sum += soa.modifier[ancestor_neighbor];
            }

            let mean = soa.mean_node_width(lm_idx, nb_idx);
            let mut move_distance = soa.prelim[nb_idx] + left_mod_sum + self.spacing + mean
                - soa.prelim[lm_idx]
                - right_mod_sum;

            if move_distance > 0.0 {
                // Count left siblings between node and ancestor_neighbor
                let mut current = node_idx;
                let mut left_siblings = 0usize;
                let mut found = false;
                let mut seen_count = 0usize;
                loop {
                    seen_count += 1;
                    if seen_count > soa.nodes.len() {
                        return;
                    }
                    if current == ancestor_neighbor {
                        found = true;
                        break;
                    }
                    left_siblings += 1;
                    match soa.left_sibling[current] {
                        Some(ls) => current = ls,
                        None => break,
                    }
                }

                if !found || left_siblings == 0 {
                    return;
                }

                let portion = move_distance / left_siblings as f64;
                current = node_idx;
                seen_count = 0;
                while current != ancestor_neighbor {
                    seen_count += 1;
                    if seen_count > soa.nodes.len() {
                        return;
                    }
                    soa.prelim[current] += move_distance;
                    soa.modifier[current] += move_distance;
                    move_distance -= portion;
                    match soa.left_sibling[current] {
                        Some(ls) => current = ls,
                        None => return,
                    }
                }
            }

            compare_depth += 1;
            if soa.is_leaf[lm_idx] {
                leftmost = soa.get_left_most(&children, compare_depth);
            } else {
                leftmost = soa.children[lm_idx].first().copied();
            }
            neighbor = leftmost.and_then(|lm| soa.left_neighbor[lm]);
        }
    }

    /// SoA second_walk: computes final node positions directly.
    /// Inlines DirectionProcessor + NodePositionProcessor — eliminates 2 full tree passes.
    fn second_walk_soa(&self, soa: &TreeSoA, idx: usize, y_coor: f64, mod_sum: f64) {
        let x_temp = soa.prelim[idx] + mod_sum;
        let y_temp = y_coor + soa.levelheight[idx] / 2.0;

        // Apply direction transform (inlined DirectionProcessor)
        let (xcoor, ycoor) = {
            let xr = x_temp.round() as i32;
            let yr = y_temp.round() as i32;
            match self.direction {
                Direction::Up => (xr, -yr),
                Direction::Right => (yr, xr),
                Direction::Left => (-yr, xr),
                _ => (xr, yr), // Down (default)
            }
        };

        // Convert to position with centering offset (inlined NodePositionProcessor)
        let pos_x = xcoor as f64 - soa.half_size_x[idx];
        let pos_y = ycoor as f64 - soa.half_size_y[idx];

        // Write final position directly — no intermediate XCOOR/YCOOR/PRELIM/MODIFIER
        if let Ok(mut guard) = soa.nodes[idx].lock() {
            let pos = guard.position();
            pos.x = pos_x;
            pos.y = pos_y;
        }

        // Recurse: first child, then right siblings
        if !soa.is_leaf[idx] {
            if let Some(&first_child) = soa.children[idx].first() {
                self.second_walk_soa(
                    soa,
                    first_child,
                    y_coor + soa.levelheight[idx] + self.spacing,
                    mod_sum + soa.modifier[idx],
                );
            }
        }

        if let Some(rs) = soa.right_sibling[idx] {
            self.second_walk_soa(soa, rs, y_coor, mod_sum);
        }
    }
}
