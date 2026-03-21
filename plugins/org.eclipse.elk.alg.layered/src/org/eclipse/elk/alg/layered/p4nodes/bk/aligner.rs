use std::collections::HashSet;

use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

use crate::org::eclipse::elk::alg::layered::graph::NodeType;

use super::aligned_layout::{BKAlignedLayout, HDirection, VDirection};
use super::neighborhood_information::NeighborhoodInformation;
use super::util::get_blocks;
use super::util::{
    edge_between, edge_key, node_id, node_margin_bottom, node_margin_top, node_size_y, node_type,
    port_offset_y,
};

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
        let trace_align = ElkTrace::global().bk_align;
        for layer in &bal.layers {
            let nodes = layer
                .lock_ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let id = node_id(&node);
                bal.root[id] = id;
                bal.align[id] = id;
                bal.inner_shift[id] = 0.0;
            }
        }

        let hdir = bal
            .hdir
            .expect("BK aligner requires a horizontal direction");
        let vdir = bal.vdir.expect("BK aligner requires a vertical direction");

        let mut layers = bal.layers.clone();
        if hdir == HDirection::Left {
            layers.reverse();
        }

        for layer in layers {
            let mut r: isize = -1;
            let mut nodes = layer
                .lock_ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            if vdir == VDirection::Up {
                r = isize::MAX;
                nodes.reverse();
            }

            for node in nodes {
                let node_id_val = node_id(&node);
                if node_id_val >= bal.align.len() {
                    continue;
                }
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
                if trace_align {
                    let node_name = node
                        .lock_ok()
                        .map(|node_guard| node_guard.designation().to_string())
                        .unwrap_or_else(|| "<poisoned>".to_string());
                    eprintln!(
                        "bk-align: node={}({node_name}) neighbors={} low={} high={} vdir={vdir:?} hdir={hdir:?}",
                        node_id_val, d, low, high
                    );
                }

                if vdir == VDirection::Up {
                    for neighbor_pair in neighbors.iter().take(high + 1).skip(low).rev() {
                        if bal.align[node_id_val] == node_id_val {
                            let neighbor = &neighbor_pair.first;
                            let edge = &neighbor_pair.second;
                            let neighbor_id = node_id(neighbor);
                            if neighbor_id >= bal.align.len() {
                                continue;
                            }
                            let neighbor_index =
                                *ni.node_index.get(neighbor_id).unwrap_or(&0) as isize;
                            let edge_marked = marked_edges.contains(&edge_key(edge));
                            if trace_align {
                                let neighbor_name = neighbor
                                    .lock_ok()
                                    .map(|node_guard| node_guard.designation().to_string())
                                    .unwrap_or_else(|| "<poisoned>".to_string());
                                eprintln!(
                                    "bk-align: try node={} neighbor={}({neighbor_name}) idx={} r={} marked={edge_marked}",
                                    node_id_val, neighbor_id, neighbor_index, r
                                );
                            }
                            if !edge_marked && r > neighbor_index {
                                bal.align[neighbor_id] = node_id_val;
                                let root = bal.root[neighbor_id];
                                bal.root[node_id_val] = root;
                                bal.align[node_id_val] = root;
                                bal.od[root] =
                                    bal.od[root] && node_type(&node) == NodeType::LongEdge;
                                r = neighbor_index;
                                if trace_align {
                                    eprintln!(
                                        "bk-align: align-set node={} neighbor={} root={} new_r={}",
                                        node_id_val, neighbor_id, root, r
                                    );
                                }
                            }
                        }
                    }
                } else {
                    for neighbor_pair in neighbors.iter().take(high + 1).skip(low) {
                        if bal.align[node_id_val] == node_id_val {
                            let neighbor = &neighbor_pair.first;
                            let edge = &neighbor_pair.second;
                            let neighbor_id = node_id(neighbor);
                            if neighbor_id >= bal.align.len() {
                                continue;
                            }
                            let neighbor_index =
                                *ni.node_index.get(neighbor_id).unwrap_or(&0) as isize;
                            let edge_marked = marked_edges.contains(&edge_key(edge));
                            if trace_align {
                                let neighbor_name = neighbor
                                    .lock_ok()
                                    .map(|node_guard| node_guard.designation().to_string())
                                    .unwrap_or_else(|| "<poisoned>".to_string());
                                eprintln!(
                                    "bk-align: try node={} neighbor={}({neighbor_name}) idx={} r={} marked={edge_marked}",
                                    node_id_val, neighbor_id, neighbor_index, r
                                );
                            }
                            if !edge_marked && r < neighbor_index {
                                bal.align[neighbor_id] = node_id_val;
                                let root = bal.root[neighbor_id];
                                bal.root[node_id_val] = root;
                                bal.align[node_id_val] = root;
                                bal.od[root] =
                                    bal.od[root] && node_type(&node) == NodeType::LongEdge;
                                r = neighbor_index;
                                if trace_align {
                                    eprintln!(
                                        "bk-align: align-set node={} neighbor={} root={} new_r={}",
                                        node_id_val, neighbor_id, root, r
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn inside_block_shift(&self, bal: &mut BKAlignedLayout) {
        let blocks = get_blocks(bal);
        let hdir = bal
            .hdir
            .expect("BK aligner requires a horizontal direction");
        let trace_inner = ElkTrace::global().bk_inner;

        for (root_id, _block) in blocks {
            let root_node = bal.nodes_by_id[root_id].clone();
            let root_name = root_node
                .lock_ok()
                .map(|node_guard| node_guard.designation().to_string())
                .unwrap_or_else(|| "<poisoned>".to_string());
            if trace_inner {
                eprintln!("bk-inner: root={root_id}({root_name}) start hdir={hdir:?}");
            }

            let mut space_above = node_margin_top(&root_node);
            let mut space_below = node_size_y(&root_node) + node_margin_bottom(&root_node);
            bal.inner_shift[root_id] = 0.0;

            let mut current = root_id;
            let max_steps = bal.align.len().max(1);
            let mut steps = 0usize;
            loop {
                let next = bal.align[current];
                if next == root_id || steps >= max_steps {
                    if steps >= max_steps && ElkTrace::global().bk_guard {
                        eprintln!(
                            "bk-guard: inside_block_shift loop1 hit max_steps root_id={} current={} next={} max_steps={}",
                            root_id, current, next, max_steps
                        );
                    }
                    break;
                }

                let edge = edge_between(&bal.nodes_by_id[current], &bal.nodes_by_id[next])
                    .expect("edge between block nodes is missing");
                if trace_inner {
                    let current_id = node_id(&bal.nodes_by_id[current]);
                    let next_id = node_id(&bal.nodes_by_id[next]);
                    let candidate_count = bal.nodes_by_id[current]
                        .lock_ok()
                        .map(|node_guard| node_guard.connected_edges())
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|candidate| {
                            candidate
                                .lock_ok()
                                .map(|edge_guard| {
                                    let src = edge_guard.source().and_then(|port| {
                                        port.lock_ok().and_then(|port_guard| port_guard.node())
                                    });
                                    let tgt = edge_guard.target().and_then(|port| {
                                        port.lock_ok().and_then(|port_guard| port_guard.node())
                                    });
                                    (src, tgt)
                                })
                                .map(|(src, tgt)| {
                                    if let (Some(src), Some(tgt)) = (src, tgt) {
                                        let src_id = node_id(&src);
                                        let tgt_id = node_id(&tgt);
                                        (src_id == current_id && tgt_id == next_id)
                                            || (src_id == next_id && tgt_id == current_id)
                                    } else {
                                        false
                                    }
                                })
                                .unwrap_or(false)
                        })
                        .count();
                    if candidate_count > 1 {
                        eprintln!(
                            "bk-inner: root={root_id} multi-edge current={current_id} next={next_id} candidates={candidate_count}"
                        );
                    }
                }
                let (source_port, target_port) = edge
                    .lock_ok()
                    .and_then(|edge_guard| Some((edge_guard.source()?, edge_guard.target()?)))
                    .unwrap();

                let port_pos_diff = if hdir == HDirection::Left {
                    port_offset_y(&target_port) - port_offset_y(&source_port)
                } else {
                    port_offset_y(&source_port) - port_offset_y(&target_port)
                };

                let next_inner_shift = bal.inner_shift[current] + port_pos_diff;
                bal.inner_shift[next] = next_inner_shift;
                if trace_inner {
                    let current_name = bal.nodes_by_id[current]
                        .lock_ok()
                        .map(|node_guard| node_guard.designation().to_string())
                        .unwrap_or_else(|| "<poisoned>".to_string());
                    let next_name = bal.nodes_by_id[next]
                        .lock_ok()
                        .map(|node_guard| node_guard.designation().to_string())
                        .unwrap_or_else(|| "<poisoned>".to_string());
                    eprintln!(
                        "bk-inner: root={root_id} step current={current}({current_name}) next={next}({next_name}) port_diff={port_pos_diff:.3} next_inner={next_inner_shift:.3}"
                    );
                }

                let next_node = &bal.nodes_by_id[next];
                space_above = space_above.max(node_margin_top(next_node) - next_inner_shift);
                space_below = space_below
                    .max(next_inner_shift + node_size_y(next_node) + node_margin_bottom(next_node));

                current = next;
                steps += 1;
            }

            let mut current = root_id;
            let max_steps = bal.align.len().max(1);
            let mut steps = 0usize;
            loop {
                bal.inner_shift[current] += space_above;
                if trace_inner {
                    let current_name = bal.nodes_by_id[current]
                        .lock_ok()
                        .map(|node_guard| node_guard.designation().to_string())
                        .unwrap_or_else(|| "<poisoned>".to_string());
                    eprintln!(
                        "bk-inner: root={root_id} apply current={current}({current_name}) inner={:.3}",
                        bal.inner_shift[current]
                    );
                }
                current = bal.align[current];
                if current == root_id || steps >= max_steps {
                    if steps >= max_steps && ElkTrace::global().bk_guard {
                        eprintln!(
                            "bk-guard: inside_block_shift loop2 hit max_steps root_id={} current={} max_steps={}",
                            root_id, current, max_steps
                        );
                    }
                    break;
                }
                steps += 1;
            }

            bal.block_size[root_id] = space_above + space_below;
            if trace_inner {
                eprintln!(
                    "bk-inner: root={root_id} done space_above={space_above:.3} space_below={space_below:.3} block_size={:.3}",
                    bal.block_size[root_id]
                );
            }
        }
    }
}

impl Default for BKAligner {
    fn default() -> Self {
        Self::new()
    }
}
