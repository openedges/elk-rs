use crate::org::eclipse::elk::alg::common::nodespacing::cellsystem::{
    CellChild, ContainerArea, DynLabel, DynLabelCell, GridContainerCell, StripContainerCell,
    Strip,
};
use crate::org::eclipse::elk::alg::common::nodespacing::internal::node_context::NodeContext;
use crate::org::eclipse::elk::alg::common::nodespacing::internal::node_label_location::NodeLabelLocation;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, NodeLabelPlacement, PortSide, SizeOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{
    LabelAdapter, NodeAdapter,
};

/// Knows how to take all of a node's labels and create the appropriate grid cells.
///
/// Faithfully ports Java's `NodeLabelCellCreator`.
pub struct NodeLabelCellCreator;

impl NodeLabelCellCreator {
    /// Iterates over all of the node's labels and creates all required cell containers
    /// and label cells.
    pub fn create_node_label_cells<N, T>(
        node_context: &mut NodeContext,
        node: &N,
        only_inside: bool,
        horizontal_layout_mode: bool,
    ) where
        T: 'static,
        N: NodeAdapter<T>,
        N::Label: 'static,
        N::LabelAdapter: 'static,
    {
        // Make sure all the relevant containers exist
        Self::create_node_label_cell_containers(node_context, only_inside);

        // Handle each of the node's labels (take ownership for DynLabel wrapping)
        for label in node.get_labels() {
            Self::handle_node_label(node_context, label, only_inside, horizontal_layout_mode);
        }
    }

    fn handle_node_label<T, L>(
        node_context: &mut NodeContext,
        label: L,
        only_inside: bool,
        horizontal_layout_mode: bool,
    ) where
        T: 'static,
        L: LabelAdapter<T> + 'static,
    {
        // Find the effective label location
        let label_placement: EnumSet<NodeLabelPlacement> =
            if label.has_property(CoreOptions::NODE_LABELS_PLACEMENT) {
                label
                    .get_property(CoreOptions::NODE_LABELS_PLACEMENT)
                    .unwrap_or_default()
            } else {
                node_context.node_label_placement.clone()
            };
        let label_location = NodeLabelLocation::from_node_label_placement(&label_placement);

        // If the label has its location fixed, we will ignore it
        if label_location == NodeLabelLocation::Undefined {
            return;
        }

        // If the label's location is on the node's outside but we only want inside node labels,
        // we will ignore it
        if only_inside && !label_location.is_inside_location() {
            return;
        }

        Self::retrieve_node_label_cell(node_context, label_location, horizontal_layout_mode)
            .add_label(DynLabel::new(label));
    }

    fn create_node_label_cell_containers(node_context: &mut NodeContext, only_inside: bool) {
        let symmetry = !node_context.size_options.contains(&SizeOptions::Asymmetrical);
        let tabular_node_labels = node_context
            .size_options
            .contains(&SizeOptions::ForceTabularNodeLabels);

        // Inside container
        let mut inside_container =
            GridContainerCell::new(tabular_node_labels, symmetry, node_context.label_cell_spacing);

        // Apply node labels padding
        let nlp = &node_context.node_labels_padding;
        let padding = inside_container.padding();
        padding.top = nlp.top;
        padding.right = nlp.right;
        padding.bottom = nlp.bottom;
        padding.left = nlp.left;

        node_context.node_container_middle_row_mut().set_cell(
            ContainerArea::Center,
            CellChild::Grid(Box::new(inside_container)),
        );

        // Outside containers, if requested
        if !only_inside {
            let mut north_container =
                StripContainerCell::new(Strip::Horizontal, symmetry, node_context.label_cell_spacing);
            north_container.padding().bottom = node_context.node_label_spacing;
            node_context
                .outside_node_label_containers
                .insert(PortSide::North, north_container);

            let mut south_container =
                StripContainerCell::new(Strip::Horizontal, symmetry, node_context.label_cell_spacing);
            south_container.padding().top = node_context.node_label_spacing;
            node_context
                .outside_node_label_containers
                .insert(PortSide::South, south_container);

            let mut west_container =
                StripContainerCell::new(Strip::Vertical, symmetry, node_context.label_cell_spacing);
            west_container.padding().right = node_context.node_label_spacing;
            node_context
                .outside_node_label_containers
                .insert(PortSide::West, west_container);

            let mut east_container =
                StripContainerCell::new(Strip::Vertical, symmetry, node_context.label_cell_spacing);
            east_container.padding().left = node_context.node_label_spacing;
            node_context
                .outside_node_label_containers
                .insert(PortSide::East, east_container);
        }
    }

