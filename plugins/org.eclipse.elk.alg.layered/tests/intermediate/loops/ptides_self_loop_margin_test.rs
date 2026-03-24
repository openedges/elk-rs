// Test that an EAST->WEST opposing self-loop on a FixedPos compound node correctly extends
// margin.left to accommodate the self-loop routing.
//
// This reproduces the ptides_actuatorpattern N12 scenario where:
// - N12 has PORT_CONSTRAINTS=FixedPos (set after its own recursive layout)
// - P33(EAST) and P34(WEST) are self-loop ports with index=-1,2 respectively
// - P32(WEST) and P35(EAST) are regular connected ports with index=0,3
// - E21 is the self-loop P33(EAST)->P34(WEST)
// - P34 is at x=-8, so InnermostNodeMarginCalculator sets margin.left=8
// - After SelfLoopRouter, margin.left should extend to 28 because WEST uses slot 1 in this setup

use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LNode, LPort, Layer,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::{
    SelfLoopPortRestorer, SelfLoopPreProcessor, SelfLoopRouter,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn init_layered_metadata() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn run_processor(processor: &mut dyn ILayoutProcessor<LGraph>, graph: &Arc<Mutex<LGraph>>) {
    let mut monitor = NullElkProgressMonitor;
    processor.process(&mut graph.lock(), &mut monitor);
}

/// Build a graph that matches the N12 scenario in ptides_actuatorpattern:
/// - Node size: 121x30 (example N12 size)
/// - P33 (EAST, index=-1 -> id=0): at x=113, y=7, size=8x8
/// - P32 (WEST, index=0 -> id=1): at x=-8, y=7, size=8x8 (has external connection)
/// - P34 (WEST, index=2 -> id=2): at x=-8, y=15, size=8x8 (self-loop target)
/// - P35 (EAST, index=3 -> id=3): at x=113, y=15, size=8x8 (has external connection)
/// - E21: self-loop P33(EAST)->P34(WEST)
/// - margin.left pre-set to 8 (from P34/P32 at x=-8)
///
/// After SelfLoopRouter:
/// - occupied = {WEST, NORTH, EAST}
/// - routing_slot_count[WEST] = 2 and this loop uses WEST slot 1
/// - positions[WEST][1] = -(8+10) - edge_edge_distance = -28
/// - margin.left should extend to 28
#[test]
fn opposing_east_west_self_loop_fixedpos_extends_west_margin() {
    init_layered_metadata();

    let graph = LGraph::new();

    // Create the compound node (N12 analog)
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock();        // Node size: approximate N12 from ptides model
        node_guard.shape().size().x = 121.0;
        node_guard.shape().size().y = 30.0;
        node_guard.shape().position().x = 0.0;
        node_guard.shape().position().y = 0.0;
        // FixedPos: what the PORT_CONSTRAINTS escalation sets after compound node's own layout
        node_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedPos),
        );
        node_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
        // Pre-set margin.left=8 (as InnermostNodeMarginCalculator would compute from P34 at x=-8)
        node_guard.margin().left = 8.0;
    }
    graph
        .lock()
        
        .layerless_nodes_mut()
        .push(node.clone());

    // P33 (EAST, port_index=-1 -> will become id=0 after assign_port_ids sorts by index)
    // Position: x=113, y=7 inside node, size=8x8
    let p33 = LPort::new();
    {
        let mut p = p33.lock();        p.set_side(PortSide::East);
        p.shape().position().x = 113.0;
        p.shape().position().y = 7.0;
        p.shape().size().x = 8.0;
        p.shape().size().y = 8.0;
        // Simulate PORT_INDEX = -1 (sorts first)
        p.set_property(LayeredOptions::PORT_INDEX, Some(-1i32));
    }
    LPort::set_node(&p33, Some(node.clone()));

    // P32 (WEST, port_index=0 -> id=1)
    // Position: x=-8, y=7, size=8x8; has external connection
    let p32 = LPort::new();
    {
        let mut p = p32.lock();        p.set_side(PortSide::West);
        p.shape().position().x = -8.0;
        p.shape().position().y = 7.0;
        p.shape().size().x = 8.0;
        p.shape().size().y = 8.0;
        p.set_property(LayeredOptions::PORT_INDEX, Some(0i32));
    }
    LPort::set_node(&p32, Some(node.clone()));

    // P34 (WEST, port_index=2 -> id=2) - self-loop target
    // Position: x=-8, y=15, size=8x8
    let p34 = LPort::new();
    {
        let mut p = p34.lock();        p.set_side(PortSide::West);
        p.shape().position().x = -8.0;
        p.shape().position().y = 15.0;
        p.shape().size().x = 8.0;
        p.shape().size().y = 8.0;
        p.set_property(LayeredOptions::PORT_INDEX, Some(2i32));
    }
    LPort::set_node(&p34, Some(node.clone()));

    // P35 (EAST, port_index=3 -> id=3)
    // Position: x=113, y=15, size=8x8; has external connection
    let p35 = LPort::new();
    {
        let mut p = p35.lock();        p.set_side(PortSide::East);
        p.shape().position().x = 113.0;
        p.shape().position().y = 15.0;
        p.shape().size().x = 8.0;
        p.shape().size().y = 8.0;
        p.set_property(LayeredOptions::PORT_INDEX, Some(3i32));
    }
    LPort::set_node(&p35, Some(node.clone()));

    // E21: self-loop P33(EAST)->P34(WEST)
    let e21 = LEdge::new();
    LEdge::set_source(&e21, Some(p33.clone()));
    LEdge::set_target(&e21, Some(p34.clone()));

    // Add external node to give P32 and P35 connected edges (so they get CONNECTED penalty)
    let external = LNode::new(&graph);
    {
        let mut ext_guard = external.lock();        ext_guard.shape().size().x = 20.0;
        ext_guard.shape().size().y = 20.0;
    }
    graph
        .lock()
        
        .layerless_nodes_mut()
        .push(external.clone());
    let ext_east = LPort::new();
    {
        let mut p = ext_east.lock();        p.set_side(PortSide::East);
        p.shape().position().x = 20.0;
        p.shape().position().y = 10.0;
        p.shape().size().x = 8.0;
        p.shape().size().y = 8.0;
    }
    LPort::set_node(&ext_east, Some(external.clone()));
    let ext_west = LPort::new();
    {
        let mut p = ext_west.lock();        p.set_side(PortSide::West);
        p.shape().position().x = 0.0;
        p.shape().position().y = 10.0;
        p.shape().size().x = 8.0;
        p.shape().size().y = 8.0;
    }
    LPort::set_node(&ext_west, Some(external.clone()));

    // Connect P32 and P35 to external node
    let _e_p32 = {
        let e = LEdge::new();
        LEdge::set_source(&e, Some(p32.clone()));
        LEdge::set_target(&e, Some(ext_west.clone()));
        e
    };
    let _e_p35 = {
        let e = LEdge::new();
        LEdge::set_source(&e, Some(ext_east.clone()));
        LEdge::set_target(&e, Some(p35.clone()));
        e
    };

    // Run SelfLoopPreProcessor (installs holder, hides E21 but NOT ports since FixedPos)
    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);

    // Check that the self-loop holder was installed for the node
    {
        use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::InternalProperties;
        let has_holder = node
            .lock()
            
            .get_property(InternalProperties::SELF_LOOP_HOLDER)
            .is_some();
        assert!(has_holder, "SelfLoopHolder should be installed for N12 analog (FixedPos with self-loop)");
    }

    // Move node to a layer (required for SelfLoopPortRestorer and SelfLoopRouter)
    let layer = Layer::new(&graph);
    graph
        .lock()
        
        .layers_mut()
        .push(layer.clone());
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));

    // Run SelfLoopPortRestorer (for FixedPos: ports_hidden=false, only computes self-loop types)
    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);

    // Run SelfLoopRouter (the processor under test)
    let mut router = SelfLoopRouter;
    run_processor(&mut router, &graph);

    // Verify: margin.left should extend from 8 to 28.
    // In this setup WEST gets slot 1 (not 0), so one edge-edge spacing step is added:
    // baseline = -(margin.left + node_self_loop_distance) = -(8 + 10) = -18
    // slot 1 position = -18 - edge_edge_distance(10) = -28
    // update_margins_with_point(-28) -> margin.left = max(8, 28) = 28
    let margin_left = {
        let mut node_guard = node.lock();        node_guard.margin().left
    };

    assert!(
        (margin_left - 28.0).abs() < 1.0,
        "Expected margin.left=28 (WEST opposing self-loop uses slot 1 and extends from initial 8), got {margin_left}"
    );
}

