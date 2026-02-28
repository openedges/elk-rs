use std::cell::RefCell;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, Maybe};
use org_eclipse_elk_core::org::eclipse::elk::core::{
    IGraphLayoutEngine, RecursiveGraphLayoutEngine,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef;
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::ElkGraphJson;

use super::common::*;

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
