use crate::org::eclipse::elk::alg::common::nodespacing::cellsystem::{DynLabel, DynLabelCell};
use crate::org::eclipse::elk::alg::common::nodespacing::internal::node_context::NodeContext;
use crate::org::eclipse::elk::alg::common::nodespacing::internal::port_context::PortContext;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, PortLabelPlacement, PortSide,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{
    GraphElementAdapter, NodeAdapter, PortAdapter,
};

/// Creates port context objects and assigns volatile IDs to all ports. Also, unless
/// port labels are fixed, the labels are added to the port context label cells.
///
/// Faithfully ports Java's `PortContextCreator`.
pub struct PortContextCreator;

impl PortContextCreator {
    /// Creates and initializes port context objects for each of the node's ports.
    pub fn create_port_contexts<N, T>(
        node_context: &mut NodeContext,
        node: &N,
        ignore_inside_port_labels: bool,
    ) where
        T: 'static,
        N: NodeAdapter<T>,
        N::Port: 'static,
        N::PortAdapter: 'static,
    {
        let im_port_labels = !ignore_inside_port_labels
            || !node_context
                .port_labels_placement
                .contains(&PortLabelPlacement::Inside);

        for (volatile_id, port) in (0_i32..).zip(node.get_ports()) {
            let port_side = port.get_side();
            if port_side == PortSide::Undefined {
                panic!(
                    "Label and node size calculator can only be used with ports that \
                     have port sides assigned."
                );
            }

            port.set_volatile_id(volatile_id);

            Self::create_port_context(node_context, port, im_port_labels);
        }

        // Java's TreeMultimap uses comparePortContexts which sorts:
        // - NORTH/EAST: ascending by volatile_id (natural iteration order)
        // - SOUTH/WEST: descending by volatile_id (reversed)
        // This matters for port placement order (clockwise numbering means
        // SOUTH goes right-to-left, WEST goes bottom-to-top).
        for side in [PortSide::South, PortSide::West] {
            if let Some(contexts) = node_context.port_contexts.get_mut(&side) {
                contexts.reverse();
            }
        }
    }

    fn create_port_context<T, P>(
        node_context: &mut NodeContext,
        port: P,
        im_port_labels: bool,
    ) where
        T: 'static,
        P: PortAdapter<T> + 'static,
    {
        let port_size = port.get_size();
        let port_side = port.get_side();
        let port_position = port.get_position();

        let port_border_offset = port
            .get_property(CoreOptions::PORT_BORDER_OFFSET)
            .unwrap_or(0.0);
        let has_port_border_offset = port.has_property(CoreOptions::PORT_BORDER_OFFSET);
        let has_compound_connections = port.has_compound_connections();

        // Collect label sizes and positions
        let labels = port.get_labels();
        let label_sizes: Vec<KVector> = labels.iter().map(|l| l.get_size()).collect();
        let label_positions: Vec<KVector> = labels.iter().map(|l| l.get_position()).collect();

        let labels_next_to_port = port
            .get_property(CoreOptions::PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE)
            .unwrap_or(false);

        let mut port_context = PortContext::new(
            port_size,
            port_side,
            port.get_volatile_id(),
            port_position,
            port_border_offset,
            has_port_border_offset,
            has_compound_connections,
        );

        port_context.labels_next_to_port = labels_next_to_port;
        port_context.label_sizes = label_sizes;
        port_context.label_positions = label_positions;

        // If the port has labels and if port labels are to be placed, we need to remember them
        if im_port_labels && !PortLabelPlacement::is_fixed(&node_context.port_labels_placement) {
            let mut label_cell = DynLabelCell::new(node_context.label_label_spacing);
            for label in port.get_labels() {
                label_cell.add_label(DynLabel::new(label));
            }
            port_context.port_label_cell = Some(label_cell);
        }

        node_context
            .port_contexts
            .entry(port_side)
            .or_default()
            .push(port_context);
    }
}
