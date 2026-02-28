use org_eclipse_elk_core::org::eclipse::elk::core::options::{CoreOptions, Direction};
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::{ElkGraphJson, JsonImportError};

use super::common::*;

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
