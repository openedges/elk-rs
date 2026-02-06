use std::collections::HashSet;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;

use crate::org::eclipse::elk::alg::mrtree::graph::{TEdgeRef, TGraphRef, TNodeRef};
use crate::org::eclipse::elk::alg::mrtree::options::{InternalProperties, MrTreeOptions};

pub struct TreeUtil;

impl TreeUtil {
    pub fn get_root(graph: &TGraphRef) -> Option<TNodeRef> {
        let nodes = graph.lock().ok().map(|g| g.nodes().clone()).unwrap_or_default();
        nodes.into_iter().find(|node| {
            node.lock()
                .ok()
                .and_then(|mut guard| guard.get_property(InternalProperties::ROOT))
                .unwrap_or(false)
        })
    }

    pub fn depth(root: &TNodeRef) -> i32 {
        let children = Self::get_children(root);
        if children.is_empty() {
            1
        } else {
            children
                .iter()
                .map(Self::depth)
                .max()
                .unwrap_or(0)
                + 1
        }
    }

    pub fn get_children(node: &TNodeRef) -> Vec<TNodeRef> {
        let mut seen: HashSet<usize> = HashSet::new();
        let mut children = Vec::new();
        let outgoing = node
            .lock()
            .ok()
            .map(|guard| guard.outgoing_edges().clone())
            .unwrap_or_default();
        for edge in outgoing {
            if let Some(target) = edge.lock().ok().and_then(|guard| guard.target()) {
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
        let parent = node.lock().ok().and_then(|guard| guard.parent());
        if let Some(parent) = parent {
            return Self::root_distance(&parent, root) + 1;
        }
        0
    }

    pub fn get_all_incoming_edges(node: &TNodeRef, graph: &TGraphRef) -> Vec<TEdgeRef> {
        let node_id = node
            .lock()
            .ok()
            .map(|guard| guard.id())
            .unwrap_or(-1);
        let edges = graph.lock().ok().map(|g| g.edges().clone()).unwrap_or_default();
        let mut seen: HashSet<usize> = HashSet::new();
        let mut result = Vec::new();
        for edge in edges {
            let (source, target) = match edge.lock().ok() {
                Some(guard) => (guard.source(), guard.target()),
                None => (None, None),
            };
            let (Some(source), Some(target)) = (source, target) else { continue };
            let target_id = target.lock().ok().map(|guard| guard.id()).unwrap_or(-1);
            if target_id != node_id {
                continue;
            }
            let source_level = source
                .lock()
                .ok()
                .and_then(|mut guard| guard.get_property(MrTreeOptions::TREE_LEVEL))
                .unwrap_or(0);
            let target_level = target
                .lock()
                .ok()
                .and_then(|mut guard| guard.get_property(MrTreeOptions::TREE_LEVEL))
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
            let a_pos = a
                .lock()
                .ok()
                .and_then(|guard| guard.source())
                .and_then(|node| node.lock().ok().map(|g| *g.position_ref()))
                .unwrap_or_default();
            let b_pos = b
                .lock()
                .ok()
                .and_then(|guard| guard.source())
                .and_then(|node| node.lock().ok().map(|g| *g.position_ref()))
                .unwrap_or_default();
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
        let node_id = node
            .lock()
            .ok()
            .map(|guard| guard.id())
            .unwrap_or(-1);
        let edges = graph.lock().ok().map(|g| g.edges().clone()).unwrap_or_default();
        let mut seen: HashSet<usize> = HashSet::new();
        let mut result = Vec::new();
        for edge in edges {
            let (source, target) = match edge.lock().ok() {
                Some(guard) => (guard.source(), guard.target()),
                None => (None, None),
            };
            let (Some(source), Some(target)) = (source, target) else { continue };
            let source_id = source.lock().ok().map(|guard| guard.id()).unwrap_or(-1);
            if source_id != node_id {
                continue;
            }
            let source_label = source.lock().ok().and_then(|guard| guard.label().map(|l| l.to_string()));
            if let Some(label) = source_label {
                if label == "SUPER_ROOT" {
                    continue;
                }
            }
            let source_level = source
                .lock()
                .ok()
                .and_then(|mut guard| guard.get_property(MrTreeOptions::TREE_LEVEL))
                .unwrap_or(0);
            let target_level = target
                .lock()
                .ok()
                .and_then(|mut guard| guard.get_property(MrTreeOptions::TREE_LEVEL))
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
            let a_pos = a
                .lock()
                .ok()
                .and_then(|guard| guard.target())
                .and_then(|node| node.lock().ok().map(|g| *g.position_ref()))
                .unwrap_or_default();
            let b_pos = b
                .lock()
                .ok()
                .and_then(|guard| guard.target())
                .and_then(|node| node.lock().ok().map(|g| *g.position_ref()))
                .unwrap_or_default();
            a_pos
                .x
                .partial_cmp(&b_pos.x)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        result
    }

    pub fn get_first_point(edge: &TEdgeRef) -> KVector {
        let edge_guard = match edge.lock() {
            Ok(guard) => guard,
            Err(_) => return KVector::new(),
        };
        if edge_guard.bend_points_ref().is_empty() {
            if let Some(target) = edge_guard.target() {
                return target
                    .lock()
                    .ok()
                    .map(|node| *node.position_ref())
                    .unwrap_or_default();
            }
            KVector::new()
        } else {
            edge_guard.bend_points_ref().get_first()
        }
    }

    pub fn get_last_point(edge: &TEdgeRef) -> KVector {
        let edge_guard = match edge.lock() {
            Ok(guard) => guard,
            Err(_) => return KVector::new(),
        };
        if edge_guard.bend_points_ref().is_empty() {
            if let Some(source) = edge_guard.source() {
                return source
                    .lock()
                    .ok()
                    .map(|node| *node.position_ref())
                    .unwrap_or_default();
            }
            KVector::new()
        } else {
            edge_guard.bend_points_ref().get_last()
        }
    }

    pub fn get_direction(graph: &TGraphRef) -> Direction {
        graph
            .lock()
            .ok()
            .and_then(|mut g| g.get_property(MrTreeOptions::DIRECTION))
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
        let size = node
            .lock()
            .ok()
            .map(|guard| *guard.size_ref())
            .unwrap_or_default();
        match direction {
            Direction::Left => -size.x / 2.0,
            Direction::Up => -size.y / 2.0,
            Direction::Right => size.x / 2.0,
            _ => size.y / 2.0,
        }
    }

    pub fn get_node_size_vector_in_direction(node: &TNodeRef, direction: Direction) -> KVector {
        let size = node
            .lock()
            .ok()
            .map(|guard| *guard.size_ref())
            .unwrap_or_default();
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
            let edge_guard = match edge.lock() {
                Ok(guard) => guard,
                Err(_) => return false,
            };
            let source_pos = edge_guard
                .source()
                .and_then(|node| node.lock().ok().map(|guard| *guard.position_ref()))
                .unwrap_or_default();
            let target_pos = edge_guard
                .target()
                .and_then(|node| node.lock().ok().map(|guard| *guard.position_ref()))
                .unwrap_or_default();
            (source_pos, target_pos)
        };
        let edge_vec = KVector::with_values(target_pos.x - source_pos.x, target_pos.y - source_pos.y);
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
            .ok()
            .map(|guard| guard.incoming_edges().clone())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| edge.lock().ok().and_then(|guard| guard.source()))
            .collect();
        if parents.is_empty() {
            return None;
        }
        let dir_vec = Self::get_direction_vector(direction);
        let mut best: Option<(f64, TNodeRef)> = None;
        for parent in parents {
            let center = parent
                .lock()
                .ok()
                .map(|guard| {
                    let pos = guard.position_ref();
                    let size = guard.size_ref();
                    KVector::with_values(pos.x + size.x / 2.0, pos.y + size.y / 2.0)
                })
                .unwrap_or_default();
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
                if let Ok(guard) = node.lock() {
                    next_level.extend(guard.children_copy());
                }
            }
            return Self::get_left_most(&next_level, depth - 1);
        }

        if depth < 0 {
            let mut next_level: Vec<TNodeRef> = Vec::new();
            for node in current_level {
                if let Ok(guard) = node.lock() {
                    next_level.extend(guard.children_copy());
                }
            }
            if !next_level.is_empty() {
                return Self::get_left_most(&next_level, depth);
            }
        }

        current_level.first().cloned()
    }
}
