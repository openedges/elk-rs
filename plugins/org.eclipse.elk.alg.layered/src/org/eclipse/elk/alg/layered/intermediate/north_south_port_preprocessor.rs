use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

use crate::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphUtil, LNode, LNodeRef, LPort, LPortRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::internal_properties::Origin;
use crate::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions, OrderingStrategy,
};

pub struct NorthSouthPortPreprocessor;

fn trace_ns(message: &str) {
    if ElkTrace::global().ns {
        eprintln!("[north-south-pre] {message}");
    }
}

impl Default for NorthSouthPortPreprocessor {
    fn default() -> Self {
        NorthSouthPortPreprocessor
    }
}

impl ILayoutProcessor<LGraph> for NorthSouthPortPreprocessor {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Odd port side processing", 1.0);
        trace_ns("process:start");

        let layers = graph.layers().clone();
        trace_ns(&format!("layers={}", layers.len()));
        for layer in layers {
            let node_array = {
                let layer_guard = layer.lock();
                LGraphUtil::to_node_array(layer_guard.nodes())
            };
            trace_ns(&format!("layer_nodes={}", node_array.len()));

            let mut pointer: isize = -1;
            for node in node_array {
                pointer += 1;
                let node_key = std::sync::Arc::as_ptr(&node) as usize;
                trace_ns(&format!("node={node_key} pointer={pointer} begin"));

                let (node_type, port_constraints, north_ports, south_ports, graph_ref) = {
                    let mut node_guard = match node.try_lock() {            Some(guard) => guard,
            None => continue,
                    };
                    let node_type = node_guard.node_type();
                    let port_constraints = node_guard
                        .get_property(LayeredOptions::PORT_CONSTRAINTS)
                        .unwrap_or(PortConstraints::Undefined);
                    let model_order_strategy = node_guard
                        .graph()
                        .as_ref()
                        .and_then(|graph_ref| graph_ref.try_lock())
                        .and_then(|graph_guard| {
                            graph_guard.get_property(LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY)
                        })
                        .unwrap_or(OrderingStrategy::None);

                    if !port_constraints.is_order_fixed()
                        && model_order_strategy == OrderingStrategy::None
                    {
                        sort_port_list(&mut node_guard);
                    }

                    let mut north_ports = node_guard.ports_by_side(PortSide::North);
                    let mut south_ports = node_guard.ports_by_side(PortSide::South);

                    if model_order_strategy != OrderingStrategy::None {
                        north_ports = model_order_north_south_input_reversing(&north_ports);
                    }
                    south_ports.reverse();
                    if model_order_strategy != OrderingStrategy::None {
                        south_ports = model_order_north_south_input_reversing(&south_ports);
                    }

                    let graph_ref = node_guard.graph();
                    (
                        node_type,
                        port_constraints,
                        north_ports,
                        south_ports,
                        graph_ref,
                    )
                };

                if node_type != NodeType::Normal || !port_constraints.is_side_fixed() {
                    trace_ns(&format!(
                        "node={node_key} skip node_type={:?} side_fixed={}",
                        node_type,
                        port_constraints.is_side_fixed()
                    ));
                    continue;
                }

                let Some(graph_ref) = graph_ref else {
                    trace_ns(&format!("node={node_key} skip graph_ref=none"));
                    continue;
                };

                if let Some(mut node_guard) = node.try_lock() {
                    node_guard
                        .set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, Some(node.clone()));
                }

                if north_ports.is_empty() && south_ports.is_empty() {
                    trace_ns(&format!("node={node_key} skip no north/south ports"));
                    continue;
                }
                trace_ns(&format!(
                    "node={node_key} north_ports={} south_ports={}",
                    north_ports.len(),
                    south_ports.len()
                ));

                let mut north_dummy_nodes = Vec::new();
                let mut south_dummy_nodes = Vec::new();
                let mut barycenter_associates = Vec::new();

                trace_ns(&format!("node={node_key} create_dummy_nodes north:start"));
                create_dummy_nodes(
                    &graph_ref,
                    &north_ports,
                    &mut north_dummy_nodes,
                    Some(&mut south_dummy_nodes),
                    &mut barycenter_associates,
                );
                trace_ns(&format!(
                    "node={node_key} create_dummy_nodes north:done north_dummies={} south_dummies={}",
                    north_dummy_nodes.len(),
                    south_dummy_nodes.len()
                ));
                trace_ns(&format!("node={node_key} create_dummy_nodes south:start"));
                create_dummy_nodes(
                    &graph_ref,
                    &south_ports,
                    &mut south_dummy_nodes,
                    None,
                    &mut barycenter_associates,
                );
                trace_ns(&format!(
                    "node={node_key} create_dummy_nodes south:done north_dummies={} south_dummies={} bary={}",
                    north_dummy_nodes.len(),
                    south_dummy_nodes.len(),
                    barycenter_associates.len()
                ));

                let insert_point = pointer as usize;
                let successor = node.clone();
                trace_ns(&format!("node={node_key} north insertion:start"));
                for dummy in &north_dummy_nodes {
                    LNode::set_layer_at_index(dummy, insert_point, Some(layer.clone()));
                    pointer += 1;

                    if let Some(mut dummy_guard) = dummy.try_lock() {
                        dummy_guard.set_property(
                            InternalProperties::IN_LAYER_LAYOUT_UNIT,
                            Some(node.clone()),
                        );
                    }

                    if !origin_port_allows_switch(dummy) {
                        if let Some(mut dummy_guard) = dummy.try_lock() {
                            let mut constraints = dummy_guard
                                .get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
                                .unwrap_or_default();
                            constraints.push(successor.clone());
                            dummy_guard.set_property(
                                InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS,
                                Some(constraints),
                            );
                        }
                    }
                }
                trace_ns(&format!("node={node_key} north insertion:done"));

                let predecessor = node.clone();
                trace_ns(&format!("node={node_key} south insertion:start"));
                for dummy in &south_dummy_nodes {
                    LNode::set_layer_at_index(dummy, (pointer + 1) as usize, Some(layer.clone()));
                    pointer += 1;

                    if let Some(mut dummy_guard) = dummy.try_lock() {
                        dummy_guard.set_property(
                            InternalProperties::IN_LAYER_LAYOUT_UNIT,
                            Some(node.clone()),
                        );
                    }

                    if !origin_port_allows_switch(dummy) {
                        if let Some(mut pred_guard) = predecessor.try_lock() {
                            let mut constraints = pred_guard
                                .get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
                                .unwrap_or_default();
                            constraints.push(dummy.clone());
                            pred_guard.set_property(
                                InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS,
                                Some(constraints),
                            );
                        }
                    }
                }
                trace_ns(&format!("node={node_key} south insertion:done"));

                if !barycenter_associates.is_empty() {
                    if let Some(mut node_guard) = node.try_lock() {
                        node_guard.set_property(
                            InternalProperties::BARYCENTER_ASSOCIATES,
                            Some(barycenter_associates),
                        );
                    }
                }
                trace_ns(&format!("node={node_key} done"));
            }
        }

        trace_ns("process:done");
        monitor.done();
    }
}

