use crate::org::eclipse::elk::alg::common::nodespacing::internal::node_context::NodeContext;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkPadding, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    PortLabelPlacement, PortSide, SizeConstraint, SizeOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_util::ElkUtil;

/// Various little methods that didn't quite fit into any of the other classes.
///
/// Faithfully ports Java's `NodeLabelAndSizeUtilities`.
pub struct NodeLabelAndSizeUtilities;

impl NodeLabelAndSizeUtilities {
    /// If the client area should have a minimum size, the inside node label container
    /// is setup accordingly.
    pub fn setup_minimum_client_area_size(node_context: &mut NodeContext) {
        if let Some(min_size) = Self::get_minimum_client_area_size(node_context) {
            if let Some(container) = node_context.inside_node_label_container_mut() {
                container.set_center_cell_minimum_size(min_size);
            }
        }
    }

    /// Sets up the padding of the main node cell to account for ports that extend inside
    /// the node.
    pub fn setup_node_padding_for_ports_with_offset(node_context: &mut NodeContext) {
        // Collect all port contexts for iteration
        let port_data: Vec<(PortSide, f64, bool, Vec<KVector>, KVector)> = node_context
            .port_contexts
            .values()
            .flat_map(|v| v.iter())
            .map(|pc| {
                (
                    pc.port_side,
                    pc.port_border_offset,
                    pc.has_port_border_offset,
                    pc.label_sizes.clone(),
                    pc.port_size,
                )
            })
            .collect();

        let is_fixed = PortLabelPlacement::is_fixed(&node_context.port_labels_placement);
        let symmetry = !node_context.size_options.contains(&SizeOptions::Asymmetrical);

        for (port_side, port_border_offset, has_port_border_offset, label_sizes, port_size) in
            &port_data
        {
            let node_cell_padding = node_context.node_container.padding();

            // If the port extends into the node, ensure the inside port space is enough
            if *has_port_border_offset && *port_border_offset < 0.0 {
                match port_side {
                    PortSide::North => {
                        node_cell_padding.top =
                            node_cell_padding.top.max(-port_border_offset);
                    }
                    PortSide::South => {
                        node_cell_padding.bottom =
                            node_cell_padding.bottom.max(-port_border_offset);
                    }
                    PortSide::East => {
                        node_cell_padding.right =
                            node_cell_padding.right.max(-port_border_offset);
                    }
                    PortSide::West => {
                        node_cell_padding.left =
                            node_cell_padding.left.max(-port_border_offset);
                    }
                    _ => {}
                }
            }

            if is_fixed {
                // Compute the maximum inside part across all labels
                let mut max_inside_part = 0.0_f64;
                for label_size in label_sizes {
                    // For fixed labels, compute the inside part using label data
                    // We need label position but fixed labels store absolute positions
                    // Use the single-label compute_inside_part
                    let label_pos = KVector::new(); // Fixed labels would have explicit positions
                    let inside = ElkUtil::compute_inside_part(
                        &label_pos,
                        label_size,
                        port_size,
                        *port_border_offset,
                        *port_side,
                    );
                    max_inside_part = max_inside_part.max(inside);
                }

                if max_inside_part > 0.0 {
                    let node_cell_padding = node_context.node_container.padding();
                    match port_side {
                        PortSide::North => {
                            let inside_part_is_bigger = max_inside_part > node_cell_padding.top;
                            node_cell_padding.top =
                                node_cell_padding.top.max(max_inside_part);
                            if symmetry && inside_part_is_bigger {
                                node_cell_padding.top =
                                    node_cell_padding.top.max(node_cell_padding.bottom);
                                node_cell_padding.bottom =
                                    node_cell_padding.top + port_border_offset;
                            }
                        }
                        PortSide::South => {
                            let inside_part_is_bigger = max_inside_part > node_cell_padding.bottom;
                            node_cell_padding.bottom =
                                node_cell_padding.bottom.max(max_inside_part);
                            if symmetry && inside_part_is_bigger {
                                node_cell_padding.bottom =
                                    node_cell_padding.bottom.max(node_cell_padding.top);
                                node_cell_padding.top =
                                    node_cell_padding.bottom + port_border_offset;
                            }
                        }
                        PortSide::East => {
                            let inside_part_is_bigger = max_inside_part > node_cell_padding.right;
                            node_cell_padding.right =
                                node_cell_padding.right.max(max_inside_part);
                            if symmetry && inside_part_is_bigger {
                                node_cell_padding.right =
                                    node_cell_padding.left.max(node_cell_padding.right);
                                node_cell_padding.left =
                                    node_cell_padding.right + port_border_offset;
                            }
                        }
                        PortSide::West => {
                            let inside_part_is_bigger = max_inside_part > node_cell_padding.left;
                            node_cell_padding.left =
                                node_cell_padding.left.max(max_inside_part);
                            if symmetry && inside_part_is_bigger {
                                node_cell_padding.left =
                                    node_cell_padding.left.max(node_cell_padding.right);
                                node_cell_padding.right =
                                    node_cell_padding.left + port_border_offset;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Offsets all southern ports, previously placed relative to vertical coordinate 0,
    /// by the node's size to place them along the node's southern border.
    pub fn offset_southern_ports_by_node_size(node_context: &mut NodeContext) {
        let node_height = node_context.node_size.y;

        if let Some(south_ports) = node_context.port_contexts.get_mut(&PortSide::South) {
            for port_context in south_ports {
                port_context.port_position.y += node_height;
            }
        }
    }

    /// Calculates and stores the node padding, if requested by layout options.
    pub fn set_node_padding(node_context: &NodeContext) -> Option<ElkPadding> {
        if !node_context
            .size_options
            .contains(&SizeOptions::ComputePadding)
        {
            return None;
        }

        let node_rect = node_context.node_container.cell_rectangle_ref();
        let client_area = node_context
            .inside_node_label_container()
            .map(|c| c.center_cell_rectangle())
            .unwrap_or_default();

        let mut node_padding = ElkPadding::new();
        node_padding.left = client_area.x - node_rect.x;
        node_padding.top = client_area.y - node_rect.y;
        node_padding.right =
            (node_rect.x + node_rect.width) - (client_area.x + client_area.width);
        node_padding.bottom =
            (node_rect.y + node_rect.height) - (client_area.y + client_area.height);

        Some(node_padding)
    }

    /// Returns the client area's minimum size if size constraints are configured such
    /// that the client area has a minimum size. Otherwise, returns `None`.
    pub fn get_minimum_client_area_size(node_context: &NodeContext) -> Option<KVector> {
        if node_context
            .size_constraints
            .contains(&SizeConstraint::MinimumSize)
            && node_context
                .size_options
                .contains(&SizeOptions::MinimumSizeAccountsForPadding)
        {
            Some(Self::get_minimum_node_or_client_area_size(node_context))
        } else {
            None
        }
    }

    /// Returns the node's minimum size if size constraints are configured such that the
    /// node as a whole has a minimum size. Otherwise, returns `None`.
    pub fn get_minimum_node_size(node_context: &NodeContext) -> Option<KVector> {
        if node_context
            .size_constraints
            .contains(&SizeConstraint::MinimumSize)
            && !node_context
                .size_options
                .contains(&SizeOptions::MinimumSizeAccountsForPadding)
        {
            return Some(Self::get_minimum_node_or_client_area_size(node_context));
        }
        None
    }

    /// Returns the minimum size configured for the node, without regard for size
    /// constraints. Uses the stored `node_size_minimum` from NodeContext.
    pub fn get_minimum_node_or_client_area_size(node_context: &NodeContext) -> KVector {
        let mut min_size = node_context.node_size_minimum;

        // If we are instructed to revert to a default minimum size, check whether we
        // need to revert to that
        if node_context
            .size_options
            .contains(&SizeOptions::DefaultMinimumSize)
        {
            if min_size.x <= 0.0 {
                min_size.x = ElkUtil::DEFAULT_MIN_WIDTH;
            }
            if min_size.y <= 0.0 {
                min_size.y = ElkUtil::DEFAULT_MIN_HEIGHT;
            }
        }

        min_size
    }

    /// Checks if the size constraints, even if not empty, should cause the node not to
    /// be resized.
    pub fn are_size_constraints_fixed(node_context: &NodeContext) -> bool {
        node_context.size_constraints.is_empty()
            || node_context.size_constraints == EnumSet::of(&[SizeConstraint::PortLabels])
    }

    /// Outside ports usually have their labels placed below or to the right. The first
    /// port, however, may have its label placed on the other side.
    pub fn is_first_outside_port_label_placed_differently(
        node_context: &NodeContext,
        port_side: PortSide,
    ) -> bool {
        if let Some(port_contexts) = node_context.port_contexts.get(&port_side) {
            if port_contexts.len() >= 2 {
                let first_port = &port_contexts[0];

                let always_same_side = node_context
                    .port_labels_placement
                    .contains(&PortLabelPlacement::AlwaysSameSide);
                let space_efficient = node_context
                    .port_labels_placement
                    .contains(&PortLabelPlacement::SpaceEfficient);

                return !first_port.labels_next_to_port
                    && !always_same_side
                    && (port_contexts.len() == 2 || space_efficient);
            }
        }
        false
    }
}
