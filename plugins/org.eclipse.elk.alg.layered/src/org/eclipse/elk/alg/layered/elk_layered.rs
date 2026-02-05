use std::any::TypeId;
use std::collections::VecDeque;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::SharedProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::content_alignment::ContentAlignment;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::testing::TestController;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, ElkUtil, EnumSet, IElkProgressMonitor};

use crate::org::eclipse::elk::alg::layered::components::ComponentsProcessor;
use crate::org::eclipse::elk::alg::layered::compound::{
    CompoundGraphPostprocessor, CompoundGraphPreprocessor,
};
use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LGraphRef, NodeType};
use crate::org::eclipse::elk::alg::layered::graph_configurator::GraphConfigurator;
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions,
};

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

    pub fn do_layout(
        &mut self,
        lgraph: &LGraphRef,
        monitor: Option<&mut dyn IElkProgressMonitor>,
    ) {
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

    fn do_layout_with_monitor(&mut self, lgraph: &LGraphRef, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Layered layout", 1.0);

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

        monitor.done();
    }

    fn do_compound_layout_with_monitor(
        &mut self,
        lgraph: &LGraphRef,
        monitor: &mut dyn IElkProgressMonitor,
    ) {
        monitor.begin("Layered layout", 2.0);

        let mut pre_monitor = monitor.sub_task(1.0);
        if let Ok(mut graph_guard) = lgraph.lock() {
            self.notify_processor_ready_with_graph(&graph_guard, &self.compound_graph_preprocessor);
            self.compound_graph_preprocessor
                .process(&mut *graph_guard, pre_monitor.as_mut());
            self.notify_processor_finished_with_graph(&graph_guard, &self.compound_graph_preprocessor);
        }

        let mut layout_monitor = monitor.sub_task(1.0);
        self.hierarchical_layout(lgraph, layout_monitor.as_mut());

        let mut post_monitor = monitor.sub_task(1.0);
        if let Ok(mut graph_guard) = lgraph.lock() {
            self.notify_processor_ready_with_graph(&graph_guard, &self.compound_graph_postprocessor);
            self.compound_graph_postprocessor
                .process(&mut *graph_guard, post_monitor.as_mut());
            self.notify_processor_finished_with_graph(&graph_guard, &self.compound_graph_postprocessor);
        }

        monitor.done();
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

    pub fn get_layout_test_configuration(&self, state: &TestExecutionState) -> Vec<SharedProcessor<LGraph>> {
        self.processors_for_state(state).unwrap_or_default()
    }

    pub fn set_test_controller(&mut self, controller: Option<*mut TestController>) {
        self.test_controller = controller.map(|ptr| ptr as usize);
    }

    fn hierarchical_layout(&mut self, lgraph: &LGraphRef, monitor: &mut dyn IElkProgressMonitor) {
        let graphs = self.collect_all_graphs_bottom_up(lgraph);
        for graph in &graphs {
            self.graph_configurator.prepare_graph_for_layout(graph);
        }

        let total_work = graphs
            .iter()
            .map(|graph| {
                graph
                    .lock()
                    .ok()
                    .and_then(|mut guard| guard.get_property(InternalProperties::PROCESSORS))
                    .map(|processors| processors.len())
                    .unwrap_or(0)
            })
            .sum::<usize>() as f32;

        monitor.begin("Recursive hierarchical layout", total_work);

        for graph in &graphs {
            if monitor.is_canceled() {
                return;
            }
            self.layout_component(graph, monitor);
        }

        monitor.done();
    }

    fn collect_all_graphs_bottom_up(&self, root: &LGraphRef) -> Vec<LGraphRef> {
        let mut queue: VecDeque<LGraphRef> = VecDeque::new();
        let mut result: Vec<LGraphRef> = Vec::new();
        queue.push_back(root.clone());

        while let Some(graph) = queue.pop_front() {
            result.push(graph.clone());
            let nodes = graph
                .lock()
                .ok()
                .map(|guard| guard.layerless_nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                if let Some(nested) = node.lock().ok().and_then(|node_guard| node_guard.nested_graph()) {
                    queue.push_back(nested);
                }
            }
        }

        result.reverse();
        result
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

        for processor in &processors {
            if monitor.is_canceled() {
                return;
            }

            let mut sub_monitor = monitor.sub_task(monitor_progress);
            if let Ok(mut graph_guard) = lgraph.lock() {
                if let Ok(mut proc_guard) = processor.lock() {
                    self.notify_processor_ready_with_graph(&graph_guard, proc_guard.as_ref());
                    proc_guard.process(&mut *graph_guard, sub_monitor.as_mut());
                    self.notify_processor_finished_with_graph(&graph_guard, proc_guard.as_ref());
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

    fn processors_for_state(&self, state: &TestExecutionState) -> Option<Vec<SharedProcessor<LGraph>>> {
        let graph = state.graphs.get(0)?.clone();
        let mut graph_guard = graph.lock().ok()?;
        Some(graph_guard.get_property(InternalProperties::PROCESSORS).unwrap_or_default())
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
