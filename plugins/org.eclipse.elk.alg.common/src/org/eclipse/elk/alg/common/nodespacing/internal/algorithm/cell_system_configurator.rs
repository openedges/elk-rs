use crate::org::eclipse::elk::alg::common::nodespacing::cellsystem::ContainerArea;
use crate::org::eclipse::elk::alg::common::nodespacing::internal::node_context::NodeContext;
use crate::org::eclipse::elk::alg::common::nodespacing::internal::node_label_location::NodeLabelLocation;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    PortConstraints, PortSide, SizeConstraint, SizeOptions,
};

/// Configures constraints of the cell system such that the various cells contribute
/// properly to the node size calculation.
///
/// Faithfully ports Java's `CellSystemConfigurator`.
pub struct CellSystemConfigurator;

impl CellSystemConfigurator {
    /// Configures the cell system's constraints such that they work properly when
    /// calculating the required node space.
    pub fn configure_cell_system_size_contributions(node_context: &mut NodeContext) {
        // If the node has a fixed size, we don't need to change anything because the
        // cell system won't be used to calculate the node's size
        if node_context.size_constraints.is_empty() {
            return;
        }

        // Go through the different size constraint components
        if node_context
            .size_constraints
            .contains(&SizeConstraint::Ports)
        {
            // The northern and southern inside port label cells have the correct width
            // for the node
            if let Some(cell) = node_context.inside_port_label_cells.get_mut(&PortSide::North) {
                cell.set_contributes_to_minimum_width(true);
            }
            if let Some(cell) = node_context.inside_port_label_cells.get_mut(&PortSide::South) {
                cell.set_contributes_to_minimum_width(true);
            }

            // Also set on the node container's begin/end cells
            node_context
                .node_container
                .get_cell_mut(ContainerArea::Begin)
                .set_contributes_to_minimum_width(true);
            node_context
                .node_container
                .get_cell_mut(ContainerArea::End)
                .set_contributes_to_minimum_width(true);

            // For the eastern and western cells, they only give a correct height if port
            // placement is free instead of constrained
            let free_port_placement = node_context.port_constraints != PortConstraints::FixedRatio
                && node_context.port_constraints != PortConstraints::FixedPos;

            if let Some(cell) = node_context.inside_port_label_cells.get_mut(&PortSide::East) {
                cell.set_contributes_to_minimum_height(free_port_placement);
            }
            if let Some(cell) = node_context.inside_port_label_cells.get_mut(&PortSide::West) {
                cell.set_contributes_to_minimum_height(free_port_placement);
            }

            // Also set on the middle row's begin/end cells
            {
                let middle_row = node_context.node_container_middle_row_mut();
                middle_row
                    .get_cell_mut(ContainerArea::Begin)
                    .set_contributes_to_minimum_height(free_port_placement);
                middle_row
                    .get_cell_mut(ContainerArea::End)
                    .set_contributes_to_minimum_height(free_port_placement);
            }

            // The main row needs to contribute height for the east and west port label
            // cells to be able to contribute their height
            node_context
                .node_container_middle_row_mut()
                .set_contributes_to_minimum_height(free_port_placement);

            // Port labels only contribute their size if ports are accounted for as well
            if node_context
                .size_constraints
                .contains(&SizeConstraint::PortLabels)
            {
                // The port label cells contribute the space they need for inside port
                // label placement
                if let Some(cell) = node_context.inside_port_label_cells.get_mut(&PortSide::North) {
                    cell.set_contributes_to_minimum_height(true);
                }
                if let Some(cell) = node_context.inside_port_label_cells.get_mut(&PortSide::South) {
                    cell.set_contributes_to_minimum_height(true);
                }
                if let Some(cell) = node_context.inside_port_label_cells.get_mut(&PortSide::East) {
                    cell.set_contributes_to_minimum_width(true);
                }
                if let Some(cell) = node_context.inside_port_label_cells.get_mut(&PortSide::West) {
                    cell.set_contributes_to_minimum_width(true);
                }

                // Also set on node container cells
                node_context
                    .node_container
                    .get_cell_mut(ContainerArea::Begin)
                    .set_contributes_to_minimum_height(true);
                node_context
                    .node_container
                    .get_cell_mut(ContainerArea::End)
                    .set_contributes_to_minimum_height(true);

                {
                    let middle_row = node_context.node_container_middle_row_mut();
                    middle_row
                        .get_cell_mut(ContainerArea::Begin)
                        .set_contributes_to_minimum_width(true);
                    middle_row
                        .get_cell_mut(ContainerArea::End)
                        .set_contributes_to_minimum_width(true);
                }

                // The main row needs to contribute width for the east and west port
                // label cells to be able to contribute their width
                node_context
                    .node_container_middle_row_mut()
                    .set_contributes_to_minimum_width(true);
            }
        }

        if node_context
            .size_constraints
            .contains(&SizeConstraint::NodeLabels)
        {
            // The inside node label cell needs to contribute both width and height, as
            // does the middle row
            if let Some(container) = node_context.inside_node_label_container_mut() {
                container.set_contributes_to_minimum_height(true);
                container.set_contributes_to_minimum_width(true);
            }

            node_context
                .node_container_middle_row_mut()
                .set_contributes_to_minimum_height(true);
            node_context
                .node_container_middle_row_mut()
                .set_contributes_to_minimum_width(true);

            // All node label cells need to contribute height and width, but outside node
            // labels only do so unless they are configured to overhang
            let overhang = node_context
                .size_options
                .contains(&SizeOptions::OutsideNodeLabelsOverhang);

            // We need to iterate locations and set contribution flags on the cells
            // stored in the containers
            for location in NodeLabelLocation::all_defined() {
                if location.is_inside_location() {
                    let row = location.container_row().unwrap();
                    let col = location.container_column().unwrap();
                    if let Some(container) = node_context.inside_node_label_container_mut() {
                        let cell = container.get_cell_mut(row, col);
                        cell.set_contributes_to_minimum_height(true);
                        cell.set_contributes_to_minimum_width(true);
                    }
                } else {
                    let side = location.outside_side();
                    let area = match side {
                        PortSide::North | PortSide::South => location.container_column().unwrap(),
                        _ => location.container_row().unwrap(),
                    };
                    if let Some(container) =
                        node_context.outside_node_label_containers.get_mut(&side)
                    {
                        let cell = container.get_cell_mut(area);
                        if !cell.is_none() {
                            cell.set_contributes_to_minimum_height(!overhang);
                            cell.set_contributes_to_minimum_width(!overhang);
                        }
                    }
                }
            }
        }

        // If the middle cell contributes to the node size, we need to set that up as well
        if node_context
            .size_constraints
            .contains(&SizeConstraint::MinimumSize)
            && node_context
                .size_options
                .contains(&SizeOptions::MinimumSizeAccountsForPadding)
        {
            // The middle row now needs to contribute width and height
            node_context
                .node_container_middle_row_mut()
                .set_contributes_to_minimum_height(true);
            node_context
                .node_container_middle_row_mut()
                .set_contributes_to_minimum_width(true);

            // If the inside node label container is not already contributing to the
            // minimum height and width, node labels are not to be regarded.
            let already_contributing = node_context
                .inside_node_label_container()
                .map(|c| c.contributes_to_minimum_height())
                .unwrap_or(false);

            if !already_contributing {
                if let Some(container) = node_context.inside_node_label_container_mut() {
                    container.set_contributes_to_minimum_height(true);
                    container.set_contributes_to_minimum_width(true);
                    container.set_only_center_cell_contributes(true);
                }
            }
        }
    }

