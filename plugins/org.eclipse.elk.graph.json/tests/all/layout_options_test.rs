use serde_json::Value;

use org_eclipse_elk_core::org::eclipse::elk::core::math::{KVector, KVectorChain};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, Direction, EdgeLabelPlacement, NodeLabelPlacement, SizeConstraint,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkGraphElementRef,
};
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::ElkGraphJson;

use super::common::*;

#[test]
fn edge_label_placement_option_parses_enum() {
    let graph = r#"
    {
      "id": "root",
      "children": [{"id": "n1"}, {"id": "n2"}],
      "edges": [
        {"id": "e1",
         "source": "n1",
         "target": "n2",
         "labels": [
           {"text": "tail", "layoutOptions": {"org.eclipse.elk.edgeLabels.placement": "TAIL"}}
         ]
        }
      ]
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    let mut edges = node_edges(&root);
    assert_eq!(edges.len(), 1);
    let edge = edges.remove(0);
    let labels: Vec<_> = edge
        .borrow_mut()
        .element()
        .labels()
        .iter()
        .cloned()
        .collect();
    assert_eq!(labels.len(), 1);
    let label = labels.first().unwrap();
    let placement = {
        let mut label_mut = label.borrow_mut();
        let props = label_mut.shape().graph_element().properties().clone();
        props.get_property(CoreOptions::EDGE_LABELS_PLACEMENT)
    };
    assert_eq!(placement, Some(EdgeLabelPlacement::Tail));
}

#[test]
fn import_layout_options() {
    let graph = r#"
    {
      "id": "root",
      "layoutOptions": {
        "elk.direction": "DOWN"
      }
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    assert!(node_has_property(&root, CoreOptions::DIRECTION));
    assert_eq!(
        node_property(&root, CoreOptions::DIRECTION),
        Some(Direction::Down)
    );
}

#[test]
fn import_label_layout_options_node_labels_placement() {
    let graph = r#"
    {
      "id": "root",
      "children": [
        {
          "id": "n1",
          "labels": [
            {
              "id": "l1",
              "text": "Node Label",
              "layoutOptions": {
                "nodeLabels.placement": "[H_CENTER, V_TOP, INSIDE]"
              },
              "width": 40.0,
              "height": 15.0
            }
          ]
        }
      ]
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    let n1 = find_node(&node_children(&root), "n1");
    let labels = node_labels(&n1);
    assert_eq!(labels.len(), 1, "n1 should have one label");

    let placement = label_property(&labels[0], CoreOptions::NODE_LABELS_PLACEMENT)
        .expect("label-level nodeLabels.placement should be parsed");
    assert!(
        placement.contains(&NodeLabelPlacement::HCenter)
            && placement.contains(&NodeLabelPlacement::VTop)
            && placement.contains(&NodeLabelPlacement::Inside),
        "expected [H_CENTER, V_TOP, INSIDE], got {:?}",
        placement
    );
}

#[test]
fn import_node_size_constraints_enumset() {
    let graph = r#"
    {
      "id": "root",
      "children": [
        {
          "id": "n1",
          "layoutOptions": {
            "org.eclipse.elk.nodeSize.constraints": "[PORTS, PORT_LABELS, NODE_LABELS, MINIMUM_SIZE]"
          }
        }
      ]
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    let node = find_node(&node_children(&root), "n1");
    let constraints = node_property(&node, CoreOptions::NODE_SIZE_CONSTRAINTS)
        .expect("node size constraints should parse");
    assert!(
        constraints.contains(&SizeConstraint::Ports),
        "PORTS should be present in parsed node size constraints"
    );
    assert!(
        constraints.contains(&SizeConstraint::PortLabels),
        "PORT_LABELS should be present in parsed node size constraints"
    );
    assert!(
        constraints.contains(&SizeConstraint::NodeLabels),
        "NODE_LABELS should be present in parsed node size constraints"
    );
    assert!(
        constraints.contains(&SizeConstraint::MinimumSize),
        "MINIMUM_SIZE should be present in parsed node size constraints"
    );
}

#[test]
fn import_properties_legacy() {
    let graph = r#"
    {
      "id": "root",
      "properties": {
        "elk.direction": "DOWN"
      }
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    assert!(node_has_property(&root, CoreOptions::DIRECTION));
    assert_eq!(
        node_property(&root, CoreOptions::DIRECTION),
        Some(Direction::Down)
    );
}

#[test]
fn layout_options_have_priority() {
    let graph = r#"
    {
      "id": "root",
      "layoutOptions": {
        "elk.direction": "UP"
      },
      "properties": {
        "elk.direction": "DOWN"
      }
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    assert!(node_has_property(&root, CoreOptions::DIRECTION));
    assert_eq!(
        node_property(&root, CoreOptions::DIRECTION),
        Some(Direction::Up)
    );
}

#[test]
fn export_layout_options() {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(&graph, CoreOptions::DIRECTION, Direction::Up);

    let json = ElkGraphJson::for_elk(graph).to_json();
    let actual: Value = serde_json::from_str(&json).unwrap();
    let expected: Value =
        serde_json::from_str(r#"{"id":"n0","layoutOptions":{"elk.direction":"UP"}}"#).unwrap();

    assert_eq!(expected, actual);
}

#[test]
fn import_layout_options_kvector_and_chain() {
    let graph = r#"
    {
      "id": "root",
      "children": [
        {
          "id": "n1",
          "layoutOptions": { "position": "(1,2)" }
        }
      ],
      "edges": [
        {
          "id": "e1",
          "sources": [ "n1" ],
          "targets": [ "n1" ],
          "layoutOptions": { "bendPoints": "(0,0; 10,10)" }
        }
      ]
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    let node = find_node(&node_children(&root), "n1");
    let edge = node_edges(&root).remove(0);

    let pos = node_property(&node, CoreOptions::POSITION).expect("position option");
    assert_eq!(pos, KVector::with_values(1.0, 2.0));

    let bends = edge_property(&edge, CoreOptions::BEND_POINTS).expect("bendPoints option");
    assert_eq!(bends.len(), 2);
    assert_eq!(bends.get(0), KVector::with_values(0.0, 0.0));
    assert_eq!(bends.get(1), KVector::with_values(10.0, 10.0));
}

#[test]
fn export_layout_options_kvector_and_chain() {
    let graph = ElkGraphUtil::create_graph();
    let node1 = ElkGraphUtil::create_node(Some(graph.clone()));
    let node2 = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_identifier(&node1, "n1");
    set_node_identifier(&node2, "n2");

    set_node_property(
        &node1,
        CoreOptions::POSITION,
        KVector::with_values(3.0, 4.0),
    );

    let edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node1),
        ElkConnectableShapeRef::Node(node2),
    );
    let bend_points = KVectorChain::from_vectors(&[
        KVector::with_values(0.0, 0.0),
        KVector::with_values(10.0, 10.0),
    ]);
    set_edge_property(&edge, CoreOptions::BEND_POINTS, bend_points);

    let json = ElkGraphJson::for_elk(graph).to_json();
    let value: Value = serde_json::from_str(&json).unwrap();

    let child = value["children"]
        .as_array()
        .and_then(|children| {
            children
                .iter()
                .find(|child| child.get("id").and_then(|id| id.as_str()) == Some("n1"))
        })
        .expect("n1 child");
    let pos_str = child["layoutOptions"]["elk.position"]
        .as_str()
        .expect("elk.position string");
    let mut parsed_pos = KVector::new();
    parsed_pos.parse(pos_str);
    assert_eq!(parsed_pos, KVector::with_values(3.0, 4.0));

    let edge_obj = value["edges"]
        .as_array()
        .and_then(|edges| edges.first())
        .expect("edge");
    let bends_str = edge_obj["layoutOptions"]["elk.bendPoints"]
        .as_str()
        .expect("elk.bendPoints string");
    let mut parsed_bends = KVectorChain::new();
    parsed_bends.parse(bends_str);
    assert_eq!(parsed_bends.len(), 2);
    assert_eq!(parsed_bends.get(0), KVector::with_values(0.0, 0.0));
    assert_eq!(parsed_bends.get(1), KVector::with_values(10.0, 10.0));
}

#[test]
fn import_layout_options_port_and_label_position() {
    let graph = r#"
    {
      "id": "root",
      "children": [
        {
          "id": "n1",
          "ports": [
            { "id": "p1", "layoutOptions": { "position": "(7,8)" } }
          ],
          "labels": [
            { "text": "L1", "layoutOptions": { "position": "(9,10)" } }
          ]
        }
      ]
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    let node = find_node(&node_children(&root), "n1");

    let port = node_ports(&node).remove(0);
    let label = node_labels(&node).remove(0);

    let port_pos = port_property(&port, CoreOptions::POSITION).expect("port position");
    assert_eq!(port_pos, KVector::with_values(7.0, 8.0));

    let label_pos = label_property(&label, CoreOptions::POSITION).expect("label position");
    assert_eq!(label_pos, KVector::with_values(9.0, 10.0));
}

#[test]
fn export_layout_options_port_and_label_position() {
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    let port = ElkGraphUtil::create_port(Some(node.clone()));
    let label =
        ElkGraphUtil::create_label_with_text("L1", Some(ElkGraphElementRef::Node(node.clone())));

    set_port_property(&port, CoreOptions::POSITION, KVector::with_values(7.0, 8.0));
    set_label_property(
        &label,
        CoreOptions::POSITION,
        KVector::with_values(9.0, 10.0),
    );

    let json = ElkGraphJson::for_elk(graph).to_json();
    let value: Value = serde_json::from_str(&json).unwrap();

    let child = value["children"]
        .as_array()
        .and_then(|children| children.first())
        .expect("node child");
    let port_pos_str = child["ports"][0]["layoutOptions"]["elk.position"]
        .as_str()
        .expect("port position");
    let mut parsed_port = KVector::new();
    parsed_port.parse(port_pos_str);
    assert_eq!(parsed_port, KVector::with_values(7.0, 8.0));

    let label_pos_str = child["labels"][0]["layoutOptions"]["elk.position"]
        .as_str()
        .expect("label position");
    let mut parsed_label = KVector::new();
    parsed_label.parse(label_pos_str);
    assert_eq!(parsed_label, KVector::with_values(9.0, 10.0));
}
