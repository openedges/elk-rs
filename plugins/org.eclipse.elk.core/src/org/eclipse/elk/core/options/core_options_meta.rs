use std::any::{Any, TypeId};
use std::fmt;
use std::sync::{Arc, LazyLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutCategoryData, LayoutMetaDataRegistry, LayoutOptionData,
    LayoutOptionTarget, LayoutOptionType, LayoutOptionVisibility,
};
use crate::org::eclipse::elk::core::math::{ElkMargin, ElkPadding, KVector, KVectorChain};
use crate::org::eclipse::elk::core::options::{
    Alignment, ContentAlignment, CoreOptions, Direction, EdgeCoords, EdgeLabelPlacement, EdgeRouting,
    EdgeType, HierarchyHandling, NodeLabelPlacement, PackingMode, PortAlignment, PortConstraints,
    PortLabelPlacement, PortSide, ShapeCoords, SizeConstraint, SizeOptions, TopdownNodeTypes,
    TopdownSizeApproximator,
};
use crate::org::eclipse::elk::core::util::{
    EnumSet, EnumSetType, ExclusiveBounds, IndividualSpacings,
};

type ParserFn = Arc<dyn Fn(&str) -> Option<Arc<dyn Any + Send + Sync>> + Send + Sync>;

const TARGET_PARENTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Parents];
const TARGET_NODES: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Nodes];
const TARGET_EDGES: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Edges];
const TARGET_PORTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Ports];
const TARGET_LABELS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Labels];
const TARGET_PARENTS_LABELS: [LayoutOptionTarget; 2] =
    [LayoutOptionTarget::Parents, LayoutOptionTarget::Labels];
const TARGET_PARENTS_NODES: [LayoutOptionTarget; 2] =
    [LayoutOptionTarget::Parents, LayoutOptionTarget::Nodes];
const TARGET_NODES_EDGES: [LayoutOptionTarget; 2] =
    [LayoutOptionTarget::Nodes, LayoutOptionTarget::Edges];
const TARGET_NODES_LABELS: [LayoutOptionTarget; 2] =
    [LayoutOptionTarget::Nodes, LayoutOptionTarget::Labels];
const TARGET_NODES_PORTS_LABELS: [LayoutOptionTarget; 3] = [
    LayoutOptionTarget::Nodes,
    LayoutOptionTarget::Ports,
    LayoutOptionTarget::Labels,
];
const TARGET_NODES_EDGES_PORTS_LABELS: [LayoutOptionTarget; 4] = [
    LayoutOptionTarget::Nodes,
    LayoutOptionTarget::Edges,
    LayoutOptionTarget::Ports,
    LayoutOptionTarget::Labels,
];

struct OptionMeta {
    name: &'static str,
    description: &'static str,
    targets: &'static [LayoutOptionTarget],
    visibility: LayoutOptionVisibility,
    group: Option<&'static str>,
    lower_bound: Option<Arc<dyn Any + Send + Sync>>,
    upper_bound: Option<Arc<dyn Any + Send + Sync>>,
}

impl OptionMeta {
    fn visible(
        name: &'static str,
        description: &'static str,
        targets: &'static [LayoutOptionTarget],
    ) -> Self {
        OptionMeta {
            name,
            description,
            targets,
            visibility: LayoutOptionVisibility::Visible,
            group: None,
            lower_bound: None,
            upper_bound: None,
        }
    }

    fn advanced(
        name: &'static str,
        description: &'static str,
        targets: &'static [LayoutOptionTarget],
    ) -> Self {
        OptionMeta {
            visibility: LayoutOptionVisibility::Advanced,
            ..Self::visible(name, description, targets)
        }
    }

    fn hidden(
        name: &'static str,
        description: &'static str,
        targets: &'static [LayoutOptionTarget],
    ) -> Self {
        OptionMeta {
            visibility: LayoutOptionVisibility::Hidden,
            ..Self::visible(name, description, targets)
        }
    }

    fn group(mut self, group: &'static str) -> Self {
        self.group = Some(group);
        self
    }

    fn lower_bound(mut self, bound: Arc<dyn Any + Send + Sync>) -> Self {
        self.lower_bound = Some(bound);
        self
    }

}

macro_rules! apply_meta {
    ($builder:expr, $meta:expr) => {{
        let mut builder = $builder;
        let meta = $meta;
        builder = builder
            .name(meta.name)
            .description(meta.description)
            .targets(meta.targets.iter().copied().collect())
            .visibility(meta.visibility);
        if let Some(group) = meta.group {
            builder = builder.group(group);
        }
        if let Some(lower) = meta.lower_bound {
            builder = builder.lower_bound(Some(lower));
        }
        if let Some(upper) = meta.upper_bound {
            builder = builder.upper_bound(Some(upper));
        }
        builder
    }};
}

impl ILayoutMetaDataProvider for CoreOptions {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        register_categories(registry);
        register_ui_options(registry);
        register_json_options(registry);
        register_spacing_options(registry);
        register_partitioning_options(registry);
        register_node_label_options(registry);
        register_port_alignment_options(registry);
        register_port_constraints_and_misc(registry);
        register_node_size_options(registry);
        register_programmatic_options(registry);
        register_comment_and_edge_label_options(registry);
        register_font_options(registry);
        register_text_options(registry);
        register_port_options(registry);
        register_port_label_options(registry);
        register_topdown_options(registry);
        register_inside_self_loop_options(registry);
        register_edge_options(registry);
        register_global_options(registry);
        register_dependencies(registry);
    }
}