/// Verify the self-loop type is correctly detected as TwoSidesOpposing for E(EAST)->W(WEST)
#[test]
fn east_west_fixedpos_self_loop_detected_as_two_sides_opposing() {
    init_layered_metadata();

    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock();        node_guard.shape().size().x = 121.0;
        node_guard.shape().size().y = 30.0;
        node_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedPos),
        );
        node_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
    }
    graph
        .lock()
        
        .layerless_nodes_mut()
        .push(node.clone());

    let p_east = LPort::new();
    {
        let mut p = p_east.lock();        p.set_side(PortSide::East);
        p.shape().position().x = 113.0;
        p.shape().position().y = 7.0;
        p.shape().size().x = 8.0;
        p.shape().size().y = 8.0;
        p.set_property(LayeredOptions::PORT_INDEX, Some(-1i32));
    }
    LPort::set_node(&p_east, Some(node.clone()));

    let p_west = LPort::new();
    {
        let mut p = p_west.lock();        p.set_side(PortSide::West);
        p.shape().position().x = -8.0;
        p.shape().position().y = 15.0;
        p.shape().size().x = 8.0;
        p.shape().size().y = 8.0;
        p.set_property(LayeredOptions::PORT_INDEX, Some(2i32));
    }
    LPort::set_node(&p_west, Some(node.clone()));

    let _e21 = {
        let e = LEdge::new();
        LEdge::set_source(&e, Some(p_east.clone()));
        LEdge::set_target(&e, Some(p_west.clone()));
        e
    };

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);

    let layer = Layer::new(&graph);
    graph
        .lock()
        
        .layers_mut()
        .push(layer.clone());
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));

    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);

    use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::loops::SelfLoopType;
    use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::InternalProperties;

    // After SelfLoopPortRestorer, check sl_loop type
    let sl_loop_type = node
        .lock()
        
        .get_property(InternalProperties::SELF_LOOP_HOLDER)
        .and_then(|holder| {
            holder.lock().sl_hyper_loops().first().cloned()
        })
        .and_then(|sl_loop| {
            sl_loop
                .lock().self_loop_type()
        });

    assert_eq!(
        sl_loop_type,
        Some(SelfLoopType::TwoSidesOpposing),
        "EAST->WEST self-loop should be TwoSidesOpposing, got {sl_loop_type:?}"
    );
}

