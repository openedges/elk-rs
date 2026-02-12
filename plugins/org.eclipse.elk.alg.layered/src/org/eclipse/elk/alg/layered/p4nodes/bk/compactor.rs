use std::collections::{BTreeMap, VecDeque};

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNodeRef};
use crate::org::eclipse::elk::alg::layered::options::{
    EdgeStraighteningStrategy, InternalProperties, LayeredOptions, Spacings,
};

use super::aligned_layout::{BKAlignedLayout, HDirection, VDirection};
use super::i_compactor::ICompactor;
use super::neighborhood_information::NeighborhoodInformation;
use super::threshold_strategy::{NullThresholdStrategy, SimpleThresholdStrategy, ThresholdStrategy};
use super::util::{node_id, node_margin_bottom, node_margin_top, node_size_y};

pub struct BKCompactor {
    spacings: Spacings,
    spacing_node_node: f64,
    threshold_strategy: Box<dyn ThresholdStrategy>,
    sink_nodes: BTreeMap<usize, ClassNode>,
}

impl BKCompactor {
    pub fn new(graph: &mut LGraph) -> Self {
        let spacings = graph
            .get_property(InternalProperties::SPACINGS)
            .unwrap_or_else(|| panic!("Missing spacings configuration for BK compactor"));
        let spacing_node_node = graph
            .get_property(LayeredOptions::SPACING_NODE_NODE)
            .unwrap_or(0.0);
        let edge_straightening = graph
            .get_property(LayeredOptions::NODE_PLACEMENT_BK_EDGE_STRAIGHTENING)
            .unwrap_or(EdgeStraighteningStrategy::ImproveStraightness);

        let threshold_strategy: Box<dyn ThresholdStrategy> = match edge_straightening {
            EdgeStraighteningStrategy::ImproveStraightness => {
                Box::new(SimpleThresholdStrategy::new())
            }
            EdgeStraighteningStrategy::None => Box::new(NullThresholdStrategy::new()),
        };

        BKCompactor {
            spacings,
            spacing_node_node,
            threshold_strategy,
            sink_nodes: BTreeMap::new(),
        }
    }

    fn place_block(
        &mut self,
        root_id: usize,
        bal: &mut BKAlignedLayout,
        ni: &NeighborhoodInformation,
    ) {
        if bal.y[root_id].is_some() {
            return;
        }

        let vdir = bal.vdir.expect("BK compactor requires a vertical direction");

        let mut is_initial_assignment = true;
        bal.y[root_id] = Some(0.0);

        let mut current = root_id;
        let mut thresh = match vdir {
            VDirection::Down => f64::NEG_INFINITY,
            VDirection::Up => f64::INFINITY,
        };

        let max_steps = bal.align.len().max(1);
        let mut steps = 0usize;
        loop {
            let current_node = bal.nodes_by_id[current].clone();
            let layer = current_node
                .lock()
                .ok()
                .and_then(|node_guard| node_guard.layer());
            let layer_nodes = layer
                .and_then(|layer| layer.lock().ok().map(|layer_guard| layer_guard.nodes().clone()))
                .unwrap_or_default();
            let current_index = *ni.node_index.get(current).unwrap_or(&0);
            let layer_size = layer_nodes.len();

            let has_neighbor = match vdir {
                VDirection::Down => current_index > 0,
                VDirection::Up => current_index + 1 < layer_size,
            };

            if has_neighbor {
                let neighbor = match vdir {
                    VDirection::Up => layer_nodes[current_index + 1].clone(),
                    VDirection::Down => layer_nodes[current_index - 1].clone(),
                };
                let neighbor_id = node_id(&neighbor);
                let neighbor_root = bal.root[neighbor_id];

                self.place_block(neighbor_root, bal, ni);

                thresh = self.threshold_strategy.calculate_threshold(bal, thresh, root_id, current);

                if bal.sink[root_id] == root_id {
                    bal.sink[root_id] = bal.sink[neighbor_root];
                }

                if bal.sink[root_id] == bal.sink[neighbor_root] {
                    let spacing = self.spacings.get_vertical_spacing(&current_node, &neighbor);

                    if vdir == VDirection::Up {
                        let new_position = bal.y[neighbor_root].unwrap_or(0.0)
                            + bal.inner_shift[neighbor_id]
                            - node_margin_top(&neighbor)
                            - spacing
                            - node_margin_bottom(&current_node)
                            - node_size_y(&current_node)
                            - bal.inner_shift[current];

                        let current_position = bal.y[root_id].unwrap_or(0.0);
                        let updated = if is_initial_assignment {
                            is_initial_assignment = false;
                            new_position.min(thresh)
                        } else {
                            current_position.min(new_position.min(thresh))
                        };
                        bal.y[root_id] = Some(updated);
                    } else {
                        let new_position = bal.y[neighbor_root].unwrap_or(0.0)
                            + bal.inner_shift[neighbor_id]
                            + node_size_y(&neighbor)
                            + node_margin_bottom(&neighbor)
                            + spacing
                            + node_margin_top(&current_node)
                            - bal.inner_shift[current];

                        let current_position = bal.y[root_id].unwrap_or(0.0);
                        let updated = if is_initial_assignment {
                            is_initial_assignment = false;
                            new_position.max(thresh)
                        } else {
                            current_position.max(new_position.max(thresh))
                        };
                        bal.y[root_id] = Some(updated);
                    }
                } else {
                    let sink_id = bal.sink[root_id];
                    let neighbor_sink_id = bal.sink[neighbor_root];

                    if vdir == VDirection::Up {
                        let required_space = bal.y[root_id].unwrap_or(0.0)
                            + bal.inner_shift[current]
                            + node_size_y(&current_node)
                            + node_margin_bottom(&current_node)
                            + self.spacing_node_node
                            - (bal.y[neighbor_root].unwrap_or(0.0)
                                + bal.inner_shift[neighbor_id]
                                - node_margin_top(&neighbor));
                        self.add_class_edge(sink_id, neighbor_sink_id, required_space);
                    } else {
                        let required_space = bal.y[root_id].unwrap_or(0.0)
                            + bal.inner_shift[current]
                            - node_margin_top(&current_node)
                            - bal.y[neighbor_root].unwrap_or(0.0)
                            - bal.inner_shift[neighbor_id]
                            - node_size_y(&neighbor)
                            - node_margin_bottom(&neighbor)
                            - self.spacing_node_node;
                        self.add_class_edge(sink_id, neighbor_sink_id, required_space);
                    }
                }
            } else {
                thresh = self.threshold_strategy.calculate_threshold(bal, thresh, root_id, current);
            }

            current = bal.align[current];
            if current == root_id || steps >= max_steps {
                break;
            }
            steps += 1;
        }

        self.threshold_strategy.finish_block(root_id);
    }

