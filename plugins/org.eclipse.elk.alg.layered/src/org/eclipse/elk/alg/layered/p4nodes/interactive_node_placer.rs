use std::sync::{Arc, LazyLock};

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::l_node::NodeType;
use crate::org::eclipse::elk::alg::layered::graph::{ArenaSync, LGraph, NodeId};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, Spacings,
};
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

        let mut sync = ArenaSync::from_lgraph(graph);

        let layer_ids: Vec<_> = sync.arena().all_layer_ids().collect();
        for layer_id in layer_ids {
            let node_ids: Vec<_> = sync.arena().layer_nodes(layer_id).to_vec();
            place_nodes(&mut sync, &node_ids, &spacings);
        }

        // Sync node positions back to Arc graph
        sync.sync_positions_to_graph();

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

fn place_nodes(sync: &mut ArenaSync, node_ids: &[NodeId], spacings: &Spacings) {
    let mut min_valid_y = f64::NEG_INFINITY;
    let mut prev_node_type = NodeType::Normal;

    for &nid in node_ids {
        let node_type = sync.arena().node_type(nid);
        let spacing = spacings.get_vertical_spacing_for_types(node_type, prev_node_type);

        let mut pos_y = sync.arena().node_pos(nid).y;
        if node_type != NodeType::Normal {
            let original: Option<f64> = sync
                .arena()
                .node_properties(nid)
                .get_property(InternalProperties::ORIGINAL_DUMMY_NODE_POSITION);
            if let Some(original) = original {
                pos_y = original;
            } else {
                min_valid_y = min_valid_y.max(0.0);
                pos_y = min_valid_y + spacing;
            }
        }

        let margin_top = sync.arena().node_margin(nid).top;
        let margin_bottom = sync.arena().node_margin(nid).bottom;
        let size_y = sync.arena().node_size(nid).y;

        if pos_y < min_valid_y + spacing + margin_top {
            pos_y = min_valid_y + spacing + margin_top;
        }

        sync.arena_mut().node_pos_mut(nid).y = pos_y;
        min_valid_y = pos_y + size_y + margin_bottom;
        prev_node_type = node_type;
    }
}
