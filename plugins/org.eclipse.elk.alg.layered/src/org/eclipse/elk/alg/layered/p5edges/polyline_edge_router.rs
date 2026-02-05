use std::collections::HashSet;
use std::sync::{Arc, LazyLock};

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, IElkProgressMonitor};

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LGraph, LGraphUtil, LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{GraphProperties, InternalProperties, LayeredOptions};
use crate::org::eclipse::elk::alg::layered::LayeredPhases;
use crate::org::eclipse::elk::alg::layered::options::internal_properties::Origin;

static BASELINE_PROCESSOR_CONFIGURATION: LazyLock<
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
        .add(Arc::new(IntermediateProcessorStrategy::SelfLoopPortRestorer))
        .add(Arc::new(IntermediateProcessorStrategy::SelfLoopRouter));
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

pub struct PolylineEdgeRouter {
    created_junction_points: HashSet<KVector>,
}

impl PolylineEdgeRouter {
    pub fn new() -> Self {
        PolylineEdgeRouter {
            created_junction_points: HashSet::new(),
        }
    }

    pub(crate) fn is_external_west_or_east_port(node: &LNodeRef) -> bool {
        let Ok(mut node_guard) = node.lock() else {
            return false;
        };
        if node_guard.node_type() != NodeType::ExternalPort {
            return false;
        }
        let side = node_guard
            .get_property(InternalProperties::EXT_PORT_SIDE)
            .unwrap_or(PortSide::Undefined);
        matches!(side, PortSide::West | PortSide::East)
    }

