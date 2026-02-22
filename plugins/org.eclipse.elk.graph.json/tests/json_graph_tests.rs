use std::cell::RefCell;
use std::rc::Rc;

use serde_json::Value;

use org_eclipse_elk_core::org::eclipse::elk::core::math::{
    ElkMargin, ElkPadding, KVector, KVectorChain,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, Direction, EdgeLabelPlacement, NodeLabelPlacement, PortSide, SizeConstraint,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    BasicProgressMonitor, IndividualSpacings, Maybe,
};
use org_eclipse_elk_core::org::eclipse::elk::core::{
    IGraphLayoutEngine, RecursiveGraphLayoutEngine,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkEdgeSectionRef, ElkGraphElementRef, ElkNodeRef,
};
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::{ElkGraphJson, JsonImportError};

#[test]
fn graph_must_be_object() {
    ElkGraphJson::for_graph("{\"id\":1}").to_elk().unwrap();
}

#[test]
fn graph_must_be_object_fail_array() {
    assert!(matches!(
        ElkGraphJson::for_graph("[]").to_elk(),
        Err(JsonImportError::Import(_))
    ));
}

#[test]
fn small_graph_import() {
    let graph = r#"
    {
      "id": "root",
      "properties": {
        "elk.direction": "DOWN"
      },
      "children": [{"id": "n1", "width": 40, "height": 40},
                  {"id": "n2", "width": 40, "height": 40}],
      "edges": [{"id": "e1", "source": "n1", "target": "n2"}]
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    assert_eq!(node_children(&root).len(), 2);
    assert_eq!(node_edges(&root).len(), 1);

    let direction = node_property(&root, CoreOptions::DIRECTION).unwrap();
    assert_eq!(direction, Direction::Down);
}

#[test]
fn reject_sloppy_json_when_not_lenient() {
    let sloppy = r#"
        {
          // the root node
          id: "root",
          /* Now the graph */
          "children": [ {"id": "c"}; {"id": "c1"} ],
          'edges': [] // Endline comment
        }
    "#;

    assert!(matches!(
        ElkGraphJson::for_graph(sloppy).lenient(false).to_elk(),
        Err(JsonImportError::Io(_))
    ));
}

#[test]
fn accept_sloppy_json_when_lenient() {
    let sloppy = r#"
        {
          // the root node
          id: "root",
          /* Now the graph */
          "children": [ {"id": "c"}; {"id": "c1"} ],
          'edges': [] // Endline comment
        }
    "#;

    let root = ElkGraphJson::for_graph(sloppy).to_elk().unwrap();
    assert_eq!(node_identifier(&root).as_deref(), Some("root"));

    let children = node_children(&root);
    assert_eq!(children.len(), 2);
    assert_eq!(node_identifier(&children[0]).as_deref(), Some("c"));
    assert_eq!(node_identifier(&children[1]).as_deref(), Some("c1"));
}

#[test]
fn extended_edge_import() {
    let graph = r#"
    {
      "id": "root",
      "children": [{"id": "n1"}, {"id": 3}],
      "edges": [{"id": "e1",
        "sources": [ "n1" ],
        "targets": [ 3 ]
      }]
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    assert_eq!(node_children(&root).len(), 2);
    assert_eq!(node_edges(&root).len(), 1);

    let children = node_children(&root);
    let n1 = find_node(&children, "n1");
    let n3 = find_node(&children, "3");

    let edge = node_edges(&root).remove(0);
    let sources = edge.borrow().sources_ro().get(0).unwrap();
    let targets = edge.borrow().targets_ro().get(0).unwrap();

    let source_node = ElkGraphUtil::connectable_shape_to_node(&sources).unwrap();
    let target_node = ElkGraphUtil::connectable_shape_to_node(&targets).unwrap();

    assert!(Rc::ptr_eq(&source_node, &n1));
    assert!(Rc::ptr_eq(&target_node, &n3));
}

#[test]
fn primitive_edge_import() {
    let graph = r#"
    {
      "id": "root",
      "children": [{"id": "n1"}, {"id": 3}],
      "edges": [{"id": "e1",
        "source": "n1",
        "target": 3
      }]
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    assert_eq!(node_children(&root).len(), 2);
    assert_eq!(node_edges(&root).len(), 1);

    let children = node_children(&root);
    let n1 = find_node(&children, "n1");
    let n3 = find_node(&children, "3");

    let edge = node_edges(&root).remove(0);
    let sources = edge.borrow().sources_ro().get(0).unwrap();
    let targets = edge.borrow().targets_ro().get(0).unwrap();

    let source_node = ElkGraphUtil::connectable_shape_to_node(&sources).unwrap();
    let target_node = ElkGraphUtil::connectable_shape_to_node(&targets).unwrap();

    assert!(Rc::ptr_eq(&source_node, &n1));
    assert!(Rc::ptr_eq(&target_node, &n3));
}

#[test]
fn edge_containment_import() {
    let graph = r#"
    {
      "id": "root",
      "children": [
        {"id": "p"},
        {"id": "q",
          "children": [{"id": "r"}],
          "edges": [{"id": "e", "source": "p", "target": "r" }]
        }]
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    assert_eq!(node_edges(&root).len(), 1);
}

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
        let mut props = label_mut.shape().graph_element().properties().clone();
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

#[test]
fn import_individual_spacings() {
    let graph = r#"
    {
      "id": "n0",
      "children": [
        {
          "id": "outer",
          "layoutOptions": {
              "nodeLabels.padding": "[top=0.0,left=0.0,bottom=0.0,right=0.0]"
          },
          "children": [
            {
              "id": "i1",
              "labels": [
                { "text": "Node 1", "width": 40.0, "height": 15.0 }
              ],
              "layoutOptions": { "nodeLabels.placement": "[H_CENTER, V_TOP, INSIDE]" },
              "width": 60.0,
              "height": 40.0
            },
            {
              "id": "i2",
              "layoutOptions": { "nodeLabels.placement": "[H_CENTER, V_TOP, INSIDE]" },
              "individualSpacings": {
                "nodeLabels.padding": "[top=10.0,left=0.0,bottom=0.0,right=0.0]"
              },
              "labels": [
                { "text": "Node 2", "width": 40.0, "height": 15.0 }
              ],
              "width": 60.0,
              "height": 40.0
            }
          ]
        }
      ]
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    let outer = find_node(&node_children(&root), "outer");
    let children = node_children(&outer);
    let i1 = find_node(&children, "i1");
    let i2 = find_node(&children, "i2");

    assert!(!node_has_property(&i1, CoreOptions::SPACING_INDIVIDUAL));
    assert!(node_has_property(&i2, CoreOptions::SPACING_INDIVIDUAL));

    let individual = node_property(&i2, CoreOptions::SPACING_INDIVIDUAL).unwrap();
    assert!(individual
        .properties()
        .has_property(CoreOptions::NODE_LABELS_PADDING));
}

#[test]
fn import_individual_spacings_with_ports_surrounding() {
    let graph = r#"
    {
      "id": "n0",
      "children": [
        {
          "id": "n1",
          "individualSpacings": {
            "spacing.portsSurrounding": "[top=2.0,left=8.0,bottom=6.0,right=4.0]"
          }
        }
      ]
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    let child = find_node(&node_children(&root), "n1");

    let mut individual =
        node_property(&child, CoreOptions::SPACING_INDIVIDUAL).expect("individual spacings");
    let margin = individual
        .properties_mut()
        .get_property(CoreOptions::SPACING_PORTS_SURROUNDING)
        .expect("portsSurrounding");

    assert_eq!(margin, ElkMargin::with_values(2.0, 4.0, 6.0, 8.0));
}

#[test]
fn export_individual_spacings() {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(&graph, CoreOptions::SPACING_NODE_NODE, 10.0);

    let mut individual = IndividualSpacings::new();
    individual
        .properties_mut()
        .set_property(CoreOptions::SPACING_NODE_NODE, Some(20.0));
    set_node_property(&graph, CoreOptions::SPACING_INDIVIDUAL, individual);

    let json = ElkGraphJson::for_elk(graph)
        .omit_unknown_layout_options(true)
        .to_json();

    assert!(json.contains("individualSpacings"));
    assert!(json.contains("nodeNode"));
    assert!(json.contains("10"));
    assert!(json.contains("20"));
}

#[test]
fn export_individual_spacings_with_padding() {
    let graph = ElkGraphUtil::create_graph();

    let mut individual = IndividualSpacings::new();
    individual.properties_mut().set_property(
        CoreOptions::NODE_LABELS_PADDING,
        Some(ElkPadding::with_values(1.0, 2.0, 3.0, 4.0)),
    );
    set_node_property(&graph, CoreOptions::SPACING_INDIVIDUAL, individual);

    let json = ElkGraphJson::for_elk(graph)
        .omit_unknown_layout_options(true)
        .to_json();

    assert!(json.contains("individualSpacings"));
    assert!(json.contains("elk.nodeLabels.padding"));
    assert!(json.contains("[top=1,left=4,bottom=3,right=2]"));
}

#[test]
fn export_individual_spacings_with_ports_surrounding() {
    let graph = ElkGraphUtil::create_graph();

    let mut individual = IndividualSpacings::new();
    individual.properties_mut().set_property(
        CoreOptions::SPACING_PORTS_SURROUNDING,
        Some(ElkMargin::with_values(2.0, 4.0, 6.0, 8.0)),
    );
    set_node_property(&graph, CoreOptions::SPACING_INDIVIDUAL, individual);

    let json = ElkGraphJson::for_elk(graph)
        .omit_unknown_layout_options(true)
        .to_json();

    assert!(json.contains("individualSpacings"));
    assert!(json.contains("elk.spacing.portsSurrounding"));
    assert!(json.contains("[top=2,left=8,bottom=6,right=4]"));
}

#[test]
fn export_no_individual_spacings() {
    let graph = ElkGraphUtil::create_graph();
    let json = ElkGraphJson::for_elk(graph)
        .omit_unknown_layout_options(true)
        .to_json();
    assert!(!json.contains("individualSpacings"));
    assert!(!json.contains("IndividualSpacings"));
}

#[test]
fn edge_section_import() {
    let graph = r#"
    {
      "id": "root",
      "properties": {
        "direction": "DOWN"
      },
      "children": [{"id": "n1"}, {"id": "n2"}],
      "edges": [{"id": "e1",
        "sources": [ "n1" ],
        "targets": [ "n2" ],
        "sections": [{
            "id": "s1",
            "startPoint": {"x": 1, "y": 1},
            "endPoint": {"x": 2, "y": 2},
            "incomingShape": "n1",
            "outgoingSections": [ 44 ]
        },{
            "id": 44,
            "startPoint": {"x": 3, "y": 3},
            "endPoint": {"x": 4, "y": 4},
            "incomingSections": [ "s1" ],
            "outgoingShape": "n2"
        }]
      }]
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    assert_eq!(node_children(&root).len(), 2);
    let children = node_children(&root);
    let n1 = find_node(&children, "n1");
    let n2 = find_node(&children, "n2");

    let edge = node_edges(&root).remove(0);
    let sections = edge_sections(&edge);
    assert_eq!(sections.len(), 2);

    let s1 = find_section(&sections, "s1");
    let s44 = find_section(&sections, "44");

    let incoming = s1.borrow().incoming_shape().unwrap();
    let outgoing = s44.borrow().outgoing_shape().unwrap();
    assert!(incoming.ptr_eq(&ElkConnectableShapeRef::Node(n1)));
    assert!(outgoing.ptr_eq(&ElkConnectableShapeRef::Node(n2)));

    let s1_out = s1.borrow().outgoing_sections();
    assert_eq!(s1_out.len(), 1);
    assert!(Rc::ptr_eq(&s1_out[0], &s44));

    let s44_in = s44.borrow().incoming_sections();
    assert_eq!(s44_in.len(), 1);
    assert!(Rc::ptr_eq(&s44_in[0], &s1));
}

#[test]
fn preserve_section_ids() {
    let graph = r#"
    {
      id: "root",
      properties: {
        'elk.algorithm': 'random'
      },
      children: [
        { id: "n1", width: 10, height: 10 },
        { id: "n2", width: 10, height: 10 }
      ],
      edges: [{
        id: "e1", sources: [ "n1" ], targets: [ "n2" ],
        sections: [{
          id: "xyz",
          startPoint: { x: 0, y: 0 },
          bendPoints: [{ x: 20, y: 0 }],
          endPoint: { x: 50, y: 0 }
        }]
      }]
    }"#;

    let shared = Rc::new(RefCell::new(parse_lenient_json(graph)));
    let mut importer = Maybe::default();
    let root = ElkGraphJson::for_graph_shared(shared.clone())
        .remember_importer(&mut importer)
        .to_elk()
        .unwrap();

    let edge = node_edges(&root).remove(0);
    let section = find_section(&edge_sections(&edge), "xyz");
    assert!(section.borrow().identifier().is_some());

    let mut engine = RecursiveGraphLayoutEngine::new();
    engine.layout(&root, &mut BasicProgressMonitor::new());

    importer
        .get_mut()
        .expect("importer")
        .transfer_layout(&root)
        .unwrap();

    let serialized = serde_json::to_string(&*shared.borrow()).unwrap();
    assert!(serialized.contains("\"id\":\"xyz\""));
}

#[test]
fn do_not_add_empty_sections_array_extended_edge() {
    let graph = r#"
    {
      id: "root",
      children: [
        { id: "n1", width: 10, height: 10 },
        { id: "n2", width: 10, height: 10 }
      ],
      edges: [{ id: "e1", sources: [ "n1" ], targets: [ "n2" ] }]
    }"#;

    let s = to_json_graph_and_back(graph);
    assert!(!s.contains("\"sections\""));
}

#[test]
fn do_not_add_empty_sections_array_primitive_edge() {
    let graph = r#"
    {
      id: "root",
      children: [
        { id: "n1", width: 10, height: 10 },
        { id: "n2", width: 10, height: 10 }
      ],
      edges: [{ id: "e1", source: "n1", target: "n2" }]
    }"#;

    let s = to_json_graph_and_back(graph);
    assert!(!s.contains("\"sections\""));
}

#[test]
fn transfer_layout_ok() {
    let graph = r#"
    {
      "id": "root",
      "properties": {
        "algorithm": "random",
        "direction": "DOWN"
      },
      "children": [{"id": "n1", "width": 40, "height": 40},
                  {"id": "n2", "width": 40, "height": 40}],
      "edges": [{"id": "e1", "source": "n1", "target": "n2"}]
    }
    "#;

    let mut importer = Maybe::default();
    let root = ElkGraphJson::for_graph(graph)
        .remember_importer(&mut importer)
        .to_elk()
        .unwrap();

    importer
        .get_mut()
        .expect("importer")
        .transfer_layout(&root)
        .unwrap();
}

#[test]
fn transfer_layout_fail_when_graph_changes() {
    let graph = r#"
    {
      "id": "root",
      "properties": {
        "algorithm": "random",
        "direction": "DOWN"
      },
      "children": [{"id": "n1", "width": 40, "height": 40},
                  {"id": "n2", "width": 40, "height": 40}],
      "edges": [{"id": "e1", "source": "n1", "target": "n2"}]
    }
    "#;

    let mut importer = Maybe::default();
    let root = ElkGraphJson::for_graph(graph)
        .remember_importer(&mut importer)
        .to_elk()
        .unwrap();

    let _ = ElkGraphUtil::create_node(Some(root.clone()));

    match importer.get_mut().expect("importer").transfer_layout(&root) {
        Err(_) => {}
        Ok(_) => panic!("expected transfer layout error"),
    }
}

#[test]
fn transfer_layout_edge_sections_exist() {
    let graph = r#"
    {
      "id": "root",
      "properties": {
        "algorithm": "random",
        "direction": "DOWN"
      },
      "children": [{"id": "n1", "width": 40, "height": 40},
                  {"id": "n2", "width": 40, "height": 40}],
      "edges": [{"id": "e1", "source": "n1", "target": "n2"}]
    }
    "#;

    let shared = Rc::new(RefCell::new(parse_lenient_json(graph)));

    let mut importer = Maybe::default();
    let root = ElkGraphJson::for_graph_shared(shared.clone())
        .remember_importer(&mut importer)
        .to_elk()
        .unwrap();

    let mut engine = RecursiveGraphLayoutEngine::new();
    engine.layout(&root, &mut BasicProgressMonitor::new());

    importer
        .get_mut()
        .expect("importer")
        .transfer_layout(&root)
        .unwrap();

    let root_value = shared.borrow();
    let children = root_value
        .get("children")
        .and_then(|value| value.as_array())
        .expect("children");
    for child in children {
        let obj = child.as_object().expect("child object");
        assert!(obj.get("x").and_then(|v| v.as_f64()).is_some());
        assert!(obj.get("y").and_then(|v| v.as_f64()).is_some());
    }

    let edges = root_value
        .get("edges")
        .and_then(|value| value.as_array())
        .expect("edges");
    for edge in edges {
        let obj = edge.as_object().expect("edge object");
        let sections = obj
            .get("sections")
            .and_then(|value| value.as_array())
            .expect("sections");
        for section in sections {
            let section_obj = section.as_object().expect("section object");
            assert!(section_obj.get("id").is_some());
            assert!(section_obj.get("startPoint").is_some());
            assert!(section_obj.get("endPoint").is_some());
        }
        assert!(obj.get("container").is_some());
    }
}

#[test]
fn transfer_layout_compensates_fixed_order_vertical_port_surrounding_height() {
    let graph = r#"
    {
      "id": "root",
      "children": [
        {
          "id": "N1",
          "layoutOptions": {
            "org.eclipse.elk.portConstraints": "FIXED_ORDER",
            "org.eclipse.elk.nodeSize.constraints": "[PORTS, PORT_LABELS, NODE_LABELS, MINIMUM_SIZE]"
          },
          "ports": [
            { "id": "P1", "layoutOptions": { "org.eclipse.elk.port.side": "EAST" } },
            { "id": "P2", "layoutOptions": { "org.eclipse.elk.port.side": "EAST" } },
            { "id": "P3", "layoutOptions": { "org.eclipse.elk.port.side": "EAST" } }
          ]
        }
      ]
    }
    "#;

    let shared = Rc::new(RefCell::new(parse_lenient_json(graph)));
    let mut importer = Maybe::default();
    let root = ElkGraphJson::for_graph_shared(shared.clone())
        .remember_importer(&mut importer)
        .to_elk()
        .unwrap();

    let n1 = find_node(&node_children(&root), "N1");
    {
        let mut node_ref = n1.borrow_mut();
        let shape = node_ref.connectable().shape();
        shape.set_location(12.0, 12.0);
        shape.set_width(20.0);
        shape.set_height(20.0);
    }

    let mut ports = node_ports(&n1);
    ports.sort_by_key(|port| {
        let mut port_ref = port.borrow_mut();
        port_ref
            .connectable()
            .shape()
            .graph_element()
            .identifier()
            .map(|id| id.to_string())
            .unwrap_or_default()
    });
    for (port, y) in ports.iter().zip([10.0_f64, 20.0, 30.0]) {
        set_port_property(port, CoreOptions::PORT_SIDE, PortSide::East);
        let mut port_ref = port.borrow_mut();
        let shape = port_ref.connectable().shape();
        shape.set_location(20.0, y);
        shape.set_width(0.0);
        shape.set_height(0.0);
    }

    importer
        .get_mut()
        .expect("importer")
        .transfer_layout(&root)
        .unwrap();

    let root_value = shared.borrow();
    let n1_obj = root_value["children"]
        .as_array()
        .and_then(|children| {
            children
                .iter()
                .find(|child| child.get("id").and_then(|id| id.as_str()) == Some("N1"))
        })
        .and_then(|child| child.as_object())
        .expect("N1");

    assert_eq!(n1_obj.get("height").and_then(|v| v.as_f64()), Some(40.0));
}

#[test]
fn transfer_layout_writes_junction_points() {
    let graph = r#"
    {
      "id": "root",
      "properties": {
        "algorithm": "random",
        "direction": "DOWN"
      },
      "children": [{"id": "n1", "width": 40, "height": 40},
                  {"id": "n2", "width": 40, "height": 40}],
      "edges": [{"id": "e1", "source": "n1", "target": "n2"}]
    }
    "#;

    let shared = Rc::new(RefCell::new(parse_lenient_json(graph)));
    let mut importer = Maybe::default();
    let root = ElkGraphJson::for_graph_shared(shared.clone())
        .remember_importer(&mut importer)
        .to_elk()
        .unwrap();

    let edge = node_edges(&root).remove(0);
    let junctions = KVectorChain::from_vectors(&[
        KVector::with_values(1.0, 2.0),
        KVector::with_values(3.0, 4.0),
    ]);
    set_edge_property(&edge, CoreOptions::JUNCTION_POINTS, junctions);

    importer
        .get_mut()
        .expect("importer")
        .transfer_layout(&root)
        .unwrap();

    let root_value = shared.borrow();
    let edges = root_value
        .get("edges")
        .and_then(|value| value.as_array())
        .expect("edges");
    let obj = edges[0].as_object().expect("edge object");
    let points = obj
        .get("junctionPoints")
        .and_then(|value| value.as_array())
        .expect("junctionPoints");
    assert_eq!(points.len(), 2);
    assert_eq!(points[0]["x"].as_f64().unwrap(), 1.0);
    assert_eq!(points[0]["y"].as_f64().unwrap(), 2.0);
    assert_eq!(points[1]["x"].as_f64().unwrap(), 3.0);
    assert_eq!(points[1]["y"].as_f64().unwrap(), 4.0);
}

#[test]
fn transfer_layout_shape_coords_root() {
    let graph = r#"
    {
      "id": "root",
      "children": [
        {
          "id": "parent",
          "layoutOptions": { "shapeCoords": "ROOT" },
          "children": [
            { "id": "child" }
          ]
        }
      ]
    }
    "#;

    let shared = Rc::new(RefCell::new(parse_lenient_json(graph)));
    let mut importer = Maybe::default();
    let root = ElkGraphJson::for_graph_shared(shared.clone())
        .remember_importer(&mut importer)
        .to_elk()
        .unwrap();

    let parent = find_node(&node_children(&root), "parent");
    let child = find_node(&node_children(&parent), "child");

    set_node_location(&root, 0.0, 0.0);
    set_node_location(&parent, 10.0, 20.0);
    set_node_location(&child, 5.0, 6.0);

    importer
        .get_mut()
        .expect("importer")
        .transfer_layout(&root)
        .unwrap();

    let root_value = shared.borrow();
    let parent_obj = root_value["children"]
        .as_array()
        .and_then(|children| {
            children
                .iter()
                .find(|child| child.get("id").and_then(|id| id.as_str()) == Some("parent"))
        })
        .and_then(|child| child.as_object())
        .expect("parent");
    let child_obj = parent_obj["children"]
        .as_array()
        .and_then(|children| {
            children
                .iter()
                .find(|child| child.get("id").and_then(|id| id.as_str()) == Some("child"))
        })
        .and_then(|child| child.as_object())
        .expect("child");

    assert_eq!(child_obj.get("x").and_then(|v| v.as_f64()), Some(15.0));
    assert_eq!(child_obj.get("y").and_then(|v| v.as_f64()), Some(26.0));
}

#[test]
fn transfer_layout_edge_coords_root() {
    let graph = r#"
    {
      "id": "root",
      "layoutOptions": { "edgeCoords": "ROOT" },
      "children": [
        {
          "id": "parent",
          "children": [
            { "id": "n1" },
            { "id": "n2" }
          ]
        }
      ],
      "edges": [
        { "id": "e1", "source": "n1", "target": "n2" }
      ]
    }
    "#;

    let shared = Rc::new(RefCell::new(parse_lenient_json(graph)));
    let mut importer = Maybe::default();
    let root = ElkGraphJson::for_graph_shared(shared.clone())
        .remember_importer(&mut importer)
        .to_elk()
        .unwrap();

    let parent = find_node(&node_children(&root), "parent");
    let mut edges = node_edges(&parent);
    assert_eq!(edges.len(), 1);
    let edge = edges.remove(0);

    set_node_location(&root, 0.0, 0.0);
    set_node_location(&parent, 100.0, 50.0);

    let junctions = KVectorChain::from_vectors(&[
        KVector::with_values(10.0, 20.0),
        KVector::with_values(30.0, 40.0),
    ]);
    set_edge_property(&edge, CoreOptions::JUNCTION_POINTS, junctions);

    importer
        .get_mut()
        .expect("importer")
        .transfer_layout(&root)
        .unwrap();

    let root_value = shared.borrow();
    let edge_obj = root_value["edges"]
        .as_array()
        .and_then(|edges| {
            edges
                .iter()
                .find(|edge| edge.get("id").and_then(|id| id.as_str()) == Some("e1"))
        })
        .and_then(|edge| edge.as_object())
        .expect("edge");
    let points = edge_obj["junctionPoints"]
        .as_array()
        .expect("junctionPoints");
    assert_eq!(points[0]["x"].as_f64().unwrap(), 110.0);
    assert_eq!(points[0]["y"].as_f64().unwrap(), 70.0);
    assert_eq!(points[1]["x"].as_f64().unwrap(), 130.0);
    assert_eq!(points[1]["y"].as_f64().unwrap(), 90.0);
}

#[test]
fn transfer_layout_edge_coords_parent() {
    let graph = r#"
    {
      "id": "root",
      "children": [
        {
          "id": "a",
          "layoutOptions": { "edgeCoords": "PARENT" },
          "children": [
            { "id": "n1" }
          ],
          "edges": [
            { "id": "e1", "source": "n1", "target": "n2" }
          ]
        },
        {
          "id": "b",
          "children": [
            { "id": "n2" }
          ]
        }
      ]
    }
    "#;

    let shared = Rc::new(RefCell::new(parse_lenient_json(graph)));
    let mut importer = Maybe::default();
    let root = ElkGraphJson::for_graph_shared(shared.clone())
        .remember_importer(&mut importer)
        .to_elk()
        .unwrap();

    let a = find_node(&node_children(&root), "a");
    let mut edges = node_edges(&root);
    assert_eq!(edges.len(), 1);
    let edge = edges.remove(0);

    set_node_location(&root, 0.0, 0.0);
    set_node_location(&a, 100.0, 50.0);

    let junctions = KVectorChain::from_vectors(&[
        KVector::with_values(120.0, 70.0),
        KVector::with_values(150.0, 80.0),
    ]);
    set_edge_property(&edge, CoreOptions::JUNCTION_POINTS, junctions);

    importer
        .get_mut()
        .expect("importer")
        .transfer_layout(&root)
        .unwrap();

    let root_value = shared.borrow();
    let a_obj = root_value["children"]
        .as_array()
        .and_then(|children| {
            children
                .iter()
                .find(|child| child.get("id").and_then(|id| id.as_str()) == Some("a"))
        })
        .and_then(|child| child.as_object())
        .expect("a");
    let edge_obj = a_obj["edges"]
        .as_array()
        .and_then(|edges| {
            edges
                .iter()
                .find(|edge| edge.get("id").and_then(|id| id.as_str()) == Some("e1"))
        })
        .and_then(|edge| edge.as_object())
        .expect("edge");
    let points = edge_obj["junctionPoints"]
        .as_array()
        .expect("junctionPoints");
    assert_eq!(points[0]["x"].as_f64().unwrap(), 20.0);
    assert_eq!(points[0]["y"].as_f64().unwrap(), 20.0);
    assert_eq!(points[1]["x"].as_f64().unwrap(), 50.0);
    assert_eq!(points[1]["y"].as_f64().unwrap(), 30.0);
}

fn parse_lenient_json(input: &str) -> Value {
    json5::from_str(input).expect("lenient json")
}

fn to_json_graph_and_back(graph: &str) -> String {
    let shared = Rc::new(RefCell::new(parse_lenient_json(graph)));
    let mut importer = Maybe::default();
    let root = ElkGraphJson::for_graph_shared(shared.clone())
        .remember_importer(&mut importer)
        .to_elk()
        .unwrap();
    importer
        .get_mut()
        .expect("importer")
        .transfer_layout(&root)
        .unwrap();
    let serialized = serde_json::to_string(&*shared.borrow()).unwrap();
    serialized
}

fn node_children(node: &ElkNodeRef) -> Vec<ElkNodeRef> {
    node.borrow_mut().children().iter().cloned().collect()
}

fn node_edges(node: &ElkNodeRef) -> Vec<ElkEdgeRef> {
    let mut node_mut = node.borrow_mut();
    node_mut.contained_edges().iter().cloned().collect()
}

fn node_ports(
    node: &ElkNodeRef,
) -> Vec<org_eclipse_elk_graph::org::eclipse::elk::graph::ElkPortRef> {
    let mut node_mut = node.borrow_mut();
    node_mut.ports().iter().cloned().collect()
}

fn node_labels(
    node: &ElkNodeRef,
) -> Vec<org_eclipse_elk_graph::org::eclipse::elk::graph::ElkLabelRef> {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .labels()
        .iter()
        .cloned()
        .collect()
}

fn node_identifier(node: &ElkNodeRef) -> Option<String> {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .identifier()
        .map(|value| value.to_string())
}

fn find_node(nodes: &[ElkNodeRef], id: &str) -> ElkNodeRef {
    nodes
        .iter()
        .find(|node| node_identifier(node).as_deref() == Some(id))
        .cloned()
        .expect("node")
}

fn edge_sections(edge: &ElkEdgeRef) -> Vec<ElkEdgeSectionRef> {
    let mut edge_mut = edge.borrow_mut();
    edge_mut.sections().iter().cloned().collect()
}

fn find_section(sections: &[ElkEdgeSectionRef], id: &str) -> ElkEdgeSectionRef {
    sections
        .iter()
        .find(|section| section.borrow().identifier() == Some(id))
        .cloned()
        .expect("section")
}

fn node_has_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
) -> bool {
    let mut node_ref = node.borrow_mut();
    node_ref
        .connectable()
        .shape()
        .graph_element()
        .properties()
        .has_property(property)
}

fn node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
) -> Option<T> {
    let mut node_ref = node.borrow_mut();
    node_ref
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(property)
}

fn edge_property<T: Clone + Send + Sync + 'static>(
    edge: &ElkEdgeRef,
    property: &Property<T>,
) -> Option<T> {
    let mut edge_ref = edge.borrow_mut();
    edge_ref.element().properties_mut().get_property(property)
}

fn port_property<T: Clone + Send + Sync + 'static>(
    port: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkPortRef,
    property: &Property<T>,
) -> Option<T> {
    let mut port_ref = port.borrow_mut();
    port_ref
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(property)
}

fn label_property<T: Clone + Send + Sync + 'static>(
    label: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkLabelRef,
    property: &Property<T>,
) -> Option<T> {
    let mut label_ref = label.borrow_mut();
    label_ref
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(property)
}

fn set_node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
    value: T,
) {
    let mut node_ref = node.borrow_mut();
    node_ref
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn set_edge_property<T: Clone + Send + Sync + 'static>(
    edge: &ElkEdgeRef,
    property: &Property<T>,
    value: T,
) {
    let mut edge_ref = edge.borrow_mut();
    edge_ref
        .element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn set_port_property<T: Clone + Send + Sync + 'static>(
    port: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkPortRef,
    property: &Property<T>,
    value: T,
) {
    let mut port_ref = port.borrow_mut();
    port_ref
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn set_label_property<T: Clone + Send + Sync + 'static>(
    label: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkLabelRef,
    property: &Property<T>,
    value: T,
) {
    let mut label_ref = label.borrow_mut();
    label_ref
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn set_node_identifier(node: &ElkNodeRef, value: &str) {
    let mut node_ref = node.borrow_mut();
    node_ref
        .connectable()
        .shape()
        .graph_element()
        .set_identifier(Some(value.to_string()));
}

fn set_node_location(node: &ElkNodeRef, x: f64, y: f64) {
    let mut node_ref = node.borrow_mut();
    node_ref.connectable().shape().set_location(x, y);
}
