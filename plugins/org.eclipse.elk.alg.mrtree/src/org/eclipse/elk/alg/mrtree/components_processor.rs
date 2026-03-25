use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::{TEdgeRef, TGraph, TGraphRef};
use crate::org::eclipse::elk::alg::mrtree::intermediate::graph_bounds_processor::GraphBoundsProcessor;
use crate::org::eclipse::elk::alg::mrtree::options::{InternalProperties, MrTreeOptions};

pub struct ComponentsProcessor;

impl ComponentsProcessor {
    pub fn new() -> Self {
        ComponentsProcessor
    }

    pub fn split(&self, graph: &TGraphRef) -> Vec<TGraphRef> {
        let (separate, nodes, edges, properties) = {
            let graph_guard = graph.lock();
            let separate = graph_guard
                .get_property(MrTreeOptions::SEPARATE_CONNECTED_COMPONENTS)
                .unwrap_or(true);
            (
                separate,
                graph_guard.nodes().clone(),
                graph_guard.edges().clone(),
                graph_guard.properties().clone(),
            )
        };

        if !separate {
            return vec![graph.clone()];
        }

        let mut id_to_index: HashMap<i32, usize> = HashMap::new();
        for (idx, node) in nodes.iter().enumerate() {
            let id = {
                let guard = node.lock();
                guard.id()
            };
            id_to_index.insert(id, idx);
        }

        let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
        for edge in &edges {
            let (source, target) = {
                let guard = edge.lock();
                (guard.source(), guard.target())
            };
            let (Some(source), Some(target)) = (source, target) else {
                continue;
            };
            let source_id = { let guard = source.lock(); guard.id() };
            let target_id = { let guard = target.lock(); guard.id() };
            if let (Some(&s_idx), Some(&t_idx)) =
                (id_to_index.get(&source_id), id_to_index.get(&target_id))
            {
                adjacency[s_idx].push(t_idx);
                adjacency[t_idx].push(s_idx);
            }
        }

        let mut visited = vec![false; nodes.len()];
        let mut components: Vec<TGraphRef> = Vec::new();

        for start_idx in 0..nodes.len() {
            if visited[start_idx] {
                continue;
            }
            let mut queue = VecDeque::new();
            let mut comp_indices: Vec<usize> = Vec::new();
            visited[start_idx] = true;
            queue.push_back(start_idx);

            while let Some(idx) = queue.pop_front() {
                comp_indices.push(idx);
                for &neighbor in &adjacency[idx] {
                    if !visited[neighbor] {
                        visited[neighbor] = true;
                        queue.push_back(neighbor);
                    }
                }
            }

            let comp_graph = TGraph::new();
            {
                let mut comp_guard = comp_graph.lock();
                comp_guard.properties_mut().copy_properties(&properties);
                for &idx in &comp_indices {
                    comp_guard.nodes_mut().push(nodes[idx].clone());
                }
            }

            let comp_node_keys: HashSet<usize> = comp_indices
                .iter()
                .map(|&idx| Arc::as_ptr(&nodes[idx]) as usize)
                .collect();
            let mut comp_edges: Vec<TEdgeRef> = Vec::new();
            let mut seen_edges: HashSet<usize> = HashSet::new();
            for edge in &edges {
                let (source, target) = {
                    let guard = edge.lock();
                    (guard.source(), guard.target())
                };
                let (Some(source), Some(target)) = (source, target) else {
                    continue;
                };
                let source_key = Arc::as_ptr(&source) as usize;
                let target_key = Arc::as_ptr(&target) as usize;
                if comp_node_keys.contains(&source_key) && comp_node_keys.contains(&target_key) {
                    let edge_key = Arc::as_ptr(edge) as usize;
                    if seen_edges.insert(edge_key) {
                        comp_edges.push(edge.clone());
                    }
                }
            }

            {
                let mut comp_guard = comp_graph.lock();
                comp_guard.edges_mut().extend(comp_edges);
            }

            components.push(comp_graph);
        }

        if components.len() > 1 {
            for comp in &components {
                let comp_guard = comp.lock();
                for (next_id, node) in comp_guard.nodes().iter().enumerate() {
                    let mut node_guard = node.lock();
                    node_guard.set_id(next_id as i32);
                }
            }
        }

        components
    }

