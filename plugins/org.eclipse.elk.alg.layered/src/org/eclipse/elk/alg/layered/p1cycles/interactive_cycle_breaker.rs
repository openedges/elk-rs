use std::sync::Arc;
use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LEdgeRef, LGraph, LNodeRef};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

static INTERMEDIATE_PROCESSING_CONFIGURATION: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config.add_before(
        LayeredPhases::P1CycleBreaking,
        Arc::new(IntermediateProcessorStrategy::InteractiveExternalPortPositioner),
    );
    config.add_after(
        LayeredPhases::P5EdgeRouting,
        Arc::new(IntermediateProcessorStrategy::ReversedEdgeRestorer),
    );
    config
});

pub struct InteractiveCycleBreaker;

impl InteractiveCycleBreaker {
    pub fn new() -> Self {
        InteractiveCycleBreaker
    }

    fn find_cycles(start: &LNodeRef, rev_edges: &mut Vec<LEdgeRef>) {
        // Iterative DFS matching Java's recursive approach.
        // Stack stores: (node, cached outgoing edges, edge_index)
        // Edges are cached once when a node is pushed to avoid repeated mutex locks.
        let mut stack: Vec<(LNodeRef, Vec<(LEdgeRef, LNodeRef)>, usize)> = Vec::new();

        let start_edges = Self::get_outgoing(start);
        if let Ok(mut guard) = start.lock() {
            guard.shape().graph_element().id = -1;
        }
        stack.push((start.clone(), start_edges, 0));

        while let Some(top) = stack.last_mut() {
            let idx = top.2;
            if idx >= top.1.len() {
                // Done with this node - mark as finished
                let node = top.0.clone();
                if let Ok(mut guard) = node.lock() {
                    guard.shape().graph_element().id = 0;
                }
                stack.pop();
                continue;
            }

            top.2 += 1;
            let edge = top.1[idx].0.clone();
            let target = top.1[idx].1.clone();

            let target_id = target
                .lock()
                .ok()
                .map(|mut g| g.shape().graph_element().id)
                .unwrap_or(0);

            if target_id < 0 {
                // back edge -> cycle detected
                rev_edges.push(edge);
            } else if target_id > 0 {
                // unvisited - cache edges, mark as visiting, push
                let target_edges = Self::get_outgoing(&target);
                if let Ok(mut guard) = target.lock() {
                    guard.shape().graph_element().id = -1;
                }
                stack.push((target, target_edges, 0));
            }
        }
    }

    fn get_outgoing(node: &LNodeRef) -> Vec<(LEdgeRef, LNodeRef)> {
        node.lock()
            .ok()
            .map(|g| g.outgoing_edges())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| {
                let target = edge
                    .lock()
                    .ok()
                    .and_then(|eg| eg.target())
                    .and_then(|p| p.lock().ok().and_then(|pg| pg.node()))?;
                if Arc::ptr_eq(node, &target) {
                    return None; // skip self-loops
                }
                Some((edge, target))
            })
            .collect()
    }
}

impl Default for InteractiveCycleBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for InteractiveCycleBreaker {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Interactive cycle breaking", 1.0);

        let nodes = graph.layerless_nodes().clone();
        let mut rev_edges: Vec<LEdgeRef> = Vec::new();

        // Phase 1: reverse edges that go backwards based on interactive reference point
        for node in &nodes {
            if let Ok(mut guard) = node.lock() {
                guard.shape().graph_element().id = 1;
            }
        }

        for source in &nodes {
            let source_x = source
                .lock()
                .ok()
                .and_then(|mut g| g.interactive_reference_point().map(|v| v.x))
                .unwrap_or(0.0);

            let outgoing = source
                .lock()
                .ok()
                .map(|g| g.outgoing_edges())
                .unwrap_or_default();

            for edge in outgoing {
                let target = edge
                    .lock()
                    .ok()
                    .and_then(|eg| eg.target())
                    .and_then(|p| p.lock().ok().and_then(|pg| pg.node()));
                let Some(target) = target else { continue };

                let is_same = Arc::ptr_eq(source, &target);
                if is_same {
                    continue;
                }

                let target_x = target
                    .lock()
                    .ok()
                    .and_then(|mut g| g.interactive_reference_point().map(|v| v.x))
                    .unwrap_or(0.0);

                if target_x < source_x {
                    rev_edges.push(edge);
                }
            }
        }

        let dummy_graph = LGraph::new();
        for edge in rev_edges.drain(..) {
            LEdge::reverse(&edge, &dummy_graph, true);
        }

        // Phase 2: DFS to catch remaining cycles
        // (could happen if some nodes have the same horizontal position)
        rev_edges.clear();

        // Re-initialize all nodes to id = 1 (unvisited) for Phase 2
        for node in &nodes {
            if let Ok(mut guard) = node.lock() {
                guard.shape().graph_element().id = 1;
            }
        }

        for node in &nodes {
            let is_unvisited = node
                .lock()
                .ok()
                .map(|mut g| g.shape().graph_element().id > 0)
                .unwrap_or(false);

            if is_unvisited {
                Self::find_cycles(node, &mut rev_edges);
            }
        }

        // Reverse the edges marked during Phase 2
        for edge in rev_edges.drain(..) {
            LEdge::reverse(&edge, &dummy_graph, true);
        }

        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        Some(LayoutProcessorConfiguration::create_from(
            &INTERMEDIATE_PROCESSING_CONFIGURATION,
        ))
    }
}
