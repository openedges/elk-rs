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
            let graph_guard = graph.lock();
            graph_guard.nodes().clone()
        };

        self.eliminated = Vec::new();
        self.visited = vec![0; nodes.len()];

        for (id, node) in nodes.into_iter().enumerate() {
            {
                let mut node_guard = node.lock();
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
            let graph_guard = graph.lock();
            graph_guard.nodes().clone()
        };

        for node in nodes {
            let id = node.lock().id() as usize;
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
                let edge_guard = edge.lock();
                (edge_guard.source(), edge_guard.target())
            };
            if let Some(source) = source {
                {
                    let mut node_guard = source.lock();
                    node_guard.remove_outgoing(edge);
                }
            }
            if let Some(target) = target {
                {
                    let mut node_guard = target.lock();
                    node_guard.remove_incoming(edge);
                }
            }
        }

        {
            let mut graph_guard = graph.lock();
            graph_guard.set_property(
                InternalProperties::REMOVABLE_EDGES,
                Some(self.eliminated.clone()),
            );
        }
    }

    fn dfs(&mut self, node: &TNodeRef) {
        let id = node.lock().id() as usize;
        if let Some(slot) = self.visited.get_mut(id) {
            *slot = 1;
        }

        let outgoing = node
            .lock().outgoing_edges().clone();
        for edge in outgoing {
            let target = edge.lock().target();
            let Some(target) = target else {
                continue;
            };
            let target_id = target.lock().id() as usize;
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
            let id = node.lock().id() as usize;
            if let Some(slot) = self.visited.get_mut(id) {
                *slot = 1;
            }

            let outgoing = node
                .lock().outgoing_edges().clone();
            for edge in outgoing {
                let target = edge.lock().target();
                let Some(target) = target else {
                    continue;
                };
                let target_id = target.lock().id() as usize;
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
