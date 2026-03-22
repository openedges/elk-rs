use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::{TEdgeRef, TGraphRef, TNodeRef};
use crate::org::eclipse::elk::alg::mrtree::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::mrtree::options::{
    InternalProperties, MrTreeOptions, OrderWeighting,
};
use crate::org::eclipse::elk::alg::mrtree::tree_layout_phases::TreeLayoutPhases;

pub struct NodeOrderer {
    weighting: OrderWeighting,
    debug: bool,
}

impl Default for NodeOrderer {
    fn default() -> Self {
        Self {
            weighting: OrderWeighting::ModelOrder,
            debug: false,
        }
    }
}

impl ILayoutPhase<TreeLayoutPhases, TGraphRef> for NodeOrderer {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Processor arrange node", 1.0);

        let (debug, weighting, root) = {
            let mut graph_guard = graph.lock();
            let debug = graph_guard
                .get_property(MrTreeOptions::DEBUG_MODE)
                .unwrap_or(false);
            let weighting = graph_guard
                .get_property(MrTreeOptions::WEIGHTING)
                .unwrap_or(OrderWeighting::ModelOrder);
            let root = graph_guard
                .nodes()
                .iter()
                .find(|node| {
                    let mut node_guard = node.lock();
                    node_guard.get_property(InternalProperties::ROOT).unwrap_or(false)
                })
                .cloned();
            (debug, weighting, root)
        };

        self.debug = debug;
        self.weighting = weighting;

        if let Some(root) = root {
            let mut level = vec![root];
            match self.weighting {
                OrderWeighting::Fan | OrderWeighting::Descendants => {
                    self.order_level_fan_descendants(&mut level, progress_monitor.sub_task(1.0));
                }
                OrderWeighting::Constraint => {
                    self.order_level_constraint(&mut level, progress_monitor.sub_task(1.0));
                }
                OrderWeighting::ModelOrder => {}
            }
        }

        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        graph: &TGraphRef,
    ) -> Option<LayoutProcessorConfiguration<TreeLayoutPhases, TGraphRef>> {
        let mut config = LayoutProcessorConfiguration::create();
        config
            .before(TreeLayoutPhases::P2NodeOrdering)
            .add(std::sync::Arc::new(IntermediateProcessorStrategy::RootProc));

        // FanProc only needed for Fan/Descendants/Constraint weighting (not ModelOrder)
        let weighting = {
            let mut g = graph.lock();
            g.get_property(MrTreeOptions::WEIGHTING).unwrap_or(OrderWeighting::ModelOrder)
        };
        if weighting != OrderWeighting::ModelOrder {
            config.add(std::sync::Arc::new(IntermediateProcessorStrategy::FanProc));
        }
        // LevelProc removed — InternalProperties::LEVEL is unused

        Some(config)
    }
}

