use std::collections::BTreeMap;
use rustc_hash::FxHashMap;

use crate::org::eclipse::elk::alg::common::nodespacing::cellsystem::{
    AtomicCell, CellChild, ContainerArea, DynLabelCell, GridContainerCell, StripContainerCell,
    Strip,
};
use super::node_label_location::NodeLabelLocation;
use super::port_context::PortContext;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMargin, ElkPadding, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, NodeLabelPlacement, PortAlignment, PortConstraints, PortLabelPlacement,
    PortSide, SizeConstraint, SizeOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, IndividualSpacings};
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{
    GraphElementAdapter, NodeAdapter,
};

/// Data holder class to be passed around to avoid having too much state in the size
/// calculation classes. Some of the most relevant settings are copied into variables for
/// convenience. The node's size is stored in a separate variable in this context, to be
/// used by the algorithm. Once the algorithm has finished it can apply the calculated
/// size to the node by calling `apply_node_size()`.
///
/// Faithfully ports Java's `NodeContext` from
/// `org.eclipse.elk.alg.common.nodespacing.internal`.
pub struct NodeContext {
    // === Convenience Access to Things ===

    /// The node's size. This will be used during the algorithm, to be applied (or not) once
    /// it is finished.
    pub node_size: KVector,
    /// Whether this node has stuff inside it or not.
    pub treat_as_compound_node: bool,
    /// The node's size constraints.
    pub size_constraints: EnumSet<SizeConstraint>,
    /// The node's size options.
    pub size_options: EnumSet<SizeOptions>,
    /// Port constraints set on the node.
    pub port_constraints: PortConstraints,
    /// Whether port labels are placed inside or outside.
    pub port_labels_placement: EnumSet<PortLabelPlacement>,
    /// Whether to treat port labels as a group when centering them next to eastern or
    /// western ports.
    pub port_labels_treat_as_group: bool,
    /// Where node labels are placed by default.
    pub node_label_placement: EnumSet<NodeLabelPlacement>,
    /// Space to leave around the node label area.
    pub node_labels_padding: ElkPadding,
    /// Space between a node and its outside labels.
    pub node_label_spacing: f64,
    /// Space between two labels.
    pub label_label_spacing: f64,
    /// Space between two different label cells.
    pub label_cell_spacing: f64,
    /// Space between a port and another port.
    pub port_port_spacing: f64,
    /// Horizontal space between a port and its labels.
    pub port_label_spacing_horizontal: f64,
    /// Vertical space between a port and its labels.
    pub port_label_spacing_vertical: f64,
    /// Margin to leave around the set of ports on each side.
    pub surrounding_port_margins: ElkMargin,
    /// Whether node is being laid out in top-down layout mode.
    pub topdown_layout: bool,
    /// Whether node size is fixed by the graph size property.
    pub node_size_fixed_graph_size: bool,
    /// The node's minimum size (extracted from NODE_SIZE_MINIMUM property).
    pub node_size_minimum: KVector,

    // === Port Alignment per side ===
    pub port_alignment_north: Option<PortAlignment>,
    pub port_alignment_south: Option<PortAlignment>,
    pub port_alignment_east: Option<PortAlignment>,
    pub port_alignment_west: Option<PortAlignment>,
    pub port_alignment_default: PortAlignment,

    // === Port Contexts ===
    /// Context objects that hold more information about each port. Sorted by PortSide ordinal.
    /// Within each side: NORTH/EAST ascending by volatile_id, SOUTH/WEST descending.
    /// (Matching Java's `TreeMultimap.create(comparePortSides, comparePortContexts)`.)
    pub port_contexts: BTreeMap<PortSide, Vec<PortContext>>,

    // === Cell System ===
    /// The main cell that holds all the cells that make up the node.
    pub node_container: StripContainerCell,

    // === Inside Port Label Cells ===
    /// All cells that describe the space required for ports and for inside port labels.
    /// Indexed by PortSide.
    pub inside_port_label_cells: FxHashMap<PortSide, AtomicCell>,

    // === Outside Node Label Containers ===
    /// All container cells that hold label cells for outside node labels.
    /// Indexed by PortSide.
    pub outside_node_label_containers: FxHashMap<PortSide, StripContainerCell>,

    // === Label Cells ===
    /// All of the label cells created for possible node labels, both inside and outside.
    /// Indexed by NodeLabelLocation.
    pub node_label_cells: FxHashMap<NodeLabelLocation, DynLabelCell>,
}

