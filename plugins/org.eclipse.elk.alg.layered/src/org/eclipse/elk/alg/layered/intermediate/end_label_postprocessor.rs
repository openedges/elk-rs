use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, NodeType};
use crate::org::eclipse::elk::alg::layered::options::internal_properties::EndLabelMap;
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;

pub struct EndLabelPostprocessor;

impl ILayoutProcessor<LGraph> for EndLabelPostprocessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("End label post-processing", 1.0);

        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock().nodes().clone();
            for node in nodes {
                let should_process = {
                    let mut node_guard = node.lock();
                    matches!(
                        node_guard.node_type(),
                        NodeType::Normal | NodeType::ExternalPort
                    ) && node_guard
                        .shape()
                        .graph_element()
                        .properties()
                        .has_property(InternalProperties::END_LABELS)
                };

                if should_process {
                    process_node(&node);
                }
            }
        }

        monitor.done();
    }
}

fn process_node(node: &crate::org::eclipse::elk::alg::layered::graph::LNodeRef) {
    let (node_pos, end_label_cells) = {
        let mut guard = node.lock();
        (
            *guard.shape().position_ref(),
            guard.get_property(InternalProperties::END_LABELS),
        )
    };

    let Some(end_label_cells) = end_label_cells else {
        return;
    };
    if end_label_cells.is_empty() {
        {
            let mut guard = node.lock();
            guard.set_property::<EndLabelMap>(InternalProperties::END_LABELS, None);
        }
        return;
    }

    for label_cell in end_label_cells.values() {
        {
            let mut cell_guard = label_cell.lock();
            let rect = cell_guard.cell_rectangle();
            rect.move_by(&node_pos);
            cell_guard.apply_label_layout();
        }
    }

    {
        let mut guard = node.lock();
        guard.set_property::<EndLabelMap>(InternalProperties::END_LABELS, None);
    }
}