fn register_ui_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        CoreOptions::ALGORITHM,
        LayoutOptionType::String,
        OptionMeta::visible(
            "Layout Algorithm",
            "Select a specific layout algorithm.",
            &TARGET_PARENTS,
        ),
    );
    register_option(
        registry,
        CoreOptions::RESOLVED_ALGORITHM,
        LayoutOptionType::Object,
        OptionMeta::hidden(
            "Resolved Layout Algorithm",
            "Meta data associated with the selected algorithm.",
            &TARGET_PARENTS,
        ),
    );
    register_enum_option(
        registry,
        CoreOptions::ALIGNMENT,
        alignment_variants(),
        OptionMeta::advanced(
            "Alignment",
            "Alignment of the selected node relative to other nodes; the exact meaning depends on the used algorithm.",
            &TARGET_NODES,
        ),
    );
    register_option(
        registry,
        CoreOptions::ASPECT_RATIO,
        LayoutOptionType::Double,
        OptionMeta::advanced(
            "Aspect Ratio",
            "The desired aspect ratio of the drawing, that is the quotient of width by height.",
            &TARGET_PARENTS,
        )
        .lower_bound(bound_exclusive_lower(0.0)),
    );
    register_option(
        registry,
        CoreOptions::BEND_POINTS,
        LayoutOptionType::Object,
        OptionMeta::hidden(
            "Bend Points",
            concat!(
                "A fixed list of bend points for the edge. This is used by the 'Fixed Layout' algorithm to ",
                "specify a pre-defined routing for an edge. The vector chain must include the source point, ",
                "any bend points, and the target point, so it must have at least two points."
            ),
            &TARGET_EDGES,
        ),
    );
    register_enumset_option(
        registry,
        CoreOptions::CONTENT_ALIGNMENT,
        ContentAlignment::variants(),
        OptionMeta::advanced(
            "Content Alignment",
            concat!(
                "Specifies how the content of a node are aligned. Each node can individually control the ",
                "alignment of its contents. If a node should be aligned top left in its parent node, the parent ",
                "node should specify that option."
            ),
            &TARGET_PARENTS,
        ),
    );
    register_option(
        registry,
        CoreOptions::DEBUG_MODE,
        LayoutOptionType::Boolean,
        OptionMeta::advanced(
            "Debug Mode",
            "Whether additional debug information shall be generated.",
            &TARGET_PARENTS,
        ),
    );
    register_enum_option(
        registry,
        CoreOptions::DIRECTION,
        direction_variants(),
        OptionMeta::visible(
            "Direction",
            "Overall direction of edges: horizontal (right / left) or vertical (down / up).",
            &TARGET_PARENTS,
        ),
    );
    register_enum_option(
        registry,
        CoreOptions::EDGE_ROUTING,
        edge_routing_variants(),
        OptionMeta::visible(
            "Edge Routing",
            concat!(
                "What kind of edge routing style should be applied for the content of a parent node. ",
                "Algorithms may also set this option to single edges in order to mark them as splines. ",
                "The bend point list of edges with this option set to SPLINES must be interpreted as control ",
                "points for a piecewise cubic spline."
            ),
            &TARGET_PARENTS,
        ),
    );
    register_option(
        registry,
        CoreOptions::EXPAND_NODES,
        LayoutOptionType::Boolean,
        OptionMeta::advanced(
            "Expand Nodes",
            "If active, nodes are expanded to fill the area of their parent.",
            &TARGET_PARENTS,
        ),
    );
    register_enum_option(
        registry,
        CoreOptions::HIERARCHY_HANDLING,
        hierarchy_variants(),
        OptionMeta::advanced(
            "Hierarchy Handling",
            concat!(
                "Determines whether separate layout runs are triggered for different compound nodes in a ",
                "hierarchical graph. Setting a node's hierarchy handling to INCLUDE_CHILDREN will lay out ",
                "that node and all of its descendants in a single layout run, until a descendant is encountered ",
                "which has its hierarchy handling set to SEPARATE_CHILDREN. In general, SEPARATE_CHILDREN will ",
                "ensure that a new layout run is triggered for a node with that setting. Including multiple levels ",
                "of hierarchy in a single layout run may allow cross-hierarchical edges to be laid out properly. ",
                "If the root node is set to INHERIT (or not set at all), the default behavior is SEPARATE_CHILDREN."
            ),
            &TARGET_PARENTS_NODES,
        ),
    );
    register_option(
        registry,
        CoreOptions::PADDING,
        LayoutOptionType::Object,
        OptionMeta::advanced(
            "Padding",
            concat!(
                "The padding to be left to a parent element's border when placing child elements. This can ",
                "also serve as an output option of a layout algorithm if node size calculation is setup appropriately."
            ),
            &TARGET_PARENTS_NODES,
        ),
    );
    register_option(
        registry,
        CoreOptions::INTERACTIVE,
        LayoutOptionType::Boolean,
        OptionMeta::advanced(
            "Interactive",
            concat!(
                "Whether the algorithm should be run in interactive mode for the content of a parent node. ",
                "What this means exactly depends on how the specific algorithm interprets this option. ",
                "Usually in the interactive mode algorithms try to modify the current layout as little as possible."
            ),
            &TARGET_PARENTS,
        ),
    );
    register_option(
        registry,
        CoreOptions::INTERACTIVE_LAYOUT,
        LayoutOptionType::Boolean,
        OptionMeta::advanced(
            "Interactive Layout",
            "Whether the graph should be changeable interactively and by setting constraints.",
            &TARGET_PARENTS,
        ),
    );
    register_option(
        registry,
        CoreOptions::OMIT_NODE_MICRO_LAYOUT,
        LayoutOptionType::Boolean,
        OptionMeta::advanced(
            "Omit Node Micro Layout",
            concat!(
                "Node micro layout comprises the computation of node dimensions (if requested), the placement of ",
                "ports and their labels, and the placement of node labels. The functionality is implemented ",
                "independent of any specific layout algorithm and should not have any negative impact on the ",
                "layout algorithm's performance itself. Yet, if any unforeseen behavior occurs, this option allows ",
                "to deactivate the micro layout."
            ),
            &TARGET_PARENTS,
        ),
    );
    register_enum_option(
        registry,
        CoreOptions::BOX_PACKING_MODE,
        packing_mode_variants(),
        OptionMeta::visible(
            "Box Layout Mode",
            concat!(
                "Configures the packing mode used by the BoxLayoutProvider. If SIMPLE is not required ",
                "(neither priorities are used nor the interactive mode), GROUP_DEC can improve the ",
                "packing and decrease the area. GROUP_MIXED and GROUP_INC may, in very specific ",
                "scenarios, work better."
            ),
            &TARGET_PARENTS,
        )
        .group("box"),
    );
}

fn register_json_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_enum_option(
        registry,
        CoreOptions::JSON_SHAPE_COORDS,
        shape_coords_variants(),
        OptionMeta::visible(
            "Shape Coords",
            "For layouts transferred into JSON graphs, specify the coordinate system to be used for nodes, ports, and labels of nodes and ports.",
            &TARGET_PARENTS,
        )
        .group("json"),
    );
    register_enum_option(
        registry,
        CoreOptions::JSON_EDGE_COORDS,
        edge_coords_variants(),
        OptionMeta::visible(
            "Edge Coords",
            "For layouts transferred into JSON graphs, specify the coordinate system to be used for edge route points and edge labels.",
            &TARGET_PARENTS,
        )
        .group("json"),
    );
}

