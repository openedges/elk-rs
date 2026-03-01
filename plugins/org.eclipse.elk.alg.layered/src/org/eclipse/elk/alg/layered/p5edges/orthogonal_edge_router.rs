use std::sync::Arc;
use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, IElkProgressMonitor};

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LGraphUtil};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions,
};
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::direction::RoutingDirection;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::OrthogonalRoutingGenerator;
use crate::org::eclipse::elk::alg::layered::p5edges::polyline_edge_router::PolylineEdgeRouter;
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

static TRACE_COMPOUND_WIDTH: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_COMPOUND_WIDTH").is_some());
static DISABLE_NS: LazyLock<bool> =
    LazyLock::new(|| std::env::var("ELK_DISABLE_NS").is_ok());

static HYPEREDGE_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config.add_before(
        LayeredPhases::P4NodePlacement,
        Arc::new(IntermediateProcessorStrategy::HyperedgeDummyMerger),
    );
    config
});

static INVERTED_PORT_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config.add_before(
        LayeredPhases::P3NodeOrdering,
        Arc::new(IntermediateProcessorStrategy::InvertedPortProcessor),
    );
    config
});

static NORTH_SOUTH_PORT_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .add_before(
            LayeredPhases::P3NodeOrdering,
            Arc::new(IntermediateProcessorStrategy::NorthSouthPortPreprocessor),
        )
        .add_after(
            LayeredPhases::P5EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::NorthSouthPortPostprocessor),
        );
    config
});

static HIERARCHICAL_PORT_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .add_before(
            LayeredPhases::P3NodeOrdering,
            Arc::new(IntermediateProcessorStrategy::HierarchicalPortConstraintProcessor),
        )
        .add_before(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::HierarchicalPortDummySizeProcessor),
        )
        .add_after(
            LayeredPhases::P5EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::HierarchicalPortOrthogonalEdgeRouter),
        );
    config
});

static SELF_LOOP_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .add_before(
            LayeredPhases::P1CycleBreaking,
            Arc::new(IntermediateProcessorStrategy::SelfLoopPreprocessor),
        )
        .add_after(
            LayeredPhases::P5EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::SelfLoopPostprocessor),
        )
        .before(LayeredPhases::P4NodePlacement)
        .add(Arc::new(
            IntermediateProcessorStrategy::SelfLoopPortRestorer,
        ))
        .add(Arc::new(IntermediateProcessorStrategy::SelfLoopRouter));
    config
});

static HYPERNODE_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config.add_after(
        LayeredPhases::P5EdgeRouting,
        Arc::new(IntermediateProcessorStrategy::HypernodeProcessor),
    );
    config
});

static CENTER_EDGE_LABEL_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .add_before(
            LayeredPhases::P2Layering,
            Arc::new(IntermediateProcessorStrategy::LabelDummyInserter),
        )
        .add_before(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::LabelDummySwitcher),
        )
        .add_before(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::LabelSideSelector),
        )
        .add_after(
            LayeredPhases::P5EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::LabelDummyRemover),
        );
    config
});

static END_EDGE_LABEL_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .add_before(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::LabelSideSelector),
        )
        .add_before(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::EndLabelPreprocessor),
        )
        .add_after(
            LayeredPhases::P5EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::EndLabelPostprocessor),
        );
    config
});

pub struct OrthogonalEdgeRouter;

impl OrthogonalEdgeRouter {
    pub fn new() -> Self {
        OrthogonalEdgeRouter
    }
}

impl Default for OrthogonalEdgeRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for OrthogonalEdgeRouter {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Orthogonal edge routing", 1.0);

