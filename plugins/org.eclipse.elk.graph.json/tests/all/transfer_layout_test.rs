use std::cell::RefCell;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::math::{KVector, KVectorChain};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{CoreOptions, PortSide};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, Maybe};
use org_eclipse_elk_core::org::eclipse::elk::core::{
    IGraphLayoutEngine, RecursiveGraphLayoutEngine,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::ElkGraphJson;

use super::common::*;

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