impl NodeOrderer {
    fn order_level_fan_descendants(
        &self,
        current_level: &mut [TNodeRef],
        mut progress_monitor: Box<dyn IElkProgressMonitor>,
    ) {
        progress_monitor.begin("Processor arrange level", 1.0);

        let sort_property = match self.weighting {
            OrderWeighting::Descendants => InternalProperties::DESCENDANTS,
            _ => InternalProperties::FAN,
        };

        // Pre-extract sort keys to avoid locks in O(n log n) comparator
        let mut sort_keys: Vec<i32> = current_level
            .iter()
            .map(|n| {
                let mut g = n.lock();
                g.get_property(sort_property).unwrap_or(0)
            })
            .collect();
        {
            let keys = &sort_keys;
            let mut indices: Vec<usize> = (0..current_level.len()).collect();
            indices.sort_by(|&a, &b| keys[a].cmp(&keys[b]));
            let orig: Vec<_> = current_level.to_vec();
            let orig_keys = sort_keys.clone();
            for (i, &idx) in indices.iter().enumerate() {
                current_level[i] = orig[idx].clone();
                sort_keys[i] = orig_keys[idx];
            }
        }

        let mut first_occ = current_level.len();
        for val in sort_keys.iter().rev() {
            if *val == 0 {
                first_occ = first_occ.saturating_sub(1);
            } else {
                break;
            }
        }

        let inners = current_level[..first_occ].to_vec();
        let mut leaves = current_level[first_occ..].to_vec();

        let mut pos: i32 = 0;

        if inners.is_empty() {
            for leaf in leaves {
                {
                    let mut node_guard = leaf.lock();
                    node_guard.set_property(InternalProperties::POSITION, Some(pos));
                }
                pos += 1;
            }
        } else {
            let size = inners.len().max(1);
            for parent in &inners {
                {
                    let mut parent_guard = parent.lock();
                    parent_guard.set_property(InternalProperties::POSITION, Some(pos));
                }
                pos += 1;

                let mut children = parent
                    .lock().children_copy();
                self.order_level_fan_descendants(
                    &mut children,
                    progress_monitor.sub_task(1.0 / size as f32),
                );

                // Pre-extract position keys to avoid locks in comparator
                {
                    let pos_keys: Vec<i32> = children
                        .iter()
                        .map(|c| {
                            let mut g = c.lock();
                            g.get_property(InternalProperties::POSITION).unwrap_or(0)
                        })
                        .collect();
                    let mut indices: Vec<usize> = (0..children.len()).collect();
                    indices.sort_by(|&a, &b| pos_keys[b].cmp(&pos_keys[a]));
                    let orig: Vec<_> = children.clone();
                    for (i, &idx) in indices.iter().enumerate() {
                        children[i] = orig[idx].clone();
                    }
                }

                let sorted_edges = reorder_edges(parent, &children);
                {
                    let mut parent_guard = parent.lock();
                    parent_guard.replace_outgoing_edges(sorted_edges);
                }

                let mut fill_gap = {
                    let node_guard = parent.lock();
                    node_guard.outgoing_edges().len()
                };
                while fill_gap > 0 && !leaves.is_empty() {
                    if let Some(leaf) = leaves.pop() {
                        let val = {
                            let mut node_guard = leaf.lock();
                            node_guard.get_property(sort_property).unwrap_or(0)
                        };
                        if val == 0 {
                            {
                                let mut node_guard = leaf.lock();
                                node_guard.set_property(InternalProperties::POSITION, Some(pos));
                            }
                            pos += 1;
                            fill_gap -= 1;
                        } else {
                            leaves.push(leaf);
                            break;
                        }
                    }
                }
            }
        }

        progress_monitor.done();
    }

