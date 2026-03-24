use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::elk_graph_json::ElkGraphJson;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphArenaSync;

#[test]
fn arena_sync_captures_simple_graph() {
    let json = r#"{
        "id": "root",
        "children": [
            { "id": "n1", "width": 100, "height": 50,
              "ports": [{ "id": "p1" }],
              "labels": [{ "id": "l1", "text": "Node1" }]
            },
            { "id": "n2", "width": 80, "height": 40 }
        ],
        "edges": [
            { "id": "e1", "sources": ["n1"], "targets": ["n2"],
              "sections": [{ "id": "s1",
                "startPoint": {"x": 100, "y": 25},
                "endPoint": {"x": 0, "y": 20},
                "bendPoints": [{"x": 150, "y": 25}]
              }]
            }
        ]
    }"#;

    let root = ElkGraphJson::for_graph(json)
        .lenient(true)
        .to_elk()
        .expect("parse failed");

    let sync = ElkGraphArenaSync::from_root(&root);
    let a = sync.arena();

    // root + n1 + n2 = 3 nodes
    assert_eq!(a.node_count(), 3, "node count");
    // p1 = 1 port
    assert_eq!(a.port_count(), 1, "port count");
    // e1 = 1 edge
    assert_eq!(a.edge_count(), 1, "edge count");
    // l1 = 1 label
    assert_eq!(a.label_count(), 1, "label count");
    // s1 = 1 section
    assert_eq!(a.section_count(), 1, "section count");
    // 1 bend point
    assert_eq!(a.bend_count(), 1, "bend count");

    // Check n1 dimensions
    let n1_id = sync.node_id(&root.borrow_mut().children().iter().next().unwrap().clone()).unwrap();
    assert_eq!(a.node_width[n1_id.idx()], 100.0);
    assert_eq!(a.node_height[n1_id.idx()], 50.0);

    // Check label text
    assert_eq!(a.label_text[0], "Node1");

    // Check section bend point
    assert_eq!(a.bend_x[0], 150.0);
    assert_eq!(a.bend_y[0], 25.0);

    // Check section start/end
    assert_eq!(a.section_start_x[0], 100.0);
    assert_eq!(a.section_end_y[0], 20.0);
}

#[test]
fn arena_sync_bidirectional_mapping() {
    let json = r#"{
        "id": "root",
        "children": [
            { "id": "n1", "ports": [{ "id": "p1" }] },
            { "id": "n2" }
        ],
        "edges": [
            { "id": "e1", "sources": ["p1"], "targets": ["n2"] }
        ]
    }"#;

    let root = ElkGraphJson::for_graph(json)
        .lenient(true)
        .to_elk()
        .expect("parse failed");

    let sync = ElkGraphArenaSync::from_root(&root);

    // Root node round-trip
    let root_id = sync.node_id(&root).unwrap();
    assert!(std::rc::Rc::ptr_eq(sync.node_ref(root_id), &root));

    // Edge source/target connectivity
    let a = sync.arena();
    assert_eq!(a.edge_sources[0].len(), 1, "edge should have 1 source");
    assert_eq!(a.edge_targets[0].len(), 1, "edge should have 1 target");
}
