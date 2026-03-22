use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, NodeType};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;

pub struct EndLabelSorter;

impl ILayoutProcessor<LGraph> for EndLabelSorter {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Sort end labels", 1.0);

        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock().nodes().clone();
            for node in nodes {
                if node
                    .lock_ok()
                    .map(|node_guard| node_guard.node_type() != NodeType::Label)
                    .unwrap_or(true)
                {
                    continue;
                }
                sort_represented_labels(&node);
            }
        }

        monitor.done();
    }
}

fn sort_represented_labels(node: &crate::org::eclipse::elk::alg::layered::graph::LNodeRef) {
    let labels = node
        .lock_ok()
        .and_then(|mut node_guard| node_guard.get_property(InternalProperties::REPRESENTED_LABELS))
        .unwrap_or_default();
    if labels.len() < 2 {
        return;
    }

    let mut sorted = labels;
    sorted.sort_by(|left, right| {
        let left_text = left
            .lock_ok()
            .map(|label_guard| label_guard.text().to_owned())
            .unwrap_or_default();
        let right_text = right
            .lock_ok()
            .map(|label_guard| label_guard.text().to_owned())
            .unwrap_or_default();
        left_text.cmp(&right_text)
    });

    {
        let mut node_guard = node.lock();
        node_guard.set_property(InternalProperties::REPRESENTED_LABELS, Some(sorted));
    }
}