impl NodeContext {
    /// Creates a new context object for the given node, fully initialized with the node's
    /// settings. All properties are extracted from the adapter at construction time.
    pub fn new<N, T>(node: &N) -> Self
    where
        T: 'static,
        N: NodeAdapter<T>,
        N::Graph: GraphElementAdapter<T>,
    {
        let node_size = KVector::with_values(node.get_size().x, node.get_size().y);

        // Top-down layout
        let topdown_layout = node
            .get_property(CoreOptions::TOPDOWN_LAYOUT)
            .unwrap_or(false);

        // Compound node
        let treat_as_compound_node = node.is_compound_node()
            || node
                .get_property(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE)
                .unwrap_or(false);

        // Core size settings
        let size_constraints = node
            .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
            .unwrap_or_default();
        let size_options = node
            .get_property(CoreOptions::NODE_SIZE_OPTIONS)
            .unwrap_or_default();
        let port_constraints = node
            .get_property(CoreOptions::PORT_CONSTRAINTS)
            .unwrap_or(PortConstraints::Undefined);
        let port_labels_placement = node
            .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
            .unwrap_or_default();
        let port_labels_treat_as_group = node
            .get_property(CoreOptions::PORT_LABELS_TREAT_AS_GROUP)
            .unwrap_or(true);
        let node_label_placement = node
            .get_property(CoreOptions::NODE_LABELS_PLACEMENT)
            .unwrap_or_default();

        // Copy spacings for convenience using IndividualSpacings
        let node_labels_padding =
            IndividualSpacings::get_individual_or_inherited_adapter(
                node,
                CoreOptions::NODE_LABELS_PADDING,
            )
            .unwrap_or_default();
        let node_label_spacing =
            IndividualSpacings::get_individual_or_inherited_adapter(
                node,
                CoreOptions::SPACING_LABEL_NODE,
            )
            .unwrap_or(0.0);
        let label_label_spacing =
            IndividualSpacings::get_individual_or_inherited_adapter(
                node,
                CoreOptions::SPACING_LABEL_LABEL,
            )
            .unwrap_or(0.0);
        let port_port_spacing =
            IndividualSpacings::get_individual_or_inherited_adapter(
                node,
                CoreOptions::SPACING_PORT_PORT,
            )
            .unwrap_or(0.0);
        let port_label_spacing_horizontal =
            IndividualSpacings::get_individual_or_inherited_adapter(
                node,
                CoreOptions::SPACING_LABEL_PORT_HORIZONTAL,
            )
            .unwrap_or(0.0);
        let port_label_spacing_vertical =
            IndividualSpacings::get_individual_or_inherited_adapter(
                node,
                CoreOptions::SPACING_LABEL_PORT_VERTICAL,
            )
            .unwrap_or(0.0);
        let surrounding_port_margins =
            IndividualSpacings::get_individual_or_inherited_adapter(
                node,
                CoreOptions::SPACING_PORTS_SURROUNDING,
            )
            .unwrap_or_default();

        let label_cell_spacing = 2.0 * label_label_spacing;

        // Port alignment per side
        let port_alignment_default = node
            .get_property(CoreOptions::PORT_ALIGNMENT_DEFAULT)
            .unwrap_or(PortAlignment::Justified);
        let port_alignment_north = if node.has_property(CoreOptions::PORT_ALIGNMENT_NORTH) {
            node.get_property(CoreOptions::PORT_ALIGNMENT_NORTH)
        } else {
            None
        };
        let port_alignment_south = if node.has_property(CoreOptions::PORT_ALIGNMENT_SOUTH) {
            node.get_property(CoreOptions::PORT_ALIGNMENT_SOUTH)
        } else {
            None
        };
        let port_alignment_east = if node.has_property(CoreOptions::PORT_ALIGNMENT_EAST) {
            node.get_property(CoreOptions::PORT_ALIGNMENT_EAST)
        } else {
            None
        };
        let port_alignment_west = if node.has_property(CoreOptions::PORT_ALIGNMENT_WEST) {
            node.get_property(CoreOptions::PORT_ALIGNMENT_WEST)
        } else {
            None
        };

        // Node minimum size
        let node_size_minimum = node
            .get_property(CoreOptions::NODE_SIZE_MINIMUM)
            .unwrap_or_else(KVector::new);

        // NODE_SIZE_FIXED_GRAPH_SIZE comes from the parent graph
        let node_size_fixed_graph_size = if let Some(graph) = node.get_graph() {
            graph
                .get_property(CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE)
                .unwrap_or(false)
        } else {
            false
        };

        // Create main cells (the others will be created later by the algorithm phases)
        let symmetry = !size_options.contains(&SizeOptions::Asymmetrical);
        let mut node_container = StripContainerCell::new(Strip::Vertical, symmetry, 0.0);

        let middle_row = StripContainerCell::new(Strip::Horizontal, symmetry, 0.0);
        node_container.set_cell(ContainerArea::Center, CellChild::Strip(Box::new(middle_row)));

        NodeContext {
            node_size,
            treat_as_compound_node,
            size_constraints,
            size_options,
            port_constraints,
            port_labels_placement,
            port_labels_treat_as_group,
            node_label_placement,
            node_labels_padding,
            node_label_spacing,
            label_label_spacing,
            label_cell_spacing,
            port_port_spacing,
            port_label_spacing_horizontal,
            port_label_spacing_vertical,
            surrounding_port_margins,
            topdown_layout,
            node_size_fixed_graph_size,
            node_size_minimum,
            port_alignment_north,
            port_alignment_south,
            port_alignment_east,
            port_alignment_west,
            port_alignment_default,
            port_contexts: BTreeMap::new(),
            node_container,
            inside_port_label_cells: FxHashMap::default(),
            outside_node_label_containers: FxHashMap::default(),
            node_label_cells: FxHashMap::default(),
        }
    }