fn sort_port_list(node: &mut LNode) {
    let mut ports = node.ports().clone();
    if ports.len() <= 1 {
        return;
    }

    let mut ports_with_metadata = Vec::with_capacity(ports.len());
    let mut in_ports = 0i32;
    let mut in_out_ports = ports.len() as i32;
    let mut out_ports = 2 * ports.len() as i32;
    for (index, port) in ports.drain(..).enumerate() {
        let (side, incoming, outgoing) = {
            let guard = port.lock();
            (
                guard.side(),
                !guard.incoming_edges().is_empty(),
                !guard.outgoing_edges().is_empty(),
            )
        };

        let id = match side {
            PortSide::East | PortSide::West | PortSide::Undefined => -1i32,
            PortSide::North | PortSide::South => {
                if incoming && outgoing {
                    let id = in_out_ports;
                    in_out_ports += 1;
                    id
                } else if incoming {
                    let id = in_ports;
                    in_ports += 1;
                    id
                } else if outgoing {
                    let id = out_ports;
                    out_ports += 1;
                    id
                } else {
                    let id = in_ports;
                    in_ports += 1;
                    id
                }
            }
        };

        ports_with_metadata.push((index, port, side, id));
    }

    ports_with_metadata.sort_by(
        |(index_left, _, side_left, id_left), (index_right, _, side_right, id_right)| {
            if side_left != side_right {
                return side_left.cmp(side_right);
            }

            if id_left == id_right {
                return index_left.cmp(index_right);
            }

            if *side_left == PortSide::North {
                id_left.cmp(id_right)
            } else {
                id_right.cmp(id_left)
            }
        },
    );

    *node.ports_mut() = ports_with_metadata
        .into_iter()
        .map(|(_, port, _, _)| port)
        .collect();
    node.cache_port_sides();
}

