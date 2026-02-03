use std::cell::RefCell;
use std::rc::Rc;

use serde_json::Value;

use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMargin, ElkPadding};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{CoreOptions, Direction};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    BasicProgressMonitor, IndividualSpacings, Maybe,
};
use org_eclipse_elk_core::org::eclipse::elk::core::{
    IGraphLayoutEngine, RecursiveGraphLayoutEngine,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkEdgeSectionRef, ElkNodeRef,
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
fn id_missing_fails() {
    assert_import_error("{}");
}

#[test]
fn id_wrong_type_number_fails() {
    assert_import_error("{ id: 1.2 }");
}

#[test]
fn id_wrong_type_object_fails() {
    assert_import_error("{ id: {} }");
}

#[test]
fn id_wrong_type_array_fails() {
    assert_import_error("{ id: [] }");
}

#[test]
fn id_wrong_type_boolean_fails() {
    assert_import_error("{ id: true }");
}

#[test]
fn id_string_ok() {
    ElkGraphJson::for_graph("{ id: 'foo' }").to_elk().unwrap();
}

#[test]
fn id_int_ok() {
    ElkGraphJson::for_graph("{ id: 3 }").to_elk().unwrap();
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
    assert_eq!(node_property(&root, CoreOptions::DIRECTION), Some(Direction::Down));
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
    assert_eq!(node_property(&root, CoreOptions::DIRECTION), Some(Direction::Down));
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
    assert_eq!(node_property(&root, CoreOptions::DIRECTION), Some(Direction::Up));
}

#[test]
fn export_layout_options() {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(&graph, CoreOptions::DIRECTION, Direction::Up);

    let json = ElkGraphJson::for_elk(graph).to_json();
    let actual: Value = serde_json::from_str(&json).unwrap();
    let expected: Value = serde_json::from_str(
        r#"{"id":"n0","layoutOptions":{"elk.direction":"UP"}}"#,
    )
    .unwrap();

    assert_eq!(expected, actual);
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

    let mut individual = node_property(&child, CoreOptions::SPACING_INDIVIDUAL)
        .expect("individual spacings");
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
    individual
        .properties_mut()
        .set_property(CoreOptions::NODE_LABELS_PADDING, Some(ElkPadding::with_values(1.0, 2.0, 3.0, 4.0)));
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
    individual
        .properties_mut()
        .set_property(CoreOptions::SPACING_PORTS_SURROUNDING, Some(ElkMargin::with_values(2.0, 4.0, 6.0, 8.0)));
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

    match importer
        .get_mut()
        .expect("importer")
        .transfer_layout(&root)
    {
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

fn assert_import_error(input: &str) {
    assert!(matches!(
        ElkGraphJson::for_graph(input).to_elk(),
        Err(JsonImportError::Import(_))
    ));
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

fn set_node_identifier(node: &ElkNodeRef, value: &str) {
    let mut node_ref = node.borrow_mut();
    node_ref
        .connectable()
        .shape()
        .graph_element()
        .set_identifier(Some(value.to_string()));
}
