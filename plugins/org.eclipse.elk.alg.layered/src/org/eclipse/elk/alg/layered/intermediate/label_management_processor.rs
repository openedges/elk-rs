use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::labels::LabelManagementOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphUtil, LLabelRef, LNodeRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};

pub struct LabelManagementProcessor {
    center_labels: bool,
}

impl LabelManagementProcessor {
    const MIN_WIDTH_PORT_LABELS: f64 = 20.0;
    const MIN_WIDTH_NODE_LABELS: f64 = 40.0;
    pub const MIN_WIDTH_EDGE_LABELS: f64 = 60.0;

    pub fn new(center_labels: bool) -> Self {
        Self { center_labels }
    }
}

impl ILayoutProcessor<LGraph> for LabelManagementProcessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Label management", 1.0);

        let Some(label_manager) = layered_graph.get_property(LabelManagementOptions::LABEL_MANAGER)
        else {
            monitor.done();
            return;
        };

        let edge_label_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_EDGE_LABEL)
            .unwrap_or(2.0);
        let label_label_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_LABEL_LABEL)
            .unwrap_or(1.0);
        let direction = layered_graph
            .get_property(LayeredOptions::DIRECTION)
            .unwrap_or(Direction::Right);
        let vertical_layout = direction.is_vertical();

        if self.center_labels {
            manage_center_labels(
                layered_graph,
                &label_manager,
                edge_label_spacing,
                label_label_spacing,
                direction,
                vertical_layout,
            );
        } else {
            manage_non_center_labels(
                layered_graph,
                &label_manager,
                label_label_spacing,
                vertical_layout,
            );
        }

        monitor.done();
    }
}

fn manage_non_center_labels(
    graph: &mut LGraph,
    label_manager: &Arc<dyn org_eclipse_elk_core::org::eclipse::elk::core::labels::ILabelManager>,
    label_label_spacing: f64,
    vertical_layout: bool,
) {
    for layer in graph.layers().clone() {
        let nodes = layer
            .lock().nodes().clone();

        for node in nodes {
            let (node_type, node_labels, ports, top_comments, bottom_comments, outgoing_edges) =
                match node.lock_ok() {
            Some(mut node_guard) => (
                        node_guard.node_type(),
                        node_guard.labels().clone(),
                        node_guard.ports().clone(),
                        node_guard
                            .get_property(InternalProperties::TOP_COMMENTS)
                            .unwrap_or_default(),
                        node_guard
                            .get_property(InternalProperties::BOTTOM_COMMENTS)
                            .unwrap_or_default(),
                        node_guard.outgoing_edges(),
                    ),
            None => continue,
                };

            if node_type == NodeType::Normal {
                do_manage_labels(
                    label_manager,
                    &node_labels,
                    LabelManagementProcessor::MIN_WIDTH_NODE_LABELS,
                    label_label_spacing,
                    vertical_layout,
                );

                for port in ports {
                    let labels = port
                        .lock().labels().clone();
                    do_manage_labels(
                        label_manager,
                        &labels,
                        LabelManagementProcessor::MIN_WIDTH_PORT_LABELS,
                        label_label_spacing,
                        vertical_layout,
                    );
                }

                do_manage_attached_comment_labels(
                    label_manager,
                    &top_comments,
                    LabelManagementProcessor::MIN_WIDTH_NODE_LABELS,
                    vertical_layout,
                );
                do_manage_attached_comment_labels(
                    label_manager,
                    &bottom_comments,
                    LabelManagementProcessor::MIN_WIDTH_NODE_LABELS,
                    vertical_layout,
                );
            }

            for edge in outgoing_edges {
                let labels = edge
                    .lock().labels().clone();
                do_manage_labels(
                    label_manager,
                    &labels,
                    LabelManagementProcessor::MIN_WIDTH_EDGE_LABELS,
                    0.0,
                    vertical_layout,
                );
            }
        }
    }
}

fn do_manage_attached_comment_labels(
    label_manager: &Arc<dyn org_eclipse_elk_core::org::eclipse::elk::core::labels::ILabelManager>,
    comment_nodes: &[LNodeRef],
    min_width_node_labels: f64,
    vertical_layout: bool,
) {
    for comment_node in comment_nodes {
        let labels = comment_node
            .lock().labels().clone();
        if labels.is_empty() {
            continue;
        }

        do_manage_labels(
            label_manager,
            &labels,
            min_width_node_labels,
            0.0,
            vertical_layout,
        );
    }
}

fn manage_center_labels(
    graph: &mut LGraph,
    label_manager: &Arc<dyn org_eclipse_elk_core::org::eclipse::elk::core::labels::ILabelManager>,
    edge_label_spacing: f64,
    label_label_spacing: f64,
    direction: Direction,
    vertical_layout: bool,
) {
    for layer in graph.layers().clone() {
        let max_width = LabelManagementProcessor::MIN_WIDTH_EDGE_LABELS.max(
            LGraphUtil::find_max_non_dummy_node_width(&layer, direction, false),
        );

        let nodes = layer
            .lock().nodes().clone();

        for node in nodes {
            let (node_type, connected_edges, represented_labels, node_labels) = match node.lock_ok() {
            Some(mut node_guard) => (
                    node_guard.node_type(),
                    node_guard.connected_edges(),
                    node_guard
                        .get_property(InternalProperties::REPRESENTED_LABELS)
                        .unwrap_or_default(),
                    node_guard.labels().clone(),
                ),
            None => continue,
            };

            if node_type != NodeType::Label {
                continue;
            }

            let edge_thickness = connected_edges
                .first()
                .and_then(|edge| {
                    edge.lock_ok().and_then(|mut edge_guard| {
                        edge_guard.get_property(CoreOptions::EDGE_THICKNESS)
                    })
                })
                .unwrap_or(1.0);

            let labels = if represented_labels.is_empty() {
                node_labels
            } else {
                represented_labels
            };
            let required = do_manage_labels(
                label_manager,
                &labels,
                max_width,
                label_label_spacing,
                vertical_layout,
            );

            {
                let mut node_guard = node.lock();
                node_guard.shape().size().x = required.x;
                node_guard.shape().size().y = required.y + edge_thickness + edge_label_spacing;
            }
        }
    }
}

fn do_manage_labels(
    label_manager: &Arc<dyn org_eclipse_elk_core::org::eclipse::elk::core::labels::ILabelManager>,
    labels: &[LLabelRef],
    target_width: f64,
    label_label_spacing: f64,
    vertical_layout: bool,
) -> KVector {
    let mut required = KVector::new();
    if labels.is_empty() {
        return required;
    }

    for label in labels {
        let Some(mut label_guard) = label.lock_ok() else {
            continue;
        };

        let origin = label_guard.get_property(InternalProperties::ORIGIN);
        if let Some(origin) = origin {
            if let Some(new_size) = label_manager.manage_label_size(&origin, target_width) {
                if vertical_layout {
                    label_guard.shape().size().x = new_size.y;
                    label_guard.shape().size().y = new_size.x;
                } else {
                    label_guard.shape().size().x = new_size.x;
                    label_guard.shape().size().y = new_size.y;
                }
            }
        }

        let label_size = *label_guard.shape().size_ref();
        if vertical_layout {
            required.x += label_label_spacing + label_size.x;
            required.y = required.y.max(label_size.y);
        } else {
            required.x = required.x.max(label_size.x);
            required.y += label_label_spacing + label_size.y;
        }
    }

    if vertical_layout {
        required.x -= label_label_spacing;
    } else {
        required.y -= label_label_spacing;
    }

    required
}