    pub fn pack(&self, components: &[TGraphRef]) -> TGraphRef {
        if components.is_empty() {
            return TGraph::new();
        }
        if components.len() == 1 {
            self.apply_padding_and_normalize_positions(&components[0]);
            return components[0].clone();
        }

        for graph in components {
            let nodes = {
                let g = graph.lock();
                g.nodes().clone()
            };
            let mut priority = 0;
            let mut minx = f64::MAX;
            let mut miny = f64::MAX;
            let mut maxx = f64::MIN;
            let mut maxy = f64::MIN;
            for node in nodes {
                let node_guard = node.lock();
                priority += node_guard
                    .get_property(MrTreeOptions::PRIORITY)
                    .unwrap_or(0);
                let pos = node_guard.position_ref();
                let size = node_guard.size_ref();
                minx = minx.min(pos.x);
                miny = miny.min(pos.y);
                maxx = maxx.max(pos.x + size.x);
                maxy = maxy.max(pos.y + size.y);
            }
            {
                let mut graph_guard = graph.lock();
                graph_guard.set_property(MrTreeOptions::PRIORITY, Some(priority));
                graph_guard.set_property(
                    InternalProperties::BB_UPLEFT,
                    Some(KVector::with_values(minx, miny)),
                );
                graph_guard.set_property(
                    InternalProperties::BB_LOWRIGHT,
                    Some(KVector::with_values(maxx, maxy)),
                );
            }
        }

        let mut components_sorted: Vec<TGraphRef> = components.to_vec();
        components_sorted.sort_by(|a, b| {
            let (prio_a, size_a) = component_sort_values(a);
            let (prio_b, size_b) = component_sort_values(b);
            let prio_cmp = prio_b.cmp(&prio_a);
            if prio_cmp == std::cmp::Ordering::Equal {
                size_a
                    .partial_cmp(&size_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                prio_cmp
            }
        });

        let result = TGraph::new();
        {
            let mut result_guard = result.lock();
            let first_guard = components_sorted[0].lock();
            result_guard
                .properties_mut()
                .copy_properties(first_guard.properties());
        }

        let (max_row_width, spacing) = {
            let mut max_row_width: f64 = 0.0;
            let mut total_area: f64 = 0.0;
            for graph in &components_sorted {
                let graph_guard = graph.lock();
                let size = graph_guard
                    .get_property(InternalProperties::BB_LOWRIGHT)
                    .unwrap_or_default();
                let min = graph_guard
                    .get_property(InternalProperties::BB_UPLEFT)
                    .unwrap_or_default();
                let mut diff = size;
                diff.sub(&min);
                max_row_width = max_row_width.max(diff.x);
                total_area += diff.x * diff.y;
            }
            let aspect = {
                let g = result.lock();
                g.get_property(MrTreeOptions::ASPECT_RATIO).unwrap_or(1.0)
            };
            max_row_width = max_row_width.max(total_area.sqrt() * aspect);
            let spacing = {
                let g = result.lock();
                g.get_property(MrTreeOptions::SPACING_NODE_NODE).unwrap_or(0.0)
            };
            (max_row_width, spacing)
        };

        let mut xpos = 0.0;
        let mut ypos = 0.0;
        let mut highest_box = 0.0;
        for graph in &components_sorted {
            let (size, min) = {
                let graph_guard = graph.lock();
                let mut size = graph_guard
                    .get_property(InternalProperties::BB_LOWRIGHT)
                    .unwrap_or_default();
                let min = graph_guard
                    .get_property(InternalProperties::BB_UPLEFT)
                    .unwrap_or_default();
                size.sub(&min);
                (size, min)
            };
            if xpos + size.x > max_row_width {
                xpos = 0.0;
                ypos += highest_box + spacing;
                highest_box = 0.0;
            }
            self.move_graph(&result, graph, xpos - min.x, ypos - min.y);
            highest_box = highest_box.max(size.y);
            xpos += size.x + spacing;
        }

        let mut bounds_processor = GraphBoundsProcessor;
        let mut monitor = NullElkProgressMonitor;
        let mut result_ref = result.clone();
        bounds_processor.process(&mut result_ref, &mut monitor);
        self.apply_padding_and_normalize_positions(&result);

        result
    }

    fn apply_padding_and_normalize_positions(&self, graph: &TGraphRef) {
        let padding = {
            let g = graph.lock();
            g.get_property(MrTreeOptions::PADDING).unwrap_or_default()
        };

        {
            let mut graph_guard = graph.lock();
            graph_guard.set_property(
                InternalProperties::BB_UPLEFT,
                Some(KVector::with_values(0.0, 0.0)),
            );
        }

        let xmin = {
            let g = graph.lock();
            g.get_property(InternalProperties::GRAPH_XMIN).unwrap_or(0.0)
        };
        let ymin = {
            let g = graph.lock();
            g.get_property(InternalProperties::GRAPH_YMIN).unwrap_or(0.0)
        };

        let offset_x = padding.left - xmin;
        let offset_y = padding.top - ymin;
        self.move_graph(&TGraph::new(), graph, offset_x, offset_y);
    }

    fn move_graph(
        &self,
        dest_graph: &TGraphRef,
        source_graph: &TGraphRef,
        offsetx: f64,
        offsety: f64,
    ) {
        let (nodes, edges, source_min) = {
            let source_guard = source_graph.lock();
            let nodes = source_guard.nodes().clone();
            let edges = source_guard.edges().clone();
            let source_min = source_guard
                .get_property(InternalProperties::BB_UPLEFT)
                .unwrap_or_default();
            (nodes, edges, source_min)
        };

        let mut graph_offset = KVector::with_values(offsetx, offsety);
        graph_offset.sub(&source_min);

        {
            let mut dest_guard = dest_graph.lock();
            for node in &nodes {
                {
                    let mut node_guard = node.lock();
                    node_guard.position().add(&graph_offset);
                }
                dest_guard.nodes_mut().push(node.clone());
            }

            let mut seen_edges: HashSet<usize> = HashSet::new();
            for edge in edges {
                let edge_key = Arc::as_ptr(&edge) as usize;
                if !seen_edges.insert(edge_key) {
                    continue;
                }
                {
                    let mut edge_guard = edge.lock();
                    for bendpoint in edge_guard.bend_points().iter_mut() {
                        bendpoint.add(&graph_offset);
                    }
                }
                dest_guard.edges_mut().push(edge);
            }
        }
    }
}

impl Default for ComponentsProcessor {
    fn default() -> Self {
        Self::new()
    }
}

fn component_sort_values(graph: &TGraphRef) -> (i32, f64) {
    let guard = graph.lock();
    let priority = guard.get_property(MrTreeOptions::PRIORITY).unwrap_or(0);
    let mut size = guard
        .get_property(InternalProperties::BB_LOWRIGHT)
        .unwrap_or_default();
    let min = guard
        .get_property(InternalProperties::BB_UPLEFT)
        .unwrap_or_default();
    size.sub(&min);
    (priority, size.x * size.y)
}
