use std::sync::{Arc, LazyLock};

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::LGraph;
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{GraphProperties, InternalProperties};
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

static HIERARCHY_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config.add_before(
        LayeredPhases::P5EdgeRouting,
        Arc::new(IntermediateProcessorStrategy::HierarchicalPortPositionProcessor),
    );
    config
});

pub struct SimpleNodePlacer;

impl SimpleNodePlacer {
    pub fn new() -> Self {
        SimpleNodePlacer
    }
}

impl Default for SimpleNodePlacer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for SimpleNodePlacer {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Simple node placement", 1.0);

        let spacings = graph
            .get_property(InternalProperties::SPACINGS)
            .unwrap_or_else(|| {
                panic!("Missing spacings configuration for simple node placement");
            });

        let layers = graph.layers().clone();
        let mut max_height = 0.0;

        for layer_ref in layers.iter() {
            let nodes = layer_ref
                .lock().nodes().clone();

            let mut layer_height = 0.0;
            let mut last_node = None;
            for node in nodes.iter() {
                if let Some(prev) = &last_node {
                    layer_height += spacings.get_vertical_spacing(node, prev);
                }
                {
                    let mut node_guard = node.lock();
                    let size_y = node_guard.shape().size_ref().y;
                    let margin_top = node_guard.margin().top;
                    let margin_bottom = node_guard.margin().bottom;
                    layer_height += margin_top + size_y + margin_bottom;
                }
                last_node = Some(node.clone());
            }

            {
                let mut layer_guard = layer_ref.lock();
                layer_guard.size().y = layer_height;
            }
            if layer_height > max_height {
                max_height = layer_height;
            }
        }

        for layer_ref in layers.iter() {
            let (layer_height, nodes) = {
                let layer_guard = layer_ref.lock();
                (layer_guard.size_ref().y, layer_guard.nodes().clone())
            };

            let mut pos = (max_height - layer_height) / 2.0;
            let mut last_node = None;
            for node in nodes {
                if let Some(prev) = &last_node {
                    pos += spacings.get_vertical_spacing(&node, prev);
                }
                {
                    let mut node_guard = node.lock();
                    let margin_top = node_guard.margin().top;
                    let margin_bottom = node_guard.margin().bottom;
                    let size_y = node_guard.shape().size_ref().y;
                    pos += margin_top;
                    node_guard.shape().position().y = pos;
                    pos += size_y + margin_bottom;
                }
                last_node = Some(node);
            }
        }

        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        if graph
            .get_property(InternalProperties::GRAPH_PROPERTIES)
            .is_some_and(|props| props.contains(&GraphProperties::ExternalPorts))
        {
            Some(LayoutProcessorConfiguration::create_from(
                &HIERARCHY_PROCESSING_ADDITIONS,
            ))
        } else {
            None
        }
    }
}
