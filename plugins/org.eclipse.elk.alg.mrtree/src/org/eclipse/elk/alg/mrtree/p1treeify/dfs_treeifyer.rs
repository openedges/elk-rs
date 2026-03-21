use std::collections::VecDeque;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::{TEdgeRef, TGraphRef, TNodeRef};
use crate::org::eclipse::elk::alg::mrtree::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::mrtree::options::{
    InternalProperties, MrTreeOptions, TreeifyingOrder,
};
use crate::org::eclipse::elk::alg::mrtree::tree_layout_phases::TreeLayoutPhases;

#[derive(Default)]
pub struct DFSTreeifyer {
    visited: Vec<i32>,
    eliminated: Vec<TEdgeRef>,
}

impl ILayoutPhase<TreeLayoutPhases, TGraphRef> for DFSTreeifyer {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("DFS Treeifying phase", 1.0);

        self.init(graph);
        self.collect_edges(graph);

        self.eliminated.clear();
        self.visited.clear();
        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &TGraphRef,
    ) -> Option<LayoutProcessorConfiguration<TreeLayoutPhases, TGraphRef>> {
        let mut config = LayoutProcessorConfiguration::create();
        config.add_after(
            TreeLayoutPhases::P3NodePlacement,
            std::sync::Arc::new(IntermediateProcessorStrategy::DetreeifyingProc),
        );
        Some(config)
    }
}

impl DFSTreeifyer {
    fn init(&mut self, graph: &TGraphRef) {
        let nodes = {
            let graph_guard = match graph.lock_ok() {
            Some(guard) => guard,
            None => return,
            };
            graph_guard.nodes().clone()
        };

        self.eliminated = Vec::new();
        self.visited = vec![0; nodes.len()];

        for (id, node) in nodes.into_iter().enumerate() {
            if let Some(mut node_guard) = node.lock_ok() {
                node_guard.set_id(id as i32);
            }
        }
    }

    fn collect_edges(&mut self, graph: &TGraphRef) {
        let treeifying_order = graph
            .lock_ok()
            .and_then(|mut graph_guard| graph_guard.get_property(MrTreeOptions::SEARCH_ORDER))
            .unwrap_or(TreeifyingOrder::Dfs);

        let nodes = {
            let graph_guard = match graph.lock_ok() {
            Some(guard) => guard,
            None => return,
            };
            graph_guard.nodes().clone()
        };

        for node in nodes {
            let id = node.lock_ok().map(|n| n.id()).unwrap_or(0) as usize;
            if self.visited.get(id).copied().unwrap_or(0) == 0 {
                match treeifying_order {
                    TreeifyingOrder::Dfs => self.dfs(&node),
                    TreeifyingOrder::Bfs => self.bfs(&node),
                }
                if let Some(slot) = self.visited.get_mut(id) {
                    *slot = 2;
                }
            }
        }

        for edge in &self.eliminated {
            let (source, target) = {
                let edge_guard = match edge.lock_ok() {
            Some(guard) => guard,
            None => continue,
                };
                (edge_guard.source(), edge_guard.target())
            };
            if let Some(source) = source {
                if let Some(mut node_guard) = source.lock_ok() {
                    node_guard.remove_outgoing(edge);
                }
            }
            if let Some(target) = target {
                if let Some(mut node_guard) = target.lock_ok() {
                    node_guard.remove_incoming(edge);
                }
            }
        }

        if let Some(mut graph_guard) = graph.lock_ok() {
            graph_guard.set_property(
                InternalProperties::REMOVABLE_EDGES,
                Some(self.eliminated.clone()),
            );
        }
    }

    fn dfs(&mut self, node: &TNodeRef) {
        let id = node.lock_ok().map(|n| n.id()).unwrap_or(0) as usize;
        if let Some(slot) = self.visited.get_mut(id) {
            *slot = 1;
        }

        let outgoing = node
            .lock_ok()
            .map(|n| n.outgoing_edges().clone())
            .unwrap_or_default();
        for edge in outgoing {
            let target = edge.lock_ok().and_then(|e| e.target());
            let Some(target) = target else {
                continue;
            };
            let target_id = target.lock_ok().map(|n| n.id()).unwrap_or(0) as usize;
            let visited = *self.visited.get(target_id).unwrap_or(&0);
            if visited == 1 {
                self.eliminated.push(edge.clone());
            } else if visited == 2 {
                if let Some(slot) = self.visited.get_mut(target_id) {
                    *slot = 1;
                }
            } else {
                self.dfs(&target);
            }
        }
    }

    fn bfs(&mut self, start_node: &TNodeRef) {
        let mut queue: VecDeque<TNodeRef> = VecDeque::new();
        queue.push_back(start_node.clone());

        while let Some(node) = queue.pop_front() {
            let id = node.lock_ok().map(|n| n.id()).unwrap_or(0) as usize;
            if let Some(slot) = self.visited.get_mut(id) {
                *slot = 1;
            }

            let outgoing = node
                .lock_ok()
                .map(|n| n.outgoing_edges().clone())
                .unwrap_or_default();
            for edge in outgoing {
                let target = edge.lock_ok().and_then(|e| e.target());
                let Some(target) = target else {
                    continue;
                };
                let target_id = target.lock_ok().map(|n| n.id()).unwrap_or(0) as usize;
                let visited = *self.visited.get(target_id).unwrap_or(&0);
                if visited == 1 {
                    self.eliminated.push(edge.clone());
                } else if visited == 2 {
                    if let Some(slot) = self.visited.get_mut(target_id) {
                        *slot = 1;
                    }
                } else {
                    queue.push_back(target);
                }
            }
        }
    }
}