fn register_spacing_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        CoreOptions::SPACING_COMMENT_COMMENT,
        LayoutOptionType::Double,
        OptionMeta::visible(
            "Comment Comment Spacing",
            concat!(
                "Spacing to be preserved between a comment box and other comment boxes connected to the same node. ",
                "The space left between comment boxes of different nodes is controlled by the node-node spacing."
            ),
            &TARGET_PARENTS,
        )
        .group("spacing")
        .lower_bound(bound_f64(0.0)),
    );
    register_option(
        registry,
        CoreOptions::SPACING_COMMENT_NODE,
        LayoutOptionType::Double,
        OptionMeta::visible(
            "Comment Node Spacing",
            concat!(
                "Spacing to be preserved between a node and its connected comment boxes. The space left between a node ",
                "and the comments of another node is controlled by the node-node spacing."
            ),
            &TARGET_PARENTS,
        )
        .group("spacing")
        .lower_bound(bound_f64(0.0)),
    );
    register_option(
        registry,
        CoreOptions::SPACING_COMPONENT_COMPONENT,
        LayoutOptionType::Double,
        OptionMeta::visible(
            "Components Spacing",
            "Spacing to be preserved between pairs of connected components. This option is only relevant if separateConnectedComponents is activated.",
            &TARGET_PARENTS,
        )
        .group("spacing")
        .lower_bound(bound_f64(0.0)),
    );
    register_option(
        registry,
        CoreOptions::SPACING_EDGE_EDGE,
        LayoutOptionType::Double,
        OptionMeta::visible(
            "Edge Spacing",
            concat!(
                "Spacing to be preserved between any two edges. Note that while this can somewhat easily be satisfied ",
                "for the segments of orthogonally drawn edges, it is harder for general polylines or splines."
            ),
            &TARGET_PARENTS,
        )
        .group("spacing")
        .lower_bound(bound_f64(0.0)),
    );
    register_option(
        registry,
        CoreOptions::SPACING_EDGE_LABEL,
        LayoutOptionType::Double,
        OptionMeta::visible(
            "Edge Label Spacing",
            concat!(
                "The minimal distance to be preserved between a label and the edge it is associated with. ",
                "Note that the placement of a label is influenced by the edgeLabels.placement option."
            ),
            &TARGET_PARENTS,
        )
        .group("spacing")
        .lower_bound(bound_f64(0.0)),
    );
    register_option(
        registry,
        CoreOptions::SPACING_EDGE_NODE,
        LayoutOptionType::Double,
        OptionMeta::visible(
            "Edge Node Spacing",
            "Spacing to be preserved between nodes and edges.",
            &TARGET_PARENTS,
        )
        .group("spacing")
        .lower_bound(bound_f64(0.0)),
    );
    register_option(
        registry,
        CoreOptions::SPACING_LABEL_LABEL,
        LayoutOptionType::Double,
        OptionMeta::visible(
            "Label Spacing",
            "Determines the amount of space to be left between two labels of the same graph element.",
            &TARGET_PARENTS,
        )
        .group("spacing")
        .lower_bound(bound_f64(0.0)),
    );
    register_option(
        registry,
        CoreOptions::SPACING_LABEL_NODE,
        LayoutOptionType::Double,
        OptionMeta::visible(
            "Label Node Spacing",
            concat!(
                "Spacing to be preserved between labels and the border of node they are associated with. ",
                "Note that the placement of a label is influenced by the nodeLabels.placement option."
            ),
            &TARGET_PARENTS,
        )
        .group("spacing")
        .lower_bound(bound_f64(0.0)),
    );
    register_option(
        registry,
        CoreOptions::SPACING_LABEL_PORT_HORIZONTAL,
        LayoutOptionType::Double,
        OptionMeta::visible(
            "Horizontal spacing between Label and Port",
            concat!(
                "Horizontal spacing to be preserved between labels and the ports they are associated with. ",
                "Note that the placement of a label is influenced by the portLabels.placement option."
            ),
            &TARGET_PARENTS,
        )
        .group("spacing"),
    );
    register_option(
        registry,
        CoreOptions::SPACING_LABEL_PORT_VERTICAL,
        LayoutOptionType::Double,
        OptionMeta::visible(
            "Vertical spacing between Label and Port",
            concat!(
                "Vertical spacing to be preserved between labels and the ports they are associated with. ",
                "Note that the placement of a label is influenced by the portLabels.placement option."
            ),
            &TARGET_PARENTS,
        )
        .group("spacing"),
    );
    register_option(
        registry,
        CoreOptions::SPACING_NODE_NODE,
        LayoutOptionType::Double,
        OptionMeta::visible(
            "Node Spacing",
            "The minimal distance to be preserved between each two nodes.",
            &TARGET_PARENTS,
        )
        .group("spacing")
        .lower_bound(bound_f64(0.0)),
    );
    register_option(
        registry,
        CoreOptions::SPACING_NODE_SELF_LOOP,
        LayoutOptionType::Double,
        OptionMeta::visible(
            "Node Self Loop Spacing",
            "Spacing to be preserved between a node and its self loops.",
            &TARGET_PARENTS,
        )
        .group("spacing")
        .lower_bound(bound_f64(0.0)),
    );
    register_option(
        registry,
        CoreOptions::SPACING_PORT_PORT,
        LayoutOptionType::Double,
        OptionMeta::visible(
            "Port Spacing",
            "Spacing between pairs of ports of the same node.",
            &TARGET_PARENTS_NODES,
        )
        .group("spacing")
        .lower_bound(bound_f64(0.0)),
    );
    register_option(
        registry,
        CoreOptions::SPACING_INDIVIDUAL,
        LayoutOptionType::Object,
        OptionMeta::advanced(
            "Individual Spacing",
            concat!(
                "Allows to specify individual spacing values for graph elements that shall be different from ",
                "the value specified for the element's parent."
            ),
            &TARGET_NODES_EDGES_PORTS_LABELS,
        )
        .group("spacing"),
    );
    register_option(
        registry,
        CoreOptions::SPACING_PORTS_SURROUNDING,
        LayoutOptionType::Object,
        OptionMeta::advanced(
            "Additional Port Space",
            concat!(
                "Additional space around the sets of ports on each node side. For each side of a node, ",
                "this option can reserve additional space before and after the ports on each side. For example, ",
                "a top spacing of 20 makes sure that the first port on the western and eastern side is 20 units away ",
                "from the northern border."
            ),
            &TARGET_PARENTS,
        )
        .group("spacing"),
    );
}

fn register_partitioning_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        CoreOptions::PARTITIONING_PARTITION,
        LayoutOptionType::Int,
        OptionMeta::advanced(
            "Layout Partition",
            concat!(
                "Partition to which the node belongs. This requires Layout Partitioning to be active. Nodes with lower ",
                "partition IDs will appear to the left of nodes with higher partition IDs (assuming a left-to-right layout ",
                "direction)."
            ),
            &TARGET_PARENTS_NODES,
        )
        .group("partitioning"),
    );
    register_option(
        registry,
        CoreOptions::PARTITIONING_ACTIVATE,
        LayoutOptionType::Boolean,
        OptionMeta::advanced(
            "Layout Partitioning",
            concat!(
                "Whether to activate partitioned layout. This will allow to group nodes through the Layout Partition option. ",
                "A pair of nodes with different partition indices is then placed such that the node with lower index is ",
                "placed to the left of the other node (with left-to-right layout direction). Depending on the layout algorithm, ",
                "this may only be guaranteed to work if all nodes have a layout partition configured, or at least if edges that ",
                "cross partitions are not part of a partition-crossing cycle."
            ),
            &TARGET_PARENTS,
        )
        .group("partitioning"),
    );
}

fn register_node_label_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        CoreOptions::NODE_LABELS_PADDING,
        LayoutOptionType::Object,
        OptionMeta::advanced(
            "Node Label Padding",
            "Define padding for node labels that are placed inside of a node.",
            &TARGET_PARENTS,
        )
        .group("nodeLabels"),
    );
    register_enumset_option(
        registry,
        CoreOptions::NODE_LABELS_PLACEMENT,
        NodeLabelPlacement::variants(),
        OptionMeta::visible(
            "Node Label Placement",
            "Hints for where node labels are to be placed; if empty, the node label's position is not modified.",
            &TARGET_NODES_LABELS,
        )
        .group("nodeLabels"),
    );
}

fn register_port_alignment_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_enum_option(
        registry,
        CoreOptions::PORT_ALIGNMENT_DEFAULT,
        port_alignment_variants(),
        OptionMeta::visible(
            "Port Alignment",
            "Defines the default port distribution for a node. May be overridden for each side individually.",
            &TARGET_NODES,
        )
        .group("portAlignment"),
    );
    register_enum_option(
        registry,
        CoreOptions::PORT_ALIGNMENT_NORTH,
        port_alignment_variants(),
        OptionMeta::advanced(
            "Port Alignment (North)",
            "Defines how ports on the northern side are placed, overriding the node's general port alignment.",
            &TARGET_NODES,
        )
        .group("portAlignment"),
    );
    register_enum_option(
        registry,
        CoreOptions::PORT_ALIGNMENT_SOUTH,
        port_alignment_variants(),
        OptionMeta::advanced(
            "Port Alignment (South)",
            "Defines how ports on the southern side are placed, overriding the node's general port alignment.",
            &TARGET_NODES,
        )
        .group("portAlignment"),
    );
    register_enum_option(
        registry,
        CoreOptions::PORT_ALIGNMENT_WEST,
        port_alignment_variants(),
        OptionMeta::advanced(
            "Port Alignment (West)",
            "Defines how ports on the western side are placed, overriding the node's general port alignment.",
            &TARGET_NODES,
        )
        .group("portAlignment"),
    );
    register_enum_option(
        registry,
        CoreOptions::PORT_ALIGNMENT_EAST,
        port_alignment_variants(),
        OptionMeta::advanced(
            "Port Alignment (East)",
            "Defines how ports on the eastern side are placed, overriding the node's general port alignment.",
            &TARGET_NODES,
        )
        .group("portAlignment"),
    );
}

