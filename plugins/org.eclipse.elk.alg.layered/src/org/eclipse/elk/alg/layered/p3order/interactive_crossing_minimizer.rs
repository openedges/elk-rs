use std::cmp::Ordering;
use std::sync::{Arc, LazyLock};

use rustc_hash::FxHashMap;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InteractiveReferencePoint, InternalProperties, LayeredOptions, Origin,
};
use crate::org::eclipse::elk::alg::layered::p3order::counting::{
    init_initializables, IInitializable,
};
use crate::org::eclipse::elk::alg::layered::p3order::i_sweep_port_distributor::ISweepPortDistributor;
use crate::org::eclipse::elk::alg::layered::p3order::NodeRelativePortDistributor;
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

static INTERMEDIATE_PROCESSING_CONFIGURATION: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut configuration = LayoutProcessorConfiguration::create();
    configuration
        .add_before(
            LayeredPhases::P3NodeOrdering,
            Arc::new(IntermediateProcessorStrategy::LongEdgeSplitter),
        )
        .add_before(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::InLayerConstraintProcessor),
        )
        .add_after(
            LayeredPhases::P5EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::LongEdgeJoiner),
        );
    configuration
});

pub struct InteractiveCrossingMinimizer;

impl InteractiveCrossingMinimizer {
    pub fn new() -> Self {
        InteractiveCrossingMinimizer
    }

    fn node_ptr_id(node: &LNodeRef) -> usize {
        Arc::as_ptr(node) as usize
    }

    fn has_successor_constraint(node: &LNodeRef, successor: &LNodeRef) -> bool {
        node.lock_ok()
            .and_then(|mut node_guard| {
                node_guard.get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
            })
            .map(|constraints| {
                constraints
                    .iter()
                    .any(|candidate| Arc::ptr_eq(candidate, successor))
            })
            .unwrap_or(false)
    }

    fn compare_nodes_by_pos(
        node1: &LNodeRef,
        node2: &LNodeRef,
        positions: &FxHashMap<usize, f64>,
    ) -> Ordering {
        let pos1 = positions
            .get(&Self::node_ptr_id(node1))
            .copied()
            .unwrap_or(0.0);
        let pos2 = positions
            .get(&Self::node_ptr_id(node2))
            .copied()
            .unwrap_or(0.0);

        let compare = pos1.partial_cmp(&pos2).unwrap_or(Ordering::Equal);
        if compare != Ordering::Equal {
            return compare;
        }

        if Self::has_successor_constraint(node1, node2) {
            return Ordering::Less;
        }
        if Self::has_successor_constraint(node2, node1) {
            return Ordering::Greater;
        }

        Ordering::Equal
    }

    fn get_pos(
        node: &LNodeRef,
        horiz_pos: f64,
        interactive_reference_point: InteractiveReferencePoint,
    ) -> f64 {
        let node_type = node
            .lock_ok()
            .map(|node_guard| node_guard.node_type())
            .unwrap_or(NodeType::Normal);

        match node_type {
            NodeType::LongEdge => {
                if let Some(position) = Self::long_edge_position(node, horiz_pos) {
                    return position;
                }
            }
            NodeType::NorthSouthPort => {
                if let Some(position) = Self::north_south_port_position(node) {
                    return position;
                }
            }
            _ => {}
        }

        node.lock_ok()
            .map(|mut node_guard| {
                let (pos_y, size_y) = {
                    let shape = node_guard.shape();
                    (shape.position_ref().y, shape.size_ref().y)
                };
                match interactive_reference_point {
                    InteractiveReferencePoint::Center => pos_y + size_y / 2.0,
                    InteractiveReferencePoint::TopLeft => pos_y,
                }
            })
            .unwrap_or(0.0)
    }

    fn long_edge_position(node: &LNodeRef, horiz_pos: f64) -> Option<f64> {
        let edge = node
            .lock_ok()
            .and_then(|mut node_guard| node_guard.get_property(InternalProperties::ORIGIN))
            .and_then(|origin| match origin {
                Origin::LEdge(edge) => Some(edge),
                _ => None,
            })?;

        let mut bend_points = edge
            .lock_ok()
            .map(|edge_guard| edge_guard.bend_points_ref().clone())
            .unwrap_or_default();
        let reversed = edge
            .lock_ok()
            .and_then(|mut edge_guard| edge_guard.get_property(InternalProperties::REVERSED))
            .unwrap_or(false);
        if reversed {
            bend_points = KVectorChain::reverse(&bend_points);
        }

        let source = node.lock_ok().and_then(|mut node_guard| {
            node_guard.get_property(InternalProperties::LONG_EDGE_SOURCE)
        });
        if let Some(source_point) = source.and_then(|source_port| {
            source_port
                .lock_ok()
                .and_then(|source_guard| source_guard.absolute_anchor())
        }) {
            if horiz_pos <= source_point.x {
                return Some(source_point.y);
            }
            bend_points.insert(0, source_point);
        }

        let target = node.lock_ok().and_then(|mut node_guard| {
            node_guard.get_property(InternalProperties::LONG_EDGE_TARGET)
        });
        if let Some(target_point) = target.and_then(|target_port| {
            target_port
                .lock_ok()
                .and_then(|target_guard| target_guard.absolute_anchor())
        }) {
            if target_point.x <= horiz_pos {
                return Some(target_point.y);
            }
            bend_points.add_vector(target_point);
        }

        let points = bend_points.to_array();
        if points.len() < 2 {
            return None;
        }

        let mut point1 = points[0];
        let mut point2 = points[1];
        for point in points.iter().skip(2) {
            if point2.x >= horiz_pos {
                break;
            }
            point1 = point2;
            point2 = *point;
        }

        let delta_x = point2.x - point1.x;
        if delta_x.abs() <= f64::EPSILON {
            return Some(point2.y);
        }

        Some(point1.y + (horiz_pos - point1.x) / delta_x * (point2.y - point1.y))
    }