fn model_order_north_south_input_reversing(ports: &[LPortRef]) -> Vec<LPortRef> {
    let mut incoming_ports = Vec::new();
    let mut outgoing_ports = Vec::new();

    for port in ports {
        let is_incoming = {
            let port_guard = port.lock();
            !port_guard.incoming_edges().is_empty()
        };
        if is_incoming {
            incoming_ports.push(port.clone());
        } else {
            outgoing_ports.push(port.clone());
        }
    }

    incoming_ports.reverse();
    incoming_ports.append(&mut outgoing_ports);
    incoming_ports
}

fn create_dummy_nodes(
    graph: &crate::org::eclipse::elk::alg::layered::graph::LGraphRef,
    ports: &[LPortRef],
    dummy_nodes: &mut Vec<LNodeRef>,
    mut opposing_side_dummy_nodes: Option<&mut Vec<LNodeRef>>,
    barycenter_associates: &mut Vec<LNodeRef>,
) {
    let mut in_ports = Vec::with_capacity(ports.len());
    let mut out_ports = Vec::with_capacity(ports.len());
    let mut in_out_ports = Vec::with_capacity(ports.len());
    let mut same_side_self_loop_edges = Vec::new();
    let mut north_south_self_loop_edges = Vec::new();

    for port in ports {
        let port_side = port
            .lock().side();
        let outgoing = {
            let port_guard = port.lock();
            LGraphUtil::to_edge_array(port_guard.outgoing_edges())
        };

        for edge in outgoing {
            let (source_port, target_port) = {
                let edge_guard = edge.lock();
                (edge_guard.source(), edge_guard.target())
            };
            let (Some(source_port), Some(target_port)) = (source_port, target_port) else {
                continue;
            };
            let source_node = source_port
                .lock().node();
            let target_node = target_port
                .lock().node();
            let (Some(source_node), Some(target_node)) = (source_node, target_node) else {
                continue;
            };

            if !std::sync::Arc::ptr_eq(&source_node, &target_node) {
                continue;
            }

            let target_side = target_port
                .lock().side();
            if port_side == target_side {
                same_side_self_loop_edges.push(edge);
            } else if port_side == PortSide::North && target_side == PortSide::South {
                north_south_self_loop_edges.push(edge);
            }
        }
    }

    for edge in north_south_self_loop_edges {
        if let Some(opposing_side_dummy_nodes) = opposing_side_dummy_nodes.as_deref_mut() {
            create_north_south_self_loop_dummy_nodes(
                graph,
                &edge,
                dummy_nodes,
                opposing_side_dummy_nodes,
                PortSide::East,
            );
        }
    }

    for edge in same_side_self_loop_edges {
        create_same_side_self_loop_dummy_node(graph, &edge, dummy_nodes);
    }

    for port in ports {
        let (has_in, has_out) = {
            let port_guard = port.lock();
            (
                !port_guard.incoming_edges().is_empty(),
                !port_guard.outgoing_edges().is_empty(),
            )
        };
        if has_in && has_out {
            in_out_ports.push(port.clone());
        } else if has_in {
            in_ports.push(port.clone());
        } else if has_out {
            out_ports.push(port.clone());
        }
    }

    for in_port in in_ports {
        let dummy = create_dummy_node(graph, Some(&in_port), None, dummy_nodes);
        barycenter_associates.push(dummy);
    }

    for out_port in out_ports {
        let dummy = create_dummy_node(graph, None, Some(&out_port), dummy_nodes);
        barycenter_associates.push(dummy);
    }

    for in_out_port in in_out_ports {
        let dummy = create_dummy_node(graph, Some(&in_out_port), Some(&in_out_port), dummy_nodes);
        barycenter_associates.push(dummy);
    }
}

