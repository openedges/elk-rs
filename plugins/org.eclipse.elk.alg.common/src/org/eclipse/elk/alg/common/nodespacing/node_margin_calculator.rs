use std::marker::PhantomData;

use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMargin, ElkRectangle, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{CoreOptions, EdgeLabelPlacement, PortLabelPlacement};
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{
    EdgeAdapter, GraphAdapter, GraphElementAdapter, LabelAdapter, NodeAdapter, PortAdapter,
};

pub struct NodeMarginCalculator<'a, G, T>
where
    G: GraphAdapter<T>,
{
    adapter: &'a G,
    include_labels: bool,
    include_ports: bool,
    include_port_labels: bool,
    include_edge_head_tail_labels: bool,
    _phantom: PhantomData<T>,
}

impl<'a, G, T> NodeMarginCalculator<'a, G, T>
where
    G: GraphAdapter<T>,
{
    pub fn new(adapter: &'a G) -> Self {
        NodeMarginCalculator {
            adapter,
            include_labels: true,
            include_ports: true,
            include_port_labels: true,
            include_edge_head_tail_labels: true,
            _phantom: PhantomData,
        }
    }

    pub fn exclude_labels(&mut self) -> &mut Self {
        self.include_labels = false;
        self
    }

    pub fn exclude_ports(&mut self) -> &mut Self {
        self.include_ports = false;
        self
    }

    pub fn exclude_port_labels(&mut self) -> &mut Self {
        self.include_port_labels = false;
        self
    }

    pub fn exclude_edge_head_tail_labels(&mut self) -> &mut Self {
        self.include_edge_head_tail_labels = false;
        self
    }

    pub fn process(&mut self) {
        let spacing = self
            .adapter
            .get_property(CoreOptions::SPACING_LABEL_NODE)
            .unwrap_or(0.0);
        for node in self.adapter.get_nodes() {
            self.process_node_with_spacing(&node, spacing);
        }
    }

    pub fn process_node(&mut self, node: &G::NodeAdapter) {
        let spacing = self
            .adapter
            .get_property(CoreOptions::SPACING_LABEL_NODE)
            .unwrap_or(0.0);
        self.process_node_with_spacing(node, spacing);
    }

    fn process_node_with_spacing(&mut self, node: &G::NodeAdapter, label_spacing: f64) {
        let node_pos = node.get_position();
        let node_size = node.get_size();
        let mut bounding_box = ElkRectangle::with_values(
            node_pos.x,
            node_pos.y,
            node_size.x,
            node_size.y,
        );
        let mut element_box = ElkRectangle::new();

        if self.include_labels {
            for label in node.get_labels() {
                let label_pos = label.get_position();
                let label_size = label.get_size();
                element_box.x = label_pos.x + node_pos.x;
                element_box.y = label_pos.y + node_pos.y;
                element_box.width = label_size.x;
                element_box.height = label_size.y;
                bounding_box.union(&element_box);
            }
        }

        for port in node.get_ports() {
            let port_pos = port.get_position();
            let port_size = port.get_size();
            let port_x = port_pos.x + node_pos.x;
            let port_y = port_pos.y + node_pos.y;

            if self.include_ports {
                element_box.x = port_x;
                element_box.y = port_y;
                element_box.width = port_size.x;
                element_box.height = port_size.y;
                bounding_box.union(&element_box);
            }

            if self.include_port_labels {
                for label in port.get_labels() {
                    let label_pos = label.get_position();
                    let label_size = label.get_size();
                    element_box.x = label_pos.x + port_x;
                    element_box.y = label_pos.y + port_y;
                    element_box.width = label_size.x;
                    element_box.height = label_size.y;
                    bounding_box.union(&element_box);
                }
            }

            if self.include_edge_head_tail_labels {
                let mut required_port_label_space = KVector::with_values(-label_spacing, -label_spacing);
                if node
                    .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                    .map(|set| set.contains(&PortLabelPlacement::Outside))
                    .unwrap_or(false)
                {
                    for label in port.get_labels() {
                        let label_size = label.get_size();
                        required_port_label_space.x += label_size.x + label_spacing;
                        required_port_label_space.y += label_size.y + label_spacing;
                    }
                }
                required_port_label_space.x = required_port_label_space.x.max(0.0);
                required_port_label_space.y = required_port_label_space.y.max(0.0);

                self.process_edge_head_tail_labels(
                    &mut bounding_box,
                    port.get_outgoing_edges(),
                    port.get_incoming_edges(),
                    node,
                    Some(&port),
                    Some(&required_port_label_space),
                    label_spacing,
                );
            }
        }

        if self.include_edge_head_tail_labels {
            self.process_edge_head_tail_labels(
                &mut bounding_box,
                node.get_outgoing_edges(),
                node.get_incoming_edges(),
                node,
                None,
                None,
                label_spacing,
            );
        }

        let mut margin = ElkMargin::from_other(&node.get_margin());
        margin.top = (node_pos.y - bounding_box.y).max(0.0);
        margin.bottom = (bounding_box.y + bounding_box.height - (node_pos.y + node_size.y)).max(0.0);
        margin.left = (node_pos.x - bounding_box.x).max(0.0);
        margin.right = (bounding_box.x + bounding_box.width - (node_pos.x + node_size.x)).max(0.0);
        node.set_margin(margin);
    }

    fn process_edge_head_tail_labels<E, ET>(
        &self,
        bounding_box: &mut ElkRectangle,
        outgoing_edges: Vec<E>,
        incoming_edges: Vec<E>,
        node: &G::NodeAdapter,
        port: Option<&PortAdapterOf<G, T>>,
        port_label_space: Option<&KVector>,
        label_spacing: f64,
    ) where
        E: EdgeAdapter<ET>,
    {
        let mut label_box = ElkRectangle::new();

        for edge in outgoing_edges {
            for label in edge.get_labels() {
                let placement = label
                    .get_property(CoreOptions::EDGE_LABELS_PLACEMENT)
                    .unwrap_or(EdgeLabelPlacement::Center);
                if placement == EdgeLabelPlacement::Tail {
                    self.compute_label_box(
                        &mut label_box,
                        &label,
                        false,
                        node,
                        port,
                        port_label_space,
                        label_spacing,
                    );
                    bounding_box.union(&label_box);
                }
            }
        }

        for edge in incoming_edges {
            for label in edge.get_labels() {
                let placement = label
                    .get_property(CoreOptions::EDGE_LABELS_PLACEMENT)
                    .unwrap_or(EdgeLabelPlacement::Center);
                if placement == EdgeLabelPlacement::Head {
                    self.compute_label_box(
                        &mut label_box,
                        &label,
                        true,
                        node,
                        port,
                        port_label_space,
                        label_spacing,
                    );
                    bounding_box.union(&label_box);
                }
            }
        }
    }

    fn compute_label_box<L, LT>(
        &self,
        label_box: &mut ElkRectangle,
        label: &L,
        incoming_edge: bool,
        node: &G::NodeAdapter,
        port: Option<&PortAdapterOf<G, T>>,
        port_label_space: Option<&KVector>,
        label_spacing: f64,
    ) where
        L: LabelAdapter<LT>,
    {
        let node_pos = node.get_position();
        label_box.x = node_pos.x;
        label_box.y = node_pos.y;
        if let Some(port) = port {
            let port_pos = port.get_position();
            label_box.x += port_pos.x;
            label_box.y += port_pos.y;
        }

        let label_size = label.get_size();
        label_box.width = label_size.x;
        label_box.height = label_size.y;

        if port.is_none() {
            if incoming_edge {
                label_box.x -= label_spacing + label_size.x;
            } else {
                label_box.x += node.get_size().x + label_spacing;
            }
            return;
        }

        let port = port.expect("port exists");
        let port_size = port.get_size();
        let port_label_space = port_label_space.copied().unwrap_or(KVector::new());

        match port.get_side() {
            org_eclipse_elk_core::org::eclipse::elk::core::options::PortSide::Undefined
            | org_eclipse_elk_core::org::eclipse::elk::core::options::PortSide::East => {
                label_box.x += port_size.x + label_spacing + port_label_space.x + label_spacing;
            }
            org_eclipse_elk_core::org::eclipse::elk::core::options::PortSide::West => {
                label_box.x -= label_spacing + port_label_space.x + label_spacing + label_size.x;
            }
            org_eclipse_elk_core::org::eclipse::elk::core::options::PortSide::North => {
                label_box.x += port_size.x + label_spacing;
                label_box.y -= label_spacing + port_label_space.y + label_spacing + label_size.y;
            }
            org_eclipse_elk_core::org::eclipse::elk::core::options::PortSide::South => {
                label_box.x += port_size.x + label_spacing;
                label_box.y += port_size.y + label_spacing + port_label_space.y + label_spacing;
            }
        }
    }
}

type NodeType<G, T> = <G as GraphAdapter<T>>::Node;
type NodeAdapterOf<G, T> = <G as GraphAdapter<T>>::NodeAdapter;
type PortAdapterOf<G, T> = <NodeAdapterOf<G, T> as NodeAdapter<NodeType<G, T>>>::PortAdapter;
