use std::collections::HashSet;

use crate::org::eclipse::elk::alg::layered::graph::NodeType;

use super::aligned_layout::{BKAlignedLayout, HDirection, VDirection};
use super::neighborhood_information::NeighborhoodInformation;
use super::util::{edge_between, edge_key, node_id, node_margin_bottom, node_margin_top, node_size_y, node_type, port_offset_y};
use super::util::get_blocks;

pub struct BKAligner;

impl BKAligner {
    pub fn new() -> Self {
        BKAligner
    }

    pub fn vertical_alignment(
        &self,
        bal: &mut BKAlignedLayout,
        ni: &NeighborhoodInformation,
        marked_edges: &HashSet<usize>,
    ) {
        for layer in &bal.layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let id = node_id(&node);
                bal.root[id] = id;
                bal.align[id] = id;
                bal.inner_shift[id] = 0.0;
            }
        }

        let hdir = bal.hdir.expect("BK aligner requires a horizontal direction");
        let vdir = bal.vdir.expect("BK aligner requires a vertical direction");

        let mut layers = bal.layers.clone();
        if hdir == HDirection::Left {
            layers.reverse();
        }

        for layer in layers {
            let mut r: isize = -1;
            let mut nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            if vdir == VDirection::Up {
                r = isize::MAX;
                nodes.reverse();
            }

            for node in nodes {
                let node_id_val = node_id(&node);
                let neighbors = if hdir == HDirection::Left {
                    ni.right_neighbors
                        .get(node_id_val)
                        .cloned()
                        .unwrap_or_default()
                } else {
                    ni.left_neighbors
                        .get(node_id_val)
                        .cloned()
                        .unwrap_or_default()
                };

                if neighbors.is_empty() {
                    continue;
                }

                let d = neighbors.len();
                let low = (d - 1) / 2;
                let high = d / 2;

                if vdir == VDirection::Up {
                    for m in (low..=high).rev() {
                        if bal.align[node_id_val] == node_id_val {
                            let neighbor_pair = &neighbors[m];
                            let neighbor = &neighbor_pair.first;
                            let edge = &neighbor_pair.second;
                            let neighbor_id = node_id(neighbor);
                            let neighbor_index = *ni.node_index.get(neighbor_id).unwrap_or(&0) as isize;

                            if !marked_edges.contains(&edge_key(edge)) && r > neighbor_index {
                                bal.align[neighbor_id] = node_id_val;
                                let root = bal.root[neighbor_id];
                                bal.root[node_id_val] = root;
                                bal.align[node_id_val] = root;
                                bal.od[root] =
                                    bal.od[root] && node_type(&node) == NodeType::LongEdge;
                                r = neighbor_index;
                            }
                        }
                    }
                } else {
                    for m in low..=high {
                        if bal.align[node_id_val] == node_id_val {
                            let neighbor_pair = &neighbors[m];
                            let neighbor = &neighbor_pair.first;
                            let edge = &neighbor_pair.second;
                            let neighbor_id = node_id(neighbor);
                            let neighbor_index = *ni.node_index.get(neighbor_id).unwrap_or(&0) as isize;

                            if !marked_edges.contains(&edge_key(edge)) && r < neighbor_index {
                                bal.align[neighbor_id] = node_id_val;
                                let root = bal.root[neighbor_id];
                                bal.root[node_id_val] = root;
                                bal.align[node_id_val] = root;
                                bal.od[root] =
                                    bal.od[root] && node_type(&node) == NodeType::LongEdge;
                                r = neighbor_index;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn inside_block_shift(&self, bal: &mut BKAlignedLayout) {
        let blocks = get_blocks(bal);
        let hdir = bal.hdir.expect("BK aligner requires a horizontal direction");

        for (root_id, _block) in blocks {
            let root_node = bal.nodes_by_id[root_id].clone();

            let mut space_above = node_margin_top(&root_node);
            let mut space_below = node_size_y(&root_node) + node_margin_bottom(&root_node);
            bal.inner_shift[root_id] = 0.0;

            let mut current = root_id;
            let max_steps = bal.align.len().max(1);
            let mut steps = 0usize;
            loop {
                let next = bal.align[current];
                if next == root_id || steps >= max_steps {
                    break;
                }

                let edge = edge_between(&bal.nodes_by_id[current], &bal.nodes_by_id[next])
                    .expect("edge between block nodes is missing");
                let (source_port, target_port) = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| Some((edge_guard.source()?, edge_guard.target()?)))
                    .unwrap();

                let port_pos_diff = if hdir == HDirection::Left {
                    port_offset_y(&target_port) - port_offset_y(&source_port)
                } else {
                    port_offset_y(&source_port) - port_offset_y(&target_port)
                };

                let next_inner_shift = bal.inner_shift[current] + port_pos_diff;
                bal.inner_shift[next] = next_inner_shift;

                let next_node = &bal.nodes_by_id[next];
                space_above = space_above.max(node_margin_top(next_node) - next_inner_shift);
                space_below =
                    space_below.max(next_inner_shift + node_size_y(next_node) + node_margin_bottom(next_node));

                current = next;
                steps += 1;
            }

            let mut current = root_id;
            let max_steps = bal.align.len().max(1);
            let mut steps = 0usize;
            loop {
                bal.inner_shift[current] += space_above;
                current = bal.align[current];
                if current == root_id || steps >= max_steps {
                    break;
                }
                steps += 1;
            }

            bal.block_size[root_id] = space_above + space_below;
        }
    }
}
