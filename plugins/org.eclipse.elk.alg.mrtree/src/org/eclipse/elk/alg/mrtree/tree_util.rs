use std::collections::HashSet;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;

use crate::org::eclipse::elk::alg::mrtree::graph::{TEdgeRef, TGraphRef, TNodeRef};
use crate::org::eclipse::elk::alg::mrtree::options::{InternalProperties, MrTreeOptions};

pub struct TreeUtil;

impl TreeUtil {
    pub fn get_root(graph: &TGraphRef) -> Option<TNodeRef> {
        let nodes = graph.lock().nodes().clone();
        nodes.into_iter().find(|node| {
            node.lock()
                .get_property(InternalProperties::ROOT)
                .unwrap_or(false)
        })
    }

    pub fn depth(root: &TNodeRef) -> i32 {
        let children = Self::get_children(root);
        if children.is_empty() {
            1
        } else {
            children.iter().map(Self::depth).max().unwrap_or(0) + 1
        }
    }

    pub fn get_children(node: &TNodeRef) -> Vec<TNodeRef> {
        let mut seen: HashSet<usize> = HashSet::new();
        let mut children = Vec::new();
        let outgoing = node.lock().outgoing_edges().clone();
        for edge in outgoing {
            let target = edge.lock().target();
            if let Some(target) = target {
                let key = Arc::as_ptr(&target) as usize;
                if seen.insert(key) {
                    children.push(target);
                }
            }
        }
        children
    }

    pub fn root_distance(node: &TNodeRef, root: &TNodeRef) -> i32 {
        if Arc::ptr_eq(node, root) {
            return 0;
        }
        let parent = node.lock().parent();
        if let Some(parent) = parent {
            return Self::root_distance(&parent, root) + 1;
        }
        0
    }