    fn order_level_constraint(
        &self,
        current_level: &mut [TNodeRef],
        mut progress_monitor: Box<dyn IElkProgressMonitor>,
    ) {
        progress_monitor.begin("Processor arrange level", 1.0);

        if self.debug {
            progress_monitor.log("OrderLevelConstraint!");
        }

        let len = current_level.len();
        let mut undefined_nodes: Vec<TNodeRef> = Vec::new();
        let mut in_bound_nodes: Vec<TNodeRef> = Vec::new();
        let mut out_of_bound_nodes: Vec<TNodeRef> = Vec::new();

        for node in current_level.iter() {
            let constraint = {
                let mut node_guard = node.lock();
                node_guard.get_property(MrTreeOptions::POSITION_CONSTRAINT).unwrap_or(-1)
            };
            if constraint < 0 {
                undefined_nodes.push(node.clone());
            } else if constraint < len as i32 {
                in_bound_nodes.push(node.clone());
            } else {
                out_of_bound_nodes.push(node.clone());
            }
        }

        let mut sorted_nodes: Vec<Option<TNodeRef>> = vec![None; len];

        let mut idx = 0;
        while idx < in_bound_nodes.len() {
            let node = in_bound_nodes[idx].clone();
            let target_pos = {
                let mut node_guard = node.lock();
                node_guard.get_property(MrTreeOptions::POSITION_CONSTRAINT).unwrap_or(-1)
            };
            if target_pos >= 0
                && target_pos < len as i32
                && sorted_nodes[target_pos as usize].is_none()
            {
                sorted_nodes[target_pos as usize] = Some(node);
                in_bound_nodes.remove(idx);
            } else {
                idx += 1;
            }
        }

        idx = 0;
        while idx < in_bound_nodes.len() {
            let node = in_bound_nodes[idx].clone();
            let target_pos = {
                let mut node_guard = node.lock();
                node_guard.get_property(MrTreeOptions::POSITION_CONSTRAINT).unwrap_or(-1)
            };
            let mut j = 0;
            loop {
                let new_target_pos = target_pos + j;
                if new_target_pos >= 0
                    && new_target_pos < len as i32
                    && sorted_nodes[new_target_pos as usize].is_none()
                {
                    sorted_nodes[new_target_pos as usize] = Some(node.clone());
                    in_bound_nodes.remove(idx);
                    break;
                }
                let new_target_pos = target_pos - j;
                if new_target_pos >= 0
                    && new_target_pos < len as i32
                    && sorted_nodes[new_target_pos as usize].is_none()
                {
                    sorted_nodes[new_target_pos as usize] = Some(node.clone());
                    in_bound_nodes.remove(idx);
                    break;
                }
                j += 1;
            }
        }

        // Pre-extract constraint keys to avoid locks in comparator
        {
            let keys: Vec<i32> = out_of_bound_nodes
                .iter()
                .map(|n| {
                    let mut g = n.lock();
                    g.get_property(MrTreeOptions::POSITION_CONSTRAINT).unwrap_or(0)
                })
                .collect();
            let mut indices: Vec<usize> = (0..out_of_bound_nodes.len()).collect();
            indices.sort_by(|&a, &b| keys[b].cmp(&keys[a]));
            let orig = out_of_bound_nodes.clone();
            for (i, &idx) in indices.iter().enumerate() {
                out_of_bound_nodes[i] = orig[idx].clone();
            }
        }

        for slot in sorted_nodes.iter_mut().rev() {
            if slot.is_none() && !out_of_bound_nodes.is_empty() {
                *slot = Some(out_of_bound_nodes.remove(0));
            }
        }

        for slot in sorted_nodes.iter_mut() {
            if slot.is_none() && !undefined_nodes.is_empty() {
                *slot = Some(undefined_nodes.remove(0));
            }
        }

        for (index, node) in sorted_nodes.iter().enumerate() {
            if let Some(node) = node {
                {
                    let mut node_guard = node.lock();
                    node_guard.set_property(InternalProperties::POSITION, Some(index as i32));
                }
            }
        }

        let inners: Vec<TNodeRef> = current_level
            .iter()
            .filter(|node| {
                let mut node_guard = node.lock();
                node_guard.get_property(InternalProperties::FAN).unwrap_or(0) != 0
            })
            .cloned()
            .collect();

        let size = inners.len().max(1);
        for parent in inners {
            let mut children = parent
                .lock().children_copy();
            self.order_level_constraint(
                &mut children,
                progress_monitor.sub_task(1.0 / size as f32),
            );
            // Pre-extract position keys to avoid locks in comparator
            {
                let pos_keys: Vec<i32> = children
                    .iter()
                    .map(|c| {
                        let mut g = c.lock();
                        g.get_property(InternalProperties::POSITION).unwrap_or(0)
                    })
                    .collect();
                let mut indices: Vec<usize> = (0..children.len()).collect();
                indices.sort_by(|&a, &b| pos_keys[a].cmp(&pos_keys[b]));
                let orig: Vec<_> = children.clone();
                for (i, &idx) in indices.iter().enumerate() {
                    children[i] = orig[idx].clone();
                }
            }

            let sorted_edges = reorder_edges(&parent, &children);
            {
                let mut parent_guard = parent.lock();
                parent_guard.replace_outgoing_edges(sorted_edges);
            }
        }

        progress_monitor.done();
    }
}

fn reorder_edges(parent: &TNodeRef, children: &[TNodeRef]) -> Vec<TEdgeRef> {
    let outgoing = parent
        .lock().outgoing_edges().clone();

    let mut result: Vec<TEdgeRef> = Vec::new();
    for child in children {
        for edge in &outgoing {
            if edge
                .lock().target()
                .map(|target| std::sync::Arc::ptr_eq(&target, child))
                .unwrap_or(false)
            {
                result.push(edge.clone());
            }
        }
    }
    result
}
