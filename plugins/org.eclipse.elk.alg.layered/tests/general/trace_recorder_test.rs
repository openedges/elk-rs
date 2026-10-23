use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LGraph, LNode, LPort, Layer,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::trace_recorder::serialize_lgraph_snapshot;

fn temp_trace_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("elk-rs-trace-recorder-test-{nanos}"))
}

#[test]
fn trace_recorder_keeps_layerless_nodes_at_minus_one_even_in_layer_list() {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    graph
        .lock()
        
        .layers_mut()
        .push(layer.clone());

    // Mimics comment post-processing behavior: node is appended into a layer
    // list without assigning node.layer.
    let node = LNode::new(&graph);
    layer
        .lock()
        
        .nodes_mut()
        .push(node);

    let trace_dir = temp_trace_dir();
    {
        let graph_guard = graph.lock();        serialize_lgraph_snapshot(&graph_guard, 0, "CommentPostprocessor", &trace_dir)
            .expect("serialize snapshot");
    }

    let snapshot_path = trace_dir.join("step_000_CommentPostprocessor.json");
    let snapshot_bytes = fs::read(&snapshot_path).expect("read snapshot");
    let snapshot: serde_json::Value =
        serde_json::from_slice(&snapshot_bytes).expect("parse snapshot json");
    let layer_value = snapshot["layers"][0]["nodes"][0]["layer"].as_i64();
    assert_eq!(layer_value, Some(-1));

    let _ = fs::remove_dir_all(&trace_dir);
}

#[test]
fn trace_recorder_preserves_inside_self_loop_passthrough_width_and_east_port_x() {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock();        node_guard.shape().size().x = 24.0;
        node_guard.shape().size().y = 24.0;
        node_guard.set_property(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE, Some(true));
    }
    graph
        .lock()
        
        .layerless_nodes_mut()
        .push(node.clone());

    let west_port = LPort::new();
    LPort::set_node(&west_port, Some(node.clone()));
    {
        let mut west_guard = west_port.lock();        west_guard.set_side(PortSide::West);
        west_guard.shape().position().x = 0.0;
        west_guard.shape().position().y = 12.0;
        west_guard.shape().size().x = 0.0;
        west_guard.shape().size().y = 0.0;
    }

    let east_port = LPort::new();
    LPort::set_node(&east_port, Some(node.clone()));
    {
        let mut east_guard = east_port.lock();        east_guard.set_side(PortSide::East);
        east_guard.shape().position().x = 24.0;
        east_guard.shape().position().y = 12.0;
        east_guard.shape().size().x = 0.0;
        east_guard.shape().size().y = 0.0;
    }

    let trace_dir = temp_trace_dir();
    {
        let graph_guard = graph.lock();        serialize_lgraph_snapshot(&graph_guard, 0, "EdgeAndLayerConstraintEdgeReverser", &trace_dir)
            .expect("serialize snapshot");
    }

    let snapshot_path = trace_dir.join("step_000_EdgeAndLayerConstraintEdgeReverser.json");
    let snapshot_bytes = fs::read(&snapshot_path).expect("read snapshot");
    let snapshot: serde_json::Value =
        serde_json::from_slice(&snapshot_bytes).expect("parse snapshot json");

    let node_width = snapshot["nodes"][0]["width"].as_f64();
    assert_eq!(node_width, Some(24.0));

    let ports = snapshot["nodes"][0]["ports"]
        .as_array()
        .expect("ports array");
    let east_port_x = ports
        .iter()
        .find(|port| port["side"].as_str() == Some("EAST"))
        .and_then(|port| port["x"].as_f64());
    assert_eq!(east_port_x, Some(24.0));

    let _ = fs::remove_dir_all(&trace_dir);
}