fn register_port_constraints_and_misc(registry: &mut dyn LayoutMetaDataRegistry) {
    register_enum_option(
        registry,
        CoreOptions::PORT_CONSTRAINTS,
        port_constraints_variants(),
        OptionMeta::visible(
            "Port Constraints",
            "Defines constraints of the position of the ports of a node.",
            &TARGET_NODES,
        ),
    );
    register_option(
        registry,
        CoreOptions::POSITION,
        LayoutOptionType::Object,
        OptionMeta::advanced(
            "Position",
            concat!(
                "The position of a node, port, or label. This is used by the 'Fixed Layout' algorithm to ",
                "specify a pre-defined position."
            ),
            &TARGET_NODES_PORTS_LABELS,
        ),
    );
    register_option(
        registry,
        CoreOptions::PRIORITY,
        LayoutOptionType::Int,
        OptionMeta::advanced(
            "Priority",
            "Defines the priority of an object; its meaning depends on the specific layout algorithm and the context where it is used.",
            &TARGET_NODES_EDGES,
        ),
    );
    register_option(
        registry,
        CoreOptions::RANDOM_SEED,
        LayoutOptionType::Int,
        OptionMeta::advanced(
            "Randomization Seed",
            concat!(
                "Seed used for pseudo-random number generators to control the layout algorithm. If the value is 0, ",
                "the seed shall be determined pseudo-randomly (e.g. from the system time)."
            ),
            &TARGET_PARENTS,
        ),
    );
    register_option(
        registry,
        CoreOptions::SEPARATE_CONNECTED_COMPONENTS,
        LayoutOptionType::Boolean,
        OptionMeta::visible(
            "Separate Connected Components",
            "Whether each connected component should be processed separately.",
            &TARGET_PARENTS,
        ),
    );
}

fn register_node_size_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_enumset_option(
        registry,
        CoreOptions::NODE_SIZE_CONSTRAINTS,
        SizeConstraint::variants(),
        OptionMeta::visible(
            "Node Size Constraints",
            concat!(
                "What should be taken into account when calculating a node's size. Empty size constraints ",
                "specify that a node's size is already fixed and should not be changed."
            ),
            &TARGET_NODES,
        )
        .group("nodeSize"),
    );
    register_enumset_option(
        registry,
        CoreOptions::NODE_SIZE_OPTIONS,
        SizeOptions::variants(),
        OptionMeta::visible(
            "Node Size Options",
            concat!(
                "Options modifying the behavior of the size constraints set on a node. Each member of the set specifies ",
                "something that should be taken into account when calculating node sizes. The empty set corresponds to no ",
                "further modifications."
            ),
            &TARGET_NODES,
        )
        .group("nodeSize"),
    );
    register_option(
        registry,
        CoreOptions::NODE_SIZE_MINIMUM,
        LayoutOptionType::Object,
        OptionMeta::advanced(
            "Node Size Minimum",
            "The minimal size to which a node can be reduced.",
            &TARGET_NODES,
        )
        .group("nodeSize"),
    );
    register_option(
        registry,
        CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE,
        LayoutOptionType::Boolean,
        OptionMeta::visible(
            "Fixed Graph Size",
            concat!(
                "By default, the fixed layout provider will enlarge a graph until it is large enough to contain ",
                "its children. If this option is set, it will not do so."
            ),
            &TARGET_PARENTS,
        )
        .group("nodeSize"),
    );
}

fn register_programmatic_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        CoreOptions::JUNCTION_POINTS,
        LayoutOptionType::Object,
        OptionMeta::hidden(
            "Junction Points",
            concat!(
                "This option is not used as option, but as output of the layout algorithms. It is attached to edges ",
                "and determines the points where junction symbols should be drawn in order to represent hyperedges with ",
                "orthogonal routing. Whether such points are computed depends on the chosen layout algorithm and edge ",
                "routing style. The points are put into the vector chain with no specific order."
            ),
            &TARGET_EDGES,
        ),
    );
    register_option(
        registry,
        CoreOptions::HYPERNODE,
        LayoutOptionType::Boolean,
        OptionMeta::hidden(
            "Hypernode",
            "Whether the node should be handled as a hypernode.",
            &TARGET_NODES,
        ),
    );
    register_option(
        registry,
        CoreOptions::LABEL_MANAGER,
        LayoutOptionType::Object,
        OptionMeta::hidden(
            "Label Manager",
            "Label managers can shorten labels upon a layout algorithm's request.",
            &TARGET_PARENTS_LABELS,
        ),
    );
    register_option(
        registry,
        CoreOptions::SCALE_FACTOR,
        LayoutOptionType::Double,
        OptionMeta::hidden(
            "Scale Factor",
            concat!(
                "The scaling factor to be applied to the corresponding node in recursive layout. It causes the ",
                "corresponding node's size to be adjusted, and its ports and labels to be sized and placed accordingly ",
                "after the layout of that node has been determined (and before the node itself and its siblings are arranged). ",
                "The scaling is not reverted afterwards, so the resulting layout graph contains the adjusted size and position data."
            ),
            &TARGET_NODES,
        )
        .lower_bound(bound_exclusive_lower(0.0)),
    );
    register_option(
        registry,
        CoreOptions::CHILD_AREA_WIDTH,
        LayoutOptionType::Double,
        OptionMeta::hidden(
            "Child Area Width",
            "The width of the area occupied by the laid out children of a node.",
            &TARGET_PARENTS,
        ),
    );
    register_option(
        registry,
        CoreOptions::CHILD_AREA_HEIGHT,
        LayoutOptionType::Double,
        OptionMeta::hidden(
            "Child Area Height",
            "The height of the area occupied by the laid out children of a node.",
            &TARGET_PARENTS,
        ),
    );
}

fn register_comment_and_edge_label_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        CoreOptions::COMMENT_BOX,
        LayoutOptionType::Boolean,
        OptionMeta::visible(
            "Comment Box",
            concat!(
                "Whether the node should be regarded as a comment box instead of a regular node. In that case its ",
                "placement should be similar to how labels are handled. Any edges incident to a comment box specify to ",
                "which graph elements the comment is related."
            ),
            &TARGET_NODES,
        ),
    );
    register_enum_option(
        registry,
        CoreOptions::EDGE_LABELS_PLACEMENT,
        edge_label_placement_variants(),
        OptionMeta::visible(
            "Edge Label Placement",
            "Gives a hint on where to put edge labels.",
            &TARGET_LABELS,
        )
        .group("edgeLabels"),
    );
    register_option(
        registry,
        CoreOptions::EDGE_LABELS_INLINE,
        LayoutOptionType::Boolean,
        OptionMeta::visible(
            "Inline Edge Labels",
            concat!(
                "If true, an edge label is placed directly on its edge. May only apply to center edge labels. ",
                "This kind of label placement is only advisable if the label's rendering is such that it is not ",
                "crossed by its edge and thus stays legible."
            ),
            &TARGET_LABELS,
        )
        .group("edgeLabels"),
    );
}

fn register_font_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        CoreOptions::FONT_NAME,
        LayoutOptionType::String,
        OptionMeta::hidden("Font Name", "Font name used for a label.", &TARGET_LABELS).group("font"),
    );
    register_option(
        registry,
        CoreOptions::FONT_SIZE,
        LayoutOptionType::Int,
        OptionMeta::hidden("Font Size", "Font size used for a label.", &TARGET_LABELS)
            .group("font")
            .lower_bound(bound_i32(1)),
    );
}

fn register_text_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        CoreOptions::SOFTWRAPPING_FUZZINESS,
        LayoutOptionType::Double,
        OptionMeta::visible(
            "Softwrapping Fuzziness",
            concat!(
                "Determines the amount of fuzziness to be used when performing softwrapping on labels. ",
                "The value expresses the percent of overhang that is permitted for each line. If the next line would ",
                "take up less space than this threshold, it is appended to the current line instead of being placed in a new line."
            ),
            &TARGET_LABELS,
        ),
    );
    register_option(
        registry,
        CoreOptions::MARGINS,
        LayoutOptionType::Object,
        OptionMeta::visible(
            "Margins",
            concat!(
                "Margins define additional space around the actual bounds of a graph element. For instance, ports or labels ",
                "being placed on the outside of a node's border might introduce such a margin. The margin is used to guarantee ",
                "non-overlap of other graph elements with those ports or labels."
            ),
            &TARGET_NODES,
        ),
    );
    register_option(
        registry,
        CoreOptions::NO_LAYOUT,
        LayoutOptionType::Boolean,
        OptionMeta::visible(
            "No Layout",
            concat!(
                "No layout is done for the associated element. This is used to mark parts of a diagram to avoid their inclusion ",
                "in the layout graph, or to mark parts of the layout graph to prevent layout engines from processing them. If you ",
                "wish to exclude the contents of a compound node from automatic layout, while the node itself is still considered ",
                "on its own layer, use the 'Fixed Layout' algorithm for that node."
            ),
            &TARGET_NODES_EDGES_PORTS_LABELS,
        ),
    );
}

