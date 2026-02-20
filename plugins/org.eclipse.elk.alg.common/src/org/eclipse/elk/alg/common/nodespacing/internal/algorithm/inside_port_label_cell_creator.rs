use crate::org::eclipse::elk::alg::common::nodespacing::cellsystem::{
    AtomicCell, CellChild, ContainerArea, StripContainerCell,
};
use crate::org::eclipse::elk::alg::common::nodespacing::internal::node_context::NodeContext;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{PortLabelPlacement, PortSide};

/// Sets up the inside port label cells. These are set up even when there are no inside
/// port labels since they also determine how much space we need to place ports along
/// the node borders.
///
/// Faithfully ports Java's `InsidePortLabelCellCreator`.
pub struct InsidePortLabelCellCreator;

impl InsidePortLabelCellCreator {
    /// Creates all the inside port label cells. Setting them up is left to the port and
    /// port label placement code.
    pub fn create_inside_port_label_cells(node_context: &mut NodeContext) {
        // Create all inside port label cells.
        // NORTH → nodeContainer BEGIN area, SOUTH → nodeContainer END area
        Self::create_inside_port_label_cell(
            &mut node_context.node_container,
            ContainerArea::Begin,
            PortSide::North,
            &mut node_context.inside_port_label_cells,
        );
        Self::create_inside_port_label_cell(
            &mut node_context.node_container,
            ContainerArea::End,
            PortSide::South,
            &mut node_context.inside_port_label_cells,
        );

        // WEST → middleRow BEGIN area, EAST → middleRow END area
        {
            let middle_row = node_context
                .node_container
                .get_cell_mut(ContainerArea::Center)
                .as_strip_mut()
                .expect("node_container center cell should be a StripContainerCell");

            Self::create_inside_port_label_cell(
                middle_row,
                ContainerArea::Begin,
                PortSide::West,
                &mut node_context.inside_port_label_cells,
            );
            Self::create_inside_port_label_cell(
                middle_row,
                ContainerArea::End,
                PortSide::East,
                &mut node_context.inside_port_label_cells,
            );
        }

        Self::setup_north_or_south_port_label_cell(node_context, PortSide::North);
        Self::setup_north_or_south_port_label_cell(node_context, PortSide::South);
        Self::setup_east_or_west_port_label_cell(node_context, PortSide::East);
        Self::setup_east_or_west_port_label_cell(node_context, PortSide::West);
    }

    fn create_inside_port_label_cell(
        container: &mut StripContainerCell,
        container_area: ContainerArea,
        port_side: PortSide,
        inside_port_label_cells: &mut std::collections::HashMap<PortSide, AtomicCell>,
    ) {
        let port_label_cell = AtomicCell::new();
        // Store a copy in the HashMap for direct access. Since Rust doesn't support shared
        // mutable references, the HashMap and container hold separate copies. After layout,
        // we need to sync the cell rectangles from the container to the HashMap via
        // NodeContext::sync_inside_port_label_cell_rectangles().
        inside_port_label_cells.insert(port_side, port_label_cell.clone());
        container.set_cell(container_area, CellChild::Atomic(port_label_cell));
    }

