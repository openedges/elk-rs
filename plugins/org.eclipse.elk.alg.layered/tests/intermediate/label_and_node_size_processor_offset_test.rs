use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::elk_layered::ElkLayered;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_importer::ElkGraphImporter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkNodeRef, ElkPortRef,
};

#[test]
fn label_and_node_size_processor_applies_port_offsets() {
    init_layered_options();

    let graph = ElkGraphUtil::create_graph();
    let constraints = [
        PortConstraints::FixedSide,
        PortConstraints::FixedOrder,
        PortConstraints::FixedRatio,
        PortConstraints::FixedPos,
    ];

    let mut previous_east: Option<ElkPortRef> = None;
    for constraint in constraints {
        let node = ElkGraphUtil::create_node(Some(graph.clone()));
        set_node_size(&node, 100.0, 100.0);
        set_node_property(&node, LayeredOptions::PORT_CONSTRAINTS, constraint);

        add_port(&node, PortSide::North, 50.0, -300.0);
        add_port(&node, PortSide::South, 50.0, 300.0);
        let east = add_port(&node, PortSide::East, 300.0, 50.0);
        let west = add_port(&node, PortSide::West, -300.0, 50.0);

        if let Some(prev) = previous_east.take() {
            ElkGraphUtil::create_simple_edge(
                ElkConnectableShapeRef::Port(prev),
                ElkConnectableShapeRef::Port(west.clone()),
            );
        }
        previous_east = Some(east);
    }

    let lgraph = import_lgraph(&graph);
    let mut layered = ElkLayered::new();
    layered.do_layout(&lgraph, None);

    let layers = lgraph.lock().layers().clone();
    for layer in layers {
        let nodes = layer.lock().nodes().clone();
        for node in nodes {
            let (node_w, node_h) = {
                let mut node_guard = node.lock();                let size = node_guard.shape().size_ref();
                (size.x, size.y)
            };
            let ports = node.lock().ports().clone();
            for port in ports {
                let (side, pos_x, pos_y, port_w, port_h) = {
                    let mut port_guard = port.lock();                    let side = port_guard.side();
                    let shape = port_guard.shape();
                    let pos_x = shape.position_ref().x;
                    let pos_y = shape.position_ref().y;
                    let port_w = shape.size_ref().x;
                    let port_h = shape.size_ref().y;
                    (side, pos_x, pos_y, port_w, port_h)
                };

                match side {
                    PortSide::North => {
                        assert_close(-port_h, pos_y, "north port does not touch border");
                    }
                    PortSide::South => {
                        assert_close(node_h, pos_y, "south port does not touch border");
                    }
                    PortSide::East => {
                        assert_close(node_w, pos_x, "east port does not touch border");
                    }
                    PortSide::West => {
                        assert_close(-port_w, pos_x, "west port does not touch border");
                    }
                    _ => {}
                }
            }
        }
    }
}

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn add_port(node: &ElkNodeRef, side: PortSide, x: f64, y: f64) -> ElkPortRef {
    let port = ElkGraphUtil::create_port(Some(node.clone()));
    set_port_property(&port, LayeredOptions::PORT_BORDER_OFFSET, 0.0);
    set_port_property(&port, LayeredOptions::PORT_SIDE, side);
    {
        let mut port_mut = port.borrow_mut();
        let shape = port_mut.connectable().shape();
        shape.set_dimensions(10.0, 10.0);
        shape.set_x(x);
        shape.set_y(y);
    }
    port
}

fn set_node_size(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

fn set_node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
    value: T,
) {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn set_port_property<T: Clone + Send + Sync + 'static>(
    port: &ElkPortRef,
    property: &Property<T>,
    value: T,
) {
    let mut port_mut = port.borrow_mut();
    port_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn assert_close(expected: f64, actual: f64, message: &str) {
    assert!(
        (expected - actual).abs() <= 1e-6,
        "{}: expected {:.3}, got {:.3}",
        message,
        expected,
        actual
    );
}

fn import_lgraph(
    root: &ElkNodeRef,
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef {
    let mut origin_store = OriginStore::new();
    let mut importer = ElkGraphImporter::new(&mut origin_store);
    importer.import_graph(root)
}