fn register_port_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        CoreOptions::PORT_ANCHOR,
        LayoutOptionType::Object,
        OptionMeta::visible(
            "Port Anchor Offset",
            "The offset to the port position where connections shall be attached.",
            &TARGET_PORTS,
        )
        .group("port"),
    );
    register_option(
        registry,
        CoreOptions::PORT_INDEX,
        LayoutOptionType::Int,
        OptionMeta::visible(
            "Port Index",
            concat!(
                "The index of a port in the fixed order around a node. The order is assumed as clockwise, ",
                "starting with the leftmost port on the top side. This option must be set if Port Constraints is set to ",
                "FIXED_ORDER and no specific positions are given for the ports. Additionally, the Port Side option must be ",
                "defined in this case."
            ),
            &TARGET_PORTS,
        )
        .group("port"),
    );
    register_enum_option(
        registry,
        CoreOptions::PORT_SIDE,
        PortSide::variants(),
        OptionMeta::visible(
            "Port Side",
            concat!(
                "The side of a node on which a port is situated. This option must be set if Port Constraints is set to ",
                "FIXED_SIDE or FIXED_ORDER and no specific positions are given for the ports."
            ),
            &TARGET_PORTS,
        )
        .group("port"),
    );
    register_option(
        registry,
        CoreOptions::PORT_BORDER_OFFSET,
        LayoutOptionType::Double,
        OptionMeta::visible(
            "Port Border Offset",
            concat!(
                "The offset of ports on the node border. With a positive offset the port is moved outside of the node, ",
                "while with a negative offset the port is moved towards the inside. An offset of 0 means that the port is placed ",
                "directly on the node border."
            ),
            &TARGET_PORTS,
        )
        .group("port"),
    );
}

fn register_port_label_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_enumset_option(
        registry,
        CoreOptions::PORT_LABELS_PLACEMENT,
        PortLabelPlacement::variants(),
        OptionMeta::visible(
            "Port Label Placement",
            "Decides on a placement method for port labels; if empty, the node label's position is not modified.",
            &TARGET_NODES,
        )
        .group("portLabels"),
    );
    register_option(
        registry,
        CoreOptions::PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE,
        LayoutOptionType::Boolean,
        OptionMeta::hidden(
            "Port Labels Next to Port",
            "Use portLabels.placement: NEXT_TO_PORT_IF_POSSIBLE.",
            &TARGET_NODES,
        )
        .group("portLabels"),
    );
    register_option(
        registry,
        CoreOptions::PORT_LABELS_TREAT_AS_GROUP,
        LayoutOptionType::Boolean,
        OptionMeta::visible(
            "Treat Port Labels as Group",
            concat!(
                "If this option is true (default), the labels of a port will be treated as a group when it comes to centering ",
                "them next to their port. If this option is false, only the first label will be centered next to the port, with ",
                "the others being placed below. This only applies to labels of eastern and western ports and will have no effect ",
                "if labels are not placed next to their port."
            ),
            &TARGET_NODES,
        )
        .group("portLabels"),
    );
}

fn register_topdown_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        CoreOptions::TOPDOWN_LAYOUT,
        LayoutOptionType::Boolean,
        OptionMeta::advanced(
            "Topdown Layout",
            concat!(
                "Turns topdown layout on and off. If this option is enabled, hierarchical layout will be computed first for ",
                "the root node and then for its children recursively. Layouts are then scaled down to fit the area provided by ",
                "their parents. Graphs must follow a certain structure for topdown layout to work properly."
            ),
            &TARGET_PARENTS,
        ),
    );
    register_option(
        registry,
        CoreOptions::TOPDOWN_SIZE_CATEGORIES,
        LayoutOptionType::Int,
        OptionMeta::advanced(
            "Number of size categories",
            "Defines the number of categories to use for the FIXED_INTEGER_RATIO_BOXES size approximator.",
            &TARGET_PARENTS,
        )
        .group("topdown")
        .lower_bound(bound_i32(1)),
    );
    register_option(
        registry,
        CoreOptions::TOPDOWN_SIZE_CATEGORIES_HIERARCHICAL_NODE_WEIGHT,
        LayoutOptionType::Int,
        OptionMeta::advanced(
            "Weight of a node containing children for determining the graph size",
            concat!(
                "When determining the graph size for the size categorisation, this value determines how many times a node ",
                "containing children is weighted more than a simple node. For example setting this value to four would result in ",
                "a graph containing a simple node and a hierarchical node to be counted as having a size of five."
            ),
            &TARGET_PARENTS,
        )
        .group("topdown")
        .lower_bound(bound_i32(1)),
    );
    register_option(
        registry,
        CoreOptions::TOPDOWN_SCALE_FACTOR,
        LayoutOptionType::Double,
        OptionMeta::hidden(
            "Topdown Scale Factor",
            concat!(
                "The scaling factor to be applied to the nodes laid out within the node in recursive topdown layout. ",
                "The difference to Scale Factor is that the node itself is not scaled. This value has to be set on hierarchical nodes."
            ),
            &TARGET_PARENTS,
        )
        .group("topdown")
        .lower_bound(bound_exclusive_lower(0.0)),
    );
    register_option(
        registry,
        CoreOptions::TOPDOWN_SIZE_APPROXIMATOR,
        LayoutOptionType::Object,
        OptionMeta::advanced(
            "Topdown Size Approximator",
            concat!(
                "The size approximator to be used to set sizes of hierarchical nodes during topdown layout. The default value ",
                "is null, which results in nodes keeping whatever size is defined for them."
            ),
            &TARGET_NODES,
        )
        .group("topdown"),
    );
    register_option(
        registry,
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH,
        LayoutOptionType::Double,
        OptionMeta::advanced(
            "Topdown Hierarchical Node Width",
            concat!(
                "The fixed size of a hierarchical node when using topdown layout. If this value is set on a parallel node it ",
                "applies to its children, when set on a hierarchical node it applies to the node itself."
            ),
            &TARGET_PARENTS_NODES,
        )
        .group("topdown"),
    );
    register_option(
        registry,
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO,
        LayoutOptionType::Double,
        OptionMeta::advanced(
            "Topdown Hierarchical Node Aspect Ratio",
            concat!(
                "The fixed aspect ratio of a hierarchical node when using topdown layout. Default is 1/sqrt(2). If this value ",
                "is set on a parallel node it applies to its children, when set on a hierarchical node it applies to the node itself."
            ),
            &TARGET_PARENTS_NODES,
        )
        .group("topdown"),
    );
    register_enum_option(
        registry,
        CoreOptions::TOPDOWN_NODE_TYPE,
        topdown_node_variants(),
        OptionMeta::advanced(
            "Topdown Node Type",
            concat!(
                "The different node types used for topdown layout. If the node type is set to PARALLEL_NODE the algorithm must be ",
                "set to a TopdownLayoutProvider such as TopdownPacking."
            ),
            &TARGET_NODES,
        )
        .group("topdown"),
    );
    register_option(
        registry,
        CoreOptions::TOPDOWN_SCALE_CAP,
        LayoutOptionType::Double,
        OptionMeta::advanced(
            "Topdown Scale Cap",
            concat!(
                "Determines the upper limit for the topdown scale factor. The default value is 1.0 which ensures that nested children ",
                "never end up appearing larger than their parents in terms of unit sizes such as the font size. If the limit is larger, ",
                "nodes will fully utilize the available space."
            ),
            &TARGET_PARENTS,
        )
        .group("topdown"),
    );
}