#[test]
fn trace_recorder_does_not_compact_passthrough_without_inside_self_loop_flag() {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    graph
        .lock()
        
        .layers_mut()
        .push(layer.clone());

    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock();        node_guard.shape().size().x = 24.0;
        node_guard.shape().size().y = 24.0;
    }
    layer
        .lock()
        
        .nodes_mut()
        .push(node.clone());

    let west_port = LPort::new();
    LPort::set_node(&west_port, Some(node.clone()));
    {
        let mut west_guard = west_port.lock();        west_guard.set_side(PortSide::West);
        west_guard.shape().position().x = 0.0;
        west_guard.shape().position().y = 12.0;
    }

    let east_port = LPort::new();
    LPort::set_node(&east_port, Some(node.clone()));
    {
        let mut east_guard = east_port.lock();        east_guard.set_side(PortSide::East);
        east_guard.shape().position().x = 24.0;
        east_guard.shape().position().y = 12.0;
    }

    let trace_dir = temp_trace_dir();
    {
        let graph_guard = graph.lock();        serialize_lgraph_snapshot(&graph_guard, 11, "InLayerConstraintProcessor", &trace_dir)
            .expect("serialize snapshot");
    }

    let snapshot_path = trace_dir.join("step_011_InLayerConstraintProcessor.json");
    let snapshot_bytes = fs::read(&snapshot_path).expect("read snapshot");
    let snapshot: serde_json::Value =
        serde_json::from_slice(&snapshot_bytes).expect("parse snapshot json");

    let node_width = snapshot["layers"][0]["nodes"][0]["width"].as_f64();
    assert_eq!(node_width, Some(24.0));
    assert!(
        snapshot["layers"][0]["nodes"][0]
            .get("__insideSelfLoopsActivate")
            .is_none()
    );

    let _ = fs::remove_dir_all(&trace_dir);
}

#[test]
fn trace_recorder_preserves_single_south_flat_node_height_and_port_y() {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock();        node_guard.shape().size().x = 64.0;
        node_guard.shape().size().y = 24.0;
    }
    graph
        .lock()
        
        .layerless_nodes_mut()
        .push(node.clone());

    let south_port_left = LPort::new();
    LPort::set_node(&south_port_left, Some(node.clone()));
    {
        let mut port_guard = south_port_left.lock();        port_guard.set_side(PortSide::South);
        port_guard.shape().position().x = 12.0;
        port_guard.shape().position().y = 24.0;
    }

    let south_port_right = LPort::new();
    LPort::set_node(&south_port_right, Some(node.clone()));
    {
        let mut port_guard = south_port_right.lock();        port_guard.set_side(PortSide::South);
        port_guard.shape().position().x = 52.0;
        port_guard.shape().position().y = 24.0;
    }

    let trace_dir = temp_trace_dir();
    {
        let graph_guard = graph.lock();        serialize_lgraph_snapshot(&graph_guard, 92, "EdgeAndLayerConstraintEdgeReverser", &trace_dir)
            .expect("serialize snapshot");
    }

    let snapshot_path = trace_dir.join("step_092_EdgeAndLayerConstraintEdgeReverser.json");
    let snapshot_bytes = fs::read(&snapshot_path).expect("read snapshot");
    let snapshot: serde_json::Value =
        serde_json::from_slice(&snapshot_bytes).expect("parse snapshot json");

    let node_height = snapshot["nodes"][0]["height"].as_f64();
    assert_eq!(node_height, Some(24.0));

    let ports = snapshot["nodes"][0]["ports"]
        .as_array()
        .expect("ports array");
    for port in ports {
        assert_eq!(port["side"].as_str(), Some("SOUTH"));
        assert_eq!(port["y"].as_f64(), Some(24.0));
    }

    let _ = fs::remove_dir_all(&trace_dir);
}

#[test]
fn trace_recorder_preserves_single_north_flat_node_height_and_keeps_port_y_zero() {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock();        node_guard.shape().size().x = 64.0;
        node_guard.shape().size().y = 24.0;
    }
    graph
        .lock()
        
        .layerless_nodes_mut()
        .push(node.clone());

    let north_port_left = LPort::new();
    LPort::set_node(&north_port_left, Some(node.clone()));
    {
        let mut port_guard = north_port_left.lock();        port_guard.set_side(PortSide::North);
        port_guard.shape().position().x = 12.0;
        port_guard.shape().position().y = 0.0;
    }

    let north_port_right = LPort::new();
    LPort::set_node(&north_port_right, Some(node.clone()));
    {
        let mut port_guard = north_port_right.lock();        port_guard.set_side(PortSide::North);
        port_guard.shape().position().x = 52.0;
        port_guard.shape().position().y = 0.0;
    }

    let trace_dir = temp_trace_dir();
    {
        let graph_guard = graph.lock();        serialize_lgraph_snapshot(&graph_guard, 114, "EdgeAndLayerConstraintEdgeReverser", &trace_dir)
            .expect("serialize snapshot");
    }

    let snapshot_path = trace_dir.join("step_114_EdgeAndLayerConstraintEdgeReverser.json");
    let snapshot_bytes = fs::read(&snapshot_path).expect("read snapshot");
    let snapshot: serde_json::Value =
        serde_json::from_slice(&snapshot_bytes).expect("parse snapshot json");

    let node_height = snapshot["nodes"][0]["height"].as_f64();
    assert_eq!(node_height, Some(24.0));

    let ports = snapshot["nodes"][0]["ports"]
        .as_array()
        .expect("ports array");
    for port in ports {
        assert_eq!(port["side"].as_str(), Some("NORTH"));
        assert_eq!(port["y"].as_f64(), Some(0.0));
    }

    let _ = fs::remove_dir_all(&trace_dir);
}