        let node_node_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS)
            .unwrap_or(0.0);
        let edge_edge_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_EDGE_EDGE_BETWEEN_LAYERS)
            .unwrap_or(0.0);
        let edge_node_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_EDGE_NODE_BETWEEN_LAYERS)
            .unwrap_or(0.0);

        let mut routing_generator = OrthogonalRoutingGenerator::new(
            RoutingDirection::WestToEast,
            edge_edge_spacing,
            Some("phase5".to_string()),
        );

        let layers = layered_graph.layers().clone();
        if *TRACE_COMPOUND_WIDTH {
            let layer_info: Vec<String> = layers.iter().enumerate().map(|(i, layer)| {
                let node_count = layer.lock().ok().map(|g| g.nodes().len()).unwrap_or(0);
                let nodes_str = layer.lock().ok().map(|g| {
                    g.nodes().iter().map(|n| {
                        n.lock().ok().map(|ng| format!("{:?}",
                            ng.node_type()
                        )).unwrap_or("?".to_string())
                    }).collect::<Vec<_>>().join(", ")
                }).unwrap_or_default();
                format!("L{}[{}]: {}", i, node_count, nodes_str)
            }).collect();
            eprintln!("[compound-width] edge_router layers={} detail: {}", layers.len(), layer_info.join(" | "));
        }
        // Java uses `float xpos` (f32). Truncate through f32 at each step for parity.
        let mut xpos: f64 = 0.0;
        let mut left_layer = None;
        let mut left_layer_nodes = None;
        let mut left_layer_index: i32 = -1;

        let mut layer_index = 0;
        loop {
            let right_layer = if layer_index < layers.len() {
                Some(layers[layer_index].clone())
            } else {
                None
            };

            let right_layer_nodes = right_layer.as_ref().and_then(|layer| {
                layer
                    .lock()
                    .ok()
                    .map(|layer_guard| layer_guard.nodes().clone())
            });
            let right_layer_index = if right_layer.is_some() {
                layer_index as i32
            } else {
                layers.len() as i32 - 1
            };

            if let Some(left_layer_ref) = &left_layer {
                LGraphUtil::place_nodes_horizontally(left_layer_ref, xpos);
                let left_width = left_layer_ref
                    .lock()
                    .ok()
                    .map(|layer_guard| layer_guard.size_ref().x)
                    .unwrap_or(0.0);
                xpos = (xpos + left_width) as f32 as f64;
            }

            // Java: float startPos = leftLayer == null ? xpos : xpos + edgeNodeSpacing;
            let start_pos = if left_layer.is_none() {
                xpos as f32 as f64
            } else {
                (xpos + edge_node_spacing) as f32 as f64
            };

            let slots_count = routing_generator.route_edges(
                monitor,
                layered_graph,
                left_layer_nodes.as_deref(),
                left_layer_index,
                right_layer_nodes.as_deref(),
                start_pos,
            );

            let is_left_layer_external = left_layer
                .as_ref()
                .and_then(|layer| {
                    layer
                        .lock()
                        .ok()
                        .map(|layer_guard| layer_guard.nodes().clone())
                })
                .map(|nodes| {
                    nodes
                        .iter()
                        .all(PolylineEdgeRouter::is_external_west_or_east_port)
                })
                .unwrap_or(true);
            let is_right_layer_external = right_layer_nodes
                .as_ref()
                .map(|nodes| {
                    nodes
                        .iter()
                        .all(PolylineEdgeRouter::is_external_west_or_east_port)
                })
                .unwrap_or(true);

            if slots_count > 0 {
                // Java: float routingWidth — truncate through f32 at each step
                let mut routing_width =
                    ((slots_count as f32 - 1.0_f32) * edge_edge_spacing as f32) as f64;
                if left_layer.is_some() {
                    routing_width = (routing_width as f32 + edge_node_spacing as f32) as f64;
                }
                if right_layer.is_some() {
                    routing_width = (routing_width as f32 + edge_node_spacing as f32) as f64;
                }
                if routing_width < node_node_spacing
                    && !is_left_layer_external
                    && !is_right_layer_external
                {
                    routing_width = node_node_spacing;
                }
                xpos = (xpos + routing_width) as f32 as f64;
            } else if !is_left_layer_external && !is_right_layer_external {
                xpos = (xpos + node_node_spacing) as f32 as f64;
            }
            if *TRACE_COMPOUND_WIDTH {
                eprintln!("[compound-width] edge_router: layer_index={} xpos={} slots={} left_ext={} right_ext={}",
                    layer_index, xpos, slots_count, is_left_layer_external, is_right_layer_external);
            }

            left_layer = right_layer;
            left_layer_nodes = right_layer_nodes;
            left_layer_index = right_layer_index;

            if left_layer.is_none() {
                break;
            }
            layer_index += 1;
            if layer_index > layers.len() {
                break;
            }
        }

        if *TRACE_COMPOUND_WIDTH {
            eprintln!("[compound-width] edge_router: FINAL xpos={} graph_size_x={}", xpos, xpos as f32 as f64);
        }
        layered_graph.size().x = xpos as f32 as f64;
        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        let graph_properties = graph
            .get_property_ref(InternalProperties::GRAPH_PROPERTIES)
            .unwrap_or_else(EnumSet::none_of);
        let mut configuration = LayoutProcessorConfiguration::create();

        if graph_properties.contains(&GraphProperties::Hyperedges) {
            configuration.add_all(&HYPEREDGE_PROCESSING_ADDITIONS);
            configuration.add_all(&INVERTED_PORT_PROCESSING_ADDITIONS);
        }

        if graph_properties.contains(&GraphProperties::NonFreePorts)
            || graph
                .get_property_ref(LayeredOptions::FEEDBACK_EDGES)
                .unwrap_or(false)
        {
            configuration.add_all(&INVERTED_PORT_PROCESSING_ADDITIONS);
            if graph_properties.contains(&GraphProperties::NorthSouthPorts) && !*DISABLE_NS {
                configuration.add_all(&NORTH_SOUTH_PORT_PROCESSING_ADDITIONS);
            }
        }

        if graph_properties.contains(&GraphProperties::ExternalPorts) {
            configuration.add_all(&HIERARCHICAL_PORT_PROCESSING_ADDITIONS);
        }

        if graph_properties.contains(&GraphProperties::SelfLoops) {
            configuration.add_all(&SELF_LOOP_PROCESSING_ADDITIONS);
        }

        if graph_properties.contains(&GraphProperties::Hypernodes) {
            configuration.add_all(&HYPERNODE_PROCESSING_ADDITIONS);
        }

        if graph_properties.contains(&GraphProperties::CenterLabels) {
            configuration.add_all(&CENTER_EDGE_LABEL_PROCESSING_ADDITIONS);
        }

        if graph_properties.contains(&GraphProperties::EndLabels) {
            configuration.add_all(&END_EDGE_LABEL_PROCESSING_ADDITIONS);
        }

        Some(configuration)
    }
}