    fn retrieve_node_label_cell(
        node_context: &mut NodeContext,
        node_label_location: NodeLabelLocation,
        horizontal_layout_mode: bool,
    ) -> &mut DynLabelCell {
        if !node_context.node_label_cells.contains_key(&node_label_location) {
            // The node label cell doesn't exist yet, so create one and add it to the relevant
            // container
            let mut node_label_cell =
                DynLabelCell::new_with_layout_mode(node_context.label_label_spacing, horizontal_layout_mode);

            // Set alignment based on location
            if let Some(h_align) = node_label_location.horizontal_alignment() {
                node_label_cell.set_horizontal_alignment(h_align);
            }
            if let Some(v_align) = node_label_location.vertical_alignment() {
                node_label_cell.set_vertical_alignment(v_align);
            }

            // Find the correct container and add the cell to it
            if node_label_location.is_inside_location() {
                let row = node_label_location.container_row().unwrap();
                let col = node_label_location.container_column().unwrap();
                node_context
                    .inside_node_label_container_mut()
                    .expect("inside node label container should exist")
                    .set_cell(row, col, CellChild::Label(node_label_cell));

                // Return a reference to the label cell we just inserted
                let grid = node_context.inside_node_label_container_mut().unwrap();
                match grid.get_cell_mut(row, col) {
                    CellChild::Label(lc) => return lc,
                    _ => unreachable!(),
                }
            } else {
                let outside_side = node_label_location.outside_side();
                let container_cell = node_context
                    .outside_node_label_containers
                    .get_mut(&outside_side)
                    .expect("outside container should exist");

                match outside_side {
                    PortSide::North | PortSide::South => {
                        node_label_cell.set_contributes_to_minimum_height(true);
                        let area = node_label_location.container_column().unwrap();
                        container_cell.set_cell(area, CellChild::Label(node_label_cell));

                        match container_cell.get_cell_mut(area) {
                            CellChild::Label(lc) => return lc,
                            _ => unreachable!(),
                        }
                    }
                    PortSide::West | PortSide::East => {
                        node_label_cell.set_contributes_to_minimum_width(true);
                        let area = node_label_location.container_row().unwrap();
                        container_cell.set_cell(area, CellChild::Label(node_label_cell));

                        match container_cell.get_cell_mut(area) {
                            CellChild::Label(lc) => return lc,
                            _ => unreachable!(),
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }

        // Cell already exists - find it and return a reference
        // We need to look it up from the container where it was placed
        if node_label_location.is_inside_location() {
            let row = node_label_location.container_row().unwrap();
            let col = node_label_location.container_column().unwrap();
            let grid = node_context.inside_node_label_container_mut().unwrap();
            match grid.get_cell_mut(row, col) {
                CellChild::Label(lc) => lc,
                _ => panic!("expected label cell"),
            }
        } else {
            let outside_side = node_label_location.outside_side();
            let container_cell = node_context
                .outside_node_label_containers
                .get_mut(&outside_side)
                .unwrap();
            let area = match outside_side {
                PortSide::North | PortSide::South => node_label_location.container_column().unwrap(),
                _ => node_label_location.container_row().unwrap(),
            };
            match container_cell.get_cell_mut(area) {
                CellChild::Label(lc) => lc,
                _ => panic!("expected label cell"),
            }
        }
    }
}
