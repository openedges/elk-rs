use std::cmp::Ordering;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;

use crate::org::eclipse::elk::alg::force::graph::{FEdgeRef, FGraph, FLabelRef, FNodeRef};
use crate::org::eclipse::elk::alg::force::options::{ForceOptions, InternalProperties};

pub struct ComponentsProcessor;

impl ComponentsProcessor {
    pub fn new() -> Self {
        ComponentsProcessor
    }

    pub fn split(&self, graph: FGraph) -> Vec<FGraph> {
        let separate = graph
            .get_property(ForceOptions::SEPARATE_CONNECTED_COMPONENTS)
            .unwrap_or(true);
        if !separate {
            return vec![graph];
        }

        let node_count = graph.nodes().len();
        let mut visited = vec![false; node_count];
        let incidence = Self::build_incidence_lists(&graph);

        let mut components = Vec::new();
        for node in graph.nodes() {
            let node_id = node.lock().id();
            if visited[node_id] {
                continue;
            }

            let mut component = FGraph::new();
            component.copy_properties(graph.properties());
            Self::dfs(node, None, &mut component, &mut visited, &incidence);
            components.push(component);
        }

        if components.len() > 1 {
            for comp in &components {
                let mut id = 0_usize;
                for node in comp.nodes() {
                    {
                        let mut node_guard = node.lock();
                        node_guard.set_id(id);
                        id += 1;
                    }
                }
            }
        }

        components
    }

    pub fn recombine(&self, mut components: Vec<FGraph>) -> FGraph {
        if components.len() == 1 {
            return components.remove(0);
        }
        if components.is_empty() {
            return FGraph::new();
        }

        for graph in &mut components {
            let mut priority = 0_i32;
            let mut min_x = f64::MAX;
            let mut min_y = f64::MAX;
            let mut max_x = f64::MIN;
            let mut max_y = f64::MIN;

            for node in graph.nodes() {
                {
                    let node_guard = node.lock();
                    priority += node_guard.get_property(ForceOptions::PRIORITY).unwrap_or(1);
                    let pos = node_guard.position_ref();
                    let size = node_guard.size_ref();
                    min_x = min_x.min(pos.x - size.x / 2.0);
                    min_y = min_y.min(pos.y - size.y / 2.0);
                    max_x = max_x.max(pos.x + size.x / 2.0);
                    max_y = max_y.max(pos.y + size.y / 2.0);
                }
            }

            graph.set_property(ForceOptions::PRIORITY, Some(priority));
            graph.set_property(
                InternalProperties::BB_UPLEFT,
                Some(KVector::with_values(min_x, min_y)),
            );
            graph.set_property(
                InternalProperties::BB_LOWRIGHT,
                Some(KVector::with_values(max_x, max_y)),
            );
        }

        components.sort_by(|graph1, graph2| {
            let prio1 = graph1
                .properties()
                .clone()
                .get_property(ForceOptions::PRIORITY)
                .unwrap_or(0);
            let prio2 = graph2
                .properties()
                .clone()
                .get_property(ForceOptions::PRIORITY)
                .unwrap_or(0);
            let prio = prio2.cmp(&prio1);
            if prio != Ordering::Equal {
                return prio;
            }

            let size1 = {
                let props = graph1.properties().clone();
                let up = props
                    .get_property(InternalProperties::BB_UPLEFT)
                    .unwrap_or_default();
                let low = props
                    .get_property(InternalProperties::BB_LOWRIGHT)
                    .unwrap_or_default();
                let mut size = KVector::from_vector(&low);
                size.sub(&up);
                size
            };
            let size2 = {
                let props = graph2.properties().clone();
                let up = props
                    .get_property(InternalProperties::BB_UPLEFT)
                    .unwrap_or_default();
                let low = props
                    .get_property(InternalProperties::BB_LOWRIGHT)
                    .unwrap_or_default();
                let mut size = KVector::from_vector(&low);
                size.sub(&up);
                size
            };
            let area1 = size1.x * size1.y;
            let area2 = size2.x * size2.y;
            area1.partial_cmp(&area2).unwrap_or(Ordering::Equal)
        });

        let mut result = FGraph::new();
        result.copy_properties(components[0].properties());

        let mut max_row_width: f64 = 0.0;
        let mut total_area: f64 = 0.0;
        for graph in &components {
            let size = Self::bounding_size(graph);
            max_row_width = max_row_width.max(size.x);
            total_area += size.x * size.y;
        }

        let aspect_ratio = result
            .get_property(ForceOptions::ASPECT_RATIO)
            .unwrap_or(1.0);
        max_row_width = max_row_width.max(total_area.sqrt() * aspect_ratio);
        let spacing = result
            .get_property(ForceOptions::SPACING_NODE_NODE)
            .unwrap_or(0.0);

        let mut xpos = 0.0;
        let mut ypos = 0.0;
        let mut highest_box = 0.0;

        for graph in components {
            let size = Self::bounding_size(&graph);
            if xpos + size.x > max_row_width {
                xpos = 0.0;
                ypos += highest_box + spacing;
                highest_box = 0.0;
            }

            Self::move_graph(&mut result, graph, xpos, ypos);
            highest_box = highest_box.max(size.y);
            xpos += size.x + spacing;
        }

        result
    }