/// Diagnostic test: dump intermediate state after each stage to identify where margin.left fails to extend
#[test]
fn debug_trace_margin_extension() {
    init_layered_metadata();

    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock();        node_guard.shape().size().x = 121.0;
        node_guard.shape().size().y = 30.0;
        node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedPos));
        node_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
        node_guard.margin().left = 8.0;
    }
    graph.lock().layerless_nodes_mut().push(node.clone());

    let p33 = LPort::new();
    {
        let mut p = p33.lock();        p.set_side(PortSide::East);
        p.shape().position().x = 113.0;
        p.shape().position().y = 7.0;
        p.shape().size().x = 8.0;
        p.shape().size().y = 8.0;
        p.set_property(LayeredOptions::PORT_INDEX, Some(-1i32));
    }
    LPort::set_node(&p33, Some(node.clone()));

    let p32 = LPort::new();
    {
        let mut p = p32.lock();        p.set_side(PortSide::West);
        p.shape().position().x = -8.0;
        p.shape().position().y = 7.0;
        p.shape().size().x = 8.0;
        p.shape().size().y = 8.0;
        p.set_property(LayeredOptions::PORT_INDEX, Some(0i32));
    }
    LPort::set_node(&p32, Some(node.clone()));

    let p34 = LPort::new();
    {
        let mut p = p34.lock();        p.set_side(PortSide::West);
        p.shape().position().x = -8.0;
        p.shape().position().y = 15.0;
        p.shape().size().x = 8.0;
        p.shape().size().y = 8.0;
        p.set_property(LayeredOptions::PORT_INDEX, Some(2i32));
    }
    LPort::set_node(&p34, Some(node.clone()));

    let p35 = LPort::new();
    {
        let mut p = p35.lock();        p.set_side(PortSide::East);
        p.shape().position().x = 113.0;
        p.shape().position().y = 15.0;
        p.shape().size().x = 8.0;
        p.shape().size().y = 8.0;
        p.set_property(LayeredOptions::PORT_INDEX, Some(3i32));
    }
    LPort::set_node(&p35, Some(node.clone()));

    let e21 = LEdge::new();
    LEdge::set_source(&e21, Some(p33.clone()));
    LEdge::set_target(&e21, Some(p34.clone()));

    let external = LNode::new(&graph);
    {
        let mut ext_guard = external.lock();        ext_guard.shape().size().x = 20.0;
        ext_guard.shape().size().y = 20.0;
    }
    graph.lock().layerless_nodes_mut().push(external.clone());

    let ext_east = LPort::new();
    {
        let mut p = ext_east.lock();        p.set_side(PortSide::East);
        p.shape().position().x = 20.0;
        p.shape().position().y = 10.0;
        p.shape().size().x = 8.0;
        p.shape().size().y = 8.0;
    }
    LPort::set_node(&ext_east, Some(external.clone()));
    let ext_west = LPort::new();
    {
        let mut p = ext_west.lock();        p.set_side(PortSide::West);
        p.shape().position().x = 0.0;
        p.shape().position().y = 10.0;
        p.shape().size().x = 8.0;
        p.shape().size().y = 8.0;
    }
    LPort::set_node(&ext_west, Some(external.clone()));

    let _e_p32 = {
        let e = LEdge::new();
        LEdge::set_source(&e, Some(p32.clone()));
        LEdge::set_target(&e, Some(ext_west.clone()));
        e
    };
    let _e_p35 = {
        let e = LEdge::new();
        LEdge::set_source(&e, Some(ext_east.clone()));
        LEdge::set_target(&e, Some(p35.clone()));
        e
    };

    // Check initial state
    {
        let is_self_loop = e21.lock().is_self_loop();
        eprintln!("[DIAG] e21.is_self_loop() before preprocessor = {}", is_self_loop);
        let node_guard = node.lock();        eprintln!("[DIAG] node.outgoing_edges().len() = {}", node_guard.outgoing_edges().len());
        eprintln!("[DIAG] node.ports().len() = {}", node_guard.ports().len());
    }

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);

    {
        use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::InternalProperties;
        let holder = node.lock().get_property(InternalProperties::SELF_LOOP_HOLDER);
        eprintln!("[DIAG] After preprocessor: has_holder = {}", holder.is_some());
        if let Some(holder) = holder {
            let holder_guard = holder.lock();            let loops = holder_guard.sl_hyper_loops();
            eprintln!("[DIAG] sl_hyper_loops.len() = {}", loops.len());
            for (i, sl_loop) in loops.iter().enumerate() {
                let loop_guard = sl_loop.lock();                eprintln!("[DIAG] loop[{}]: sl_edges.len()={}, sl_ports.len()={}, self_loop_type={:?}",
                    i, loop_guard.sl_edges().len(), loop_guard.sl_ports().len(), loop_guard.self_loop_type());
                eprintln!("[DIAG] loop[{}]: occupied_port_sides={:?}", i, loop_guard.occupied_port_sides());
            }
        }
    }

    let layer = Layer::new(&graph);
    graph.lock().layers_mut().push(layer.clone());
    LNode::set_layer(&node, Some(layer));
    graph.lock().layerless_nodes_mut().retain(|candidate| !Arc::ptr_eq(candidate, &node));

    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);

    {
        use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::InternalProperties;
        let holder = node.lock().get_property(InternalProperties::SELF_LOOP_HOLDER);
        if let Some(holder) = holder {
            let holder_guard = holder.lock();            let loops = holder_guard.sl_hyper_loops();
            eprintln!("[DIAG] After restorer: sl_hyper_loops.len() = {}", loops.len());
            for (i, sl_loop) in loops.iter().enumerate() {
                let loop_guard = sl_loop.lock();                eprintln!("[DIAG] loop[{}]: self_loop_type={:?}, sl_edges.len()={}", i, loop_guard.self_loop_type(), loop_guard.sl_edges().len());
            }
            eprintln!("[DIAG] holder.routing_slot_count = {:?}", holder_guard.routing_slot_count());
        }
    }

    let mut router = SelfLoopRouter;
    run_processor(&mut router, &graph);

    {
        use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::InternalProperties;
        use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide as PS;
        let holder = node.lock().get_property(InternalProperties::SELF_LOOP_HOLDER);
        if let Some(holder) = holder {
            let holder_guard = holder.lock();            eprintln!("[DIAG] After router: holder.routing_slot_count = {:?}", holder_guard.routing_slot_count());
            let loops = holder_guard.sl_hyper_loops();
            for (i, sl_loop) in loops.iter().enumerate() {
                let loop_guard = sl_loop.lock();                eprintln!("[DIAG2] loop[{}]: occupied_port_sides={:?}", i, loop_guard.occupied_port_sides());
                eprintln!("[DIAG2] loop[{}]: routing_slots N={} E={} S={} W={}",
                    i,
                    loop_guard.routing_slot(PS::North),
                    loop_guard.routing_slot(PS::East),
                    loop_guard.routing_slot(PS::South),
                    loop_guard.routing_slot(PS::West),
                );
                if let Some(lp) = loop_guard.leftmost_port() {
                    let lport = lp.lock().l_port().clone();
                    let side = lport.lock().side();
                    eprintln!("[DIAG2] loop[{}]: leftmost_side={:?}", i, side);
                }
                if let Some(rp) = loop_guard.rightmost_port() {
                    let lport = rp.lock().l_port().clone();
                    let side = lport.lock().side();
                    eprintln!("[DIAG2] loop[{}]: rightmost_side={:?}", i, side);
                }
            }
        }
    }

    let margin_left = node.lock().margin().left;
    eprintln!("[DIAG] After router: margin.left = {}", margin_left);

    // Just print, don't assert, to see full trace
    eprintln!("[DIAG] Expected margin.left = 28.0");
}