    fn process_node(&mut self, node: &LNodeRef, layer_left_x_pos: f64, max_acceptable_x_diff: f64) {
        let layer_right_x_pos = node
            .lock()
            .ok()
            .and_then(|node_guard| node_guard.layer())
            .and_then(|layer| layer.lock().ok().map(|layer_guard| layer_guard.size_ref().x))
            .map(|size_x| layer_left_x_pos + size_x)
            .unwrap_or(layer_left_x_pos);

        let ports = node
            .lock()
            .ok()
            .map(|node_guard| node_guard.ports().clone())
            .unwrap_or_default();

        for port in ports {
            let Some(mut absolute_port_anchor) = port.lock().ok().and_then(|port_guard| port_guard.absolute_anchor())
            else {
                continue;
            };

            if node
                .lock()
                .ok()
                .map(|node_guard| node_guard.node_type() == NodeType::NorthSouthPort)
                .unwrap_or(false)
            {
                let origin_port = port
                    .lock()
                    .ok()
                    .and_then(|mut port_guard| port_guard.get_property(InternalProperties::ORIGIN));
                if let Some(Origin::LPort(origin_port)) = origin_port {
                    if let Some(origin_anchor) = origin_port.lock().ok().and_then(|p| p.absolute_anchor()) {
                        absolute_port_anchor.x = origin_anchor.x;
                        if let Ok(mut node_guard) = node.lock() {
                            node_guard.shape().position().x = absolute_port_anchor.x;
                        }
                    }
                }
            }

            let mut bend_point = KVector::with_values(0.0, absolute_port_anchor.y);

            let port_side = port
                .lock()
                .ok()
                .map(|port_guard| port_guard.side())
                .unwrap_or(PortSide::Undefined);
            match port_side {
                PortSide::East => bend_point.x = layer_right_x_pos,
                PortSide::West => bend_point.x = layer_left_x_pos,
                _ => continue,
            }

            let x_distance = (absolute_port_anchor.x - bend_point.x).abs();
            if x_distance <= max_acceptable_x_diff && !Self::is_in_layer_dummy(node) {
                continue;
            }

            let add_junction_point = port
                .lock()
                .ok()
                .map(|port_guard| {
                    port_guard.incoming_edges().len() + port_guard.outgoing_edges().len() > 1
                })
                .unwrap_or(false);

            let connected_edges = port
                .lock()
                .ok()
                .map(|port_guard| port_guard.connected_edges())
                .unwrap_or_default();
            for edge in connected_edges {
                let other_port = edge
                    .lock()
                    .ok()
                    .map(|edge_guard| edge_guard.other_port(&port));

                let Some(other_port) = other_port else {
                    continue;
                };
                let other_anchor_y = other_port
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.absolute_anchor())
                    .map(|anchor| anchor.y)
                    .unwrap_or(absolute_port_anchor.y);
                if (other_anchor_y - bend_point.y).abs() > MIN_VERT_DIFF {
                    self.add_bend_point(&edge, &bend_point, add_junction_point, &port);
                }
            }
        }
    }

    fn process_in_layer_edge(&self, edge: &LEdgeRef, layer_x_pos: f64, edge_spacing: f64) {
        let Some((source_port, target_port)) = edge
            .lock()
            .ok()
            .and_then(|edge_guard| Some((edge_guard.source()?, edge_guard.target()?)))
        else {
            return;
        };

        let source_anchor_y = source_port
            .lock()
            .ok()
            .and_then(|port_guard| port_guard.absolute_anchor())
            .map(|anchor| anchor.y)
            .unwrap_or(0.0);
        let target_anchor_y = target_port
            .lock()
            .ok()
            .and_then(|port_guard| port_guard.absolute_anchor())
            .map(|anchor| anchor.y)
            .unwrap_or(0.0);
        let mid_y = (source_anchor_y + target_anchor_y) / 2.0;

        let bend_point = if source_port
            .lock()
            .ok()
            .map(|port_guard| port_guard.side() == PortSide::East)
            .unwrap_or(false)
        {
            let layer_width = source_port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.node())
                .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.layer()))
                .and_then(|layer| layer.lock().ok().map(|layer_guard| layer_guard.size_ref().x))
                .unwrap_or(0.0);
            KVector::with_values(layer_x_pos + layer_width + edge_spacing, mid_y)
        } else {
            KVector::with_values(layer_x_pos - edge_spacing, mid_y)
        };

        if let Ok(mut edge_guard) = edge.lock() {
            edge_guard.bend_points().add_first_values(bend_point.x, bend_point.y);
        }
    }

    fn calculate_west_in_layer_edge_y_diff(&self, layer: &crate::org::eclipse::elk::alg::layered::graph::LayerRef) -> f64 {
        let nodes = layer
            .lock()
            .ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();
        let mut max_y_diff: f64 = 0.0;
        for node in nodes {
            let outgoing_edges = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();
            for edge in outgoing_edges {
                let Some((source_port, target_port)) = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| Some((edge_guard.source()?, edge_guard.target()?)))
                else {
                    continue;
                };
                let target_layer = target_port
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.node())
                    .and_then(|node_ref| node_ref.lock().ok().and_then(|node_guard| node_guard.layer()));
                if target_layer
                    .as_ref()
                    .map(|target_layer| Arc::ptr_eq(target_layer, layer))
                    .unwrap_or(false)
                    && source_port
                        .lock()
                        .ok()
                        .map(|port_guard| port_guard.side() == PortSide::West)
                        .unwrap_or(false)
                {
                    let source_pos = source_port
                        .lock()
                        .ok()
                        .and_then(|port_guard| port_guard.absolute_anchor())
                        .map(|anchor| anchor.y)
                        .unwrap_or(0.0);
                    let target_pos = target_port
                        .lock()
                        .ok()
                        .and_then(|port_guard| port_guard.absolute_anchor())
                        .map(|anchor| anchor.y)
                        .unwrap_or(0.0);
                    max_y_diff = max_y_diff.max((target_pos - source_pos).abs());
                }
            }
        }
        max_y_diff
    }

    fn add_bend_point(
        &mut self,
        edge: &LEdgeRef,
        bend_point: &KVector,
        add_junction_point: bool,
        curr_port: &crate::org::eclipse::elk::alg::layered::graph::LPortRef,
    ) {
        let should_add = edge
            .lock()
            .ok()
            .map(|edge_guard| {
                !edge_guard.is_self_loop()
                    && (edge_guard.is_in_layer_edge()
                        || curr_port
                            .lock()
                            .ok()
                            .and_then(|port_guard| port_guard.absolute_anchor())
                            .map(|anchor| anchor != *bend_point)
                            .unwrap_or(true))
            })
            .unwrap_or(false);

        if !should_add {
            return;
        }

        let is_source = edge
            .lock()
            .ok()
            .and_then(|edge_guard| edge_guard.source())
            .map(|source| Arc::ptr_eq(&source, curr_port))
            .unwrap_or(false);

        if let Ok(mut edge_guard) = edge.lock() {
            if is_source {
                edge_guard
                    .bend_points()
                    .add_first_values(bend_point.x, bend_point.y);
            } else {
                edge_guard.bend_points().add_vector(*bend_point);
            }

            if add_junction_point && !self.created_junction_points.contains(bend_point) {
                let mut junction_points = edge_guard
                    .get_property(LayeredOptions::JUNCTION_POINTS)
                    .unwrap_or_else(KVectorChain::new);
                junction_points.add_vector(*bend_point);
                edge_guard.set_property(LayeredOptions::JUNCTION_POINTS, Some(junction_points));
                self.created_junction_points.insert(*bend_point);
            }
        }
    }

    fn is_in_layer_dummy(node: &LNodeRef) -> bool {
        if node
            .lock()
            .ok()
            .map(|node_guard| node_guard.node_type() == NodeType::LongEdge)
            .unwrap_or(false)
        {
            let edges = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.connected_edges())
                .unwrap_or_default();
            return edges.iter().any(|edge| {
                edge.lock()
                    .ok()
                    .map(|edge_guard| edge_guard.is_in_layer_edge())
                    .unwrap_or(false)
            });
        }
        false
    }
}

impl Default for PolylineEdgeRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for PolylineEdgeRouter {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Polyline edge routing", 1.0);

