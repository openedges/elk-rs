use std::any::TypeId;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::SharedProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::content_alignment::ContentAlignment;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::testing::TestController;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    BasicProgressMonitor, ElkUtil, EnumSet, IElkProgressMonitor,
};

use crate::org::eclipse::elk::alg::layered::components::ComponentsProcessor;
use crate::org::eclipse::elk::alg::layered::compound::{
    CompoundGraphPostprocessor, CompoundGraphPreprocessor,
};
use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LGraphRef, NodeType};
use crate::org::eclipse::elk::alg::layered::graph_configurator::GraphConfigurator;
use crate::org::eclipse::elk::alg::layered::options::{
    CrossingMinimizationStrategy, GraphProperties, GreedySwitchType, InternalProperties,
    LayeredOptions,
};
use crate::org::eclipse::elk::alg::layered::trace_recorder;

/// Global step counter for trace recording. Uses AtomicUsize so it can be
/// incremented from `run_processor_for_graph` without requiring `&mut self`.
static TRACE_STEP_COUNTER: AtomicUsize = AtomicUsize::new(0);
static TRACE_LAYOUT_DEPTH: AtomicUsize = AtomicUsize::new(0);

static TRACE: LazyLock<bool> = LazyLock::new(|| std::env::var("ELK_TRACE").is_ok());
static TRACE_EDGE_WIRING: LazyLock<bool> =
    LazyLock::new(|| std::env::var("ELK_TRACE_EDGE_WIRING").is_ok());
static TRACE_NODES: LazyLock<bool> =
    LazyLock::new(|| std::env::var("ELK_TRACE_NODES").is_ok());
static TRACE_NODES_FILTER: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("ELK_TRACE_NODES_FILTER").ok());
static TRACE_DIR: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("ELK_TRACE_DIR").ok());
static TRACE_CROSSMIN: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_CROSSMIN").is_some());
static TRACE_RESIZE: LazyLock<bool> =
    LazyLock::new(|| std::env::var("ELK_TRACE_RESIZE").is_ok());

#[cfg(debug_assertions)]
fn trace_step(message: &str) {
    if *TRACE {
        eprintln!("[elk-layered] {message}");
    }
}

#[cfg(not(debug_assertions))]
fn trace_step(_message: &str) {}

#[cfg(debug_assertions)]
fn trace_edge_wiring(graph: &LGraph, stage: &str) {
    if !*TRACE_EDGE_WIRING {
        return;
    }

    let layers = graph.layers().clone();
    if layers.len() < 2 {
        return;
    }

    for layer_index in 0..(layers.len() - 1) {
        let current_layer = layers[layer_index].clone();
        let next_layer = layers[layer_index + 1].clone();
        let nodes = current_layer
            .lock()
            .ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();

        for node in nodes {
            let ports = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.ports().clone())
                .unwrap_or_default();

            for source_port in ports {
                let outgoing = source_port
                    .lock()
                    .ok()
                    .map(|port_guard| port_guard.outgoing_edges().clone())
                    .unwrap_or_default();

                for edge in outgoing {
                    let target_port = edge.lock().ok().and_then(|edge_guard| edge_guard.target());
                    let Some(target_port) = target_port else {
                        continue;
                    };

                    let target_layer = target_port
                        .lock()
                        .ok()
                        .and_then(|port_guard| port_guard.node())
                        .and_then(|node| {
                            node.lock().ok().and_then(|node_guard| node_guard.layer())
                        });
                    let Some(target_layer) = target_layer else {
                        continue;
                    };

                    if !Arc::ptr_eq(&target_layer, &next_layer) {
                        continue;
                    }

                    let source_desc = source_port
                        .lock()
                        .ok()
                        .map(|mut port_guard| port_guard.to_string())
                        .unwrap_or_else(|| "<poisoned-source-port>".to_owned());
                    let target_desc = target_port
                        .lock()
                        .ok()
                        .map(|mut port_guard| port_guard.to_string())
                        .unwrap_or_else(|| "<poisoned-target-port>".to_owned());

                    eprintln!(
                        "rust-wiring: stage={} layer={} {} -> {}",
                        stage, layer_index, source_desc, target_desc
                    );
                }
            }
        }
    }
}

