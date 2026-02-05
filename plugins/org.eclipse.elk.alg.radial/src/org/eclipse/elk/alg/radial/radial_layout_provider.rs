use std::rc::Rc;
use std::sync::Arc;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::NodeMicroLayout;
use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::algorithm_assembler::{AlgorithmAssembler, SharedProcessor};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::unsupported_configuration::UnsupportedConfigurationException;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::internal_properties::InternalProperties;
use crate::org::eclipse::elk::alg::radial::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::radial::options::{CompactionStrategy, RadialOptions};
use crate::org::eclipse::elk::alg::radial::radial_layout_phases::RadialLayoutPhases;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;

pub struct RadialLayoutProvider;

impl RadialLayoutProvider {
    pub fn new() -> Self {
        RadialLayoutProvider
    }

    fn assemble_algorithm(&mut self, layout_graph: &ElkNodeRef) -> Vec<SharedProcessor<ElkNodeRef>> {
        let mut algorithm_assembler: AlgorithmAssembler<RadialLayoutPhases, ElkNodeRef> =
            AlgorithmAssembler::create();

        let p1_factory: Arc<dyn ILayoutPhaseFactory<RadialLayoutPhases, ElkNodeRef>> =
            Arc::new(RadialLayoutPhases::P1NodePlacement);
        let p2_factory: Arc<dyn ILayoutPhaseFactory<RadialLayoutPhases, ElkNodeRef>> =
            Arc::new(RadialLayoutPhases::P2EdgeRouting);

        algorithm_assembler.set_phase(RadialLayoutPhases::P1NodePlacement, p1_factory);
        algorithm_assembler.set_phase(RadialLayoutPhases::P2EdgeRouting, p2_factory);

        let mut configuration = LayoutProcessorConfiguration::create();
        configuration.add_before(
            RadialLayoutPhases::P2EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::OverlapRemoval),
        );

        let compactor = node_get_property(layout_graph, RadialOptions::COMPACTOR)
            .unwrap_or(CompactionStrategy::None);
        if compactor != CompactionStrategy::None {
            configuration.add_before(
                RadialLayoutPhases::P2EdgeRouting,
                Arc::new(IntermediateProcessorStrategy::Compaction),
            );
        }

        if node_get_property(layout_graph, RadialOptions::ROTATE).unwrap_or(false) {
            configuration.add_before(
                RadialLayoutPhases::P2EdgeRouting,
                Arc::new(IntermediateProcessorStrategy::Rotation),
            );
        }

        configuration.add_before(
            RadialLayoutPhases::P2EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::GraphSizeCalculation),
        );

        if node_get_property(layout_graph, RadialOptions::ROTATION_OUTGOING_EDGE_ANGLES)
            .unwrap_or(false)
        {
            configuration.add_after(
                RadialLayoutPhases::P2EdgeRouting,
                Arc::new(IntermediateProcessorStrategy::OutgoingEdgeAngles),
            );
        }

        algorithm_assembler.add_processor_configuration(&configuration);
        algorithm_assembler.build(layout_graph)
    }
}

impl Default for RadialLayoutProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphLayoutEngine for RadialLayoutProvider {
    fn layout(&mut self, layout_graph: &ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        let algorithm = self.assemble_algorithm(layout_graph);
        progress_monitor.begin("Radial layout", algorithm.len() as f32);

        if !node_get_property(layout_graph, RadialOptions::OMIT_NODE_MICRO_LAYOUT).unwrap_or(false) {
            NodeMicroLayout::for_graph(layout_graph.clone()).execute();
        }

        let root = RadialUtil::find_root(layout_graph);
        let Some(root) = root else {
            panic!(
                "{}",
                UnsupportedConfigurationException::new("The given graph is not a tree!")
            );
        };
        let root_id = Rc::as_ptr(&root) as usize;
        node_set_property(layout_graph, InternalProperties::ROOT_NODE, Some(root_id));

        let mut layout_radius = node_get_property(layout_graph, RadialOptions::RADIUS).unwrap_or(0.0);
        if layout_radius == 0.0 {
            layout_radius = RadialUtil::find_largest_node_in_graph(layout_graph);
        }
        node_set_property(layout_graph, RadialOptions::RADIUS, Some(layout_radius));

        let mut graph_ref = layout_graph.clone();
        for (index, processor) in algorithm.iter().enumerate() {
            if progress_monitor.is_canceled() {
                return;
            }
            if progress_monitor.is_logging_enabled() {
                let tag = format!("{}-Before processor", index);
                progress_monitor.log_graph(layout_graph, &tag);
            }
            let mut sub = progress_monitor.sub_task(1.0);
            let mut processor_guard = processor.lock().expect("processor lock");
            processor_guard.process(&mut graph_ref, sub.as_mut());
        }

        if progress_monitor.is_logging_enabled() {
            let tag = format!("{}-Finished", algorithm.len());
            progress_monitor.log_graph(layout_graph, &tag);
        }

        progress_monitor.done();
    }
}

impl AbstractLayoutProvider for RadialLayoutProvider {}

fn node_get_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
) -> Option<T> {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    props.get_property(property)
}

fn node_set_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
    value: Option<T>,
) {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    props.set_property(property, value);
}