    fn north_south_port_position(node: &LNodeRef) -> Option<f64> {
        let dummy_port = node
            .lock_ok()
            .and_then(|node_guard| node_guard.ports().first().cloned())?;
        let origin_port = dummy_port
            .lock_ok()
            .and_then(|mut dummy_port_guard| {
                dummy_port_guard.get_property(InternalProperties::ORIGIN)
            })
            .and_then(|origin| match origin {
                Origin::LPort(port) => Some(port),
                _ => None,
            })?;

        let side = origin_port
            .lock_ok()
            .map(|origin_port_guard| origin_port_guard.side())
            .unwrap_or(PortSide::Undefined);
        let origin_node = origin_port
            .lock_ok()
            .and_then(|origin_port_guard| origin_port_guard.node())?;
        let (node_y, node_height) = origin_node.lock_ok().map(|mut origin_node_guard| {
            (
                origin_node_guard.shape().position_ref().y,
                origin_node_guard.shape().size_ref().y,
            )
        })?;

        match side {
            PortSide::North => Some(node_y),
            PortSide::South => Some(node_y + node_height),
            _ => None,
        }
    }
}

impl Default for InteractiveCrossingMinimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for InteractiveCrossingMinimizer {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Interactive crossing minimization", 1.0);

        let layers = layered_graph.layers().clone();
        for (layer_index, layer_ref) in layers.iter().enumerate() {
            if let Some(mut layer_guard) = layer_ref.lock_ok() {
                layer_guard.graph_element().id = layer_index as i32;
            }
        }

        let node_order = layered_graph.to_node_array();
        let interactive_reference_point = layered_graph
            .get_property(LayeredOptions::INTERACTIVE_REFERENCE_POINT)
            .unwrap_or(InteractiveReferencePoint::Center);
        let mut port_distributor = NodeRelativePortDistributor::new(node_order.len());
        let mut initializables: Vec<&mut dyn IInitializable> = vec![&mut port_distributor];
        init_initializables(&mut initializables, &node_order);

        let mut port_count = 0i32;
        for (layer_index, layer_ref) in layers.iter().enumerate() {
            let layer_nodes = layer_ref
                .lock_ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            let mut horiz_pos = 0.0;
            let mut positioned_nodes = 0usize;
            for node in &layer_nodes {
                if let Some(mut node_guard) = node.lock_ok() {
                    if node_guard.shape().position_ref().x > 0.0 {
                        horiz_pos += node_guard.shape().position_ref().x
                            + node_guard.shape().size_ref().x / 2.0;
                        positioned_nodes += 1;
                    }
                    for port in node_guard.ports() {
                        if let Some(mut port_guard) = port.lock_ok() {
                            port_guard.shape().graph_element().id = port_count;
                        }
                        port_count += 1;
                    }
                }
            }
            if positioned_nodes > 0 {
                horiz_pos /= positioned_nodes as f64;
            }

            let mut positions: FxHashMap<usize, f64> = FxHashMap::with_capacity_and_hasher(layer_nodes.len(), Default::default());
            for (node_index, node) in layer_nodes.iter().enumerate() {
                let pos = Self::get_pos(node, horiz_pos, interactive_reference_point);
                if let Some(mut node_guard) = node.lock_ok() {
                    node_guard.shape().graph_element().id = node_index as i32;
                    if node_guard.node_type() == NodeType::LongEdge {
                        node_guard.set_property(
                            InternalProperties::ORIGINAL_DUMMY_NODE_POSITION,
                            Some(pos),
                        );
                    }
                }
                positions.insert(Self::node_ptr_id(node), pos);
            }

            if let Some(mut layer_guard) = layer_ref.lock_ok() {
                layer_guard
                    .nodes_mut()
                    .sort_by(|node1, node2| Self::compare_nodes_by_pos(node1, node2, &positions));
            }

            port_distributor.distribute_ports_while_sweeping(&node_order, layer_index, true);
        }

        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        let mut configuration =
            LayoutProcessorConfiguration::create_from(&INTERMEDIATE_PROCESSING_CONFIGURATION);
        let graph_properties = graph
            .get_property_ref(InternalProperties::GRAPH_PROPERTIES)
            .unwrap_or_default();
        if graph_properties.contains(&GraphProperties::NonFreePorts) {
            configuration.add_before(
                LayeredPhases::P3NodeOrdering,
                Arc::new(IntermediateProcessorStrategy::PortListSorter),
            );
        }

        Some(configuration)
    }
}