#[cfg(not(debug_assertions))]
fn trace_edge_wiring(_graph: &LGraph, _stage: &str) {}

#[cfg(debug_assertions)]
fn trace_node_positions(graph: &LGraph, stage: &str) {
    if !*TRACE_NODES {
        return;
    }

    let filter = TRACE_NODES_FILTER.clone();
    let mut nodes: Vec<(Option<usize>, usize, _)> = graph
        .layerless_nodes()
        .iter()
        .cloned()
        .map(|node| (None, 0, node))
        .collect();
    for (layer_idx, layer) in graph.layers().iter().enumerate() {
        if let Ok(layer_guard) = layer.lock() {
            nodes.extend(
                layer_guard
                    .nodes()
                    .iter()
                    .enumerate()
                    .map(|(node_idx, node)| (Some(layer_idx), node_idx, node.clone())),
            );
        }
    }

    for (layer_idx, layer_node_index, node) in nodes {
        let (
            designation,
            node_id,
            node_type,
            pos,
            size,
            margin_top,
            margin_bottom,
            label_opt,
            has_in_layer_unit,
        ) = {
            let mut node_guard = match node.lock() {
                Ok(guard) => guard,
                Err(_) => continue,
            };
            let designation = node_guard.designation();
            let node_id = node_guard.shape().graph_element().id;
            let node_type = node_guard.node_type();
            let pos = *node_guard.shape().position_ref();
            let size = *node_guard.shape().size_ref();
            let margin_top = node_guard.margin().top;
            let margin_bottom = node_guard.margin().bottom;
            let has_in_layer_unit = node_guard
                .get_property(InternalProperties::IN_LAYER_LAYOUT_UNIT)
                .is_some();
            let label_opt = node_guard.labels().first().and_then(|label| {
                label
                    .lock()
                    .ok()
                    .map(|label_guard| label_guard.text().to_string())
            });
            (
                designation,
                node_id,
                node_type,
                pos,
                size,
                margin_top,
                margin_bottom,
                label_opt,
                has_in_layer_unit,
            )
        };

        if let Some(filter) = &filter {
            if !designation.contains(filter)
                && !label_opt
                    .as_deref()
                    .is_some_and(|label| label.contains(filter))
            {
                continue;
            }
        }

        eprintln!(
            "[elk-layered][nodes] stage={} layer={} layer_idx={} node={} id={} type={:?} label={:?} has_unit={} pos=({:.3},{:.3}) size=({:.3},{:.3}) margin=({:.3},{:.3})",
            stage,
            if layer_idx.is_some() {
                layer_idx
                    .map(|index| index.to_string())
                    .unwrap_or_else(|| "?".to_owned())
            } else {
                "layerless".to_owned()
            },
            layer_node_index,
            designation,
            node_id,
            node_type,
            label_opt,
            has_in_layer_unit,
            pos.x,
            pos.y,
            size.x,
            size.y,
            margin_top,
            margin_bottom
        );
    }
}

#[cfg(not(debug_assertions))]
fn trace_node_positions(_graph: &LGraph, _stage: &str) {}

pub struct ElkLayered {
    graph_configurator: GraphConfigurator,
    components_processor: ComponentsProcessor,
    compound_graph_preprocessor: CompoundGraphPreprocessor,
    compound_graph_postprocessor: CompoundGraphPostprocessor,
    test_controller: Option<usize>,
}

pub struct TestExecutionState {
    graphs: Vec<LGraphRef>,
    step: usize,
}