fn create_dummy_node(
    graph: &crate::org::eclipse::elk::alg::layered::graph::LGraphRef,
    in_port: Option<&LPortRef>,
    out_port: Option<&LPortRef>,
    dummy_nodes: &mut Vec<LNodeRef>,
) -> LNodeRef {
    let dummy = LNode::new(graph);
    {
        let mut dummy_guard = dummy.lock();
        dummy_guard.set_node_type(NodeType::NorthSouthPort);
        dummy_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedPos),
        );
    }

    if let Some(in_port) = in_port {
        let dummy_input_port = LPort::new();
        {
            let mut port_guard = dummy_input_port.lock();
            port_guard.set_property(
                InternalProperties::ORIGIN,
                Some(Origin::LPort(in_port.clone())),
            );
            port_guard.set_side(PortSide::West);
        }
        LPort::set_node(&dummy_input_port, Some(dummy.clone()));

        let edges = {
            let port_guard = in_port.lock();
            LGraphUtil::to_edge_array(port_guard.incoming_edges())
        };
        for edge in edges {
            crate::org::eclipse::elk::alg::layered::graph::LEdge::set_target(
                &edge,
                Some(dummy_input_port.clone()),
            );
        }

        {
            let mut port_guard = in_port.lock();
            port_guard.set_property(InternalProperties::PORT_DUMMY, Some(dummy.clone()));
            if let Some(node) = port_guard.node() {
                {
                    let mut dummy_guard = dummy.lock();
                    dummy_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LNode(node)));
                }
            }
        }
    }

    if let Some(out_port) = out_port {
        let dummy_output_port = LPort::new();
        {
            let mut port_guard = dummy_output_port.lock();
            port_guard.set_property(
                InternalProperties::ORIGIN,
                Some(Origin::LPort(out_port.clone())),
            );
            port_guard.set_side(PortSide::East);
        }
        LPort::set_node(&dummy_output_port, Some(dummy.clone()));

        let edges = {
            let port_guard = out_port.lock();
            LGraphUtil::to_edge_array(port_guard.outgoing_edges())
        };
        for edge in edges {
            crate::org::eclipse::elk::alg::layered::graph::LEdge::set_source(
                &edge,
                Some(dummy_output_port.clone()),
            );
        }

        {
            let mut port_guard = out_port.lock();
            port_guard.set_property(InternalProperties::PORT_DUMMY, Some(dummy.clone()));
            if let Some(node) = port_guard.node() {
                {
                    let mut dummy_guard = dummy.lock();
                    dummy_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LNode(node)));
                }
            }
        }
    }

    dummy_nodes.push(dummy.clone());
    dummy
}

fn create_same_side_self_loop_dummy_node(
    graph: &crate::org::eclipse::elk::alg::layered::graph::LGraphRef,
    self_loop: &crate::org::eclipse::elk::alg::layered::graph::LEdgeRef,
    dummy_nodes: &mut Vec<LNodeRef>,
) {
    let source_port = self_loop
        .lock().source();
    let target_port = self_loop
        .lock().target();
    let (Some(source_port), Some(target_port)) = (source_port, target_port) else {
        return;
    };

    let dummy = LNode::new(graph);
    {
        let mut dummy_guard = dummy.lock();
        dummy_guard.set_node_type(NodeType::NorthSouthPort);
        dummy_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedPos),
        );
        dummy_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LEdge(self_loop.clone())),
        );
    }

    let dummy_input_port = LPort::new();
    {
        let mut dummy_input_port_guard = dummy_input_port.lock();
        dummy_input_port_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(target_port.clone())),
        );
        dummy_input_port_guard.set_side(PortSide::West);
    }
    LPort::set_node(&dummy_input_port, Some(dummy.clone()));

    let dummy_output_port = LPort::new();
    {
        let mut dummy_output_port_guard = dummy_output_port.lock();
        dummy_output_port_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(source_port.clone())),
        );
        dummy_output_port_guard.set_side(PortSide::East);
    }
    LPort::set_node(&dummy_output_port, Some(dummy.clone()));

    {
        let mut source_port_guard = source_port.lock();
        source_port_guard.set_property(InternalProperties::PORT_DUMMY, Some(dummy.clone()));
    }
    {
        let mut target_port_guard = target_port.lock();
        target_port_guard.set_property(InternalProperties::PORT_DUMMY, Some(dummy.clone()));
    }

    crate::org::eclipse::elk::alg::layered::graph::LEdge::set_source(self_loop, None);
    crate::org::eclipse::elk::alg::layered::graph::LEdge::set_target(self_loop, None);

    dummy_nodes.push(dummy);
}