    fn build_incidence_lists(graph: &FGraph) -> Vec<Vec<FEdgeRef>> {
        let n = graph.nodes().len();
        let mut incidence = vec![Vec::new(); n];

        for node in graph.nodes() {
            {
                let node_guard = node.lock();
                if node_guard.id() < n {
                    incidence[node_guard.id()] = Vec::new();
                }
            }
        }

        for edge in graph.edges() {
            let (source_id, target_id) = {
                let edge_guard = edge.lock();
                let source_id = edge_guard.source().map(|node| node.lock().id());
                let target_id = edge_guard.target().map(|node| node.lock().id());
                match (source_id, target_id) {
                    (Some(source_id), Some(target_id)) => (source_id, target_id),
                    _ => continue,
                }
            };
            if source_id < n {
                incidence[source_id].push(edge.clone());
            }
            if target_id < n {
                incidence[target_id].push(edge.clone());
            }
        }

        incidence
    }

    fn dfs(
        node: &FNodeRef,
        last: Option<&FNodeRef>,
        component: &mut FGraph,
        visited: &mut [bool],
        incidence: &[Vec<FEdgeRef>],
    ) {
        let node_id = node.lock().id();
        if visited[node_id] {
            return;
        }
        visited[node_id] = true;
        component.nodes_mut().push(node.clone());

        for edge in &incidence[node_id] {
            let (source, target, labels) = {
                let edge_guard = edge.lock();
                let source = edge_guard.source();
                let target = edge_guard.target();
                let labels: Vec<FLabelRef> = edge_guard.labels().to_vec();
                (source, target, labels)
            };

            if let Some(last) = last {
                let last_is_source = source
                    .as_ref()
                    .map(|n| Arc::ptr_eq(n, last))
                    .unwrap_or(false);
                let last_is_target = target
                    .as_ref()
                    .map(|n| Arc::ptr_eq(n, last))
                    .unwrap_or(false);
                if last_is_source || last_is_target {
                    continue;
                }
            }

            if let Some(source) = source.as_ref() {
                if !Arc::ptr_eq(source, node) {
                    Self::dfs(source, Some(node), component, visited, incidence);
                }
            }
            if let Some(target) = target.as_ref() {
                if !Arc::ptr_eq(target, node) {
                    Self::dfs(target, Some(node), component, visited, incidence);
                }
            }
            component.edges_mut().push(edge.clone());
            component.labels_mut().extend(labels);
        }
    }

    fn bounding_size(graph: &FGraph) -> KVector {
        let props = graph.properties().clone();
        let up = props
            .get_property(InternalProperties::BB_UPLEFT)
            .unwrap_or_default();
        let low = props
            .get_property(InternalProperties::BB_LOWRIGHT)
            .unwrap_or_default();
        let mut size = KVector::from_vector(&low);
        size.sub(&up);
        size
    }

    fn move_graph(dest: &mut FGraph, source: FGraph, offset_x: f64, offset_y: f64) {
        let offset = {
            let props = source.properties().clone();
            let up = props
                .get_property(InternalProperties::BB_UPLEFT)
                .unwrap_or_default();
            let mut vec = KVector::with_values(offset_x, offset_y);
            vec.sub(&up);
            vec
        };

        for node in source.nodes() {
            {
                let mut node_guard = node.lock();
                node_guard.position().add(&offset);
            }
            dest.nodes_mut().push(node.clone());
        }

        for edge in source.edges() {
            {
                let mut edge_guard = edge.lock();
                for bend in edge_guard.bendpoints_mut() {
                    {
                        let mut bend_guard = bend.lock();
                        bend_guard.position().add(&offset);
                    }
                }
            }
            dest.edges_mut().push(edge.clone());
        }

        for label in source.labels() {
            {
                let mut label_guard = label.lock();
                label_guard.position().add(&offset);
            }
            dest.labels_mut().push(label.clone());
        }
    }
}

impl Default for ComponentsProcessor {
    fn default() -> Self {
        Self::new()
    }
}