#[test]
fn trace_recorder_preserves_non_centered_passthrough_width_and_east_port_x() {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock();        node_guard.shape().size().x = 24.0;
        node_guard.shape().size().y = 46.0;
    }
    graph
        .lock()
        
        .layerless_nodes_mut()
        .push(node.clone());

    let west_top = LPort::new();
    LPort::set_node(&west_top, Some(node.clone()));
    {
        let mut port_guard = west_top.lock();        port_guard.set_side(PortSide::West);
        port_guard.shape().position().x = 0.0;
        port_guard.shape().position().y = 12.0;
    }

    let west_bottom = LPort::new();
    LPort::set_node(&west_bottom, Some(node.clone()));
    {
        let mut port_guard = west_bottom.lock();        port_guard.set_side(PortSide::West);
        port_guard.shape().position().x = 0.0;
        port_guard.shape().position().y = 33.0;
    }

    let east_top = LPort::new();
    LPort::set_node(&east_top, Some(node.clone()));
    {
        let mut port_guard = east_top.lock();        port_guard.set_side(PortSide::East);
        port_guard.shape().position().x = 24.0;
        port_guard.shape().position().y = 12.0;
    }

    let east_bottom = LPort::new();
    LPort::set_node(&east_bottom, Some(node.clone()));
    {
        let mut port_guard = east_bottom.lock();        port_guard.set_side(PortSide::East);
        port_guard.shape().position().x = 24.0;
        port_guard.shape().position().y = 33.0;
    }

    let trace_dir = temp_trace_dir();
    {
        let graph_guard = graph.lock();        serialize_lgraph_snapshot(&graph_guard, 158, "EdgeAndLayerConstraintEdgeReverser", &trace_dir)
            .expect("serialize snapshot");
    }

    let snapshot_path = trace_dir.join("step_158_EdgeAndLayerConstraintEdgeReverser.json");
    let snapshot_bytes = fs::read(&snapshot_path).expect("read snapshot");
    let snapshot: serde_json::Value =
        serde_json::from_slice(&snapshot_bytes).expect("parse snapshot json");

    let node_width = snapshot["nodes"][0]["width"].as_f64();
    assert_eq!(node_width, Some(24.0));

    let east_port_x: Vec<f64> = snapshot["nodes"][0]["ports"]
        .as_array()
        .expect("ports array")
        .iter()
        .filter(|port| port["side"].as_str() == Some("EAST"))
        .filter_map(|port| port["x"].as_f64())
        .collect();
    assert_eq!(east_port_x, vec![24.0, 24.0]);

    let _ = fs::remove_dir_all(&trace_dir);
}

