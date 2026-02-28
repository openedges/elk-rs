use std::rc::Rc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::ElkGraphJson;

use super::common::*;

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