impl ElkLayered {
    pub fn new() -> Self {
        ElkLayered {
            graph_configurator: GraphConfigurator::new(),
            components_processor: ComponentsProcessor::new(),
            compound_graph_preprocessor: CompoundGraphPreprocessor::new(),
            compound_graph_postprocessor: CompoundGraphPostprocessor::new(),
            test_controller: None,
        }
    }

    pub fn do_layout(&mut self, lgraph: &LGraphRef, monitor: Option<&mut dyn IElkProgressMonitor>) {
        match monitor {
            Some(monitor) => self.do_layout_with_monitor(lgraph, monitor),
            None => {
                let mut default_monitor = BasicProgressMonitor::new();
                default_monitor.with_max_hierarchy_levels(0);
                self.do_layout_with_monitor(lgraph, &mut default_monitor);
            }
        }
    }

    pub fn do_compound_layout(
        &mut self,
        lgraph: &LGraphRef,
        monitor: Option<&mut dyn IElkProgressMonitor>,
    ) {
        match monitor {
            Some(monitor) => self.do_compound_layout_with_monitor(lgraph, monitor),
            None => {
                let mut default_monitor = BasicProgressMonitor::new();
                default_monitor.with_max_hierarchy_levels(0);
                self.do_compound_layout_with_monitor(lgraph, &mut default_monitor);
            }
        }
    }

    /// Reset the trace step counter. Call this at the start of each layout run.
    pub fn reset_trace_step_counter() {
        TRACE_STEP_COUNTER.store(0, Ordering::Relaxed);
    }

