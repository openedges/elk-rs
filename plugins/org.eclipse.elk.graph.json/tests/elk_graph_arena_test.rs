use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::elk_graph_json::ElkGraphJson;
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api::layout_json;
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

#[test]
fn arena_captures_post_layout_positions() {
    let input = r#"{
        "id": "root",
        "layoutOptions": { "org.eclipse.elk.algorithm": "layered" },
        "children": [
            { "id": "n1", "width": 40, "height": 20 },
            { "id": "n2", "width": 40, "height": 20 }
        ],
        "edges": [
            { "id": "e1", "sources": ["n1"], "targets": ["n2"] }
        ]
    }"#;

    // Run full layout pipeline
    let output = layout_json(input, "{}").expect("layout failed");
    let out_val: serde_json::Value = serde_json::from_str(&output).expect("parse output");

    // Re-import the ORIGINAL graph, run layout, then build arena from post-layout state
    let root = ElkGraphJson::for_graph(input)
        .lenient(true)
        .to_elk()
        .expect("import failed");
    {
        let mut engine = org_eclipse_elk_core::org::eclipse::elk::core::recursive_graph_layout_engine::RecursiveGraphLayoutEngine::new();
        let mut monitor = org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor::new();
        engine.layout(&root, &mut monitor);
    }

    let sync = ElkGraphArenaSync::from_root(&root);
    let a = sync.arena();

    // Arena should capture post-layout positions (non-zero for laid-out nodes)
    // n1 and n2 should have been placed by the layered algorithm
    let children: Vec<_> = root.borrow_mut().children().iter().cloned().collect();
    let n1_id = sync.node_id(&children[0]).unwrap();
    let n2_id = sync.node_id(&children[1]).unwrap();

    // Verify arena positions match the live ElkGraph positions
    let (live_n1_x, live_n1_y) = {
        let mut n = children[0].borrow_mut();
        let s = n.connectable().shape();
        (s.x(), s.y())
    };
    assert_eq!(a.node_x[n1_id.idx()], live_n1_x, "n1 x should match live");
    assert_eq!(a.node_y[n1_id.idx()], live_n1_y, "n1 y should match live");

    // Both nodes should have positions assigned (at least one non-zero coordinate)
    let n1_placed = a.node_x[n1_id.idx()] != 0.0 || a.node_y[n1_id.idx()] != 0.0;
    let n2_placed = a.node_x[n2_id.idx()] != 0.0 || a.node_y[n2_id.idx()] != 0.0;
    assert!(n1_placed || n2_placed, "at least one node should be placed at non-origin");

    // Verify JSON output has the same positions as arena
    if let Some(children_json) = out_val.get("children").and_then(|c| c.as_array()) {
        for child in children_json {
            let id = child.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let jx = child.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let jy = child.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if id == "n1" {
                assert!((a.node_x[n1_id.idx()] - jx).abs() < 0.01, "n1 arena x={} vs json x={}", a.node_x[n1_id.idx()], jx);
                assert!((a.node_y[n1_id.idx()] - jy).abs() < 0.01, "n1 arena y={} vs json y={}", a.node_y[n1_id.idx()], jy);
            }
        }
    }
}