    /// The padding of east and west inside port label cells was originally set to the
    /// surrounding port margins. In the free case, the paddings must be such that the
    /// port placement will start below the northern and southern inside port label space,
    /// but still respects the surrounding port margins.
    pub fn update_vertical_inside_port_label_cell_padding(node_context: &mut NodeContext) {
        // We only care for the free port placement case
        if node_context.port_constraints == PortConstraints::FixedRatio
            || node_context.port_constraints == PortConstraints::FixedPos
        {
            return;
        }

        // Calculate where the east and west port cells will end up
        let north_cell_height = node_context
            .inside_port_label_cells
            .get(&PortSide::North)
            .map(|c| c.minimum_height())
            .unwrap_or(0.0);
        let south_cell_height = node_context
            .inside_port_label_cells
            .get(&PortSide::South)
            .map(|c| c.minimum_height())
            .unwrap_or(0.0);

        let top_border_offset = node_context.node_container.padding_ref().top
            + north_cell_height
            + node_context.label_cell_spacing;
        let bottom_border_offset = node_context.node_container.padding_ref().bottom
            + south_cell_height
            + node_context.label_cell_spacing;

        // Get current paddings for east and west cells
        let east_top = node_context
            .inside_port_label_cells
            .get(&PortSide::East)
            .map(|c| c.padding_ref().top)
            .unwrap_or(0.0);
        let east_bottom = node_context
            .inside_port_label_cells
            .get(&PortSide::East)
            .map(|c| c.padding_ref().bottom)
            .unwrap_or(0.0);
        let west_top = node_context
            .inside_port_label_cells
            .get(&PortSide::West)
            .map(|c| c.padding_ref().top)
            .unwrap_or(0.0);
        let west_bottom = node_context
            .inside_port_label_cells
            .get(&PortSide::West)
            .map(|c| c.padding_ref().bottom)
            .unwrap_or(0.0);

        // Calculate how much top/bottom padding we actually need
        let top_padding = 0.0_f64
            .max(east_top - top_border_offset)
            .max(west_top - top_border_offset);
        let bottom_padding = 0.0_f64
            .max(east_bottom - bottom_border_offset)
            .max(west_bottom - bottom_border_offset);

        // Update paddings
        if let Some(cell) = node_context.inside_port_label_cells.get_mut(&PortSide::East) {
            cell.padding().top = top_padding;
            cell.padding().bottom = bottom_padding;
        }
        if let Some(cell) = node_context.inside_port_label_cells.get_mut(&PortSide::West) {
            cell.padding().top = top_padding;
            cell.padding().bottom = bottom_padding;
        }

        // Also update in the middle row's cells
        let middle_row = node_context.node_container_middle_row_mut();
        if let Some(atomic) = middle_row
            .get_cell_mut(ContainerArea::End)
            .as_atomic_mut()
        {
            atomic.padding().top = top_padding;
            atomic.padding().bottom = bottom_padding;
        }
        if let Some(atomic) = middle_row
            .get_cell_mut(ContainerArea::Begin)
            .as_atomic_mut()
        {
            atomic.padding().top = top_padding;
            atomic.padding().bottom = bottom_padding;
        }
    }
}