#[test]
fn trace_recorder_preserves_passthrough_with_pre_shifted_east_ports() {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock();        node_guard.shape().size().x = 24.0;
        node_guard.shape().size().y = 64.0;
    }
    graph
        .lock()
        
        .layerless_nodes_mut()
        .push(node.clone());

    for y in [12.0, 32.0, 52.0] {
        let west = LPort::new();
        LPort::set_node(&west, Some(node.clone()));
        {
            let mut port_guard = west.lock();            port_guard.set_side(PortSide::West);
            port_guard.shape().position().x = 0.0;
            port_guard.shape().position().y = y;
        }

        let east = LPort::new();
        LPort::set_node(&east, Some(node.clone()));
        {
            let mut port_guard = east.lock();            port_guard.set_side(PortSide::East);
            port_guard.shape().position().x = 4.0;
            port_guard.shape().position().y = y;
        }
    }

    let trace_dir = temp_trace_dir();
    {
        let graph_guard = graph.lock();        serialize_lgraph_snapshot(&graph_guard, 144, "EdgeAndLayerConstraintEdgeReverser", &trace_dir)
            .expect("serialize snapshot");
    }

    let snapshot_path = trace_dir.join("step_144_EdgeAndLayerConstraintEdgeReverser.json");
    let snapshot_bytes = fs::read(&snapshot_path).expect("read snapshot");
    let snapshot: serde_json::Value =
        serde_json::from_slice(&snapshot_bytes).expect("parse snapshot json");

    let node_width = snapshot["nodes"][0]["width"].as_f64();
    assert_eq!(node_width, Some(24.0));

    let east_port_x: Vec<f64> = snapshot["nodes"][0]["ports"]
        .as_array()
        .expect("ports array")
        .iter()
        .filter(|port| port["side"].as_str() == Some("EAST"))
        .filter_map(|port| port["x"].as_f64())
        .collect();
    assert_eq!(east_port_x, vec![4.0, 4.0, 4.0]);

    let _ = fs::remove_dir_all(&trace_dir);
}

#[test]
fn trace_recorder_does_not_compact_passthrough_when_rows_do_not_align() {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock();        node_guard.shape().size().x = 24.0;
        node_guard.shape().size().y = 44.0;
    }
    graph
        .lock()
        
        .layerless_nodes_mut()
        .push(node.clone());

    let west = LPort::new();
    LPort::set_node(&west, Some(node.clone()));
    {
        let mut port_guard = west.lock();        port_guard.set_side(PortSide::West);
        port_guard.shape().position().x = 0.0;
        port_guard.shape().position().y = 12.0;
    }

    let east = LPort::new();
    LPort::set_node(&east, Some(node.clone()));
    {
        let mut port_guard = east.lock();        port_guard.set_side(PortSide::East);
        port_guard.shape().position().x = 24.0;
        port_guard.shape().position().y = 32.0;
    }

    let trace_dir = temp_trace_dir();
    {
        let graph_guard = graph.lock();        serialize_lgraph_snapshot(&graph_guard, 23, "EdgeAndLayerConstraintEdgeReverser", &trace_dir)
            .expect("serialize snapshot");
    }

    let snapshot_path = trace_dir.join("step_023_EdgeAndLayerConstraintEdgeReverser.json");
    let snapshot_bytes = fs::read(&snapshot_path).expect("read snapshot");
    let snapshot: serde_json::Value =
        serde_json::from_slice(&snapshot_bytes).expect("parse snapshot json");

    let node_width = snapshot["nodes"][0]["width"].as_f64();
    assert_eq!(node_width, Some(24.0));

    let east_port_x = snapshot["nodes"][0]["ports"][1]["x"].as_f64();
    assert_eq!(east_port_x, Some(24.0));

    let _ = fs::remove_dir_all(&trace_dir);
}

#[test]
fn trace_records_cross_hierarchy_endpoints_with_unique_ids() {
    use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LEdge;
    let graph = LGraph::new();
    let nested = LGraph::new();
    let source = LNode::new(&graph);
    let target = LNode::new(&nested);
    graph.lock().layerless_nodes_mut().push(source.clone());
    nested.lock().layerless_nodes_mut().push(target.clone());
    let source_port = LPort::new();
    let target_port = LPort::new();
    LPort::set_node(&source_port, Some(source));
    LPort::set_node(&target_port, Some(target));
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source_port));
    LEdge::set_target(&edge, Some(target_port));
    let dir = temp_trace_dir();
    serialize_lgraph_snapshot(&graph.lock(), 0, "Test", &dir).unwrap();
    let data: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("step_000_Test.json")).unwrap()).unwrap();
    assert_eq!(data["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(data["externalNodes"].as_array().unwrap().len(), 1);
    assert_ne!(data["nodes"][0]["id"], data["externalNodes"][0]["id"]);
    assert_eq!(
        data["edges"][0]["targetNode"],
        data["externalNodes"][0]["id"]
    );
    assert_eq!(
        data["edges"][0]["targetPort"],
        data["externalNodes"][0]["ports"][0]["id"]
    );
    fs::remove_dir_all(dir).unwrap();
}
