use std::sync::{Arc, LazyLock};

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::l_node::NodeType;
use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNodeRef};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, Spacings};
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

pub struct InteractiveNodePlacer;

impl InteractiveNodePlacer {
    pub fn new() -> Self {
        InteractiveNodePlacer
    }
}

impl Default for InteractiveNodePlacer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for InteractiveNodePlacer {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Interactive node placement", 1.0);

        let spacings = graph
            .get_property(InternalProperties::SPACINGS)
            .unwrap_or_else(|| {
                panic!("Missing spacings configuration for interactive node placement");
            });

        let layers = graph.layers().clone();
        for layer_ref in layers {
            let nodes = layer_ref
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            place_nodes(&nodes, &spacings);
        }

        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        Some(LayoutProcessorConfiguration::create_from(
            &HIERARCHY_PROCESSING_ADDITIONS,
        ))
    }
}

fn place_nodes(nodes: &[LNodeRef], spacings: &Spacings) {
    let mut min_valid_y = f64::NEG_INFINITY;
    let mut prev_node_type = NodeType::Normal;

    for node in nodes {
        if let Ok(mut node_guard) = node.lock() {
            let node_type = node_guard.node_type();
            let spacing = spacings.get_vertical_spacing_for_types(node_type, prev_node_type);

            let mut pos_y = node_guard.shape().position_ref().y;
            if node_type != NodeType::Normal {
                let original =
                    node_guard.get_property(InternalProperties::ORIGINAL_DUMMY_NODE_POSITION);
                if let Some(original) = original {
                    pos_y = original;
                } else {
                    min_valid_y = min_valid_y.max(0.0);
                    pos_y = min_valid_y + spacing;
                }
            }

            let margin_top = node_guard.margin().top;
            let margin_bottom = node_guard.margin().bottom;
            let size_y = node_guard.shape().size_ref().y;

            if pos_y < min_valid_y + spacing + margin_top {
                pos_y = min_valid_y + spacing + margin_top;
            }

            node_guard.shape().position().y = pos_y;
            min_valid_y = pos_y + size_y + margin_bottom;
            prev_node_type = node_type;
        }
    }
}
