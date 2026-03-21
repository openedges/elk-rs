use std::sync::Arc;
use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LEdgeRef, LGraph, LNodeRef};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{InteractiveReferencePoint, LayeredOptions};
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

type StackEntry = (LNodeRef, Vec<(LEdgeRef, LNodeRef)>, usize);

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

    fn reference_x(node: &LNodeRef, reference: InteractiveReferencePoint) -> f64 {
        node.lock_ok()
            .map(|mut node_guard| {
                let shape = node_guard.shape();
                let pos_x = shape.position_ref().x;
                match reference {
                    InteractiveReferencePoint::Center => pos_x + shape.size_ref().x / 2.0,
                    InteractiveReferencePoint::TopLeft => pos_x,
                }
            })
            .unwrap_or(0.0)
    }

    fn find_cycles(start: &LNodeRef, rev_edges: &mut Vec<LEdgeRef>) {
        // Iterative DFS matching Java's recursive approach.
        // Stack stores: (node, cached outgoing edges, edge_index)
        // Edges are cached once when a node is pushed to avoid repeated mutex locks.
        let mut stack: Vec<StackEntry> = Vec::new();

        let start_edges = Self::get_outgoing(start);
        if let Some(mut guard) = start.lock_ok() {
            guard.shape().graph_element().id = -1;
        }
        stack.push((start.clone(), start_edges, 0));

        while let Some(top) = stack.last_mut() {
            let idx = top.2;
            if idx >= top.1.len() {
                // Done with this node - mark as finished
                let node = top.0.clone();
                if let Some(mut guard) = node.lock_ok() {
                    guard.shape().graph_element().id = 0;
                }
                stack.pop();
                continue;
            }

            top.2 += 1;
            let edge = top.1[idx].0.clone();
            let target = top.1[idx].1.clone();

            let target_id = target
                .lock_ok()
                .map(|mut g| g.shape().graph_element().id)
                .unwrap_or(0);

            if target_id < 0 {
                // back edge -> cycle detected
                rev_edges.push(edge);
            } else if target_id > 0 {
                // unvisited - cache edges, mark as visiting, push
                let target_edges = Self::get_outgoing(&target);
                if let Some(mut guard) = target.lock_ok() {
                    guard.shape().graph_element().id = -1;
                }
                stack.push((target, target_edges, 0));
            }
        }
    }

    fn get_outgoing(node: &LNodeRef) -> Vec<(LEdgeRef, LNodeRef)> {
        node.lock_ok()
            .map(|g| g.outgoing_edges())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| {
                let target = edge
                    .lock_ok()
                    .and_then(|eg| eg.target())
                    .and_then(|p| p.lock_ok().and_then(|pg| pg.node()))?;
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
        let reference_point = graph
            .get_property(LayeredOptions::INTERACTIVE_REFERENCE_POINT)
            .unwrap_or(InteractiveReferencePoint::Center);
        let mut rev_edges: Vec<LEdgeRef> = Vec::new();

        // Phase 1: reverse edges that go backwards based on interactive reference point
        for node in &nodes {
            if let Some(mut guard) = node.lock_ok() {
                guard.shape().graph_element().id = 1;
            }
        }

        for source in &nodes {
            let source_x = Self::reference_x(source, reference_point);

            let outgoing = source
                .lock_ok()
                .map(|g| g.outgoing_edges())
                .unwrap_or_default();

            for edge in outgoing {
                let target = edge
                    .lock_ok()
                    .and_then(|eg| eg.target())
                    .and_then(|p| p.lock_ok().and_then(|pg| pg.node()));
                let Some(target) = target else { continue };

                let is_same = Arc::ptr_eq(source, &target);
                if is_same {
                    continue;
                }

                let target_x = Self::reference_x(&target, reference_point);

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
            if let Some(mut guard) = node.lock_ok() {
                guard.shape().graph_element().id = 1;
            }
        }

        for node in &nodes {
            let is_unvisited = node
                .lock_ok()
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
