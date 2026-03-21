use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::{
    PartitionMidprocessor, PartitionPostprocessor,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::InternalProperties;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn new_graph_with_layers(count: usize) -> (LGraphRef, Vec<LayerRef>) {
    let graph = LGraph::new();
    let mut layers = Vec::with_capacity(count);
    {
        let mut graph_guard = graph.lock();        for _ in 0..count {
            let layer = Layer::new(&graph);
            graph_guard.layers_mut().push(layer.clone());
            layers.push(layer);
        }
    }
    (graph, layers)
}

fn add_layered_node(graph: &LGraphRef, layer: &LayerRef, partition: i32) -> LNodeRef {
    let node = LNode::new(graph);
    {
        let mut node_guard = node.lock();        node_guard.set_property(CoreOptions::PARTITIONING_PARTITION, Some(partition));
        node_guard.set_node_type(NodeType::Normal);
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_partition_dummy_port(
    node: &LNodeRef,
    side: org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide,
) -> LPortRef {
    let port = LPort::new();
    {
        let mut port_guard = port.lock();        port_guard.set_side(side);
        port_guard.set_property(InternalProperties::PARTITION_DUMMY, Some(true));
    }
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn connect(source: &LPortRef, target: &LPortRef) {
    let edge = LEdge::new();
    edge.lock()
        
        .set_property(InternalProperties::PARTITION_DUMMY, Some(true));
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
}

#[test]
fn no_empty_layer_test() {
    let (graph, layers) = new_graph_with_layers(2);
    let first = add_layered_node(&graph, &layers[0], 0);
    let second = add_layered_node(&graph, &layers[1], 1);

    let first_port = add_partition_dummy_port(
        &first,
        org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide::East,
    );
    let second_port = add_partition_dummy_port(
        &second,
        org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide::West,
    );
    connect(&first_port, &second_port);

    let mut post = PartitionPostprocessor;
    let mut monitor = NullElkProgressMonitor;
    post.process(&mut graph.lock(), &mut monitor);

    for layer in graph.lock().layers().iter() {
        assert!(
            !layer.lock().nodes().is_empty(),
            "no empty layers must remain"
        );
    }
}

#[test]
fn test_partition_order() {
    let graph = LGraph::new();
    let mut monitor = NullElkProgressMonitor;

    let left = LNode::new(&graph);
    left.lock()
        
        .set_property(CoreOptions::PARTITIONING_PARTITION, Some(0));
    let right = LNode::new(&graph);
    right
        .lock()
        
        .set_property(CoreOptions::PARTITIONING_PARTITION, Some(1));

    {
        let mut graph_guard = graph.lock();        graph_guard.layerless_nodes_mut().push(left.clone());
        graph_guard.layerless_nodes_mut().push(right.clone());
    }

    PartitionMidprocessor.process(&mut graph.lock(), &mut monitor);

    let layer_a = Layer::new(&graph);
    let layer_b = Layer::new(&graph);
    {
        let mut graph_guard = graph.lock();        graph_guard.layers_mut().push(layer_a.clone());
        graph_guard.layers_mut().push(layer_b.clone());
    }
    LNode::set_layer(&left, Some(layer_a.clone()));
    LNode::set_layer(&right, Some(layer_b.clone()));

    PartitionPostprocessor.process(&mut graph.lock(), &mut monitor);

    let mut last_partition = -1;
    for layer in graph.lock().layers().iter() {
        let mut current_partition = -1;
        for node in layer.lock().nodes().iter() {
            let node_partition = node.lock_ok().and_then(|mut node_guard| {
                node_guard.get_property(CoreOptions::PARTITIONING_PARTITION)
            });
            if let Some(node_partition) = node_partition {
                if current_partition == -1 {
                    current_partition = node_partition;
                } else {
                    assert_eq!(
                        current_partition, node_partition,
                        "all normal nodes in a layer must share the same partition"
                    );
                }
            }
        }
        assert!(
            last_partition <= current_partition,
            "layer partitions must be monotonic"
        );
        last_partition = current_partition;
    }
}
