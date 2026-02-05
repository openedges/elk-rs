use std::cmp::Ordering;
use std::collections::HashSet;
use std::f64::consts::PI;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

use crate::org::eclipse::elk::alg::radial::internal_properties::InternalProperties;

pub struct RadialUtil;

impl RadialUtil {
    const TWO_PI: f64 = 2.0 * PI;
    const EPSILON: f64 = 1e-10;

    pub fn get_successors(node: &ElkNodeRef) -> Vec<ElkNodeRef> {
        let children: Vec<ElkNodeRef> = {
            let mut node_mut = node.borrow_mut();
            node_mut.children().iter().cloned().collect()
        };
        let mut child_keys = HashSet::new();
        for child in &children {
            child_keys.insert(node_key(child));
        }

        let mut successors = Vec::new();
        for outgoing_edge in ElkGraphUtil::all_outgoing_edges(node) {
            let source_shape = {
                let edge_borrow = outgoing_edge.borrow();
                edge_borrow.sources_ro().get(0)
            };
            if let Some(ElkConnectableShapeRef::Port(_)) = source_shape {
                continue;
            }
            let target_shape = {
                let edge_borrow = outgoing_edge.borrow();
                edge_borrow.targets_ro().get(0)
            };
            let Some(target_shape) = target_shape else { continue; };
            let Some(target) = ElkGraphUtil::connectable_shape_to_node(&target_shape) else {
                continue;
            };
            if !child_keys.contains(&node_key(&target)) {
                successors.push(target);
            }
        }
        successors
    }

    pub fn get_successor_set(node: &ElkNodeRef) -> Vec<ElkNodeRef> {
        let children: Vec<ElkNodeRef> = {
            let mut node_mut = node.borrow_mut();
            node_mut.children().iter().cloned().collect()
        };
        let mut child_keys = HashSet::new();
        for child in &children {
            child_keys.insert(node_key(child));
        }

        let mut successors = Vec::new();
        let mut seen = HashSet::new();
        for outgoing_edge in ElkGraphUtil::all_outgoing_edges(node) {
            let source_shape = {
                let edge_borrow = outgoing_edge.borrow();
                edge_borrow.sources_ro().get(0)
            };
            if let Some(ElkConnectableShapeRef::Port(_)) = source_shape {
                continue;
            }
            let target_shape = {
                let edge_borrow = outgoing_edge.borrow();
                edge_borrow.targets_ro().get(0)
            };
            let Some(target_shape) = target_shape else { continue; };
            let Some(target) = ElkGraphUtil::connectable_shape_to_node(&target_shape) else {
                continue;
            };
            let target_key = node_key(&target);
            if !child_keys.contains(&target_key) && seen.insert(target_key) {
                successors.push(target);
            }
        }
        successors
    }

