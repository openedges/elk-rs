use crate::org::eclipse::elk::alg::common::nodespacing::cellsystem::CellChild;
use crate::org::eclipse::elk::alg::common::nodespacing::internal::node_context::NodeContext;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{PortSide, SizeOptions};

/// Knows how to properly size and position outer node label containers and to place
/// node and port labels.
///
/// Faithfully ports Java's `LabelPlacer`.
pub struct LabelPlacer;

impl LabelPlacer {
    /// Places outer node label containers as well as all labels.
    pub fn place_labels(node_context: &mut NodeContext) {
        // Properly place all label cells for outer node labels
        Self::place_outer_node_label_containers(node_context);

        // Tell all node label cells to place their labels
        // Since labels are stored in the cell tree, we need to iterate through the containers
        // and apply layout to each label cell

        // Inside node labels
        if let Some(container) = node_context.inside_node_label_container_mut() {
            use crate::org::eclipse::elk::alg::common::nodespacing::cellsystem::ContainerArea;
            for row in ContainerArea::values() {
                for col in ContainerArea::values() {
                    if let CellChild::Label(label_cell) = container.get_cell_mut(*row, *col) {
                        label_cell.apply_label_layout();
                    }
                }
            }
        }

        // Outside node labels
        for side in &[PortSide::North, PortSide::South, PortSide::East, PortSide::West] {
            if let Some(container) = node_context.outside_node_label_containers.get_mut(side) {
                use crate::org::eclipse::elk::alg::common::nodespacing::cellsystem::ContainerArea;
                for area in ContainerArea::values() {
                    if let CellChild::Label(label_cell) = container.get_cell_mut(*area) {
                        label_cell.apply_label_layout();
                    }
                }
            }
        }

        // Port labels - apply layout to each port's label cell
        for port_contexts in node_context.port_contexts.values_mut() {
            for port_context in port_contexts {
                if let Some(ref mut label_cell) = port_context.port_label_cell {
                    label_cell.apply_label_layout();
                }
            }
        }
    }

    fn place_outer_node_label_containers(node_context: &mut NodeContext) {
        let outer_node_labels_overhang = node_context
            .size_options
            .contains(&SizeOptions::OutsideNodeLabelsOverhang);

        Self::place_horizontal_outer_node_label_container(
            node_context,
            outer_node_labels_overhang,
            PortSide::North,
        );
        Self::place_horizontal_outer_node_label_container(
            node_context,
            outer_node_labels_overhang,
            PortSide::South,
        );
        Self::place_vertical_outer_node_label_container(
            node_context,
            outer_node_labels_overhang,
            PortSide::East,
        );
        Self::place_vertical_outer_node_label_container(
            node_context,
            outer_node_labels_overhang,
            PortSide::West,
        );
    }

    fn place_horizontal_outer_node_label_container(
        node_context: &mut NodeContext,
        outer_node_labels_overhang: bool,
        port_side: PortSide,
    ) {
        let node_size_x = node_context.node_size.x;
        let node_size_y = node_context.node_size.y;

        if let Some(node_label_container) =
            node_context.outside_node_label_containers.get_mut(&port_side)
        {
            let min_width = node_label_container.minimum_width();
            let min_height = node_label_container.minimum_height();

            let rect = node_label_container.cell_rectangle();

            // Set the container's width and height to its minimum width and height
            rect.width = min_width;
            rect.height = min_height;

            // The container must be at least as wide as the node is
            rect.width = rect.width.max(node_size_x);

            // If node labels are not allowed to overhang and if they would do so right
            // now, make the container smaller
            if rect.width > node_size_x && !outer_node_labels_overhang {
                rect.width = node_size_x;
            }

            // Container's x coordinate
            rect.x = -(rect.width - node_size_x) / 2.0;

            // Container's y coordinate depends on whether we place the thing on the
            // northern or southern side
            match port_side {
                PortSide::North => {
                    rect.y = -rect.height;
                }
                PortSide::South => {
                    rect.y = node_size_y;
                }
                _ => {}
            }

            // Layout the container's children
            node_label_container.layout_children_horizontally();
            node_label_container.layout_children_vertically();
        }
    }

    fn place_vertical_outer_node_label_container(
        node_context: &mut NodeContext,
        outer_node_labels_overhang: bool,
        port_side: PortSide,
    ) {
        let node_size_x = node_context.node_size.x;
        let node_size_y = node_context.node_size.y;

        if let Some(node_label_container) =
            node_context.outside_node_label_containers.get_mut(&port_side)
        {
            let min_width = node_label_container.minimum_width();
            let min_height = node_label_container.minimum_height();

            let rect = node_label_container.cell_rectangle();

            // Set the container's width and height to its minimum width and height
            rect.width = min_width;
            rect.height = min_height;

            // The container must be at least as high as the node is
            rect.height = rect.height.max(node_size_y);

            // If node labels are not allowed to overhang and if they would do so right
            // now, make the container smaller
            if rect.height > node_size_y && !outer_node_labels_overhang {
                rect.height = node_size_y;
            }

            // Container's y coordinate
            rect.y = -(rect.height - node_size_y) / 2.0;

            // Container's x coordinate depends on whether we place the thing on the
            // eastern or western side
            match port_side {
                PortSide::West => {
                    rect.x = -rect.width;
                }
                PortSide::East => {
                    rect.x = node_size_x;
                }
                _ => {}
            }

            // Layout the container's children
            node_label_container.layout_children_horizontally();
            node_label_container.layout_children_vertically();
        }
    }
}