fn register_inside_self_loop_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE,
        LayoutOptionType::Boolean,
        OptionMeta::advanced(
            "Activate Inside Self Loops",
            concat!(
                "Whether this node allows to route self loops inside of it instead of around it. If set to true, this will make ",
                "the node a compound node if it is not already, and will require the layout algorithm to support compound nodes with hierarchical ports."
            ),
            &TARGET_NODES,
        )
        .group("insideSelfLoops"),
    );
    register_option(
        registry,
        CoreOptions::INSIDE_SELF_LOOPS_YO,
        LayoutOptionType::Boolean,
        OptionMeta::advanced(
            "Inside Self Loop",
            "Whether a self loop should be routed inside a node instead of around that node.",
            &TARGET_EDGES,
        )
        .group("insideSelfLoops"),
    );
}

fn register_edge_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        CoreOptions::EDGE_THICKNESS,
        LayoutOptionType::Double,
        OptionMeta::hidden(
            "Edge Thickness",
            "The thickness of an edge. This is a hint on the line width used to draw an edge, possibly requiring more space to be reserved for it.",
            &TARGET_EDGES,
        )
        .group("edge"),
    );
    register_enum_option(
        registry,
        CoreOptions::EDGE_TYPE,
        edge_type_variants(),
        OptionMeta::hidden(
            "Edge Type",
            concat!(
                "The type of an edge. This is usually used for UML class diagrams, where associations must be handled ",
                "differently from generalizations."
            ),
            &TARGET_EDGES,
        )
        .group("edge"),
    );
}

fn register_global_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        CoreOptions::ANIMATE,
        LayoutOptionType::Boolean,
        OptionMeta::visible(
            "Animate",
            "Whether the shift from the old layout to the new computed layout shall be animated.",
            &TARGET_PARENTS,
        )
        .group("global"),
    );
    register_option(
        registry,
        CoreOptions::ANIM_TIME_FACTOR,
        LayoutOptionType::Int,
        OptionMeta::visible(
            "Animation Time Factor",
            concat!(
                "Factor for computation of animation time. The higher the value, the longer the animation time. If the value is 0, ",
                "the resulting time is always equal to the minimum defined by Minimal Animation Time."
            ),
            &TARGET_PARENTS,
        )
        .group("global")
        .lower_bound(bound_i32(0)),
    );
    register_option(
        registry,
        CoreOptions::LAYOUT_ANCESTORS,
        LayoutOptionType::Boolean,
        OptionMeta::visible(
            "Layout Ancestors",
            "Whether the hierarchy levels on the path from the selected element to the root of the diagram shall be included in the layout process.",
            &TARGET_PARENTS,
        )
        .group("global"),
    );
    register_option(
        registry,
        CoreOptions::MAX_ANIM_TIME,
        LayoutOptionType::Int,
        OptionMeta::visible(
            "Maximal Animation Time",
            "The maximal time for animations, in milliseconds.",
            &TARGET_PARENTS,
        )
        .group("global")
        .lower_bound(bound_i32(0)),
    );
    register_option(
        registry,
        CoreOptions::MIN_ANIM_TIME,
        LayoutOptionType::Int,
        OptionMeta::visible(
            "Minimal Animation Time",
            "The minimal time for animations, in milliseconds.",
            &TARGET_PARENTS,
        )
        .group("global")
        .lower_bound(bound_i32(0)),
    );
    register_option(
        registry,
        CoreOptions::PROGRESS_BAR,
        LayoutOptionType::Boolean,
        OptionMeta::visible(
            "Progress Bar",
            "Whether a progress bar shall be displayed during layout computations.",
            &TARGET_PARENTS,
        )
        .group("global"),
    );
    register_option(
        registry,
        CoreOptions::VALIDATE_GRAPH,
        LayoutOptionType::Boolean,
        OptionMeta::visible(
            "Validate Graph",
            concat!(
                "Whether the graph shall be validated before any layout algorithm is applied. If this option is enabled and at least ",
                "one error is found, the layout process is aborted and a message is shown to the user."
            ),
            &TARGET_PARENTS,
        )
        .group("global"),
    );
    register_option(
        registry,
        CoreOptions::VALIDATE_OPTIONS,
        LayoutOptionType::Boolean,
        OptionMeta::visible(
            "Validate Options",
            concat!(
                "Whether layout options shall be validated before any layout algorithm is applied. If this option is enabled and at least ",
                "one error is found, the layout process is aborted and a message is shown to the user."
            ),
            &TARGET_PARENTS,
        )
        .group("global"),
    );
    register_option(
        registry,
        CoreOptions::ZOOM_TO_FIT,
        LayoutOptionType::Boolean,
        OptionMeta::visible(
            "Zoom to Fit",
            "Whether the zoom level shall be set to view the whole diagram after layout.",
            &TARGET_PARENTS,
        )
        .group("global"),
    );
}

fn register_dependencies(registry: &mut dyn LayoutMetaDataRegistry) {
    registry.add_dependency(
        CoreOptions::PARTITIONING_PARTITION.id(),
        CoreOptions::PARTITIONING_ACTIVATE.id(),
        Some(arc_any(true)),
    );
    registry.add_dependency(
        CoreOptions::TOPDOWN_LAYOUT.id(),
        CoreOptions::TOPDOWN_NODE_TYPE.id(),
        None,
    );
    registry.add_dependency(
        CoreOptions::TOPDOWN_SIZE_CATEGORIES.id(),
        CoreOptions::TOPDOWN_SIZE_APPROXIMATOR.id(),
        Some(arc_any(TopdownSizeApproximator::FixedIntegerRatioBoxes)),
    );
    registry.add_dependency(
        CoreOptions::TOPDOWN_SIZE_CATEGORIES_HIERARCHICAL_NODE_WEIGHT.id(),
        CoreOptions::TOPDOWN_SIZE_CATEGORIES.id(),
        None,
    );
    registry.add_dependency(
        CoreOptions::TOPDOWN_SCALE_FACTOR.id(),
        CoreOptions::TOPDOWN_NODE_TYPE.id(),
        Some(arc_any(TopdownNodeTypes::HierarchicalNode)),
    );
    registry.add_dependency(
        CoreOptions::TOPDOWN_SIZE_APPROXIMATOR.id(),
        CoreOptions::TOPDOWN_NODE_TYPE.id(),
        Some(arc_any(TopdownNodeTypes::HierarchicalNode)),
    );
    registry.add_dependency(
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH.id(),
        CoreOptions::TOPDOWN_NODE_TYPE.id(),
        None,
    );
    registry.add_dependency(
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO.id(),
        CoreOptions::TOPDOWN_NODE_TYPE.id(),
        None,
    );
    registry.add_dependency(
        CoreOptions::TOPDOWN_NODE_TYPE.id(),
        CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE.id(),
        None,
    );
    registry.add_dependency(
        CoreOptions::TOPDOWN_SCALE_CAP.id(),
        CoreOptions::TOPDOWN_NODE_TYPE.id(),
        Some(arc_any(TopdownNodeTypes::HierarchicalNode)),
    );
}

fn arc_any<T: Any + Send + Sync>(value: T) -> Arc<dyn Any + Send + Sync> {
    Arc::new(value)
}

fn bound_i32(value: i32) -> Arc<dyn Any + Send + Sync> {
    arc_any(value)
}

fn bound_f64(value: f64) -> Arc<dyn Any + Send + Sync> {
    arc_any(value)
}

fn bound_exclusive_lower(value: f64) -> Arc<dyn Any + Send + Sync> {
    arc_any(ExclusiveBounds::greater_than(value))
}

