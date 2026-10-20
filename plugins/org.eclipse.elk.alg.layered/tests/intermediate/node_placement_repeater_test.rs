use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LGraph, LNode, LPort, Layer, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph_configurator::GraphConfigurator;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::NodePlacementRepeater;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions, NodeFlexibility, NodePlacementStrategy, Spacings,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    PortConstraints, PortSide, SizeConstraint,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, NullElkProgressMonitor};

#[test]
fn second_placement_fixes_sizes_repositions_ports_and_disables_flexibility() {
    crate::common::issue_support::init_layered_options();
    for strategy in [
        NodePlacementStrategy::Simple,
        NodePlacementStrategy::Interactive,
        NodePlacementStrategy::LinearSegments,
        NodePlacementStrategy::BrandesKoepf,
        NodePlacementStrategy::NetworkSimplex,
    ] {
        let graph = LGraph::new();
        let spacings = Spacings::new(&graph);
        let layer = Layer::new(&graph);
        let node = LNode::new(&graph);
        {
            let mut node = node.lock();
            node.shape().size().x = 70.0;
            node.shape().size().y = 90.0;
            node.shape().position().x = 13.0;
            node.shape().position().y = 999.0;
            node.margin().right = 999.0;
            node.set_property(
                LayeredOptions::PORT_CONSTRAINTS,
                Some(PortConstraints::FixedSide),
            );
            node.set_property(
                LayeredOptions::NODE_SIZE_CONSTRAINTS,
                Some(EnumSet::of(&[SizeConstraint::MinimumSize])),
            );
            node.set_property(
                LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY,
                Some(NodeFlexibility::NodeSize),
            );
        }
        LNode::set_layer(&node, Some(layer.clone()));
        let port = LPort::new();
        LPort::set_node(&port, Some(node.clone()));
        {
            let mut port = port.lock();
            port.set_side(PortSide::East);
            port.shape().size().x = 4.0;
            port.shape().size().y = 4.0;
            // The first placer enlarged this node; this is the old border.
            port.shape().position().x = 30.0;
        }
        {
            let mut graph = graph.lock();
            graph.layers_mut().push(layer);
            graph.set_property(InternalProperties::SPACINGS, Some(spacings));
            graph.set_property(
                LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT,
                Some(NodeFlexibility::NodeSize),
            );
            graph.set_property(LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_RECOMPUTE_NODE_PLACEMENT, Some(strategy));
            NodePlacementRepeater.process(&mut graph, &mut NullElkProgressMonitor);
            assert_eq!(
                graph.get_property(
                    LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT
                ),
                Some(NodeFlexibility::None)
            );
        }
        let mut node = node.lock();
        assert_eq!(node.shape().size_ref().x, 70.0, "{strategy:?}");
        assert_eq!(node.shape().size_ref().y, 90.0, "{strategy:?}");
        assert_eq!(node.shape().position_ref().x, 13.0, "{strategy:?}");
        assert!(node.shape().position_ref().y.is_finite());
        assert!(node.shape().position_ref().y < 999.0, "{strategy:?}");
        assert_eq!(
            node.get_property(LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY),
            None
        );
        assert!(node
            .get_property(LayeredOptions::NODE_SIZE_CONSTRAINTS)
            .unwrap()
            .is_empty());
        assert_eq!(
            node.margin().right,
            4.0,
            "margin recomputed for {strategy:?}"
        );
        assert_eq!(
            port.lock().shape().position_ref().x,
            70.0,
            "port moved for {strategy:?}"
        );
    }
}

#[test]
fn repeater_resets_dummy_nodes_without_resizing_them() {
    crate::common::issue_support::init_layered_options();
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    let node = LNode::new(&graph);
    {
        let mut node = node.lock();
        node.set_node_type(NodeType::LongEdge);
        node.shape().size().y = 5.0;
        node.shape().position().y = 999.0;
    }
    LNode::set_layer(&node, Some(layer.clone()));
    let spacings = Spacings::new(&graph);
    let mut graph = graph.lock();
    graph.layers_mut().push(layer);
    graph.set_property(InternalProperties::SPACINGS, Some(spacings));
    graph.set_property(
        LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_RECOMPUTE_NODE_PLACEMENT,
        Some(NodePlacementStrategy::Simple),
    );
    NodePlacementRepeater.process(&mut graph, &mut NullElkProgressMonitor);
    let mut node = node.lock();
    assert_eq!(node.shape().size_ref().y, 5.0);
    assert_eq!(node.shape().position_ref().y, 0.0);
}

#[test]
fn configurator_schedules_repeater_before_graph_height_only_when_requested() {
    crate::common::issue_support::init_layered_options();
    for second_placer in [None, Some(NodePlacementStrategy::Simple)] {
        let graph = LGraph::new();
        let node = LNode::new(&graph);
        {
            let mut graph = graph.lock();
            graph.layerless_nodes_mut().push(node);
            graph.set_property(
                LayeredOptions::NODE_PLACEMENT_STRATEGY,
                Some(NodePlacementStrategy::NetworkSimplex),
            );
            graph.set_property(LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_RECOMPUTE_NODE_PLACEMENT, second_placer);
        }
        GraphConfigurator::new().prepare_graph_for_layout(&graph);
        let processors = graph
            .lock()
            .get_property(InternalProperties::PROCESSORS)
            .unwrap();
        let names: Vec<_> = processors.iter().map(|p| p.lock().type_name()).collect();
        let repeater = names
            .iter()
            .position(|name| name.ends_with("::NodePlacementRepeater"));
        assert_eq!(repeater.is_some(), second_placer.is_some());
        if let Some(index) = repeater {
            assert!(names[index - 1].ends_with("::NetworkSimplexPlacer"));
            let height = names
                .iter()
                .position(|name| name.ends_with("::LayerSizeAndGraphHeightCalculator"))
                .unwrap();
            assert!(index < height);
        }
    }
}