        let sloped_edge_zone_width = layered_graph
            .get_property(LayeredOptions::EDGE_ROUTING_POLYLINE_SLOPED_EDGE_ZONE_WIDTH)
            .unwrap_or(0.0);
        let node_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS)
            .unwrap_or(0.0);
        let edge_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_EDGE_EDGE_BETWEEN_LAYERS)
            .unwrap_or(0.0);
        let edge_space_fac = if node_spacing.abs() < f64::EPSILON {
            0.0
        } else {
            (edge_spacing / node_spacing).min(1.0)
        };

        let mut xpos = 0.0;
        let layers = layered_graph.layers().clone();
        if let Some(first_layer) = layers.first() {
            let y_diff = self.calculate_west_in_layer_edge_y_diff(first_layer);
            xpos = LAYER_SPACE_FAC * edge_space_fac * y_diff;
        }

        let mut layer_index = 0;
        while layer_index < layers.len() {
            let layer = &layers[layer_index];
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            let external_layer = nodes
                .iter()
                .all(|node| PolylineEdgeRouter::is_external_west_or_east_port(node));

            if external_layer && xpos > 0.0 {
                xpos -= node_spacing;
            }

            LGraphUtil::place_nodes_horizontally(layer, xpos);

            let mut max_vert_diff: f64 = 0.0;

            for node in nodes {
                let outgoing_edges = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.outgoing_edges())
                    .unwrap_or_default();

                let mut max_curr_output_y_diff: f64 = 0.0;
                for edge in outgoing_edges {
                    let Some((source_port, target_port)) = edge
                        .lock()
                        .ok()
                        .and_then(|edge_guard| Some((edge_guard.source()?, edge_guard.target()?)))
                    else {
                        continue;
                    };
                    let source_pos = source_port
                        .lock()
                        .ok()
                        .and_then(|port_guard| port_guard.absolute_anchor())
                        .map(|anchor| anchor.y)
                        .unwrap_or(0.0);
                    let target_pos = target_port
                        .lock()
                        .ok()
                        .and_then(|port_guard| port_guard.absolute_anchor())
                        .map(|anchor| anchor.y)
                        .unwrap_or(0.0);

                    let target_layer = target_port
                        .lock()
                        .ok()
                        .and_then(|port_guard| port_guard.node())
                        .and_then(|node_ref| node_ref.lock().ok().and_then(|node_guard| node_guard.layer()));

                    if target_layer
                        .as_ref()
                        .map(|target_layer| Arc::ptr_eq(target_layer, layer))
                        .unwrap_or(false)
                        && !edge.lock().ok().map(|edge_guard| edge_guard.is_self_loop()).unwrap_or(false)
                    {
                        let y_diff = (source_pos - target_pos).abs();
                        self.process_in_layer_edge(
                            &edge,
                            xpos,
                            LAYER_SPACE_FAC * edge_space_fac * y_diff,
                        );

                        if source_port
                            .lock()
                            .ok()
                            .map(|port_guard| port_guard.side() == PortSide::West)
                            .unwrap_or(false)
                        {
                            max_curr_output_y_diff = max_curr_output_y_diff.max(0.0);
                            continue;
                        }
                    }

                    max_curr_output_y_diff =
                        max_curr_output_y_diff.max((target_pos - source_pos).abs());
                }

                let node_type = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.node_type())
                    .unwrap_or(NodeType::Normal);
                match node_type {
                    NodeType::Normal
                    | NodeType::Label
                    | NodeType::LongEdge
                    | NodeType::NorthSouthPort
                    | NodeType::BreakingPoint => {
                        self.process_node(&node, xpos, sloped_edge_zone_width);
                    }
                    _ => {}
                }

                max_vert_diff = max_vert_diff.max(max_curr_output_y_diff);
            }

            if layer_index + 1 < layers.len() {
                let y_diff = self.calculate_west_in_layer_edge_y_diff(&layers[layer_index + 1]);
                max_vert_diff = max_vert_diff.max(y_diff);
            }

            let mut layer_spacing = LAYER_SPACE_FAC * edge_space_fac * max_vert_diff;
            if !external_layer && layer_index + 1 < layers.len() {
                layer_spacing += node_spacing;
            }

            let layer_width = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.size_ref().x)
                .unwrap_or(0.0);
            xpos += layer_width + layer_spacing;

            layer_index += 1;
        }

        self.created_junction_points.clear();
        layered_graph.size().x = xpos;

        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        let graph_properties = graph
            .get_property_ref(InternalProperties::GRAPH_PROPERTIES)
            .unwrap_or_else(EnumSet::none_of);

        let mut configuration = LayoutProcessorConfiguration::create_from(&BASELINE_PROCESSOR_CONFIGURATION);

        if graph_properties.contains(&GraphProperties::NorthSouthPorts) {
            configuration.add_all(&NORTH_SOUTH_PORT_PROCESSING_ADDITIONS);
        }

        if graph_properties.contains(&GraphProperties::SelfLoops) {
            configuration.add_all(&SELF_LOOP_PROCESSING_ADDITIONS);
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

const MIN_VERT_DIFF: f64 = 1.0;
const LAYER_SPACE_FAC: f64 = 0.4;
