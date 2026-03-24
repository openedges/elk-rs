use std::cmp::Ordering;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;

use crate::org::eclipse::elk::alg::force::graph::{FEdgeId, FGraph, FNodeId};
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

        let node_count = graph.nodes.len();
        let mut visited = vec![false; node_count];
        let incidence = Self::build_incidence_lists(&graph);

        let mut components = Vec::new();
        for i in 0..node_count {
            let nid = graph.nodes[i];
            let node_id = graph.arena.node_id[nid.0];
            if visited[node_id] {
                continue;
            }

            let mut comp_node_ids = Vec::new();
            let mut comp_edge_ids = Vec::new();
            Self::dfs(
                nid,
                None,
                &graph,
                &mut comp_node_ids,
                &mut comp_edge_ids,
                &mut visited,
                &incidence,
            );

            // Build a sub-FGraph by copying arena data for this component
            let mut component = FGraph::new();
            component.copy_properties(graph.properties());

            // Map old node index -> new node index
            let mut node_remap: std::collections::HashMap<usize, FNodeId> = std::collections::HashMap::new();

            for &old_nid in &comp_node_ids {
                let new_nid = component.arena.add_node();
                component.arena.node_position[new_nid.0] = graph.arena.node_position[old_nid.0];
                component.arena.node_size[new_nid.0] = graph.arena.node_size[old_nid.0];
                component.arena.node_displacement[new_nid.0] = graph.arena.node_displacement[old_nid.0];
                component.arena.node_properties[new_nid.0].copy_properties(&graph.arena.node_properties[old_nid.0]);
                component.arena.node_id[new_nid.0] = graph.arena.node_id[old_nid.0];
                component.arena.node_label[new_nid.0] = graph.arena.node_label[old_nid.0].clone();
                // parent/children not used in force layout components
                component.nodes.push(new_nid);
                node_remap.insert(old_nid.0, new_nid);
            }

            for &old_eid in &comp_edge_ids {
                let new_eid = component.arena.add_edge();
                component.arena.edge_properties[new_eid.0].copy_properties(&graph.arena.edge_properties[old_eid.0]);

                // Remap source/target
                if let Some(old_src) = graph.arena.edge_source[old_eid.0] {
                    if let Some(&new_src) = node_remap.get(&old_src.0) {
                        component.arena.edge_source[new_eid.0] = Some(new_src);
                    }
                }
                if let Some(old_tgt) = graph.arena.edge_target[old_eid.0] {
                    if let Some(&new_tgt) = node_remap.get(&old_tgt.0) {
                        component.arena.edge_target[new_eid.0] = Some(new_tgt);
                    }
                }

                // Copy bendpoints
                for &old_bid in &graph.arena.edge_bendpoints[old_eid.0] {
                    let new_bid = component.arena.add_bendpoint(new_eid);
                    component.arena.bend_position[new_bid.0] = graph.arena.bend_position[old_bid.0];
                    component.arena.bend_size[new_bid.0] = graph.arena.bend_size[old_bid.0];
                    component.arena.bend_displacement[new_bid.0] = graph.arena.bend_displacement[old_bid.0];
                    component.arena.bend_properties[new_bid.0].copy_properties(&graph.arena.bend_properties[old_bid.0]);
                    component.bendpoints.push(new_bid);
                }

                // Copy labels
                for &old_lid in &graph.arena.edge_labels[old_eid.0] {
                    let new_lid = component.arena.add_label(new_eid);
                    component.arena.label_position[new_lid.0] = graph.arena.label_position[old_lid.0];
                    component.arena.label_size[new_lid.0] = graph.arena.label_size[old_lid.0];
                    component.arena.label_displacement[new_lid.0] = graph.arena.label_displacement[old_lid.0];
                    component.arena.label_properties[new_lid.0].copy_properties(&graph.arena.label_properties[old_lid.0]);
                    component.arena.label_text[new_lid.0] = graph.arena.label_text[old_lid.0].clone();
                    component.labels.push(new_lid);
                }

                component.edges.push(new_eid);
            }

            components.push(component);
        }

        if components.len() > 1 {
            for comp in &mut components {
                for (id, &nid) in comp.nodes.iter().enumerate() {
                    comp.arena.node_id[nid.0] = id;
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

            for &nid in &graph.nodes {
                priority += graph.arena.node_properties[nid.0]
                    .get_property(ForceOptions::PRIORITY)
                    .unwrap_or(1);
                let pos = &graph.arena.node_position[nid.0];
                let size = &graph.arena.node_size[nid.0];
                min_x = min_x.min(pos.x - size.x / 2.0);
                min_y = min_y.min(pos.y - size.y / 2.0);
                max_x = max_x.max(pos.x + size.x / 2.0);
                max_y = max_y.max(pos.y + size.y / 2.0);
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
                .properties
                .get_property(ForceOptions::PRIORITY)
                .unwrap_or(0);
            let prio2 = graph2
                .properties
                .get_property(ForceOptions::PRIORITY)
                .unwrap_or(0);
            let prio = prio2.cmp(&prio1);
            if prio != Ordering::Equal {
                return prio;
            }

            let size1 = {
                let up = graph1
                    .properties
                    .get_property(InternalProperties::BB_UPLEFT)
                    .unwrap_or_default();
                let low = graph1
                    .properties
                    .get_property(InternalProperties::BB_LOWRIGHT)
                    .unwrap_or_default();
                let mut size = KVector::from_vector(&low);
                size.sub(&up);
                size
            };
            let size2 = {
                let up = graph2
                    .properties
                    .get_property(InternalProperties::BB_UPLEFT)
                    .unwrap_or_default();
                let low = graph2
                    .properties
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

    fn build_incidence_lists(graph: &FGraph) -> Vec<Vec<FEdgeId>> {
        let n = graph.nodes.len();
        let mut incidence: Vec<Vec<FEdgeId>> = vec![Vec::new(); n];

        for &eid in &graph.edges {
            let source_id = graph.arena.edge_source[eid.0]
                .map(|nid| graph.arena.node_id[nid.0]);
            let target_id = graph.arena.edge_target[eid.0]
                .map(|nid| graph.arena.node_id[nid.0]);
            if let Some(sid) = source_id {
                if sid < n {
                    incidence[sid].push(eid);
                }
            }
            if let Some(tid) = target_id {
                if tid < n {
                    incidence[tid].push(eid);
                }
            }
        }

        incidence
    }

    fn dfs(
        nid: FNodeId,
        last: Option<FNodeId>,
        graph: &FGraph,
        comp_nodes: &mut Vec<FNodeId>,
        comp_edges: &mut Vec<FEdgeId>,
        visited: &mut [bool],
        incidence: &[Vec<FEdgeId>],
    ) {
        let node_id = graph.arena.node_id[nid.0];
        if visited[node_id] {
            return;
        }
        visited[node_id] = true;
        comp_nodes.push(nid);

        for &eid in &incidence[node_id] {
            let source_nid = graph.arena.edge_source[eid.0];
            let target_nid = graph.arena.edge_target[eid.0];

            if let Some(last_nid) = last {
                let last_is_source = source_nid.map(|n| n == last_nid).unwrap_or(false);
                let last_is_target = target_nid.map(|n| n == last_nid).unwrap_or(false);
                if last_is_source || last_is_target {
                    continue;
                }
            }

            if let Some(src) = source_nid {
                if src != nid {
                    Self::dfs(src, Some(nid), graph, comp_nodes, comp_edges, visited, incidence);
                }
            }
            if let Some(tgt) = target_nid {
                if tgt != nid {
                    Self::dfs(tgt, Some(nid), graph, comp_nodes, comp_edges, visited, incidence);
                }
            }
            comp_edges.push(eid);
        }
    }

    fn bounding_size(graph: &FGraph) -> KVector {
        let up = graph
            .properties
            .get_property(InternalProperties::BB_UPLEFT)
            .unwrap_or_default();
        let low = graph
            .properties
            .get_property(InternalProperties::BB_LOWRIGHT)
            .unwrap_or_default();
        let mut size = KVector::from_vector(&low);
        size.sub(&up);
        size
    }

    fn move_graph(dest: &mut FGraph, source: FGraph, offset_x: f64, offset_y: f64) {
        let offset = {
            let up = source
                .properties
                .get_property(InternalProperties::BB_UPLEFT)
                .unwrap_or_default();
            let mut vec = KVector::with_values(offset_x, offset_y);
            vec.sub(&up);
            vec
        };

        // Copy nodes from source into dest arena with offset
        let mut node_remap: std::collections::HashMap<usize, FNodeId> = std::collections::HashMap::new();
        for &old_nid in &source.nodes {
            let new_nid = dest.arena.add_node();
            dest.arena.node_position[new_nid.0] = source.arena.node_position[old_nid.0];
            dest.arena.node_position[new_nid.0].add(&offset);
            dest.arena.node_size[new_nid.0] = source.arena.node_size[old_nid.0];
            dest.arena.node_displacement[new_nid.0] = source.arena.node_displacement[old_nid.0];
            dest.arena.node_properties[new_nid.0].copy_properties(&source.arena.node_properties[old_nid.0]);
            dest.arena.node_id[new_nid.0] = source.arena.node_id[old_nid.0];
            dest.arena.node_label[new_nid.0] = source.arena.node_label[old_nid.0].clone();
            dest.nodes.push(new_nid);
            node_remap.insert(old_nid.0, new_nid);
        }

        for &old_eid in &source.edges {
            let new_eid = dest.arena.add_edge();
            dest.arena.edge_properties[new_eid.0].copy_properties(&source.arena.edge_properties[old_eid.0]);

            if let Some(old_src) = source.arena.edge_source[old_eid.0] {
                if let Some(&new_src) = node_remap.get(&old_src.0) {
                    dest.arena.edge_source[new_eid.0] = Some(new_src);
                }
            }
            if let Some(old_tgt) = source.arena.edge_target[old_eid.0] {
                if let Some(&new_tgt) = node_remap.get(&old_tgt.0) {
                    dest.arena.edge_target[new_eid.0] = Some(new_tgt);
                }
            }

            for &old_bid in &source.arena.edge_bendpoints[old_eid.0] {
                let new_bid = dest.arena.add_bendpoint(new_eid);
                dest.arena.bend_position[new_bid.0] = source.arena.bend_position[old_bid.0];
                dest.arena.bend_position[new_bid.0].add(&offset);
                dest.arena.bend_size[new_bid.0] = source.arena.bend_size[old_bid.0];
                dest.arena.bend_displacement[new_bid.0] = source.arena.bend_displacement[old_bid.0];
                dest.arena.bend_properties[new_bid.0].copy_properties(&source.arena.bend_properties[old_bid.0]);
                dest.bendpoints.push(new_bid);
            }

            dest.edges.push(new_eid);
        }

        for &old_lid in &source.labels {
            // Find which edge this label belongs to in the source arena
            let old_eid = source.arena.label_edge[old_lid.0];
            // Find the corresponding new edge in dest
            // Labels are already associated with edges via edge_labels in add_label,
            // but we need to find the right new_eid
            let new_eid = if let Some(old_e) = old_eid {
                // Find the new edge id - source edges are in order so the index matches
                let edge_idx = source.edges.iter().position(|&e| e == old_e);
                edge_idx.map(|idx| dest.edges[dest.edges.len() - source.edges.len() + idx])
            } else {
                None
            };

            if let Some(new_eid) = new_eid {
                let new_lid = dest.arena.add_label(new_eid);
                dest.arena.label_position[new_lid.0] = source.arena.label_position[old_lid.0];
                dest.arena.label_position[new_lid.0].add(&offset);
                dest.arena.label_size[new_lid.0] = source.arena.label_size[old_lid.0];
                dest.arena.label_displacement[new_lid.0] = source.arena.label_displacement[old_lid.0];
                dest.arena.label_properties[new_lid.0].copy_properties(&source.arena.label_properties[old_lid.0]);
                dest.arena.label_text[new_lid.0] = source.arena.label_text[old_lid.0].clone();
                dest.labels.push(new_lid);
            }
        }
    }
}

impl Default for ComponentsProcessor {
    fn default() -> Self {
        Self::new()
    }
}
