
use crate::common::issue_support::init_layered_options;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LNode, LPort, Layer, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::HypernodeProcessor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

#[test]
fn moves_hypernode_to_first_join_point_and_rewrites_source_port() {
    init_layered_options();

    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    if let Some(mut graph_guard) = graph.lock_ok() {
        graph_guard.layers_mut().push(layer.clone());
        graph_guard.size().x = 220.0;
    }

    let hypernode = LNode::new(&graph);
    if let Some(mut node_guard) = hypernode.lock_ok() {
        node_guard.set_node_type(NodeType::Normal);
        node_guard.shape().position().x = 10.0;
        node_guard.shape().position().y = 20.0;
        node_guard.shape().size().x = 20.0;
        node_guard.shape().size().y = 20.0;
        node_guard.set_property(LayeredOptions::HYPERNODE, Some(true));
    }
    LNode::set_layer(&hypernode, Some(layer.clone()));

    let source_port = LPort::new();
    if let Some(mut port_guard) = source_port.lock_ok() {
        port_guard.set_side(PortSide::East);
        port_guard.shape().position().x = 20.0;
        port_guard.shape().position().y = 10.0;
    }
    LPort::set_node(&source_port, Some(hypernode.clone()));

    let target_node = LNode::new(&graph);
    if let Some(mut node_guard) = target_node.lock_ok() {
        node_guard.shape().position().x = 160.0;
        node_guard.shape().position().y = 80.0;
        node_guard.shape().size().x = 20.0;
        node_guard.shape().size().y = 20.0;
    }
    LNode::set_layer(&target_node, Some(layer.clone()));

    let target_port = LPort::new();
    if let Some(mut port_guard) = target_port.lock_ok() {
        port_guard.set_side(PortSide::West);
        port_guard.shape().position().x = 0.0;
        port_guard.shape().position().y = 10.0;
    }
    LPort::set_node(&target_port, Some(target_node.clone()));

    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source_port.clone()));
    LEdge::set_target(&edge, Some(target_port.clone()));
    if let Some(mut edge_guard) = edge.lock_ok() {
        edge_guard
            .bend_points()
            .add_vector(KVector::with_values(100.0, 100.0));
        edge_guard
            .bend_points()
            .add_vector(KVector::with_values(130.0, 140.0));

        let mut junction_points = KVectorChain::new();
        junction_points.add_vector(KVector::with_values(100.0, 100.0));
        junction_points.add_vector(KVector::with_values(170.0, 140.0));
        edge_guard.set_property(LayeredOptions::JUNCTION_POINTS, Some(junction_points));
    }

    let mut processor = HypernodeProcessor;
    let mut monitor = NullElkProgressMonitor;
    if let Some(mut graph_guard) = graph.lock_ok() {
        processor.process(&mut graph_guard, &mut monitor);
    }

    let moved_x = hypernode
        .lock_ok()
        .map(|mut node_guard| node_guard.shape().position_ref().x)
        .unwrap_or(f64::NAN);
    assert!((moved_x - 90.0).abs() < 1e-9);

    let ports = hypernode
        .lock_ok()
        .map(|node_guard| node_guard.ports().clone())
        .unwrap_or_default();
    assert_eq!(ports.len(), 3);

    let new_source = edge
        .lock_ok()
        .and_then(|edge_guard| edge_guard.source())
        .expect("edge source exists");
    assert!(!std::sync::Arc::ptr_eq(&new_source, &source_port));
    let new_source_side = new_source
        .lock_ok()
        .map(|port_guard| port_guard.side())
        .unwrap_or(PortSide::Undefined);
    assert!(matches!(new_source_side, PortSide::North | PortSide::South));

    let bend_points = edge
        .lock_ok()
        .map(|edge_guard| edge_guard.bend_points_ref().to_array())
        .unwrap_or_default();
    assert_eq!(bend_points.len(), 1);
    assert_eq!(bend_points[0], KVector::with_values(130.0, 140.0));

    let junction_points = edge
        .lock_ok()
        .and_then(|mut edge_guard| edge_guard.get_property(LayeredOptions::JUNCTION_POINTS))
        .map(|chain| chain.to_array())
        .unwrap_or_default();
    assert_eq!(junction_points.len(), 1);
    assert_eq!(junction_points[0], KVector::with_values(170.0, 140.0));
}