    /// Remove all `step_*.json` files from the ELK_TRACE_DIR directory.
    fn clear_trace_directory() {
        if let Some(trace_dir) = TRACE_DIR.as_deref() {
            if let Ok(entries) = std::fs::read_dir(trace_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "json") {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name.starts_with("step_") {
                                let _ = std::fs::remove_file(&path);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Clear stale step snapshots once at the beginning of a traced model run.
    fn prepare_trace_directory_for_run() {
        if TRACE_DIR.is_none() {
            return;
        }
        if TRACE_STEP_COUNTER.load(Ordering::Relaxed) == 0 {
            Self::clear_trace_directory();
        }
    }

    /// Record a trace snapshot when tracing is enabled for the outermost root graph.
    fn record_trace_snapshot_for_root(graph: &LGraph, processor_name: &str) {
        if TRACE_LAYOUT_DEPTH.load(Ordering::Relaxed) != 1 || graph.parent_node().is_some() {
            return;
        }
        let Some(trace_dir) = TRACE_DIR.as_deref() else {
            return;
        };

        let step = TRACE_STEP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let trace_path = PathBuf::from(trace_dir);
        if let Err(e) =
            trace_recorder::serialize_lgraph_snapshot(graph, step, processor_name, &trace_path)
        {
            eprintln!(
                "[elk-layered][trace] failed to write snapshot step={} proc={}: {}",
                step, processor_name, e
            );
        }
    }

    fn do_layout_with_monitor(
        &mut self,
        lgraph: &LGraphRef,
        monitor: &mut dyn IElkProgressMonitor,
    ) {
        monitor.begin("Layered layout", 1.0);
        let prev_depth = TRACE_LAYOUT_DEPTH.fetch_add(1, Ordering::Relaxed);
        if prev_depth == 0 {
            Self::prepare_trace_directory_for_run();
        }

        self.graph_configurator.prepare_graph_for_layout(lgraph);

        let components = self.components_processor.split(lgraph);
        if components.len() == 1 {
            self.layout_component(&components[0], monitor);
        } else {
            let comp_work = 1.0 / components.len().max(1) as f32;
            for component in &components {
                if monitor.is_canceled() {
                    return;
                }
                let mut sub_monitor = monitor.sub_task(comp_work);
                self.layout_component(component, sub_monitor.as_mut());
            }
        }

        self.components_processor.combine(&components, lgraph);
        self.resize_graph(lgraph);

        TRACE_LAYOUT_DEPTH.fetch_sub(1, Ordering::Relaxed);
        monitor.done();
    }

    fn do_compound_layout_with_monitor(
        &mut self,
        lgraph: &LGraphRef,
        monitor: &mut dyn IElkProgressMonitor,
    ) {
        let prev_depth = TRACE_LAYOUT_DEPTH.fetch_add(1, Ordering::Relaxed);
        if prev_depth == 0 {
            Self::prepare_trace_directory_for_run();
        }

        trace_step("compound layout: begin");
        monitor.begin("Layered layout", 2.0);

        let mut pre_monitor = monitor.sub_task(1.0);
        trace_step("compound layout: preprocessor start");
        if let Ok(graph_guard) = lgraph.lock() {
            self.notify_processor_ready_with_graph(&graph_guard, &self.compound_graph_preprocessor);
        }
        self.compound_graph_preprocessor
            .process_with_ref(lgraph, pre_monitor.as_mut());
        if let Ok(graph_guard) = lgraph.lock() {
            self.notify_processor_finished_with_graph(
                &graph_guard,
                &self.compound_graph_preprocessor,
            );
            Self::record_trace_snapshot_for_root(&graph_guard, "CompoundGraphPreprocessor");
        }
        trace_step("compound layout: preprocessor done");

        let mut layout_monitor = monitor.sub_task(1.0);
        trace_step("compound layout: hierarchical layout start");
        self.hierarchical_layout(lgraph, layout_monitor.as_mut());
        trace_step("compound layout: hierarchical layout done");

        let mut post_monitor = monitor.sub_task(1.0);
        trace_step("compound layout: postprocessor start");
        if let Ok(graph_guard) = lgraph.lock() {
            self.notify_processor_ready_with_graph(
                &graph_guard,
                &self.compound_graph_postprocessor,
            );
        }
        self.compound_graph_postprocessor
            .process_with_ref(lgraph, post_monitor.as_mut());
        if let Ok(graph_guard) = lgraph.lock() {
            self.notify_processor_finished_with_graph(
                &graph_guard,
                &self.compound_graph_postprocessor,
            );
            Self::record_trace_snapshot_for_root(&graph_guard, "CompoundGraphPostprocessor");
        }
        trace_step("compound layout: postprocessor done");

        monitor.done();
        trace_step("compound layout: done");
        TRACE_LAYOUT_DEPTH.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn prepare_layout_test(&mut self, lgraph: &LGraphRef) -> TestExecutionState {
        self.graph_configurator.prepare_graph_for_layout(lgraph);
        let graphs = self.components_processor.split(lgraph);
        TestExecutionState { graphs, step: 0 }
    }

    pub fn is_layout_test_finished(&self, state: &TestExecutionState) -> bool {
        let processors = match self.processors_for_state(state) {
            Some(processors) => processors,
            None => return true,
        };
        state.step >= processors.len()
    }

    pub fn run_layout_test_until<T: 'static + ILayoutProcessor<LGraph>>(
        &mut self,
        inclusive: bool,
        state: &mut TestExecutionState,
    ) {
        let processors = match self.processors_for_state(state) {
            Some(processors) => processors,
            None => return,
        };

        let target_id = TypeId::of::<T>();
        let mut phase_index = state.step;
        while phase_index < processors.len() {
            let matches = processors[phase_index]
                .lock()
                .ok()
                .map(|proc_guard| proc_guard.as_ref().type_id() == target_id)
                .unwrap_or(false);
            if matches {
                if inclusive {
                    phase_index += 1;
                }
                break;
            }
            phase_index += 1;
        }

        while state.step < phase_index {
            if let Some(processor) = processors.get(state.step) {
                self.layout_test(&state.graphs, processor);
            }
            state.step += 1;
        }
    }

    pub fn run_layout_test_step(&mut self, state: &mut TestExecutionState) {
        if self.is_layout_test_finished(state) {
            panic!("Current layout test run has finished.");
        }
        let processors = match self.processors_for_state(state) {
            Some(processors) => processors,
            None => return,
        };
        if let Some(processor) = processors.get(state.step) {
            self.layout_test(&state.graphs, processor);
        }
        state.step += 1;
    }

    pub fn get_layout_test_configuration(
        &self,
        state: &TestExecutionState,
    ) -> Vec<SharedProcessor<LGraph>> {
        self.processors_for_state(state).unwrap_or_default()
    }

    pub fn set_test_controller(&mut self, controller: Option<*mut TestController>) {
        self.test_controller = controller.map(|ptr| ptr as usize);
    }

    fn hierarchical_layout(&mut self, lgraph: &LGraphRef, monitor: &mut dyn IElkProgressMonitor) {
        let graphs = self.collect_all_graphs_bottom_up(lgraph);
        self.review_and_correct_hierarchical_processors(lgraph, &graphs);

        struct GraphAndAlgorithm {
            graph: LGraphRef,
            processors: Vec<SharedProcessor<LGraph>>,
            index: usize,
        }

        let mut total_work = 0.0;
        let mut graphs_and_algorithms: Vec<GraphAndAlgorithm> = Vec::with_capacity(graphs.len());
        for graph in &graphs {
            self.graph_configurator.prepare_graph_for_layout(graph);
            let processors = graph
                .lock()
                .ok()
                .and_then(|mut guard| guard.get_property(InternalProperties::PROCESSORS))
                .unwrap_or_default();
            total_work += processors.len() as f32;
            graphs_and_algorithms.push(GraphAndAlgorithm {
                graph: graph.clone(),
                processors,
                index: 0,
            });
        }

        monitor.begin("Recursive hierarchical layout", total_work);
        if graphs_and_algorithms.is_empty() {
            monitor.done();
            return;
        }

        let root_index = graphs_and_algorithms
            .iter()
            .position(|entry| {
                entry
                    .graph
                    .lock()
                    .ok()
                    .and_then(|graph_guard| graph_guard.parent_node())
                    .is_none()
            })
            .unwrap_or_else(|| graphs_and_algorithms.len().saturating_sub(1));

        while graphs_and_algorithms[root_index].index
            < graphs_and_algorithms[root_index].processors.len()
        {
            for entry in &mut graphs_and_algorithms {
                while !monitor.is_canceled() {
                    let (graph, processor, is_root, hierarchy_aware) = {
                        if entry.index >= entry.processors.len() {
                            break;
                        }

                        let graph = entry.graph.clone();
                        let is_root = graph
                            .lock()
                            .ok()
                            .and_then(|graph_guard| graph_guard.parent_node())
                            .is_none();
                        let processor = entry.processors[entry.index].clone();
                        entry.index += 1;
                        let hierarchy_aware = processor
                            .lock()
                            .ok()
                            .map(|processor_guard| processor_guard.is_hierarchy_aware())
                            .unwrap_or(false);
                        (graph, processor, is_root, hierarchy_aware)
                    };

                    if !hierarchy_aware {
                        let mut sub_monitor = monitor.sub_task(1.0);
                        self.run_processor_for_graph(&graph, &processor, sub_monitor.as_mut());
                        continue;
                    }

                    if is_root {
                        let mut sub_monitor = monitor.sub_task(1.0);
                        self.run_processor_for_graph(&graph, &processor, sub_monitor.as_mut());
                    }
                    break;
                }

                if monitor.is_canceled() {
                    return;
                }
            }
        }

        monitor.done();
    }

    fn collect_all_graphs_bottom_up(&self, root: &LGraphRef) -> Vec<LGraphRef> {
        // Mirror Java's stack-based push/pop order to keep hierarchy traversal stable.
        let mut collected: VecDeque<LGraphRef> = VecDeque::new();
        let mut continue_searching: VecDeque<LGraphRef> = VecDeque::new();
        collected.push_front(root.clone());
        continue_searching.push_front(root.clone());

        while let Some(graph) = continue_searching.pop_front() {
            let nodes = graph
                .lock()
                .ok()
                .map(|guard| guard.layerless_nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                if let Some(nested) = node
                    .lock()
                    .ok()
                    .and_then(|node_guard| node_guard.nested_graph())
                {
                    collected.push_front(nested.clone());
                    continue_searching.push_front(nested);
                }
            }
        }

        collected.into_iter().collect()
    }

    fn layout_component(&mut self, lgraph: &LGraphRef, monitor: &mut dyn IElkProgressMonitor) {
        let processors = {
            let mut graph_guard = match lgraph.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            graph_guard
                .get_property(InternalProperties::PROCESSORS)
                .unwrap_or_default()
        };

        if processors.is_empty() {
            return;
        }

        let monitor_progress = 1.0 / processors.len() as f32;

        for processor in processors.iter() {
            if monitor.is_canceled() {
                return;
            }

            let mut sub_monitor = monitor.sub_task(monitor_progress);
            self.run_processor_for_graph(lgraph, processor, sub_monitor.as_mut());
        }
    }

    fn run_processor_for_graph(
        &mut self,
        lgraph: &LGraphRef,
        processor: &SharedProcessor<LGraph>,
        monitor: &mut dyn IElkProgressMonitor,
    ) {
        if let Ok(mut graph_guard) = lgraph.lock() {
            if let Ok(mut proc_guard) = processor.lock() {
                let proc_name = proc_guard.type_name();
                let is_root = graph_guard.parent_node().is_none();
                if *TRACE_CROSSMIN {
                    eprintln!(
                        "crossmin: processor start graph_ptr={:p} is_root={} proc={}",
                        Arc::as_ptr(lgraph),
                        is_root,
                        proc_name
                    );
                }
                trace_step(&format!("processor start: {proc_name}"));
                self.notify_processor_ready_with_graph(&graph_guard, proc_guard.as_ref());
                proc_guard.process(&mut *graph_guard, monitor);
                self.notify_processor_finished_with_graph(&graph_guard, proc_guard.as_ref());
                trace_edge_wiring(&graph_guard, &format!("after {proc_name}"));
                trace_node_positions(&graph_guard, &format!("after {proc_name}"));
                trace_step(&format!("processor done: {proc_name}"));
                // Trace recording: serialize LGraph snapshot to JSON after each step.
                // Only trace the outermost layout (depth==1) to match Java behavior.
                if is_root {
                    Self::record_trace_snapshot_for_root(&graph_guard, proc_name);
                }
                if *TRACE_CROSSMIN {
                    eprintln!(
                        "crossmin: processor done graph_ptr={:p} is_root={} proc={}",
                        Arc::as_ptr(lgraph),
                        is_root,
                        proc_name
                    );
                }
            }
        }
    }

    fn layout_test(&mut self, graphs: &[LGraphRef], processor: &SharedProcessor<LGraph>) {
        for graph in graphs {
            if let Ok(mut graph_guard) = graph.lock() {
                if let Ok(mut proc_guard) = processor.lock() {
                    let mut monitor = BasicProgressMonitor::new();
                    proc_guard.process(&mut *graph_guard, &mut monitor);
                }
            }
        }
    }

    fn processors_for_state(
        &self,
        state: &TestExecutionState,
    ) -> Option<Vec<SharedProcessor<LGraph>>> {
        let graph = state.graphs.first()?.clone();
        let mut graph_guard = graph.lock().ok()?;
        Some(
            graph_guard
                .get_property(InternalProperties::PROCESSORS)
                .unwrap_or_default(),
        )
    }

    fn review_and_correct_hierarchical_processors(&self, root: &LGraphRef, graphs: &[LGraphRef]) {
        let (root_crossing_minimization, root_greedy_switch_type) = {
            let mut root_guard = match root.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            (
                root_guard
                    .get_property(LayeredOptions::CROSSING_MINIMIZATION_STRATEGY)
                    .unwrap_or(CrossingMinimizationStrategy::LayerSweep),
                root_guard
                    .get_property(
                        LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE,
                    )
                    .unwrap_or(GreedySwitchType::Off),
            )
        };

        for graph in graphs {
            if let Ok(mut graph_guard) = graph.lock() {
                let child_crossing_minimization = graph_guard
                    .get_property(LayeredOptions::CROSSING_MINIMIZATION_STRATEGY)
                    .unwrap_or(CrossingMinimizationStrategy::LayerSweep);
                if child_crossing_minimization != root_crossing_minimization {
                    panic!(
                        "The hierarchy aware processor {:?} in a child graph is only allowed if the root graph uses the same processor.",
                        child_crossing_minimization
                    );
                }
                graph_guard.set_property(
                    LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE,
                    Some(root_greedy_switch_type),
                );
            }
        }
    }

    fn notify_processor_ready_with_graph(
        &self,
        graph: &LGraph,
        processor: &dyn ILayoutProcessor<LGraph>,
    ) {
        let Some(controller) = self.test_controller else {
            return;
        };
        let is_root = graph.parent_node().is_none();
        let graph_any: &dyn std::any::Any = graph;
        let processor_any: &dyn std::any::Any = processor;
        unsafe {
            (*(controller as *mut TestController)).notify_processor_ready(
                graph_any,
                processor_any,
                is_root,
            );
        }
    }

    fn notify_processor_finished_with_graph(
        &self,
        graph: &LGraph,
        processor: &dyn ILayoutProcessor<LGraph>,
    ) {
        let Some(controller) = self.test_controller else {
            return;
        };
        let is_root = graph.parent_node().is_none();
        let graph_any: &dyn std::any::Any = graph;
        let processor_any: &dyn std::any::Any = processor;
        unsafe {
            (*(controller as *mut TestController)).notify_processor_finished(
                graph_any,
                processor_any,
                is_root,
            );
        }
    }

    fn resize_graph(&self, lgraph: &LGraphRef) {
        let (size_constraints, size_options, min_size, fixed_graph_size, calculated_size) = {
            let mut graph_guard = match lgraph.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let size_constraints = graph_guard
                .get_property(LayeredOptions::NODE_SIZE_CONSTRAINTS)
                .unwrap_or_else(EnumSet::none_of);
            let size_options = graph_guard
                .get_property(CoreOptions::NODE_SIZE_OPTIONS)
                .unwrap_or_else(EnumSet::none_of);
            let min_size = graph_guard
                .get_property(CoreOptions::NODE_SIZE_MINIMUM)
                .unwrap_or_else(KVector::new);
            let fixed_graph_size = graph_guard
                .get_property(CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE)
                .unwrap_or(false);
            let calculated_size = graph_guard.actual_size();
            (
                size_constraints,
                size_options,
                min_size,
                fixed_graph_size,
                calculated_size,
            )
        };

        let mut adjusted_size = calculated_size;
        if size_constraints.contains(&SizeConstraint::MinimumSize) {
            let mut min_size = min_size;
            if size_options.contains(&SizeOptions::DefaultMinimumSize) {
                if min_size.x <= 0.0 {
                    min_size.x = ElkUtil::DEFAULT_MIN_WIDTH;
                }
                if min_size.y <= 0.0 {
                    min_size.y = ElkUtil::DEFAULT_MIN_HEIGHT;
                }
            }
            adjusted_size.x = adjusted_size.x.max(min_size.x);
            adjusted_size.y = adjusted_size.y.max(min_size.y);
        }

        if *TRACE_RESIZE {
            if let Ok(graph_guard) = lgraph.lock() {
                let parent_id = graph_guard
                    .parent_node()
                    .and_then(|node| node.lock().ok().map(|mut n| n.shape().graph_element().id))
                    .unwrap_or(-1);
                let size = *graph_guard.size_ref();
                let padding = graph_guard.padding_ref().clone();
                let offset = *graph_guard.offset_ref();
                eprintln!(
                    "[elk-layered][resize] parent={} size=({:.1},{:.1}) pad=({:.1},{:.1},{:.1},{:.1}) off=({:.1},{:.1}) constraints={:?} size_options={:?} min=({:.1},{:.1}) fixed={} calc=({:.1},{:.1}) adj=({:.1},{:.1})",
                    parent_id,
                    size.x,
                    size.y,
                    padding.left,
                    padding.top,
                    padding.right,
                    padding.bottom,
                    offset.x,
                    offset.y,
                    size_constraints,
                    size_options,
                    min_size.x,
                    min_size.y,
                    fixed_graph_size,
                    calculated_size.x,
                    calculated_size.y,
                    adjusted_size.x,
                    adjusted_size.y
                );
            }
        }

        if !fixed_graph_size {
            self.resize_graph_no_really(lgraph, calculated_size, adjusted_size);
        }
    }

    fn resize_graph_no_really(&self, lgraph: &LGraphRef, old_size: KVector, new_size: KVector) {
        let (content_alignment, graph_properties) = {
            let mut graph_guard = match lgraph.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let content_alignment = graph_guard
                .get_property(CoreOptions::CONTENT_ALIGNMENT)
                .unwrap_or_else(EnumSet::none_of);
            let graph_properties = graph_guard
                .get_property(InternalProperties::GRAPH_PROPERTIES)
                .unwrap_or_else(EnumSet::none_of);
            (content_alignment, graph_properties)
        };

        if new_size.x > old_size.x {
            if content_alignment.contains(&ContentAlignment::HCenter) {
                if let Ok(mut graph_guard) = lgraph.lock() {
                    graph_guard.offset().x += (new_size.x - old_size.x) / 2.0;
                }
            } else if content_alignment.contains(&ContentAlignment::HRight) {
                if let Ok(mut graph_guard) = lgraph.lock() {
                    graph_guard.offset().x += new_size.x - old_size.x;
                }
            }
        }

        if new_size.y > old_size.y {
            if content_alignment.contains(&ContentAlignment::VCenter) {
                if let Ok(mut graph_guard) = lgraph.lock() {
                    graph_guard.offset().y += (new_size.y - old_size.y) / 2.0;
                }
            } else if content_alignment.contains(&ContentAlignment::VBottom) {
                if let Ok(mut graph_guard) = lgraph.lock() {
                    graph_guard.offset().y += new_size.y - old_size.y;
                }
            }
        }

        if graph_properties.contains(&GraphProperties::ExternalPorts)
            && (new_size.x > old_size.x || new_size.y > old_size.y)
        {
            let nodes = lgraph
                .lock()
                .ok()
                .map(|guard| guard.layerless_nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let mut node_guard = match node.lock() {
                    Ok(guard) => guard,
                    Err(_) => continue,
                };
                if node_guard.node_type() != NodeType::ExternalPort {
                    continue;
                }
                let ext_side = node_guard
                    .get_property(InternalProperties::EXT_PORT_SIDE)
                    .unwrap_or(PortSide::Undefined);
                if ext_side == PortSide::East {
                    node_guard.shape().position().x += new_size.x - old_size.x;
                } else if ext_side == PortSide::South {
                    node_guard.shape().position().y += new_size.y - old_size.y;
                }
            }
        }

        if let Ok(mut graph_guard) = lgraph.lock() {
            let padding = graph_guard.padding_ref().clone();
            graph_guard.size().x = new_size.x - padding.left - padding.right;
            graph_guard.size().y = new_size.y - padding.top - padding.bottom;
        }
    }
}

impl Default for ElkLayered {
    fn default() -> Self {
        Self::new()
    }
}