    fn add_class_edge(&mut self, source_id: usize, target_id: usize, separation: f64) {
        self.sink_nodes
            .entry(source_id)
            .or_insert_with(|| ClassNode::new(source_id));
        self.sink_nodes
            .entry(target_id)
            .or_insert_with(|| ClassNode::new(target_id));

        if let Some(target) = self.sink_nodes.get_mut(&target_id) {
            target.indegree += 1;
        }
        if let Some(source) = self.sink_nodes.get_mut(&source_id) {
            source.outgoing.push(ClassEdge {
                target: target_id,
                separation,
            });
        }
    }

    fn place_classes(&mut self, bal: &mut BKAlignedLayout, vdir: VDirection) {
        let mut sinks: VecDeque<usize> = self
            .sink_nodes
            .iter()
            .filter_map(|(&id, node)| if node.indegree == 0 { Some(id) } else { None })
            .collect();

        while let Some(node_id) = sinks.pop_front() {
            let class_shift = {
                let node = self.sink_nodes.get_mut(&node_id).expect("class node missing");
                if node.class_shift.is_none() {
                    node.class_shift = Some(0.0);
                }
                node.class_shift.unwrap_or(0.0)
            };

            let outgoing = self
                .sink_nodes
                .get(&node_id)
                .map(|node| node.outgoing.clone())
                .unwrap_or_default();
            for edge in outgoing {
                let target_id = edge.target;
                if let Some(target) = self.sink_nodes.get_mut(&target_id) {
                    let candidate = class_shift + edge.separation;
                    if target.class_shift.is_none() {
                        target.class_shift = Some(candidate);
                    } else if vdir == VDirection::Down {
                        target.class_shift = Some(target.class_shift.unwrap().min(candidate));
                    } else {
                        target.class_shift = Some(target.class_shift.unwrap().max(candidate));
                    }

                    if target.indegree > 0 {
                        target.indegree -= 1;
                    }
                    if target.indegree == 0 {
                        sinks.push_back(target_id);
                    }
                }
            }
        }

        for (&id, node) in &self.sink_nodes {
            if let Some(shift) = node.class_shift {
                bal.shift[id] = shift;
            }
        }
    }

    fn apply_final_coordinates(&self, bal: &mut BKAlignedLayout, vdir: VDirection, nodes: &[LNodeRef]) {
        for node in nodes {
            let node_idx = node_id(node);
            let root_id = bal.root[node_idx];
            let root_y = bal.y[root_id].unwrap_or(0.0);
            bal.y[node_idx] = Some(root_y);

            if node_idx == root_id {
                let sink_shift = bal.shift[bal.sink[node_idx]];
                let apply_shift = (vdir == VDirection::Up && sink_shift > f64::NEG_INFINITY)
                    || (vdir == VDirection::Down && sink_shift < f64::INFINITY);
                if apply_shift {
                    bal.y[node_idx] = Some(root_y + sink_shift);
                }
            }
        }
    }
}

impl ICompactor for BKCompactor {
    fn horizontal_compaction(&mut self, bal: &mut BKAlignedLayout, ni: &NeighborhoodInformation) {
        let vdir = bal.vdir.expect("BK compactor requires a vertical direction");
        let hdir = bal.hdir.expect("BK compactor requires a horizontal direction");

        for layer in &bal.layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let id = node_id(&node);
                bal.sink[id] = id;
                bal.shift[id] = if vdir == VDirection::Up {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                };
            }
        }
        self.sink_nodes.clear();

        let mut layers = bal.layers.clone();
        if hdir == HDirection::Left {
            layers.reverse();
        }

        self.threshold_strategy.init();
        for y in &mut bal.y {
            *y = None;
        }

        for layer in &layers {
            let mut nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            if vdir == VDirection::Up {
                nodes.reverse();
            }
            for node in nodes {
                let id = node_id(&node);
                if bal.root[id] == id {
                    self.place_block(id, bal, ni);
                }
            }
        }

        self.place_classes(bal, vdir);

        for layer in &layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            self.apply_final_coordinates(bal, vdir, &nodes);
        }

        self.threshold_strategy.post_process(bal, ni);
    }
}

#[derive(Clone)]
struct ClassNode {
    class_shift: Option<f64>,
    outgoing: Vec<ClassEdge>,
    indegree: usize,
}

impl ClassNode {
    fn new(_node_id: usize) -> Self {
        ClassNode {
            class_shift: None,
            outgoing: Vec::new(),
            indegree: 0,
        }
    }
}

#[derive(Clone)]
struct ClassEdge {
    target: usize,
    separation: f64,
}