    /// Returns a reference to the middle row strip container (the center cell of the
    /// node container).
    pub fn node_container_middle_row(&self) -> &StripContainerCell {
        self.node_container
            .get_cell(ContainerArea::Center)
            .as_strip()
            .expect("node_container center cell should be a StripContainerCell")
    }

    /// Returns a mutable reference to the middle row strip container.
    pub fn node_container_middle_row_mut(&mut self) -> &mut StripContainerCell {
        self.node_container
            .get_cell_mut(ContainerArea::Center)
            .as_strip_mut()
            .expect("node_container center cell should be a StripContainerCell")
    }

    /// Returns a reference to the inside node label container (grid).
    /// Only available after `NodeLabelCellCreator` has run.
    pub fn inside_node_label_container(&self) -> Option<&GridContainerCell> {
        self.node_container_middle_row()
            .get_cell(ContainerArea::Center)
            .as_grid()
    }

    /// Returns a mutable reference to the inside node label container (grid).
    /// Only available after `NodeLabelCellCreator` has run.
    pub fn inside_node_label_container_mut(&mut self) -> Option<&mut GridContainerCell> {
        self.node_container_middle_row_mut()
            .get_cell_mut(ContainerArea::Center)
            .as_grid_mut()
    }

    /// Returns the port alignment that applies to the given side of the node.
    /// Falls back to the default port alignment if no side-specific alignment is set.
    pub fn get_port_alignment(&self, port_side: PortSide) -> PortAlignment {
        let specific = match port_side {
            PortSide::North => self.port_alignment_north,
            PortSide::South => self.port_alignment_south,
            PortSide::East => self.port_alignment_east,
            PortSide::West => self.port_alignment_west,
            _ => None,
        };
        specific.unwrap_or(self.port_alignment_default)
    }

    /// Syncs cell rectangles from the node container's cells (which are updated by
    /// `layout_children_horizontally`/`layout_children_vertically`) to the corresponding
    /// entries in `inside_port_label_cells` HashMap.
    ///
    /// In Java, both the cell system and the EnumMap hold the same AtomicCell reference,
    /// so layout updates are automatically visible. In Rust, they are separate copies,
    /// so we need to explicitly sync after layout.
    pub fn sync_inside_port_label_cell_rectangles(&mut self) {
        // N/S cells are direct children of node_container (Begin/End)
        for (port_side, container_area) in [
            (PortSide::North, ContainerArea::Begin),
            (PortSide::South, ContainerArea::End),
        ] {
            if let Some(container_cell) = self
                .node_container
                .get_cell(container_area)
                .as_atomic()
            {
                let rect = *container_cell.cell_rectangle_ref();
                if let Some(hash_cell) = self.inside_port_label_cells.get_mut(&port_side) {
                    *hash_cell.cell_rectangle() = rect;
                }
            }
        }

        // E/W cells are children of the middle row (Begin/End)
        // We need to read from the container first, then write to the HashMap
        let east_rect = self
            .node_container_middle_row()
            .get_cell(ContainerArea::End)
            .as_atomic()
            .map(|c| *c.cell_rectangle_ref());
        let west_rect = self
            .node_container_middle_row()
            .get_cell(ContainerArea::Begin)
            .as_atomic()
            .map(|c| *c.cell_rectangle_ref());

        if let Some(rect) = east_rect {
            if let Some(hash_cell) = self.inside_port_label_cells.get_mut(&PortSide::East) {
                *hash_cell.cell_rectangle() = rect;
            }
        }
        if let Some(rect) = west_rect {
            if let Some(hash_cell) = self.inside_port_label_cells.get_mut(&PortSide::West) {
                *hash_cell.cell_rectangle() = rect;
            }
        }
    }
}
