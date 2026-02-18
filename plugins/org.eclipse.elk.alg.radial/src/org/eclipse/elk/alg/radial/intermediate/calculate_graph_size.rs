use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::options::RadialOptions;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;

#[derive(Default)]
pub struct CalculateGraphSize;

impl ILayoutProcessor<ElkNodeRef> for CalculateGraphSize {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Calculate Graph Size", 1.0);
        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "Before");
        }

        let children: Vec<ElkNodeRef> = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.children().iter().cloned().collect()
        };

        let mut min_x_pos = f64::MAX;
        let mut min_y_pos = f64::MAX;
        let mut max_x_pos = f64::MIN;
        let mut max_y_pos = f64::MIN;

        for node in &children {
            let (pos_x, pos_y, width, height, margins) = {
                let mut node_mut = node.borrow_mut();
                let margins = node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .get_property(CoreOptions::MARGINS)
                    .unwrap_or_default();
                let (x, y, width, height) = {
                    let shape = node_mut.connectable().shape();
                    (shape.x(), shape.y(), shape.width(), shape.height())
                };
                (x, y, width, height, margins)
            };

            min_x_pos = min_x_pos.min(pos_x - margins.left);
            min_y_pos = min_y_pos.min(pos_y - margins.top);
            max_x_pos = max_x_pos.max(pos_x + width + margins.right);
            max_y_pos = max_y_pos.max(pos_y + height + margins.bottom);
        }

        let padding = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::PADDING)
        }
        .unwrap_or_default();

        let mut offset = KVector::with_values(min_x_pos - padding.left, min_y_pos - padding.top);

        let mut width = max_x_pos - min_x_pos + padding.left + padding.right;
        let mut height = max_y_pos - min_y_pos + padding.top + padding.bottom;

        if {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RadialOptions::CENTER_ON_ROOT)
        }
        .unwrap_or(false)
        {
            if let Some(root) = RadialUtil::root_from_graph(graph) {
                let (root_x, root_y) = {
                    let mut root_mut = root.borrow_mut();
                    let margins = root_mut
                        .connectable()
                        .shape()
                        .graph_element()
                        .properties_mut()
                        .get_property(CoreOptions::MARGINS)
                        .unwrap_or_default();
                    let (root_x, root_y) = {
                        let shape = root_mut.connectable().shape();
                        (
                            shape.x() + shape.width() / 2.0 + (margins.left + margins.right) / 2.0
                                - offset.x,
                            shape.y() + shape.height() / 2.0 + (margins.top + margins.bottom) / 2.0
                                - offset.y,
                        )
                    };
                    (root_x, root_y)
                };

                let dx = width - root_x;
                let dy = height - root_y;

                if dx < width / 2.0 {
                    let additional_x = dx - root_x;
                    width += additional_x;
                    offset.x -= additional_x;
                } else {
                    let additional_x = root_x - dx;
                    width += additional_x;
                }

                if dy < height / 2.0 {
                    let additional_y = dy - root_y;
                    height += additional_y;
                    offset.y -= additional_y;
                } else {
                    let additional_y = root_y - dy;
                    height += additional_y;
                }
            }
        }

        for node in &children {
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            shape.set_x(shape.x() - offset.x);
            shape.set_y(shape.y() - offset.y);
        }

        let fixed_graph_size = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE)
        }
        .unwrap_or(false);

        if !fixed_graph_size {
            let mut graph_mut = graph.borrow_mut();
            let shape = graph_mut.connectable().shape();
            shape.set_width(width);
            shape.set_height(height);
        }

        {
            let mut graph_mut = graph.borrow_mut();
            let props = graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            props.set_property(
                CoreOptions::CHILD_AREA_WIDTH,
                Some(width - padding.left - padding.right),
            );
            props.set_property(
                CoreOptions::CHILD_AREA_HEIGHT,
                Some(height - padding.top - padding.bottom),
            );
        }

        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "After");
        }
    }
}
