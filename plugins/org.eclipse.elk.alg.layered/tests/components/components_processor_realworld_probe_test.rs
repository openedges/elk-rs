
use std::path::PathBuf;
use std::sync::Arc;

use crate::common::elkt_test_loader::load_layered_graph_from_elkt;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::elk_layered::ElkLayered;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::components::components_processor::ComponentsProcessor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph_configurator::GraphConfigurator;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_importer::ElkGraphImporter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{GraphProperties, InternalProperties, LayeredOptions};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;

fn import_lgraph(
    rel_path: &str,
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef {
    initialize_plain_java_layout();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../external/elk-models").join(rel_path);
    let graph = load_layered_graph_from_elkt(path.to_str().expect("utf8 path")).expect("ELKT should load");
    let mut origin_store = OriginStore::new();
    let mut importer = ElkGraphImporter::new(&mut origin_store);
    importer.import_graph(&graph)
}

fn split_sizes(lgraph: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef) -> Vec<usize> {
    ComponentsProcessor::new()
        .split(lgraph)
        .iter()
        .map(|component| {
            component
                .lock_ok()
                .map(|component_guard| component_guard.layerless_nodes().len())
                .unwrap_or_default()
        })
        .collect()
}

fn split_sizes_via_layout_prepare(
    lgraph: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef,
) -> Vec<usize> {
    let mut graph_configurator = GraphConfigurator::new();
    graph_configurator.prepare_graph_for_layout(lgraph);
    ComponentsProcessor::new()
        .split(lgraph)
        .iter()
        .map(|component| {
            component
                .lock_ok()
                .map(|component_guard| component_guard.layerless_nodes().len())
                .unwrap_or_default()
        })
        .collect()
}

fn edge_bend_stats_after_layout(
    rel_path: &str,
) -> (usize, usize, usize) {
    let lgraph = import_lgraph(rel_path);
    let mut layered = ElkLayered::new();
    layered.do_layout(&lgraph, None);

    let (layerless_nodes, layers) = lgraph
        .lock_ok()
        .map(|graph_guard| (graph_guard.layerless_nodes().clone(), graph_guard.layers().clone()))
        .unwrap_or_default();
    let mut nodes = layerless_nodes;
    for layer in layers {
        {
            let layer_guard = layer.lock();
            nodes.extend(layer_guard.nodes().iter().cloned());
        }
    }

    let mut edges = Vec::new();
    for node in nodes {
        let outgoing = node
            .lock().outgoing_edges();
        for edge in outgoing {
            if !edges.iter().any(|existing| Arc::ptr_eq(existing, &edge)) {
                edges.push(edge);
            }
        }
    }

    let edge_count = edges.len();
    let edges_with_bends = edges
        .iter()
        .filter(|edge| {
            edge.lock_ok()
                .is_some_and(|edge_guard| !edge_guard.bend_points_ref().is_empty())
        })
        .count();
    let max_bends = edges
        .iter()
        .filter_map(|edge| {
            edge.lock_ok()
                .map(|edge_guard| edge_guard.bend_points_ref().len())
        })
        .max()
        .unwrap_or(0);
    (edge_count, edges_with_bends, max_bends)
}

fn dump_node_positions_after_layout(rel_path: &str) {
    let lgraph = import_lgraph(rel_path);
    let mut layered = ElkLayered::new();
    layered.do_layout(&lgraph, None);

    let layers = lgraph
        .lock().layers().clone();

    for (layer_index, layer) in layers.iter().enumerate() {
        {
            let layer_guard = layer.lock();
            for node in layer_guard.nodes() {
                {
                    let mut node_guard = node.lock();
                    if node_guard.node_type()
                        == org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::NodeType::Normal
                    {
                        let pos = *node_guard.shape().position_ref();
                        let name = node_guard.designation();
                        eprintln!(
                            "[node-pos-probe] model={rel_path} layer={layer_index} node={name} pos=({}, {})",
                            pos.x, pos.y
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn ptolemy_flattened_models_remain_single_component_after_import() {
    let models = [
        "realworld/ptolemy/flattened/algebraic_rlc_RLC.elkt",
        "realworld/ptolemy/flattened/backtrack_primetest_PrimeTest.elkt",
        "realworld/ptolemy/flattened/continuous_cartracking_CarTracking.elkt",
    ];

    for model in models {
        let lgraph = import_lgraph(model);
        let (node_count, has_external_ports, edge_routing_before, edge_routing_prepared) = {
            let mut graph_guard = lgraph.lock();            let graph_props = graph_guard
                .get_property(InternalProperties::GRAPH_PROPERTIES)
                .unwrap_or_else(EnumSet::none_of);
            let edge_routing_before = graph_guard
                .get_property(LayeredOptions::EDGE_ROUTING)
                .unwrap_or(EdgeRouting::Undefined);
            drop(graph_guard);

            let mut graph_configurator = GraphConfigurator::new();
            graph_configurator.prepare_graph_for_layout(&lgraph);

            let mut graph_guard = lgraph.lock();            let edge_routing_prepared = graph_guard
                .get_property(LayeredOptions::EDGE_ROUTING)
                .unwrap_or(EdgeRouting::Undefined);
            (
                graph_guard.layerless_nodes().len(),
                graph_props.contains(&GraphProperties::ExternalPorts),
                edge_routing_before,
                edge_routing_prepared,
            )
        };

        let sizes = split_sizes(&lgraph);
        let prepared_sizes = split_sizes_via_layout_prepare(&lgraph);
        let (edge_count, edges_with_bends, max_bends) = edge_bend_stats_after_layout(model);
        if model.contains("algebraic_rlc_RLC.elkt") {
            dump_node_positions_after_layout(model);
        }
        eprintln!(
            "[components-probe] model={model} nodes={node_count} external_ports={has_external_ports} edge_routing_before={edge_routing_before:?} edge_routing_prepared={edge_routing_prepared:?} split_sizes={sizes:?} prepared_split_sizes={prepared_sizes:?} edge_count={edge_count} edges_with_bends={edges_with_bends} max_bends={max_bends}"
        );

        assert_eq!(sizes.len(), 1, "expected a single component for {model}");
        assert_eq!(
            sizes[0], node_count,
            "single component should contain every node for {model}"
        );
        assert_eq!(
            prepared_sizes.len(),
            1,
            "expected a single prepared component for {model}"
        );
    }
}