fn register_option<T: Clone + Send + Sync + 'static>(
    registry: &mut dyn LayoutMetaDataRegistry,
    property: &'static LazyLock<Property<T>>,
    option_type: LayoutOptionType,
    meta: OptionMeta,
) {
    let default_value = property_default_any(property);
    let mut builder = LayoutOptionData::builder()
        .id(property.id())
        .option_type(option_type)
        .default_value(default_value)
        .value_type_id(TypeId::of::<T>());
    if option_type == LayoutOptionType::Object {
        if let Some(parser) = object_parser::<T>() {
            builder = builder.parser(parser);
        }
    }
    builder = apply_meta!(builder, meta);
    registry.register_option(builder.create());
}

fn register_enum_option<T: Copy + Send + Sync + fmt::Debug + 'static>(
    registry: &mut dyn LayoutMetaDataRegistry,
    property: &'static LazyLock<Property<T>>,
    variants: &'static [T],
    meta: OptionMeta,
) {
    let default_value = property_default_any(property);
    let mut builder = LayoutOptionData::builder()
        .id(property.id())
        .option_type(LayoutOptionType::Enum)
        .default_value(default_value)
        .choices(enum_choices(variants))
        .value_type_id(TypeId::of::<T>())
        .parser(enum_parser(variants));
    builder = apply_meta!(builder, meta);
    registry.register_option(builder.create());
}

fn register_enumset_option<T: EnumSetType + Copy + Send + Sync + fmt::Debug + 'static>(
    registry: &mut dyn LayoutMetaDataRegistry,
    property: &'static LazyLock<Property<EnumSet<T>>>,
    variants: &'static [T],
    meta: OptionMeta,
) {
    let default_value = property_default_any(property);
    let mut builder = LayoutOptionData::builder()
        .id(property.id())
        .option_type(LayoutOptionType::EnumSet)
        .default_value(default_value)
        .choices(enum_choices(variants))
        .value_type_id(TypeId::of::<EnumSet<T>>())
        .parser(enumset_parser(variants));
    builder = apply_meta!(builder, meta);
    registry.register_option(builder.create());
}

fn property_default_any<T: Clone + Send + Sync + 'static>(
    property: &'static LazyLock<Property<T>>,
) -> Option<Arc<dyn Any + Send + Sync>> {
    if !property.is_cloneable() {
        return None;
    }
    property
        .get_default()
        .map(|value| Arc::new(value) as Arc<dyn Any + Send + Sync>)
}

fn enum_parser<T: Copy + Send + Sync + fmt::Debug + 'static>(
    variants: &'static [T],
) -> ParserFn {
    Arc::new(move |value| {
        parse_enum_value(value, variants)
            .map(|parsed| Arc::new(parsed) as Arc<dyn Any + Send + Sync>)
    })
}

fn enumset_parser<T: EnumSetType + Copy + Send + Sync + fmt::Debug + 'static>(
    variants: &'static [T],
) -> ParserFn {
    Arc::new(move |value| {
        parse_enumset_value(value, variants)
            .map(|parsed| Arc::new(parsed) as Arc<dyn Any + Send + Sync>)
    })
}

fn object_parser<T: 'static>() -> Option<ParserFn> {
    let type_id = TypeId::of::<T>();
    if type_id == TypeId::of::<KVector>() {
        return Some(Arc::new(|value| {
            parse_kvector_value(value)
                .map(|parsed| Arc::new(parsed) as Arc<dyn Any + Send + Sync>)
        }));
    }
    if type_id == TypeId::of::<KVectorChain>() {
        return Some(Arc::new(|value| {
            parse_kvector_chain_value(value)
                .map(|parsed| Arc::new(parsed) as Arc<dyn Any + Send + Sync>)
        }));
    }
    if type_id == TypeId::of::<ElkPadding>() {
        return Some(Arc::new(|value| {
            parse_spacing_value(value).map(|(top, right, bottom, left)| {
                Arc::new(ElkPadding::with_values(top, right, bottom, left))
                    as Arc<dyn Any + Send + Sync>
            })
        }));
    }
    if type_id == TypeId::of::<ElkMargin>() {
        return Some(Arc::new(|value| {
            parse_spacing_value(value).map(|(top, right, bottom, left)| {
                Arc::new(ElkMargin::with_values(top, right, bottom, left))
                    as Arc<dyn Any + Send + Sync>
            })
        }));
    }
    if type_id == TypeId::of::<IndividualSpacings>() {
        return Some(Arc::new(|value| {
            parse_individual_spacings_value(value)
                .map(|parsed| Arc::new(parsed) as Arc<dyn Any + Send + Sync>)
        }));
    }
    None
}

fn parse_kvector_value(value: &str) -> Option<KVector> {
    let chars: Vec<char> = value.chars().collect();
    let mut start = 0usize;
    while start < chars.len() && is_delim(chars[start], "([{\"' \t\r\n") {
        start += 1;
    }
    let mut end = chars.len();
    while end > 0 && is_delim(chars[end - 1], ")]}\"' \t\r\n") {
        end -= 1;
    }
    if start >= end {
        return None;
    }
    let slice: String = chars[start..end].iter().collect();
    let tokens: Vec<&str> = slice.split(&[',', ';', '\r', '\n'][..]).collect();
    if tokens.len() != 2 {
        return None;
    }
    let x = tokens[0].trim().parse::<f64>().ok()?;
    let y = tokens[1].trim().parse::<f64>().ok()?;
    Some(KVector::with_values(x, y))
}

fn parse_kvector_chain_value(value: &str) -> Option<KVectorChain> {
    let tokens: Vec<&str> = value
        .split(&[',', ';', '(', ')', '[', ']', '{', '}', ' ', '\t', '\n'][..])
        .collect();
    let mut chain = KVectorChain::new();
    let mut xy = 0usize;
    let mut x = 0.0;
    for token in tokens {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let parsed = token.parse::<f64>().ok()?;
        if xy.is_multiple_of(2) {
            x = parsed;
        } else {
            chain.add_values(x, parsed);
        }
        xy += 1;
    }
    Some(chain)
}

fn parse_spacing_value(value: &str) -> Option<(f64, f64, f64, f64)> {
    let chars: Vec<char> = value.chars().collect();
    let mut start = 0usize;
    while start < chars.len() && is_delim(chars[start], "([{\"' \t\r\n") {
        start += 1;
    }
    let mut end = chars.len();
    while end > 0 && is_delim(chars[end - 1], ")]}\"' \t\r\n") {
        end -= 1;
    }
    if start >= end {
        return None;
    }
    let slice: String = chars[start..end].iter().collect();
    let tokens: Vec<&str> = slice.split(&[',', ';'][..]).collect();
    let mut top = 0.0;
    let mut left = 0.0;
    let mut bottom = 0.0;
    let mut right = 0.0;
    for token in tokens {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let mut parts = token.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next()?.trim();
        let parsed = value.parse::<f64>().ok()?;
        match key {
            "top" => top = parsed,
            "left" => left = parsed,
            "bottom" => bottom = parsed,
            "right" => right = parsed,
            _ => {}
        }
    }
    Some((top, right, bottom, left))
}

fn parse_individual_spacings_value(value: &str) -> Option<IndividualSpacings> {
    let mut spacings = IndividualSpacings::new();
    spacings.parse(value).ok()?;
    Some(spacings)
}

fn is_delim(ch: char, delims: &str) -> bool {
    delims.chars().any(|value| value == ch)
}

fn parse_enumset_value<T: EnumSetType + Copy + fmt::Debug>(
    value: &str,
    variants: &'static [T],
) -> Option<EnumSet<T>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Some(EnumSet::none_of());
    }

    let mut set = EnumSet::none_of();
    for token in trimmed.split(|ch: char| ch == '[' || ch == ']' || ch == ',' || ch.is_whitespace()) {
        if token.trim().is_empty() {
            continue;
        }
        let parsed = parse_enum_value(token, variants)?;
        set.insert(parsed);
    }
    Some(set)
}