fn create_north_south_self_loop_dummy_nodes(
    graph: &crate::org::eclipse::elk::alg::layered::graph::LGraphRef,
    self_loop: &crate::org::eclipse::elk::alg::layered::graph::LEdgeRef,
    north_dummy_nodes: &mut Vec<LNodeRef>,
    south_dummy_nodes: &mut Vec<LNodeRef>,
    side: PortSide,
) {
    let source_port = self_loop
        .lock().source();
    let target_port = self_loop
        .lock().target();
    let (Some(source_port), Some(target_port)) = (source_port, target_port) else {
        return;
    };

    let north_dummy = LNode::new(graph);
    {
        let mut north_dummy_guard = north_dummy.lock();
        north_dummy_guard.set_node_type(NodeType::NorthSouthPort);
        north_dummy_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedPos),
        );
        if let Some(source_node) = source_port
            .lock().node()
        {
            north_dummy_guard
                .set_property(InternalProperties::ORIGIN, Some(Origin::LNode(source_node)));
        }
    }

    let north_dummy_output_port = LPort::new();
    {
        let mut north_dummy_output_port_guard = north_dummy_output_port.lock();
        north_dummy_output_port_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(source_port.clone())),
        );
        north_dummy_output_port_guard.set_side(side);
    }
    LPort::set_node(&north_dummy_output_port, Some(north_dummy.clone()));
    {
        let mut source_port_guard = source_port.lock();
        source_port_guard.set_property(InternalProperties::PORT_DUMMY, Some(north_dummy.clone()));
    }

    let south_dummy = LNode::new(graph);
    {
        let mut south_dummy_guard = south_dummy.lock();
        south_dummy_guard.set_node_type(NodeType::NorthSouthPort);
        south_dummy_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedPos),
        );
        if let Some(target_node) = target_port
            .lock().node()
        {
            south_dummy_guard
                .set_property(InternalProperties::ORIGIN, Some(Origin::LNode(target_node)));
        }
    }

    let south_dummy_input_port = LPort::new();
    {
        let mut south_dummy_input_port_guard = south_dummy_input_port.lock();
        south_dummy_input_port_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(target_port.clone())),
        );
        south_dummy_input_port_guard.set_side(side);
    }
    LPort::set_node(&south_dummy_input_port, Some(south_dummy.clone()));
    {
        let mut target_port_guard = target_port.lock();
        target_port_guard.set_property(InternalProperties::PORT_DUMMY, Some(south_dummy.clone()));
    }

    crate::org::eclipse::elk::alg::layered::graph::LEdge::set_source(
        self_loop,
        Some(north_dummy_output_port),
    );
    crate::org::eclipse::elk::alg::layered::graph::LEdge::set_target(
        self_loop,
        Some(south_dummy_input_port),
    );

    north_dummy_nodes.insert(0, north_dummy);
    south_dummy_nodes.push(south_dummy);
}

fn origin_port_allows_switch(dummy: &LNodeRef) -> bool {
    let origin_port = {
        let dummy_guard = dummy.lock();
        dummy_guard.ports().first().cloned()
    }
    .and_then(|port| {
        let port_guard = port.lock();
        port_guard.get_property(InternalProperties::ORIGIN)
    });
    let Some(Origin::LPort(origin_port)) = origin_port else {
        return false;
    };

    let (allows_switch, port_constraints, origin_node) = {
        let port_guard = origin_port.lock();
        (
            port_guard
                .get_property(LayeredOptions::ALLOW_NON_FLOW_PORTS_TO_SWITCH_SIDES)
                .unwrap_or(false),
            port_guard.get_property(LayeredOptions::PORT_CONSTRAINTS),
            port_guard.node(),
        )
    };

    let port_constraints = port_constraints
        .or_else(|| {
            origin_node.and_then(|node| {
                let node_guard = node.lock();
                node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS)
            })
        })
        .unwrap_or(PortConstraints::Undefined);

    if port_constraints.is_pos_fixed() {
        return false;
    }

    allows_switch
}