    pub fn get_all_incoming_edges(node: &TNodeRef, graph: &TGraphRef) -> Vec<TEdgeRef> {
        let node_id = node.lock().id();
        let edges = graph.lock().edges().clone();
        let mut seen: HashSet<usize> = HashSet::new();
        let mut result = Vec::new();
        for edge in edges {
            let (source, target) = {
                let guard = edge.lock();
                (guard.source(), guard.target())
            };
            let (Some(source), Some(target)) = (source, target) else {
                continue;
            };
            let target_id = target.lock().id();
            if target_id != node_id {
                continue;
            }
            let source_level = source
                .lock()
                .get_property(MrTreeOptions::TREE_LEVEL)
                .unwrap_or(0);
            let target_level = target
                .lock()
                .get_property(MrTreeOptions::TREE_LEVEL)
                .unwrap_or(0);
            if source_level == target_level {
                continue;
            }
            let key = Arc::as_ptr(&edge) as usize;
            if seen.insert(key) {
                result.push(edge);
            }
        }
        result.sort_by(|a, b| {
            let a_pos = {
                let a_guard = a.lock();
                a_guard
                    .source()
                    .map(|node| *node.lock().position_ref())
                    .unwrap_or_default()
            };
            let b_pos = {
                let b_guard = b.lock();
                b_guard
                    .source()
                    .map(|node| *node.lock().position_ref())
                    .unwrap_or_default()
            };
            a_pos
                .x
                .partial_cmp(&b_pos.x)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        result
    }

    pub fn get_subtree(node: &TNodeRef) -> Vec<TNodeRef> {
        let children = Self::get_children(node);
        if children.is_empty() {
            return vec![node.clone()];
        }
        let mut nodes = Vec::new();
        for child in children {
            nodes.extend(Self::get_subtree(&child));
        }
        nodes.push(node.clone());
        nodes
    }

    pub fn get_all_outgoing_edges(node: &TNodeRef, graph: &TGraphRef) -> Vec<TEdgeRef> {
        let node_id = node.lock().id();
        let edges = graph.lock().edges().clone();
        let mut seen: HashSet<usize> = HashSet::new();
        let mut result = Vec::new();
        for edge in edges {
            let (source, target) = {
                let guard = edge.lock();
                (guard.source(), guard.target())
            };
            let (Some(source), Some(target)) = (source, target) else {
                continue;
            };
            let source_id = source.lock().id();
            if source_id != node_id {
                continue;
            }
            let source_label = source.lock().label().map(|l| l.to_string());
            if let Some(label) = source_label {
                if label == "SUPER_ROOT" {
                    continue;
                }
            }
            let source_level = source
                .lock()
                .get_property(MrTreeOptions::TREE_LEVEL)
                .unwrap_or(0);
            let target_level = target
                .lock()
                .get_property(MrTreeOptions::TREE_LEVEL)
                .unwrap_or(0);
            if source_level == target_level {
                continue;
            }
            let key = Arc::as_ptr(&edge) as usize;
            if seen.insert(key) {
                result.push(edge);
            }
        }
        result.sort_by(|a, b| {
            let a_pos = {
                let a_guard = a.lock();
                a_guard
                    .target()
                    .map(|node| *node.lock().position_ref())
                    .unwrap_or_default()
            };
            let b_pos = {
                let b_guard = b.lock();
                b_guard
                    .target()
                    .map(|node| *node.lock().position_ref())
                    .unwrap_or_default()
            };
            a_pos
                .x
                .partial_cmp(&b_pos.x)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        result
    }

    pub fn get_first_point(edge: &TEdgeRef) -> KVector {
        let edge_guard = edge.lock();
        if edge_guard.bend_points_ref().is_empty() {
            if let Some(target) = edge_guard.target() {
                return *target.lock().position_ref();
            }
            KVector::new()
        } else {
            edge_guard.bend_points_ref().get_first()
        }
    }

    pub fn get_last_point(edge: &TEdgeRef) -> KVector {
        let edge_guard = edge.lock();
        if edge_guard.bend_points_ref().is_empty() {
            if let Some(source) = edge_guard.source() {
                return *source.lock().position_ref();
            }
            KVector::new()
        } else {
            edge_guard.bend_points_ref().get_last()
        }
    }

    pub fn get_direction(graph: &TGraphRef) -> Direction {
        graph
            .lock()
            .get_property(MrTreeOptions::DIRECTION)
            .unwrap_or(Direction::Down)
    }

    pub fn get_direction_vector(direction: Direction) -> KVector {
        match direction {
            Direction::Up => KVector::with_values(0.0, -1.0),
            Direction::Right => KVector::with_values(1.0, 0.0),
            Direction::Left => KVector::with_values(-1.0, 0.0),
            _ => KVector::with_values(0.0, 1.0),
        }
    }

    pub fn get_node_size_in_direction(node: &TNodeRef, direction: Direction) -> f64 {
        let size = *node.lock().size_ref();
        match direction {
            Direction::Left => -size.x / 2.0,
            Direction::Up => -size.y / 2.0,
            Direction::Right => size.x / 2.0,
            _ => size.y / 2.0,
        }
    }

    pub fn get_node_size_vector_in_direction(node: &TNodeRef, direction: Direction) -> KVector {
        let size = *node.lock().size_ref();
        match direction {
            Direction::Left => KVector::with_values(-size.x / 2.0, 0.0),
            Direction::Up => KVector::with_values(0.0, -size.y / 2.0),
            Direction::Right => KVector::with_values(size.x / 2.0, 0.0),
            _ => KVector::with_values(0.0, size.y / 2.0),
        }
    }

    pub fn turn_right(direction: Direction) -> Direction {
        match direction {
            Direction::Left => Direction::Up,
            Direction::Up => Direction::Right,
            Direction::Right => Direction::Down,
            _ => Direction::Left,
        }
    }

    pub fn turn_left(direction: Direction) -> Direction {
        match direction {
            Direction::Right => Direction::Up,
            Direction::Up => Direction::Left,
            Direction::Left => Direction::Down,
            _ => Direction::Right,
        }
    }

    pub fn to_node_border(center: &mut KVector, next: &KVector, size: &KVector) {
        let wh = size.x / 2.0;
        let hh = size.y / 2.0;
        let absx = (next.x - center.x).abs();
        let absy = (next.y - center.y).abs();
        let mut xscale = 1.0;
        let mut yscale = 1.0;
        if absx > wh {
            xscale = wh / absx;
        }
        if absy > hh {
            yscale = hh / absy;
        }
        let scale = xscale.min(yscale);
        center.x += scale * (next.x - center.x);
        center.y += scale * (next.y - center.y);
    }

    pub fn is_cycle_inducing(edge: &TEdgeRef, graph: &TGraphRef) -> bool {
        let dir_vec = Self::get_direction_vector(Self::get_direction(graph));
        let (source_pos, target_pos) = {
            let edge_guard = edge.lock();
            let source_pos = edge_guard
                .source()
                .map(|node| *node.lock().position_ref())
                .unwrap_or_default();
            let target_pos = edge_guard
                .target()
                .map(|node| *node.lock().position_ref())
                .unwrap_or_default();
            (source_pos, target_pos)
        };
        let edge_vec =
            KVector::with_values(target_pos.x - source_pos.x, target_pos.y - source_pos.y);
        dir_vec.dot_product(&edge_vec) <= 0.0
    }

    pub fn get_unique_long(a: i32, b: i32) -> u64 {
        ((a as u64) << 32) | (b as u32 as u64)
    }

    pub fn get_lowest_parent(
        node: &TNodeRef,
        _graph: &TGraphRef,
        direction: Direction,
    ) -> Option<TNodeRef> {
        let parents: Vec<TNodeRef> = node
            .lock()
            .incoming_edges()
            .clone()
            .into_iter()
            .filter_map(|edge| edge.lock().source())
            .collect();
        if parents.is_empty() {
            return None;
        }
        let dir_vec = Self::get_direction_vector(direction);
        let mut best: Option<(f64, TNodeRef)> = None;
        for parent in parents {
            let center = {
                let guard = parent.lock();
                let pos = guard.position_ref();
                let size = guard.size_ref();
                KVector::with_values(pos.x + size.x / 2.0, pos.y + size.y / 2.0)
            };
            let score = center.dot_product(&dir_vec);
            let replace = match &best {
                Some((best_score, _)) => score > *best_score,
                None => true,
            };
            if replace {
                best = Some((score, parent));
            }
        }
        best.map(|(_, parent)| parent)
    }

    pub fn get_left_most(current_level: &[TNodeRef], depth: i32) -> Option<TNodeRef> {
        if current_level.is_empty() {
            return None;
        }

        if depth > 1 {
            let mut next_level: Vec<TNodeRef> = Vec::new();
            for node in current_level {
                let guard = node.lock();
                next_level.extend(guard.children_copy());
            }
            return Self::get_left_most(&next_level, depth - 1);
        }

        if depth < 0 {
            let mut next_level: Vec<TNodeRef> = Vec::new();
            for node in current_level {
                let guard = node.lock();
                next_level.extend(guard.children_copy());
            }
            if !next_level.is_empty() {
                return Self::get_left_most(&next_level, depth);
            }
        }

        current_level.first().cloned()
    }
}
