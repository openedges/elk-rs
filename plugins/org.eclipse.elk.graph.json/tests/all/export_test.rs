use serde_json::Value;

use org_eclipse_elk_core::org::eclipse::elk::core::math::{KVector, KVectorChain};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{CoreOptions, Direction};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef;
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::ElkGraphJson;

use super::common::*;

#[test]
fn export_portless_edge() {
    let graph = ElkGraphUtil::create_graph();
    let node1 = ElkGraphUtil::create_node(Some(graph.clone()));
    let node2 = ElkGraphUtil::create_node(Some(graph.clone()));
    ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node1),
        ElkConnectableShapeRef::Node(node2),
    );

    ElkGraphJson::for_elk(graph).to_json();
}

#[test]
fn export_unique_ids() {
    let graph = ElkGraphUtil::create_graph();
    let node1 = ElkGraphUtil::create_node(Some(graph.clone()));
    let node2 = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_identifier(&node1, "foo");
    set_node_identifier(&node2, "foo");

    let json = ElkGraphJson::for_elk(graph).to_json();
    assert!(json.contains("foo_g"));
}

#[test]
fn export_ids_increasing() {
    let graph = ElkGraphUtil::create_graph();
    ElkGraphUtil::create_node(Some(graph.clone()));
    let node2 = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_identifier(&node2, "foo");
    ElkGraphUtil::create_node(Some(graph.clone()));

    let json = ElkGraphJson::for_elk(graph).to_json();
    assert!(json.contains("n1"));
    assert!(json.contains("foo"));
    assert!(json.contains("n2"));
    assert!(!json.contains("n3"));
}

#[test]
fn export_omit_unknown_properties() {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(&graph, CoreOptions::DIRECTION, Direction::Right);

    let dummy = Property::<i32>::new("foo.bar.dummy");
    {
        let mut graph_mut = graph.borrow_mut();
        graph_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(&dummy, Some(0));
    }

    let json1 = ElkGraphJson::for_elk(graph.clone())
        .omit_unknown_layout_options(true)
        .to_json();
    assert!(json1.contains("direction"));
    assert!(!json1.contains("foo.bar.dummy"));

    let json2 = ElkGraphJson::for_elk(graph)
        .omit_unknown_layout_options(false)
        .to_json();
    assert!(json2.contains("direction"));
    assert!(json2.contains("foo.bar.dummy"));
}

#[test]
fn export_dont_write_empty_junction_points() {
    let graph = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(graph.clone()));
    let n2 = ElkGraphUtil::create_node(Some(graph.clone()));
    ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1),
        ElkConnectableShapeRef::Node(n2),
    );

    let json = ElkGraphJson::for_elk(graph).omit_layout(false).to_json();
    assert!(!json.contains("layoutOptions"));
    assert!(!json.contains("junctionPoints"));
}

#[test]
fn export_junction_points() {
    let graph = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(graph.clone()));
    let n2 = ElkGraphUtil::create_node(Some(graph.clone()));
    let edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1),
        ElkConnectableShapeRef::Node(n2),
    );

    let junctions = KVectorChain::from_vectors(&[
        KVector::with_values(1.0, 2.0),
        KVector::with_values(3.0, 4.0),
    ]);
    set_edge_property(&edge, CoreOptions::JUNCTION_POINTS, junctions);

    let json = ElkGraphJson::for_elk(graph).omit_layout(false).to_json();
    let value: Value = serde_json::from_str(&json).unwrap();
    let edge_obj = value["edges"]
        .as_array()
        .and_then(|edges| edges.first())
        .expect("edge");
    let points = edge_obj["junctionPoints"]
        .as_array()
        .expect("junctionPoints");
    assert_eq!(points.len(), 2);
    assert_eq!(points[0]["x"].as_f64().unwrap(), 1.0);
    assert_eq!(points[0]["y"].as_f64().unwrap(), 2.0);
    assert_eq!(points[1]["x"].as_f64().unwrap(), 3.0);
    assert_eq!(points[1]["y"].as_f64().unwrap(), 4.0);
}