fn parse_enum_value<T: Copy + fmt::Debug>(value: &str, variants: &'static [T]) -> Option<T> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(index) = trimmed.parse::<usize>() {
        return variants.get(index).copied();
    }
    let normalized = normalize_enum_token(trimmed);
    for &variant in variants {
        if normalize_enum_token(&format!("{:?}", variant)) == normalized {
            return Some(variant);
        }
    }
    None
}

fn normalize_enum_token(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase()
}

fn enum_choices<T: fmt::Debug>(variants: &'static [T]) -> Vec<String> {
    variants
        .iter()
        .map(|variant| to_upper_snake(&format!("{:?}", variant)))
        .collect()
}

fn to_upper_snake(value: &str) -> String {
    let mut out = String::new();
    let mut prev: Option<char> = None;
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        let next = chars.peek().copied();
        if let Some(prev_ch) = prev {
            if ch.is_uppercase()
                && (prev_ch.is_lowercase()
                    || next.map(|n| n.is_lowercase()).unwrap_or(false))
            {
                out.push('_');
            }
        }
        out.push(ch.to_ascii_uppercase());
        prev = Some(ch);
    }
    out
}

fn alignment_variants() -> &'static [Alignment] {
    static VARIANTS: [Alignment; 6] = [
        Alignment::Automatic,
        Alignment::Left,
        Alignment::Right,
        Alignment::Top,
        Alignment::Bottom,
        Alignment::Center,
    ];
    &VARIANTS
}

fn direction_variants() -> &'static [Direction] {
    static VARIANTS: [Direction; 5] = [
        Direction::Undefined,
        Direction::Right,
        Direction::Left,
        Direction::Down,
        Direction::Up,
    ];
    &VARIANTS
}

fn edge_routing_variants() -> &'static [EdgeRouting] {
    static VARIANTS: [EdgeRouting; 4] = [
        EdgeRouting::Undefined,
        EdgeRouting::Polyline,
        EdgeRouting::Orthogonal,
        EdgeRouting::Splines,
    ];
    &VARIANTS
}

fn hierarchy_variants() -> &'static [HierarchyHandling] {
    static VARIANTS: [HierarchyHandling; 3] = [
        HierarchyHandling::Inherit,
        HierarchyHandling::IncludeChildren,
        HierarchyHandling::SeparateChildren,
    ];
    &VARIANTS
}

fn shape_coords_variants() -> &'static [ShapeCoords] {
    static VARIANTS: [ShapeCoords; 3] = [ShapeCoords::Inherit, ShapeCoords::Parent, ShapeCoords::Root];
    &VARIANTS
}

fn edge_coords_variants() -> &'static [EdgeCoords] {
    static VARIANTS: [EdgeCoords; 4] = [
        EdgeCoords::Inherit,
        EdgeCoords::Container,
        EdgeCoords::Parent,
        EdgeCoords::Root,
    ];
    &VARIANTS
}

fn port_alignment_variants() -> &'static [PortAlignment] {
    static VARIANTS: [PortAlignment; 5] = [
        PortAlignment::Distributed,
        PortAlignment::Justified,
        PortAlignment::Begin,
        PortAlignment::Center,
        PortAlignment::End,
    ];
    &VARIANTS
}

fn port_constraints_variants() -> &'static [PortConstraints] {
    static VARIANTS: [PortConstraints; 6] = [
        PortConstraints::Undefined,
        PortConstraints::Free,
        PortConstraints::FixedSide,
        PortConstraints::FixedOrder,
        PortConstraints::FixedRatio,
        PortConstraints::FixedPos,
    ];
    &VARIANTS
}

fn edge_label_placement_variants() -> &'static [EdgeLabelPlacement] {
    static VARIANTS: [EdgeLabelPlacement; 3] = [
        EdgeLabelPlacement::Center,
        EdgeLabelPlacement::Head,
        EdgeLabelPlacement::Tail,
    ];
    &VARIANTS
}

fn edge_type_variants() -> &'static [EdgeType] {
    static VARIANTS: [EdgeType; 6] = [
        EdgeType::None,
        EdgeType::Directed,
        EdgeType::Undirected,
        EdgeType::Association,
        EdgeType::Generalization,
        EdgeType::Dependency,
    ];
    &VARIANTS
}

fn topdown_node_variants() -> &'static [TopdownNodeTypes] {
    static VARIANTS: [TopdownNodeTypes; 3] = [
        TopdownNodeTypes::ParallelNode,
        TopdownNodeTypes::HierarchicalNode,
        TopdownNodeTypes::RootNode,
    ];
    &VARIANTS
}

fn packing_mode_variants() -> &'static [PackingMode] {
    static VARIANTS: [PackingMode; 4] = [
        PackingMode::Simple,
        PackingMode::GroupDec,
        PackingMode::GroupMixed,
        PackingMode::GroupInc,
    ];
    &VARIANTS
}

fn register_categories(registry: &mut dyn LayoutMetaDataRegistry) {
    registry.register_category(
        LayoutCategoryData::builder()
            .id("org.eclipse.elk.layered")
            .name("Layered")
            .description(concat!(
                "The layer-based method was introduced by Sugiyama, Tagawa and Toda in 1981. ",
                "It emphasizes the direction of edges by pointing as many edges as possible into the same direction. ",
                "The nodes are arranged in layers, which are sometimes called \"hierarchies\", and then reordered ",
                "such that the number of edge crossings is minimized. Afterwards, concrete coordinates are computed ",
                "for the nodes and edge bend points."
            ))
            .create(),
    );
    registry.register_category(
        LayoutCategoryData::builder()
            .id("org.eclipse.elk.orthogonal")
            .name("Orthogonal")
            .description(concat!(
                "Orthogonal methods that follow the \"topology-shape-metrics\" approach by Batini, Nardelli and ",
                "Tamassia '86. The first phase determines the topology of the drawing by applying a planarization ",
                "technique, which results in a planar representation of the graph. The orthogonal shape is computed ",
                "in the second phase, which aims at minimizing the number of edge bends, and is called ",
                "orthogonalization. The third phase leads to concrete coordinates for nodes and edge bend points by ",
                "applying a compaction method, thus defining the metrics."
            ))
            .create(),
    );
    registry.register_category(
        LayoutCategoryData::builder()
            .id("org.eclipse.elk.force")
            .name("Force")
            .description(concat!(
                "Layout algorithms that follow physical analogies by simulating a system of attractive and ",
                "repulsive forces. The first successful method of this kind was proposed by Eades in 1984."
            ))
            .create(),
    );
    registry.register_category(
        LayoutCategoryData::builder()
            .id("org.eclipse.elk.circle")
            .name("Circle")
            .description(concat!(
                "Circular layout algorithms emphasize cycles or biconnected components of a graph by arranging ",
                "them in circles. This is useful if a drawing is desired where such components are clearly grouped, ",
                "or where cycles are shown as prominent options of the graph."
            ))
            .create(),
    );
    registry.register_category(
        LayoutCategoryData::builder()
            .id("org.eclipse.elk.tree")
            .name("Tree")
            .description(concat!(
                "Specialized layout methods for trees, i.e. acyclic graphs. The regular structure of graphs that ",
                "have no undirected cycles can be emphasized using an algorithm of this type."
            ))
            .create(),
    );
    registry.register_category(
        LayoutCategoryData::builder()
            .id("org.eclipse.elk.planar")
            .name("Planar")
            .description(concat!(
                "Algorithms that require a planar or upward planar graph. Most of these algorithms are ",
                "theoretically interesting, but not practically usable."
            ))
            .create(),
    );
    registry.register_category(
        LayoutCategoryData::builder()
            .id("org.eclipse.elk.radial")
            .name("Radial")
            .description(
                "Radial layout algorithms usually position the nodes of the graph on concentric circles.",
            )
            .create(),
    );
}
