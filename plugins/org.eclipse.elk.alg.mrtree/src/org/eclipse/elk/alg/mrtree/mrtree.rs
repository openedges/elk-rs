use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::AlgorithmAssembler;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::SharedProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    BasicProgressMonitor, IElkProgressMonitor,
};

use crate::org::eclipse::elk::alg::mrtree::graph::TGraphRef;
use crate::org::eclipse::elk::alg::mrtree::options::MrTreeOptions;
use crate::org::eclipse::elk::alg::mrtree::tree_layout_phases::TreeLayoutPhases;

pub struct MrTree {
    algorithm_assembler: AlgorithmAssembler<TreeLayoutPhases, TGraphRef>,
    algorithm: Vec<SharedProcessor<TGraphRef>>,
}

impl MrTree {
    pub fn new() -> Self {
        let mut assembler = AlgorithmAssembler::create();
        // MrTree phases/processors keep mutable traversal state; per-layout instances avoid leakage
        // across connected components and repeated engine invocations.
        assembler.with_caching(false);
        MrTree {
            algorithm_assembler: assembler,
            algorithm: Vec::new(),
        }
    }

    pub fn do_layout(&mut self, graph: &TGraphRef, monitor: Option<&mut dyn IElkProgressMonitor>) {
        match monitor {
            Some(monitor) => self.do_layout_with_monitor(graph, monitor),
            None => {
                let mut default_monitor = BasicProgressMonitor::new();
                default_monitor.with_max_hierarchy_levels(0);
                self.do_layout_with_monitor(graph, &mut default_monitor);
            }
        }
    }

    fn do_layout_with_monitor(&mut self, graph: &TGraphRef, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Tree layout", 1.0);

        let debug = graph
            .lock_ok()
            .and_then(|mut g| g.get_property(MrTreeOptions::DEBUG_MODE))
            .unwrap_or(false);
        if debug {
            monitor.log("MrTree! called.");
        }

        self.update_modules(graph);
        let mut graph_ref = graph.clone();
        self.layout(&mut graph_ref, monitor.sub_task(1.0).as_mut());
        monitor.done();
    }

    fn update_modules(&mut self, graph: &TGraphRef) {
        self.algorithm_assembler.reset();
        self.algorithm_assembler
            .set_phase(
                TreeLayoutPhases::P1Treeification,
                Arc::new(TreeLayoutPhases::P1Treeification),
            )
            .set_phase(
                TreeLayoutPhases::P2NodeOrdering,
                Arc::new(TreeLayoutPhases::P2NodeOrdering),
            )
            .set_phase(
                TreeLayoutPhases::P3NodePlacement,
                Arc::new(TreeLayoutPhases::P3NodePlacement),
            )
            .set_phase(
                TreeLayoutPhases::P4EdgeRouting,
                Arc::new(TreeLayoutPhases::P4EdgeRouting),
            );

        self.algorithm = self.algorithm_assembler.build(graph);
    }

    fn layout(&mut self, graph: &mut TGraphRef, monitor: &mut dyn IElkProgressMonitor) {
        let total = self.algorithm.len().max(1) as f32;
        monitor.begin("Layout", total);

        let debug = graph
            .lock_ok()
            .and_then(|mut g| g.get_property(MrTreeOptions::DEBUG_MODE))
            .unwrap_or(false);
        if debug {
            monitor.log(&format!(
                "ELK MrTree uses the following {} modules:",
                self.algorithm.len()
            ));
        }

        for (idx, processor) in self.algorithm.iter().enumerate() {
            if monitor.is_canceled() {
                return;
            }
            if debug {
                monitor.log(&format!("   Slot {}: processor", idx));
            }
            if let Some(mut processor_guard) = processor.lock_ok() {
                let mut sub = monitor.sub_task(1.0);
                processor_guard.process(graph, sub.as_mut());
            }
        }

        monitor.done();
    }
}

impl Default for MrTree {
    fn default() -> Self {
        Self::new()
    }
}