    pub fn find_root(graph: &ElkNodeRef) -> Option<ElkNodeRef> {
        let children: Vec<ElkNodeRef> = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.children().iter().cloned().collect()
        };
        for child in children {
            let incoming = ElkGraphUtil::all_incoming_edges(&child);
            if incoming.is_empty() {
                return Some(child);
            }
        }
        None
    }

    pub fn root_from_graph(graph: &ElkNodeRef) -> Option<ElkNodeRef> {
        let root_id = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(InternalProperties::ROOT_NODE)
        };
        if let Some(root_id) = root_id {
            let children: Vec<ElkNodeRef> = {
                let mut graph_mut = graph.borrow_mut();
                graph_mut.children().iter().cloned().collect()
            };
            for child in children {
                if node_key(&child) == root_id {
                    return Some(child);
                }
            }
        }
        Self::find_root(graph)
    }

    pub fn find_root_of_node(node: &ElkNodeRef) -> ElkNodeRef {
        match Self::get_tree_parent(node) {
            Some(parent) => Self::find_root_of_node(&parent),
            None => node.clone(),
        }
    }

    pub fn get_number_of_leaves(node: &ElkNodeRef) -> f64 {
        let successors = Self::get_successors(node);
        if successors.is_empty() {
            1.0
        } else {
            successors
                .iter()
                .map(|child| Self::get_number_of_leaves(child))
                .sum()
        }
    }

    pub fn compare_polar(
        node1: &ElkNodeRef,
        node2: &ElkNodeRef,
        radial_offset: f64,
        node_offset_y: f64,
    ) -> Ordering {
        let arc1 = Self::polar_arc(node1, radial_offset, node_offset_y);
        let arc2 = Self::polar_arc(node2, radial_offset, node_offset_y);
        fuzzy_compare(arc1, arc2, Self::EPSILON)
    }

    pub fn find_largest_node_in_graph(graph: &ElkNodeRef) -> f64 {
        let children: Vec<ElkNodeRef> = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.children().iter().cloned().collect()
        };
        let mut largest_child_size: f64 = 0.0;
        for child in children {
            let (width, height) = {
                let mut child_mut = child.borrow_mut();
                let shape = child_mut.connectable().shape();
                (shape.width(), shape.height())
            };
            let diameter = (width * width + height * height).sqrt();
            largest_child_size = largest_child_size.max(diameter);

            let largest_child = Self::find_largest_node_in_graph(&child);
            largest_child_size = largest_child_size.max(largest_child);
        }
        largest_child_size
    }

    pub fn get_next_level_nodes(nodes: &[ElkNodeRef]) -> Vec<ElkNodeRef> {
        let mut successors = Vec::new();
        for node in nodes {
            let next_level_nodes = Self::get_successors(node);
            successors.extend(next_level_nodes);
        }
        successors
    }

    pub fn get_next_level_node_set(nodes: &[ElkNodeRef]) -> Vec<ElkNodeRef> {
        let mut successors = Vec::new();
        let mut seen = HashSet::new();
        for node in nodes {
            let next_level = Self::get_successor_set(node);
            for next in next_level {
                let key = node_key(&next);
                if seen.insert(key) {
                    successors.push(next);
                }
            }
        }
        successors
    }

    pub fn get_last_level_nodes(nodes: &[ElkNodeRef]) -> Vec<ElkNodeRef> {
        let mut parents = Vec::new();
        for node in nodes {
            if let Some(parent) = Self::get_tree_parent(node) {
                parents.push(parent);
            }
        }
        parents
    }

    pub fn center_nodes_on_radi(node: &ElkNodeRef, x_pos: f64, y_pos: f64) {
        let (width, height) = {
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            (shape.width(), shape.height())
        };
        let mut node_mut = node.borrow_mut();
        let shape = node_mut.connectable().shape();
        shape.set_location(x_pos - width / 2.0, y_pos - height / 2.0);
    }

    pub fn shift_closest_edge_to_radi(node: &ElkNodeRef, x_pos: f64, y_pos: f64) {
        if fuzzy_equals(x_pos, 0.0, Self::EPSILON) && fuzzy_equals(y_pos, 0.0, Self::EPSILON) {
            let (width, height) = {
                let mut node_mut = node.borrow_mut();
                let shape = node_mut.connectable().shape();
                (shape.width(), shape.height())
            };
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            shape.set_location(x_pos - width / 2.0, y_pos - height / 2.0);
            return;
        }

        let (width, height) = {
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            (shape.width(), shape.height())
        };

        let mut node_mut = node.borrow_mut();
        let shape = node_mut.connectable().shape();

        if x_pos < 0.0 {
            if y_pos < 0.0 {
                shape.set_location(x_pos - width, y_pos);
            } else {
                shape.set_location(x_pos - width, y_pos + height);
            }
        } else if y_pos < 0.0 {
            shape.set_location(x_pos, y_pos);
        } else {
            shape.set_location(x_pos, y_pos + height);
        }
    }

    pub fn get_tree_parent(node: &ElkNodeRef) -> Option<ElkNodeRef> {
        let incoming_edges = ElkGraphUtil::all_incoming_edges(node);
        let edge = incoming_edges.first()?;
        let source = {
            let edge_borrow = edge.borrow();
            edge_borrow.sources_ro().get(0)
        }?;
        ElkGraphUtil::connectable_shape_to_node(&source)
    }

    fn polar_arc(node: &ElkNodeRef, radial_offset: f64, node_offset_y: f64) -> f64 {
        let position = {
            let mut node_mut = node.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::POSITION)
        }
        .unwrap_or_else(KVector::new);

        let x_pos = position.x;
        let y_pos = position.y + node_offset_y;
        let mut arc = y_pos.atan2(x_pos);
        if arc < 0.0 {
            arc += Self::TWO_PI;
        }
        arc += radial_offset;
        if arc > Self::TWO_PI {
            arc -= Self::TWO_PI;
        }
        arc
    }
}

fn fuzzy_compare(a: f64, b: f64, epsilon: f64) -> Ordering {
    if (a - b).abs() <= epsilon {
        Ordering::Equal
    } else if a < b {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

fn fuzzy_equals(a: f64, b: f64, epsilon: f64) -> bool {
    (a - b).abs() <= epsilon
}

fn node_key(node: &ElkNodeRef) -> usize {
    Rc::as_ptr(node) as usize
}
