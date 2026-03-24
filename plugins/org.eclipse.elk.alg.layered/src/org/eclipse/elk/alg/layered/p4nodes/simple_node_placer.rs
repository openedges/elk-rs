use std::sync::{Arc, LazyLock};

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{ArenaSync, LGraph};
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

        let mut sync = ArenaSync::from_lgraph(graph);

        let layer_ids: Vec<_> = sync.arena().all_layer_ids().collect();
        let mut max_height = 0.0;

        // First pass: compute layer heights
        for &layer_id in &layer_ids {
            let node_ids: Vec<_> = sync.arena().layer_nodes(layer_id).to_vec();

            let mut layer_height = 0.0;
            let mut last_nid = None;
            for &nid in &node_ids {
                if let Some(prev_nid) = last_nid {
                    let node_ref = sync.node_ref(nid);
                    let prev_ref = sync.node_ref(prev_nid);
                    layer_height += spacings.get_vertical_spacing(node_ref, prev_ref);
                }
                let margin_top = sync.arena().node_margin(nid).top;
                let size_y = sync.arena().node_size(nid).y;
                let margin_bottom = sync.arena().node_margin(nid).bottom;
                layer_height += margin_top + size_y + margin_bottom;
                last_nid = Some(nid);
            }

            sync.arena_mut().layer_size_mut(layer_id).y = layer_height;
            if layer_height > max_height {
                max_height = layer_height;
            }
        }

        // Second pass: set node y-positions
        for &layer_id in &layer_ids {
            let layer_height = sync.arena().layer_size(layer_id).y;
            let node_ids: Vec<_> = sync.arena().layer_nodes(layer_id).to_vec();

            let mut pos = (max_height - layer_height) / 2.0;
            let mut last_nid = None;
            for &nid in &node_ids {
                if let Some(prev_nid) = last_nid {
                    let node_ref = sync.node_ref(nid);
                    let prev_ref = sync.node_ref(prev_nid);
                    pos += spacings.get_vertical_spacing(node_ref, prev_ref);
                }
                let margin_top = sync.arena().node_margin(nid).top;
                let size_y = sync.arena().node_size(nid).y;
                let margin_bottom = sync.arena().node_margin(nid).bottom;
                pos += margin_top;
                sync.arena_mut().node_pos_mut(nid).y = pos;
                pos += size_y + margin_bottom;
                last_nid = Some(nid);
            }
        }

        // Sync positions and layer sizes back to Arc graph
        sync.sync_positions_to_graph();
        sync.sync_layer_sizes_to_graph();

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