    fn setup_north_or_south_port_label_cell(node_context: &mut NodeContext, port_side: PortSide) {
        // Get the container area for this port side
        let container_area = match port_side {
            PortSide::North => ContainerArea::Begin,
            PortSide::South => ContainerArea::End,
            _ => return,
        };

        // Get the padding of the port label cell in the node container
        let cell = node_context
            .node_container
            .get_cell_mut(container_area)
            .as_atomic_mut();

        if let Some(atomic_cell) = cell {
            match port_side {
                PortSide::North => {
                    if node_context.port_label_spacing_vertical >= 0.0 {
                        atomic_cell.padding().top = node_context.port_label_spacing_vertical;
                    }
                }
                PortSide::South => {
                    if node_context.port_label_spacing_vertical >= 0.0 {
                        atomic_cell.padding().bottom = node_context.port_label_spacing_vertical;
                    }
                }
                _ => {}
            }

            atomic_cell.padding().left = node_context.surrounding_port_margins.left;
            atomic_cell.padding().right = node_context.surrounding_port_margins.right;
        }

        // Also update in inside_port_label_cells HashMap
        if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
            match port_side {
                PortSide::North => {
                    if node_context.port_label_spacing_vertical >= 0.0 {
                        cell.padding().top = node_context.port_label_spacing_vertical;
                    }
                }
                PortSide::South => {
                    if node_context.port_label_spacing_vertical >= 0.0 {
                        cell.padding().bottom = node_context.port_label_spacing_vertical;
                    }
                }
                _ => {}
            }
            cell.padding().left = node_context.surrounding_port_margins.left;
            cell.padding().right = node_context.surrounding_port_margins.right;
        }
    }

    fn setup_east_or_west_port_label_cell(node_context: &mut NodeContext, port_side: PortSide) {
        if node_context
            .port_labels_placement
            .contains(&PortLabelPlacement::Inside)
        {
            Self::calculate_width_due_to_labels(node_context, port_side);
        }
        Self::setup_top_and_bottom_padding(node_context, port_side);
    }

    fn calculate_width_due_to_labels(node_context: &mut NodeContext, port_side: PortSide) {
        // Find the maximum label width for ports on this side
        let mut max_label_width: f64 = 0.0;

        if let Some(port_contexts) = node_context.port_contexts.get(&port_side) {
            for port_context in port_contexts {
                if let Some(ref label_cell) = port_context.port_label_cell {
                    max_label_width = max_label_width.max(label_cell.minimum_width());
                }
            }
        }

        // Update the inside port label cell's minimum content area size
        let container_area = match port_side {
            PortSide::West => ContainerArea::Begin,
            PortSide::East => ContainerArea::End,
            _ => return,
        };

        // Update in the middle row's cell
        let middle_row = node_context
            .node_container
            .get_cell_mut(ContainerArea::Center)
            .as_strip_mut()
            .expect("middle row should be a strip");

        if let Some(atomic) = middle_row.get_cell_mut(container_area).as_atomic_mut() {
            atomic.minimum_content_area_size_mut().x =
                atomic.minimum_content_area_size().x.max(max_label_width);

            if atomic.minimum_content_area_size().x > 0.0 {
                match port_side {
                    PortSide::East => {
                        atomic.padding().right = node_context.port_label_spacing_horizontal;
                    }
                    PortSide::West => {
                        atomic.padding().left = node_context.port_label_spacing_horizontal;
                    }
                    _ => {}
                }
            }
        }

        // Also update in inside_port_label_cells
        if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
            cell.minimum_content_area_size_mut().x =
                cell.minimum_content_area_size().x.max(max_label_width);

            if cell.minimum_content_area_size().x > 0.0 {
                match port_side {
                    PortSide::East => {
                        cell.padding().right = node_context.port_label_spacing_horizontal;
                    }
                    PortSide::West => {
                        cell.padding().left = node_context.port_label_spacing_horizontal;
                    }
                    _ => {}
                }
            }
        }
    }

    fn setup_top_and_bottom_padding(node_context: &mut NodeContext, port_side: PortSide) {
        let container_area = match port_side {
            PortSide::West => ContainerArea::Begin,
            PortSide::East => ContainerArea::End,
            _ => return,
        };

        let top = node_context.surrounding_port_margins.top;
        let bottom = node_context.surrounding_port_margins.bottom;

        // Update in the middle row's cell
        let middle_row = node_context
            .node_container
            .get_cell_mut(ContainerArea::Center)
            .as_strip_mut()
            .expect("middle row should be a strip");

        if let Some(atomic) = middle_row.get_cell_mut(container_area).as_atomic_mut() {
            atomic.padding().top = top;
            atomic.padding().bottom = bottom;
        }

        // Also update in inside_port_label_cells
        if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
            cell.padding().top = top;
            cell.padding().bottom = bottom;
        }
    }
}
