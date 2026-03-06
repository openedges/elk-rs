use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::fmt;
use std::sync::{Arc, LazyLock, OnceLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutMetaDataRegistry, LayoutOptionData, LayoutOptionTarget,
    LayoutOptionType, LayoutOptionVisibility,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_alignment::PortAlignment;
use org_eclipse_elk_core::org::eclipse::elk::core::options::topdown_node_types::TopdownNodeTypes;
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;

use super::{
    CenterEdgeLabelPlacementStrategy, ConstraintCalculationStrategy, CrossingMinimizationStrategy,
    CuttingStrategy, CycleBreakingStrategy, DirectionCongruency, EdgeLabelSideSelection,
    EdgeStraighteningStrategy, FixedAlignment, GraphCompactionStrategy, GreedySwitchType,
    GroupOrderStrategy, InLayerConstraint, InteractiveReferencePoint, LayerConstraint,
    LayerUnzippingStrategy, LayeredOptions, LayeringStrategy, LongEdgeOrderingStrategy,
    NodeFlexibility, NodePlacementStrategy, NodePromotionStrategy, OrderingStrategy,
    PortSortingStrategy, SelfLoopDistributionStrategy, SelfLoopOrderingStrategy, SplineRoutingMode,
    ValidifyStrategy, WrappingStrategy,
};
use crate::org::eclipse::elk::alg::layered::components::ComponentOrderingStrategy;

type ParserFn = Arc<dyn Fn(&str) -> Option<Arc<dyn Any + Send + Sync>> + Send + Sync>;

pub struct LayeredMetaDataProvider;

impl ILayoutMetaDataProvider for LayeredMetaDataProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        init_reflect();
        register_spacing_options(registry);
        register_priority_options(registry);
        register_wrapping_options(registry);
        register_layer_unzipping_options(registry);
        register_cycle_breaking_options(registry);
        register_layering_options(registry);
        register_crossing_minimization_options(registry);
        register_node_placement_options(registry);
        register_edge_routing_options(registry);
        register_compaction_options(registry);
        register_high_degree_options(registry);
        register_misc_options(registry);
        register_edge_label_options(registry);
        register_consider_model_order_options(registry);
        register_option_supports(registry);
    }
}

const TARGET_PARENTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Parents];
const TARGET_EDGES: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Edges];
const TARGET_NODES: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Nodes];
const TARGET_PORTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Ports];
const TARGET_PARENTS_LABELS: [LayoutOptionTarget; 2] =
    [LayoutOptionTarget::Parents, LayoutOptionTarget::Labels];
const TARGET_NODES_EDGES_PORTS: [LayoutOptionTarget; 3] = [
    LayoutOptionTarget::Nodes,
    LayoutOptionTarget::Edges,
    LayoutOptionTarget::Ports,
];

fn register_spacing_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        LayeredOptions::SPACING_BASE_VALUE,
        LayoutOptionType::Double,
        "Spacing Base Value",
        concat!(
            "An optional base value for all other layout options of the 'spacing' group. It can be used to conveniently ",
            "alter the overall 'spaciousness' of the drawing. Whenever an explicit value is set for the other layout ",
            "options, this base value will have no effect. The base value is not inherited, i.e. it must be set for ",
            "each hierarchical node."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("spacing"),
        Some(bound_f64(0.0)),
    );

    register_option(
        registry,
        LayeredOptions::SPACING_EDGE_NODE_BETWEEN_LAYERS,
        LayoutOptionType::Double,
        "Edge Node Between Layers Spacing",
        concat!(
            "The spacing to be preserved between nodes and edges that are routed next to the node's layer. ",
            "For the spacing between nodes and edges that cross the node's layer 'spacing.edgeNode' is used."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("spacing"),
        Some(bound_f64(0.0)),
    );

    register_option(
        registry,
        LayeredOptions::SPACING_EDGE_EDGE_BETWEEN_LAYERS,
        LayoutOptionType::Double,
        "Edge Edge Between Layer Spacing",
        concat!(
            "Spacing to be preserved between pairs of edges that are routed between the same pair of layers. ",
            "Note that 'spacing.edgeEdge' is used for the spacing between pairs of edges crossing the same layer."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("spacing"),
        Some(bound_f64(0.0)),
    );

    register_option(
        registry,
        LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS,
        LayoutOptionType::Double,
        "Node Node Between Layers Spacing",
        concat!(
            "The spacing to be preserved between any pair of nodes of two adjacent layers. ",
            "Note that 'spacing.nodeNode' is used for the spacing between nodes within the layer itself."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("spacing"),
        Some(bound_f64(0.0)),
    );
}

fn register_priority_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        LayeredOptions::PRIORITY_DIRECTION,
        LayoutOptionType::Int,
        "Direction Priority",
        concat!(
            "Defines how important it is to have a certain edge point into the direction of the overall layout. ",
            "This option is evaluated during the cycle breaking phase."
        ),
        &TARGET_EDGES,
        LayoutOptionVisibility::Advanced,
        Some("priority"),
        Some(bound_i32(0)),
    );

    register_option(
        registry,
        LayeredOptions::PRIORITY_SHORTNESS,
        LayoutOptionType::Int,
        "Shortness Priority",
        concat!(
            "Defines how important it is to keep an edge as short as possible. ",
            "This option is evaluated during the layering phase."
        ),
        &TARGET_EDGES,
        LayoutOptionVisibility::Advanced,
        Some("priority"),
        Some(bound_i32(0)),
    );

    register_option(
        registry,
        LayeredOptions::PRIORITY_STRAIGHTNESS,
        LayoutOptionType::Int,
        "Straightness Priority",
        concat!(
            "Defines how important it is to keep an edge straight, i.e. aligned with one of the two axes. ",
            "This option is evaluated during node placement."
        ),
        &TARGET_EDGES,
        LayoutOptionVisibility::Advanced,
        Some("priority"),
        Some(bound_i32(0)),
    );
}

fn register_wrapping_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        LayeredOptions::WRAPPING_STRATEGY,
        LayoutOptionType::Enum,
        "Graph Wrapping Strategy",
        concat!(
            "For certain graphs and certain prescribed drawing areas it may be desirable to ",
            "split the laid out graph into chunks that are placed side by side. ",
            "The edges that connect different chunks are 'wrapped' around from the end of ",
            "one chunk to the start of the other chunk. ",
            "The points between the chunks are referred to as 'cuts'."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("wrapping"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::WRAPPING_ADDITIONAL_EDGE_SPACING,
        LayoutOptionType::Double,
        "Additional Wrapped Edges Spacing",
        concat!(
            "To visually separate edges that are wrapped from regularly routed edges an additional spacing value ",
            "can be specified in form of this layout option. The spacing is added to the regular edgeNode spacing."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("wrapping"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::WRAPPING_CORRECTION_FACTOR,
        LayoutOptionType::Double,
        "Correction Factor for Wrapping",
        concat!(
            "At times and for certain types of graphs the executed wrapping may produce results that ",
            "are consistently biased in the same fashion: either wrapping to often or to rarely. ",
            "This factor can be used to correct the bias. Internally, it is simply multiplied with ",
            "the 'aspect ratio' layout option."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("wrapping"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::WRAPPING_CUTTING_STRATEGY,
        LayoutOptionType::Enum,
        "Cutting Strategy",
        "The strategy by which the layer indexes are determined at which the layering crumbles into chunks.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("wrapping.cutting"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::WRAPPING_CUTTING_CUTS,
        LayoutOptionType::Object,
        "Manually Specified Cuts",
        "Allows the user to specify her own cuts for a certain graph.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("wrapping.cutting"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::WRAPPING_CUTTING_MSD_FREEDOM,
        LayoutOptionType::Int,
        "MSD Freedom",
        concat!(
            "The MSD cutting strategy starts with an initial guess on the number of chunks the graph ",
            "should be split into. The freedom specifies how much the strategy may deviate from this guess. ",
            "E.g. if an initial number of 3 is computed, a freedom of 1 allows 2, 3, and 4 cuts."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("wrapping.cutting.msd"),
        Some(bound_i32(0)),
    );

    register_option(
        registry,
        LayeredOptions::WRAPPING_VALIDIFY_STRATEGY,
        LayoutOptionType::Enum,
        "Validification Strategy",
        concat!(
            "When wrapping graphs, one can specify indices that are not allowed as split points. ",
            "The validification strategy makes sure every computed split point is allowed."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("wrapping.validify"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::WRAPPING_VALIDIFY_FORBIDDEN_INDICES,
        LayoutOptionType::Object,
        "Valid Indices for Wrapping",
        "",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("wrapping.validify"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::WRAPPING_MULTI_EDGE_IMPROVE_CUTS,
        LayoutOptionType::Boolean,
        "Improve Cuts",
        concat!(
            "For general graphs it is important that not too many edges wrap backwards. ",
            "Thus a compromise between evenly-distributed cuts and the total number of cut edges is sought."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("wrapping.multiEdge"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::WRAPPING_MULTI_EDGE_DISTANCE_PENALTY,
        LayoutOptionType::Double,
        "Distance Penalty When Improving Cuts ",
        "",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("wrapping.multiEdge"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::WRAPPING_MULTI_EDGE_IMPROVE_WRAPPED_EDGES,
        LayoutOptionType::Boolean,
        "Improve Wrapped Edges",
        concat!(
            "The initial wrapping is performed in a very simple way. As a consequence, edges that wrap from ",
            "one chunk to another may be unnecessarily long. Activating this option tries to shorten such edges."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("wrapping.multiEdge"),
        None,
    );
}

fn register_layer_unzipping_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        LayeredOptions::LAYER_UNZIPPING_STRATEGY,
        LayoutOptionType::Enum,
        "Layer Unzipping Strategy",
        concat!(
            "The strategy to use for unzipping a layer into multiple sublayers while maintaining ",
            "the existing ordering of nodes and edges after crossing minimization. The default ",
            "value is 'NONE'."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("layerUnzipping"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::LAYER_UNZIPPING_MINIMIZE_EDGE_LENGTH,
        LayoutOptionType::Boolean,
        "Minimize Edge Length Heuristic",
        concat!(
            "Use a heuristic to decide whether or not to actually perform the layer split with the goal of ",
            "minimizing the total edge length. This option only works when layerSplit is set to 2. ",
            "The property can be set to the nodes in a layer, ",
            "which then applies the property for the layer. If any node sets the value to true, then the value is ",
            "set to true for the entire layer."
        ),
        &TARGET_NODES,
        LayoutOptionVisibility::Visible,
        Some("layerUnzipping"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::LAYER_UNZIPPING_LAYER_SPLIT,
        LayoutOptionType::Int,
        "Unzipping Layer Split",
        concat!(
            "Defines the number of sublayers to split a layer into. The property can be set to the nodes in a layer, ",
            "which then applies the property for the layer. If multiple nodes set the value to different values, ",
            "then the lowest value is chosen."
        ),
        &TARGET_NODES,
        LayoutOptionVisibility::Advanced,
        Some("layerUnzipping"),
        Some(bound_i32(1)),
    );

    register_option(
        registry,
        LayeredOptions::LAYER_UNZIPPING_RESET_ON_LONG_EDGES,
        LayoutOptionType::Boolean,
        "Reset Alternation on Long Edges",
        concat!(
            "If set to true, nodes will always be placed in the first sublayer after a long edge when using the ",
            "ALTERNATING strategy. ",
            "Otherwise long edge dummies are treated the same as regular nodes. The default value is true. ",
            "The property can be set to the nodes in a layer, which then applies the property ",
            "for the layer. If any node sets the value to false, then the value is set to false for the entire ",
            "layer."
        ),
        &TARGET_NODES,
        LayoutOptionVisibility::Visible,
        Some("layerUnzipping"),
        None,
    );
}

fn register_cycle_breaking_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        LayeredOptions::CYCLE_BREAKING_STRATEGY,
        LayoutOptionType::Enum,
        "Cycle Breaking Strategy",
        concat!(
            "Strategy for cycle breaking. Cycle breaking looks for cycles in the graph and determines ",
            "which edges to reverse to break the cycles."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("cycleBreaking"),
        None,
    );
}

fn register_layering_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        LayeredOptions::LAYERING_STRATEGY,
        LayoutOptionType::Enum,
        "Node Layering Strategy",
        "Strategy for node layering.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("layering"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::LAYERING_LAYER_CONSTRAINT,
        LayoutOptionType::Enum,
        "Layer Constraint",
        "Determines a constraint on the placement of the node regarding the layering.",
        &TARGET_NODES,
        LayoutOptionVisibility::Advanced,
        Some("layering"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::LAYERING_LAYER_CHOICE_CONSTRAINT,
        LayoutOptionType::Int,
        "Layer Choice Constraint",
        "Allows to set a constraint regarding the layer placement of a node.",
        &TARGET_NODES,
        LayoutOptionVisibility::Advanced,
        Some("layering"),
        Some(bound_i32(-1)),
    );

    register_option(
        registry,
        LayeredOptions::LAYERING_LAYER_ID,
        LayoutOptionType::Int,
        "Layer ID",
        "Layer identifier that was calculated by ELK Layered for a node.",
        &TARGET_NODES,
        LayoutOptionVisibility::Advanced,
        Some("layering"),
        Some(bound_i32(-1)),
    );

    register_option(
        registry,
        LayeredOptions::LAYERING_MIN_WIDTH_UPPER_BOUND_ON_WIDTH,
        LayoutOptionType::Int,
        "Upper Bound On Width [MinWidth Layerer]",
        concat!(
            "Defines a loose upper bound on the width of the MinWidth layerer. ",
            "If set to -1 multiple values are tested and the best result is selected."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("layering.minWidth"),
        Some(bound_i32(-1)),
    );

    register_option(
        registry,
        LayeredOptions::LAYERING_MIN_WIDTH_UPPER_LAYER_ESTIMATION_SCALING_FACTOR,
        LayoutOptionType::Int,
        "Upper Layer Estimation Scaling Factor [MinWidth Layerer]",
        concat!(
            "Multiplied with Upper Bound On Width for defining an upper bound on the width of layers which ",
            "haven't been determined yet."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("layering.minWidth"),
        Some(bound_i32(-1)),
    );

    register_option(
        registry,
        LayeredOptions::LAYERING_NODE_PROMOTION_STRATEGY,
        LayoutOptionType::Enum,
        "Node Promotion Strategy",
        "Reduces number of dummy nodes after layering phase (if possible).",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("layering.nodePromotion"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::LAYERING_NODE_PROMOTION_MAX_ITERATIONS,
        LayoutOptionType::Int,
        "Max Node Promotion Iterations",
        "Limits the number of iterations for node promotion.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("layering.nodePromotion"),
        Some(bound_i32(0)),
    );

    register_option(
        registry,
        LayeredOptions::LAYERING_COFFMAN_GRAHAM_LAYER_BOUND,
        LayoutOptionType::Int,
        "Layer Bound",
        "The maximum number of nodes allowed per layer.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("layering.coffmanGraham"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::LAYERING_IGNORE_EDGE_IN_LAYER,
        LayoutOptionType::Boolean,
        "Ignore Edge In Layer",
        "Whether this edge should be ignored during layer assignment.",
        &TARGET_EDGES,
        LayoutOptionVisibility::Advanced,
        Some("layering"),
        None,
    );
}

fn register_crossing_minimization_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        LayeredOptions::CROSSING_MINIMIZATION_STRATEGY,
        LayoutOptionType::Enum,
        "Crossing Minimization Strategy",
        "Strategy for crossing minimization.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("crossingMinimization"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::CROSSING_MINIMIZATION_FORCE_NODE_MODEL_ORDER,
        LayoutOptionType::Boolean,
        "Force Node Model Order",
        concat!(
            "The node order given by the model does not change to produce a better layout. ",
            "This assumes that the node model order is already respected before crossing minimization."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("crossingMinimization"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS,
        LayoutOptionType::Double,
        "Hierarchical Sweepiness",
        "How likely it is to use cross-hierarchy (1) vs bottom-up (-1).",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("crossingMinimization"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_ACTIVATION_THRESHOLD,
        LayoutOptionType::Int,
        "Greedy Switch Activation Threshold",
        concat!(
            "By default it is decided automatically if the greedy switch is activated or not. ",
            "A value of 0 enforces the activation."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("crossingMinimization.greedySwitch"),
        Some(bound_i32(0)),
    );

    register_option(
        registry,
        LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE,
        LayoutOptionType::Enum,
        "Greedy Switch Crossing Minimization",
        "Greedy switch strategy executed after regular crossing minimization.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("crossingMinimization.greedySwitch"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE,
        LayoutOptionType::Enum,
        "Greedy Switch Crossing Minimization (Hierarchical)",
        "Greedy switch strategy in case hierarchical layout is used.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("crossingMinimization.greedySwitchHierarchical"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::CROSSING_MINIMIZATION_SEMI_INTERACTIVE,
        LayoutOptionType::Boolean,
        "Semi-Interactive Crossing Minimization",
        concat!(
            "Preserves the order of nodes within a layer but still minimizes crossings between edges connecting ",
            "long edge dummies."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("crossingMinimization"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::CROSSING_MINIMIZATION_IN_LAYER_PRED_OF,
        LayoutOptionType::String,
        "In Layer Predecessor Of",
        "Specifies of which node the current node is the predecessor in the same layer.",
        &TARGET_NODES,
        LayoutOptionVisibility::Visible,
        Some("crossingMinimization"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::CROSSING_MINIMIZATION_IN_LAYER_SUCC_OF,
        LayoutOptionType::String,
        "In Layer Successor Of",
        "Specifies of which node the current node is the successor in the same layer.",
        &TARGET_NODES,
        LayoutOptionVisibility::Advanced,
        Some("crossingMinimization"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::CROSSING_MINIMIZATION_POSITION_CHOICE_CONSTRAINT,
        LayoutOptionType::Int,
        "Position Choice Constraint",
        "Allows to set a constraint regarding the position placement of a node in a layer.",
        &TARGET_NODES,
        LayoutOptionVisibility::Advanced,
        Some("crossingMinimization"),
        Some(bound_i32(-1)),
    );

    register_option(
        registry,
        LayeredOptions::CROSSING_MINIMIZATION_POSITION_ID,
        LayoutOptionType::Int,
        "Position ID",
        "Position within a layer that was determined by ELK Layered for a node.",
        &TARGET_NODES,
        LayoutOptionVisibility::Advanced,
        Some("crossingMinimization"),
        Some(bound_i32(-1)),
    );
}

fn register_node_placement_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        LayeredOptions::NODE_PLACEMENT_STRATEGY,
        LayoutOptionType::Enum,
        "Node Placement Strategy",
        "Strategy for node placement.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("nodePlacement"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES,
        LayoutOptionType::Boolean,
        "Favor Straight Edges Over Balancing",
        concat!(
            "Favor straight edges over a balanced node placement. The default behavior is determined ",
            "automatically based on the used edge routing."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("nodePlacement"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::NODE_PLACEMENT_BK_EDGE_STRAIGHTENING,
        LayoutOptionType::Enum,
        "BK Edge Straightening",
        "Specifies whether the Brandes-Koepf node placer tries to increase the number of straight edges.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("nodePlacement.bk"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::NODE_PLACEMENT_BK_FIXED_ALIGNMENT,
        LayoutOptionType::Enum,
        "BK Fixed Alignment",
        "Tells the BK node placer to use a certain alignment.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("nodePlacement.bk"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::NODE_PLACEMENT_LINEAR_SEGMENTS_DEFLECTION_DAMPENING,
        LayoutOptionType::Double,
        "Linear Segments Deflection Dampening",
        "Dampens the movement of nodes to keep the diagram from getting too large.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("nodePlacement.linearSegments"),
        Some(bound_f64(0.0)),
    );

    register_option(
        registry,
        LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY,
        LayoutOptionType::Enum,
        "Node Flexibility",
        concat!(
            "Aims at shorter and straighter edges by allowing ports or node sizes to change ",
            "during network simplex node placement."
        ),
        &TARGET_NODES,
        LayoutOptionVisibility::Advanced,
        Some("nodePlacement.networkSimplex"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT,
        LayoutOptionType::Enum,
        "Node Flexibility Default",
        "Default value of the node flexibility option for the children of a hierarchical node.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("nodePlacement.networkSimplex.nodeFlexibility"),
        None,
    );
}

fn register_edge_routing_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        LayeredOptions::EDGE_ROUTING_SPLINES_MODE,
        LayoutOptionType::Enum,
        "Spline Routing Mode",
        "Specifies the way control points are assembled for each individual edge.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("edgeRouting.splines"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::EDGE_ROUTING_SPLINES_SLOPPY_LAYER_SPACING_FACTOR,
        LayoutOptionType::Double,
        "Sloppy Spline Layer Spacing Factor",
        "Spacing factor for routing area between layers when using sloppy spline routing.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("edgeRouting.splines.sloppy"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::EDGE_ROUTING_POLYLINE_SLOPED_EDGE_ZONE_WIDTH,
        LayoutOptionType::Double,
        "Sloped Edge Zone Width",
        concat!(
            "Width of the strip to the left and to the right of each layer where the polyline edge router ",
            "is allowed to refrain from ensuring that edges are routed horizontally."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("edgeRouting.polyline"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::EDGE_ROUTING_SELF_LOOP_DISTRIBUTION,
        LayoutOptionType::Enum,
        "Self-Loop Distribution",
        "Alter the distribution of the loops around the node.",
        &TARGET_NODES,
        LayoutOptionVisibility::Visible,
        Some("edgeRouting"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::EDGE_ROUTING_SELF_LOOP_ORDERING,
        LayoutOptionType::Enum,
        "Self-Loop Ordering",
        "Alter the ordering of the loops.",
        &TARGET_NODES,
        LayoutOptionVisibility::Visible,
        Some("edgeRouting"),
        None,
    );
}

fn register_compaction_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        LayeredOptions::COMPACTION_POST_COMPACTION_STRATEGY,
        LayoutOptionType::Enum,
        "Post Compaction Strategy",
        "Specifies whether and how post-process compaction is applied.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("compaction.postCompaction"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::COMPACTION_POST_COMPACTION_CONSTRAINTS,
        LayoutOptionType::Enum,
        "Post Compaction Constraint Calculation",
        "Specifies how post-process compaction constraints are calculated.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("compaction.postCompaction"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::COMPACTION_CONNECTED_COMPONENTS,
        LayoutOptionType::Boolean,
        "Connected Components Compaction",
        "Tries to further compact components (disconnected sub-graphs).",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("compaction"),
        None,
    );
}

fn register_high_degree_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        LayeredOptions::HIGH_DEGREE_NODES_TREATMENT,
        LayoutOptionType::Boolean,
        "High Degree Node Treatment",
        "Makes room around high degree nodes to place leafs and trees.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("highDegreeNodes"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::HIGH_DEGREE_NODES_THRESHOLD,
        LayoutOptionType::Int,
        "High Degree Node Threshold",
        "Whether a node is considered to have a high degree.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("highDegreeNodes"),
        Some(bound_i32(0)),
    );

    register_option(
        registry,
        LayeredOptions::HIGH_DEGREE_NODES_TREE_HEIGHT,
        LayoutOptionType::Int,
        "High Degree Node Maximum Tree Height",
        "Maximum height of a subtree connected to a high degree node to be moved to separate layers.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        Some("highDegreeNodes"),
        Some(bound_i32(0)),
    );
}

fn register_misc_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        LayeredOptions::DIRECTION_CONGRUENCY,
        LayoutOptionType::Enum,
        "Direction Congruency",
        concat!(
            "Specifies how drawings of the same graph with different layout directions compare to each other: ",
            "either a natural reading direction is preserved or the drawings are rotated versions of each other."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LayeredOptions::FEEDBACK_EDGES,
        LayoutOptionType::Boolean,
        "Feedback Edges",
        "Whether feedback edges should be highlighted by routing around the nodes.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LayeredOptions::INTERACTIVE_REFERENCE_POINT,
        LayoutOptionType::Enum,
        "Interactive Reference Point",
        "Determines which point of a node is considered by interactive layout phases.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LayeredOptions::MERGE_EDGES,
        LayoutOptionType::Boolean,
        "Merge Edges",
        "Edges that have no ports are merged so they touch the connected nodes at the same points.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LayeredOptions::MERGE_HIERARCHY_EDGES,
        LayoutOptionType::Boolean,
        "Merge Hierarchy-Crossing Edges",
        "If hierarchical layout is active, hierarchy-crossing edges use as few hierarchical ports as possible.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LayeredOptions::ALLOW_NON_FLOW_PORTS_TO_SWITCH_SIDES,
        LayoutOptionType::Boolean,
        "Allow Non-Flow Ports To Switch Sides",
        "Specifies whether non-flow ports may switch sides for FIXED_SIDE or FIXED_ORDER constraints.",
        &TARGET_PORTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LayeredOptions::PORT_SORTING_STRATEGY,
        LayoutOptionType::Enum,
        "Port Sorting Strategy",
        "Determines the way a node's ports are distributed on the sides of a node if their order is not prescribed.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LayeredOptions::THOROUGHNESS,
        LayoutOptionType::Int,
        "Thoroughness",
        "How much effort should be spent to produce a nice layout.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        Some(bound_i32(1)),
    );

    register_option(
        registry,
        LayeredOptions::UNNECESSARY_BENDPOINTS,
        LayoutOptionType::Boolean,
        "Add Unnecessary Bendpoints",
        "Adds bend points even if an edge does not change direction.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LayeredOptions::GENERATE_POSITION_AND_LAYER_IDS,
        LayoutOptionType::Boolean,
        "Generate Position and Layer IDs",
        concat!(
            "If enabled position id and layer id are generated, which are usually only used internally ",
            "when setting the interactive layout option."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );
}

fn register_edge_label_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        LayeredOptions::EDGE_LABELS_SIDE_SELECTION,
        LayoutOptionType::Enum,
        "Edge Label Side Selection",
        "Method to decide on edge label sides.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("edgeLabels"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY,
        LayoutOptionType::Enum,
        "Edge Center Label Placement Strategy",
        "Determines in which layer center labels of long edges should be placed.",
        &TARGET_PARENTS_LABELS,
        LayoutOptionVisibility::Advanced,
        Some("edgeLabels"),
        None,
    );
}

fn register_consider_model_order_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY,
        LayoutOptionType::Enum,
        "Consider Model Order",
        concat!(
            "Preserves the order of nodes and edges in the model file if this does not lead to additional edge ",
            "crossings. Depending on the strategy this is not always possible since the node and edge order might be ",
            "conflicting."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("considerModelOrder"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER,
        LayoutOptionType::Boolean,
        "Consider Port Order",
        concat!(
            "If disabled the port order of output ports is derived from the edge order and input ports are ordered by ",
            "their incoming connections. If enabled all ports are ordered by the port model order."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("considerModelOrder"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::CONSIDER_MODEL_ORDER_NO_MODEL_ORDER,
        LayoutOptionType::Boolean,
        "No Model Order",
        "Set on a node to not set a model order for this node even though it is a real node.",
        &TARGET_NODES,
        LayoutOptionVisibility::Visible,
        Some("considerModelOrder"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::CONSIDER_MODEL_ORDER_COMPONENTS,
        LayoutOptionType::Enum,
        "Consider Model Order for Components",
        concat!(
            "If set to NONE the usual ordering strategy (by cumulative node priority and size of nodes) is used. ",
            "INSIDE_PORT_SIDES orders the components with external ports only inside the groups with the same port side. ",
            "FORCE_MODEL_ORDER enforces the mode order on components. This option might produce bad alignments and sub ",
            "optimal drawings in terms of used area since the ordering should be respected."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("considerModelOrder"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::CONSIDER_MODEL_ORDER_LONG_EDGE_STRATEGY,
        LayoutOptionType::Enum,
        "Long Edge Ordering Strategy",
        concat!(
            "Indicates whether long edges are sorted under, over, or equal to nodes that have no connection to a ",
            "previous layer in a left-to-right or right-to-left layout. Under and over changes to right and left in a ",
            "vertical layout."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("considerModelOrder"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_NODE_INFLUENCE,
        LayoutOptionType::Double,
        "Crossing Counter Node Order Influence",
        concat!(
            "Indicates with what percentage (1 for 100%) violations of the node model order are weighted against the ",
            "crossings e.g. a value of 0.5 means two model order violations are as important as on edge crossing. ",
            "This allows some edge crossings in favor of preserving the model order. It is advised to set this value to ",
            "a very small positive value (e.g. 0.001) to have minimal crossing and a optimal node order. Defaults to no ",
            "influence (0)."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("considerModelOrder"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_PORT_INFLUENCE,
        LayoutOptionType::Double,
        "Crossing Counter Port Order Influence",
        concat!(
            "Indicates with what percentage (1 for 100%) violations of the port model order are weighted against the ",
            "crossings e.g. a value of 0.5 means two model order violations are as important as on edge crossing. ",
            "This allows some edge crossings in favor of preserving the model order. It is advised to set this value to ",
            "a very small positive value (e.g. 0.001) to have minimal crossing and a optimal port order. Defaults to no ",
            "influence (0)."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("considerModelOrder"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::GROUP_MODEL_ORDER_CYCLE_BREAKING_ID,
        LayoutOptionType::Int,
        "Group ID of the Node Type",
        concat!(
            "Used to define partial ordering groups during cycle breaking. A lower group id means that the group is ",
            "sorted before other groups. A group model order of 0 is the default group."
        ),
        &TARGET_NODES,
        LayoutOptionVisibility::Visible,
        Some("considerModelOrder.groupModelOrder"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::GROUP_MODEL_ORDER_CROSSING_MINIMIZATION_ID,
        LayoutOptionType::Int,
        "Group ID of the Node Type",
        concat!(
            "Used to define partial ordering groups during crossing minimization. A lower group id means that the group is ",
            "sorted before other groups. A group model order of 0 is the default group."
        ),
        &TARGET_NODES_EDGES_PORTS,
        LayoutOptionVisibility::Visible,
        Some("considerModelOrder.groupModelOrder"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::GROUP_MODEL_ORDER_COMPONENT_GROUP_ID,
        LayoutOptionType::Int,
        "Group ID of the Node Type",
        concat!(
            "Used to define partial ordering groups during component packing. A lower group id means that the group is ",
            "sorted before other groups. A group model order of 0 is the default group."
        ),
        &TARGET_NODES_EDGES_PORTS,
        LayoutOptionVisibility::Visible,
        Some("considerModelOrder.groupModelOrder"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::GROUP_MODEL_ORDER_CB_GROUP_ORDER_STRATEGY,
        LayoutOptionType::Enum,
        "Cycle Breaking Group Ordering Strategy",
        concat!(
            "Determines how to count ordering violations during cycle breaking. NONE: They do not count. ENFORCED: ",
            "A group with a higher model order is before a node with a smaller. MODEL_ORDER: The model order counts ",
            "instead of the model order group id ordering."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("considerModelOrder.groupModelOrder"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::GROUP_MODEL_ORDER_CB_PREFERRED_SOURCE_ID,
        LayoutOptionType::Int,
        "Cycle Breaking Preferred Source Id",
        "The model order group id for which should be preferred as a source if possible.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("considerModelOrder.groupModelOrder"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::GROUP_MODEL_ORDER_CB_PREFERRED_TARGET_ID,
        LayoutOptionType::Int,
        "Cycle Breaking Preferred Target Id",
        "The model order group id for which should be preferred as a target if possible.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("considerModelOrder.groupModelOrder"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY,
        LayoutOptionType::Enum,
        "Crossing Minimization Group Ordering Strategy",
        concat!(
            "Determines how to count ordering violations during crossing minimization. NONE: They do not count. ",
            "ENFORCED: A group with a lower id is before a group with a higher id. MODEL_ORDER: The model order counts ",
            "instead of the model order group id ordering."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("considerModelOrder.groupModelOrder"),
        None,
    );

    register_option(
        registry,
        LayeredOptions::GROUP_MODEL_ORDER_CM_ENFORCED_GROUP_ORDERS,
        LayoutOptionType::Object,
        "Crossing Minimization Enforced Group Orders",
        concat!(
            "Holds all group ids which are enforcing their order during crossing minimization strategies. ",
            "E.g. if only groups 2 and -1 (default) enforce their ordering. Other groups e.g. the group of ",
            "timer nodes can be ordered arbitrarily if it helps and the mentioned groups may not change ",
            "their order."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("considerModelOrder.groupModelOrder"),
        None,
    );
}

fn register_option_supports(registry: &mut dyn LayoutMetaDataRegistry) {
    let algo = LayeredOptions::ALGORITHM_ID;
    // BEGIN GENERATED OPTION SUPPORTS
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_COMMENT_COMMENT.id(),
        property_default_any(LayeredOptions::SPACING_COMMENT_COMMENT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_COMMENT_NODE.id(),
        property_default_any(LayeredOptions::SPACING_COMMENT_NODE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_COMPONENT_COMPONENT.id(),
        property_default_any(LayeredOptions::SPACING_COMPONENT_COMPONENT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_EDGE_EDGE.id(),
        property_default_any(LayeredOptions::SPACING_EDGE_EDGE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_EDGE_LABEL.id(),
        property_default_any(LayeredOptions::SPACING_EDGE_LABEL),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_EDGE_NODE.id(),
        property_default_any(LayeredOptions::SPACING_EDGE_NODE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_LABEL_LABEL.id(),
        property_default_any(LayeredOptions::SPACING_LABEL_LABEL),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_LABEL_PORT_HORIZONTAL.id(),
        property_default_any(LayeredOptions::SPACING_LABEL_PORT_HORIZONTAL),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_LABEL_PORT_VERTICAL.id(),
        property_default_any(LayeredOptions::SPACING_LABEL_PORT_VERTICAL),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_LABEL_NODE.id(),
        property_default_any(LayeredOptions::SPACING_LABEL_NODE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_NODE_NODE.id(),
        property_default_any(LayeredOptions::SPACING_NODE_NODE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_NODE_SELF_LOOP.id(),
        property_default_any(LayeredOptions::SPACING_NODE_SELF_LOOP),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_PORT_PORT.id(),
        property_default_any(LayeredOptions::SPACING_PORT_PORT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_INDIVIDUAL.id(),
        property_default_any(LayeredOptions::SPACING_INDIVIDUAL),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_BASE_VALUE.id(),
        property_default_any(LayeredOptions::SPACING_BASE_VALUE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_EDGE_EDGE_BETWEEN_LAYERS.id(),
        property_default_any(LayeredOptions::SPACING_EDGE_EDGE_BETWEEN_LAYERS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_EDGE_NODE_BETWEEN_LAYERS.id(),
        property_default_any(LayeredOptions::SPACING_EDGE_NODE_BETWEEN_LAYERS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS.id(),
        property_default_any(LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS),
    );
    registry.add_option_support(algo, LayeredOptions::PRIORITY.id(), explicit_default_any(0));
    registry.add_option_support(
        algo,
        LayeredOptions::PRIORITY_DIRECTION.id(),
        property_default_any(LayeredOptions::PRIORITY_DIRECTION),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::PRIORITY_SHORTNESS.id(),
        property_default_any(LayeredOptions::PRIORITY_SHORTNESS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::PRIORITY_STRAIGHTNESS.id(),
        property_default_any(LayeredOptions::PRIORITY_STRAIGHTNESS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::WRAPPING_STRATEGY.id(),
        property_default_any(LayeredOptions::WRAPPING_STRATEGY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::WRAPPING_ADDITIONAL_EDGE_SPACING.id(),
        property_default_any(LayeredOptions::WRAPPING_ADDITIONAL_EDGE_SPACING),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::WRAPPING_CORRECTION_FACTOR.id(),
        property_default_any(LayeredOptions::WRAPPING_CORRECTION_FACTOR),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::WRAPPING_CUTTING_STRATEGY.id(),
        property_default_any(LayeredOptions::WRAPPING_CUTTING_STRATEGY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::WRAPPING_CUTTING_CUTS.id(),
        property_default_any(LayeredOptions::WRAPPING_CUTTING_CUTS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::WRAPPING_CUTTING_MSD_FREEDOM.id(),
        property_default_any(LayeredOptions::WRAPPING_CUTTING_MSD_FREEDOM),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::WRAPPING_VALIDIFY_STRATEGY.id(),
        property_default_any(LayeredOptions::WRAPPING_VALIDIFY_STRATEGY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::WRAPPING_VALIDIFY_FORBIDDEN_INDICES.id(),
        property_default_any(LayeredOptions::WRAPPING_VALIDIFY_FORBIDDEN_INDICES),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::WRAPPING_MULTI_EDGE_IMPROVE_CUTS.id(),
        property_default_any(LayeredOptions::WRAPPING_MULTI_EDGE_IMPROVE_CUTS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::WRAPPING_MULTI_EDGE_DISTANCE_PENALTY.id(),
        property_default_any(LayeredOptions::WRAPPING_MULTI_EDGE_DISTANCE_PENALTY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::WRAPPING_MULTI_EDGE_IMPROVE_WRAPPED_EDGES.id(),
        property_default_any(LayeredOptions::WRAPPING_MULTI_EDGE_IMPROVE_WRAPPED_EDGES),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::LAYER_UNZIPPING_STRATEGY.id(),
        property_default_any(LayeredOptions::LAYER_UNZIPPING_STRATEGY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::LAYER_UNZIPPING_MINIMIZE_EDGE_LENGTH.id(),
        property_default_any(LayeredOptions::LAYER_UNZIPPING_MINIMIZE_EDGE_LENGTH),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::LAYER_UNZIPPING_LAYER_SPLIT.id(),
        property_default_any(LayeredOptions::LAYER_UNZIPPING_LAYER_SPLIT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::LAYER_UNZIPPING_RESET_ON_LONG_EDGES.id(),
        property_default_any(LayeredOptions::LAYER_UNZIPPING_RESET_ON_LONG_EDGES),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY.id(),
        property_default_any(LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT.id(),
        property_default_any(
            LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT,
        ),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::EDGE_ROUTING_SPLINES_MODE.id(),
        property_default_any(LayeredOptions::EDGE_ROUTING_SPLINES_MODE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::EDGE_ROUTING_SPLINES_SLOPPY_LAYER_SPACING_FACTOR.id(),
        property_default_any(LayeredOptions::EDGE_ROUTING_SPLINES_SLOPPY_LAYER_SPACING_FACTOR),
    );
    registry.add_option_support(
        algo,
        CoreOptions::TOPDOWN_LAYOUT.id(),
        property_default_any(CoreOptions::TOPDOWN_LAYOUT),
    );
    registry.add_option_support(
        algo,
        CoreOptions::TOPDOWN_SCALE_FACTOR.id(),
        property_default_any(CoreOptions::TOPDOWN_SCALE_FACTOR),
    );
    registry.add_option_support(
        algo,
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH.id(),
        property_default_any(CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH),
    );
    registry.add_option_support(
        algo,
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO.id(),
        property_default_any(CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::TOPDOWN_NODE_TYPE.id(),
        explicit_default_any(TopdownNodeTypes::HierarchicalNode),
    );
    registry.add_option_support(
        algo,
        CoreOptions::PADDING.id(),
        property_default_any(CoreOptions::PADDING),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::EDGE_ROUTING.id(),
        explicit_default_any(EdgeRouting::Orthogonal),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::PORT_BORDER_OFFSET.id(),
        explicit_default_any(0.0),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::RANDOM_SEED.id(),
        explicit_default_any(1),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::ASPECT_RATIO.id(),
        explicit_default_any(1.6),
    );
    registry.add_option_support(
        algo,
        CoreOptions::NO_LAYOUT.id(),
        property_default_any(CoreOptions::NO_LAYOUT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::PORT_CONSTRAINTS.id(),
        property_default_any(LayeredOptions::PORT_CONSTRAINTS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::PORT_SIDE.id(),
        property_default_any(LayeredOptions::PORT_SIDE),
    );
    registry.add_option_support(
        algo,
        CoreOptions::DEBUG_MODE.id(),
        property_default_any(CoreOptions::DEBUG_MODE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::ALIGNMENT.id(),
        property_default_any(LayeredOptions::ALIGNMENT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::HIERARCHY_HANDLING.id(),
        property_default_any(LayeredOptions::HIERARCHY_HANDLING),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SEPARATE_CONNECTED_COMPONENTS.id(),
        explicit_default_any(true),
    );
    registry.add_option_support(
        algo,
        CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE.id(),
        property_default_any(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE),
    );
    registry.add_option_support(
        algo,
        CoreOptions::INSIDE_SELF_LOOPS_YO.id(),
        property_default_any(CoreOptions::INSIDE_SELF_LOOPS_YO),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::NODE_SIZE_CONSTRAINTS.id(),
        property_default_any(LayeredOptions::NODE_SIZE_CONSTRAINTS),
    );
    registry.add_option_support(
        algo,
        CoreOptions::NODE_SIZE_OPTIONS.id(),
        property_default_any(CoreOptions::NODE_SIZE_OPTIONS),
    );
    registry.add_option_support(
        algo,
        CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE.id(),
        property_default_any(CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::DIRECTION.id(),
        property_default_any(LayeredOptions::DIRECTION),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::NODE_LABELS_PLACEMENT.id(),
        property_default_any(LayeredOptions::NODE_LABELS_PLACEMENT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::NODE_LABELS_PADDING.id(),
        property_default_any(LayeredOptions::NODE_LABELS_PADDING),
    );
    registry.add_option_support(
        algo,
        CoreOptions::PORT_LABELS_PLACEMENT.id(),
        property_default_any(CoreOptions::PORT_LABELS_PLACEMENT),
    );
    registry.add_option_support(
        algo,
        CoreOptions::PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE.id(),
        property_default_any(CoreOptions::PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE),
    );
    registry.add_option_support(
        algo,
        CoreOptions::PORT_LABELS_TREAT_AS_GROUP.id(),
        property_default_any(CoreOptions::PORT_LABELS_TREAT_AS_GROUP),
    );
    registry.add_option_support(
        algo,
        CoreOptions::PORT_ALIGNMENT_DEFAULT.id(),
        explicit_default_any(PortAlignment::Justified),
    );
    registry.add_option_support(
        algo,
        CoreOptions::PORT_ALIGNMENT_NORTH.id(),
        property_default_any(CoreOptions::PORT_ALIGNMENT_NORTH),
    );
    registry.add_option_support(
        algo,
        CoreOptions::PORT_ALIGNMENT_SOUTH.id(),
        property_default_any(CoreOptions::PORT_ALIGNMENT_SOUTH),
    );
    registry.add_option_support(
        algo,
        CoreOptions::PORT_ALIGNMENT_WEST.id(),
        property_default_any(CoreOptions::PORT_ALIGNMENT_WEST),
    );
    registry.add_option_support(
        algo,
        CoreOptions::PORT_ALIGNMENT_EAST.id(),
        property_default_any(CoreOptions::PORT_ALIGNMENT_EAST),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::UNNECESSARY_BENDPOINTS.id(),
        property_default_any(LayeredOptions::UNNECESSARY_BENDPOINTS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::LAYERING_STRATEGY.id(),
        property_default_any(LayeredOptions::LAYERING_STRATEGY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::LAYERING_NODE_PROMOTION_STRATEGY.id(),
        property_default_any(LayeredOptions::LAYERING_NODE_PROMOTION_STRATEGY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::THOROUGHNESS.id(),
        property_default_any(LayeredOptions::THOROUGHNESS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::LAYERING_LAYER_CONSTRAINT.id(),
        property_default_any(LayeredOptions::LAYERING_LAYER_CONSTRAINT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CYCLE_BREAKING_STRATEGY.id(),
        property_default_any(LayeredOptions::CYCLE_BREAKING_STRATEGY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CROSSING_MINIMIZATION_STRATEGY.id(),
        property_default_any(LayeredOptions::CROSSING_MINIMIZATION_STRATEGY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CROSSING_MINIMIZATION_FORCE_NODE_MODEL_ORDER.id(),
        property_default_any(LayeredOptions::CROSSING_MINIMIZATION_FORCE_NODE_MODEL_ORDER),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_ACTIVATION_THRESHOLD.id(),
        property_default_any(
            LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_ACTIVATION_THRESHOLD,
        ),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE.id(),
        property_default_any(LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE.id(),
        property_default_any(LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CROSSING_MINIMIZATION_SEMI_INTERACTIVE.id(),
        property_default_any(LayeredOptions::CROSSING_MINIMIZATION_SEMI_INTERACTIVE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::MERGE_EDGES.id(),
        property_default_any(LayeredOptions::MERGE_EDGES),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::MERGE_HIERARCHY_EDGES.id(),
        property_default_any(LayeredOptions::MERGE_HIERARCHY_EDGES),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::INTERACTIVE_REFERENCE_POINT.id(),
        property_default_any(LayeredOptions::INTERACTIVE_REFERENCE_POINT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::NODE_PLACEMENT_STRATEGY.id(),
        property_default_any(LayeredOptions::NODE_PLACEMENT_STRATEGY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::NODE_PLACEMENT_BK_FIXED_ALIGNMENT.id(),
        property_default_any(LayeredOptions::NODE_PLACEMENT_BK_FIXED_ALIGNMENT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::FEEDBACK_EDGES.id(),
        property_default_any(LayeredOptions::FEEDBACK_EDGES),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::NODE_PLACEMENT_LINEAR_SEGMENTS_DEFLECTION_DAMPENING.id(),
        property_default_any(LayeredOptions::NODE_PLACEMENT_LINEAR_SEGMENTS_DEFLECTION_DAMPENING),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::EDGE_ROUTING_SELF_LOOP_DISTRIBUTION.id(),
        property_default_any(LayeredOptions::EDGE_ROUTING_SELF_LOOP_DISTRIBUTION),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::EDGE_ROUTING_SELF_LOOP_ORDERING.id(),
        property_default_any(LayeredOptions::EDGE_ROUTING_SELF_LOOP_ORDERING),
    );
    registry.add_option_support(
        algo,
        CoreOptions::CONTENT_ALIGNMENT.id(),
        property_default_any(CoreOptions::CONTENT_ALIGNMENT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::NODE_PLACEMENT_BK_EDGE_STRAIGHTENING.id(),
        property_default_any(LayeredOptions::NODE_PLACEMENT_BK_EDGE_STRAIGHTENING),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::COMPACTION_POST_COMPACTION_STRATEGY.id(),
        property_default_any(LayeredOptions::COMPACTION_POST_COMPACTION_STRATEGY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::COMPACTION_POST_COMPACTION_CONSTRAINTS.id(),
        property_default_any(LayeredOptions::COMPACTION_POST_COMPACTION_CONSTRAINTS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::COMPACTION_CONNECTED_COMPONENTS.id(),
        property_default_any(LayeredOptions::COMPACTION_CONNECTED_COMPONENTS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::HIGH_DEGREE_NODES_TREATMENT.id(),
        property_default_any(LayeredOptions::HIGH_DEGREE_NODES_TREATMENT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::HIGH_DEGREE_NODES_THRESHOLD.id(),
        property_default_any(LayeredOptions::HIGH_DEGREE_NODES_THRESHOLD),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::HIGH_DEGREE_NODES_TREE_HEIGHT.id(),
        property_default_any(LayeredOptions::HIGH_DEGREE_NODES_TREE_HEIGHT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::NODE_SIZE_MINIMUM.id(),
        property_default_any(LayeredOptions::NODE_SIZE_MINIMUM),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::JUNCTION_POINTS.id(),
        property_default_any(LayeredOptions::JUNCTION_POINTS),
    );
    registry.add_option_support(
        algo,
        CoreOptions::EDGE_THICKNESS.id(),
        property_default_any(CoreOptions::EDGE_THICKNESS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::EDGE_LABELS_PLACEMENT.id(),
        property_default_any(LayeredOptions::EDGE_LABELS_PLACEMENT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::EDGE_LABELS_INLINE.id(),
        property_default_any(LayeredOptions::EDGE_LABELS_INLINE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS.id(),
        property_default_any(LayeredOptions::CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::PORT_INDEX.id(),
        property_default_any(LayeredOptions::PORT_INDEX),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::COMMENT_BOX.id(),
        property_default_any(LayeredOptions::COMMENT_BOX),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::HYPERNODE.id(),
        property_default_any(LayeredOptions::HYPERNODE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::PORT_ANCHOR.id(),
        property_default_any(LayeredOptions::PORT_ANCHOR),
    );
    registry.add_option_support(
        algo,
        CoreOptions::PARTITIONING_ACTIVATE.id(),
        property_default_any(CoreOptions::PARTITIONING_ACTIVATE),
    );
    registry.add_option_support(
        algo,
        CoreOptions::PARTITIONING_PARTITION.id(),
        property_default_any(CoreOptions::PARTITIONING_PARTITION),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::LAYERING_MIN_WIDTH_UPPER_BOUND_ON_WIDTH.id(),
        property_default_any(LayeredOptions::LAYERING_MIN_WIDTH_UPPER_BOUND_ON_WIDTH),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::LAYERING_MIN_WIDTH_UPPER_LAYER_ESTIMATION_SCALING_FACTOR.id(),
        property_default_any(
            LayeredOptions::LAYERING_MIN_WIDTH_UPPER_LAYER_ESTIMATION_SCALING_FACTOR,
        ),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::POSITION.id(),
        property_default_any(LayeredOptions::POSITION),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::ALLOW_NON_FLOW_PORTS_TO_SWITCH_SIDES.id(),
        property_default_any(LayeredOptions::ALLOW_NON_FLOW_PORTS_TO_SWITCH_SIDES),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::LAYERING_NODE_PROMOTION_MAX_ITERATIONS.id(),
        property_default_any(LayeredOptions::LAYERING_NODE_PROMOTION_MAX_ITERATIONS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::EDGE_LABELS_SIDE_SELECTION.id(),
        property_default_any(LayeredOptions::EDGE_LABELS_SIDE_SELECTION),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY.id(),
        property_default_any(LayeredOptions::EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY),
    );
    registry.add_option_support(
        algo,
        CoreOptions::MARGINS.id(),
        property_default_any(CoreOptions::MARGINS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::LAYERING_COFFMAN_GRAHAM_LAYER_BOUND.id(),
        property_default_any(LayeredOptions::LAYERING_COFFMAN_GRAHAM_LAYER_BOUND),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES.id(),
        property_default_any(LayeredOptions::NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::SPACING_PORTS_SURROUNDING.id(),
        property_default_any(LayeredOptions::SPACING_PORTS_SURROUNDING),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::DIRECTION_CONGRUENCY.id(),
        property_default_any(LayeredOptions::DIRECTION_CONGRUENCY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::PORT_SORTING_STRATEGY.id(),
        property_default_any(LayeredOptions::PORT_SORTING_STRATEGY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::EDGE_ROUTING_POLYLINE_SLOPED_EDGE_ZONE_WIDTH.id(),
        property_default_any(LayeredOptions::EDGE_ROUTING_POLYLINE_SLOPED_EDGE_ZONE_WIDTH),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CROSSING_MINIMIZATION_IN_LAYER_PRED_OF.id(),
        property_default_any(LayeredOptions::CROSSING_MINIMIZATION_IN_LAYER_PRED_OF),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CROSSING_MINIMIZATION_IN_LAYER_SUCC_OF.id(),
        property_default_any(LayeredOptions::CROSSING_MINIMIZATION_IN_LAYER_SUCC_OF),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::LAYERING_LAYER_CHOICE_CONSTRAINT.id(),
        property_default_any(LayeredOptions::LAYERING_LAYER_CHOICE_CONSTRAINT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CROSSING_MINIMIZATION_POSITION_CHOICE_CONSTRAINT.id(),
        property_default_any(LayeredOptions::CROSSING_MINIMIZATION_POSITION_CHOICE_CONSTRAINT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::INTERACTIVE_LAYOUT.id(),
        property_default_any(LayeredOptions::INTERACTIVE_LAYOUT),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::LAYERING_LAYER_ID.id(),
        property_default_any(LayeredOptions::LAYERING_LAYER_ID),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::LAYERING_IGNORE_EDGE_IN_LAYER.id(),
        property_default_any(LayeredOptions::LAYERING_IGNORE_EDGE_IN_LAYER),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CROSSING_MINIMIZATION_POSITION_ID.id(),
        property_default_any(LayeredOptions::CROSSING_MINIMIZATION_POSITION_ID),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY.id(),
        property_default_any(LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CONSIDER_MODEL_ORDER_LONG_EDGE_STRATEGY.id(),
        property_default_any(LayeredOptions::CONSIDER_MODEL_ORDER_LONG_EDGE_STRATEGY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_NODE_INFLUENCE.id(),
        property_default_any(LayeredOptions::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_NODE_INFLUENCE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_PORT_INFLUENCE.id(),
        property_default_any(LayeredOptions::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_PORT_INFLUENCE),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CONSIDER_MODEL_ORDER_NO_MODEL_ORDER.id(),
        property_default_any(LayeredOptions::CONSIDER_MODEL_ORDER_NO_MODEL_ORDER),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CONSIDER_MODEL_ORDER_COMPONENTS.id(),
        property_default_any(LayeredOptions::CONSIDER_MODEL_ORDER_COMPONENTS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER.id(),
        property_default_any(LayeredOptions::CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::GROUP_MODEL_ORDER_CYCLE_BREAKING_ID.id(),
        property_default_any(LayeredOptions::GROUP_MODEL_ORDER_CYCLE_BREAKING_ID),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::GROUP_MODEL_ORDER_CROSSING_MINIMIZATION_ID.id(),
        property_default_any(LayeredOptions::GROUP_MODEL_ORDER_CROSSING_MINIMIZATION_ID),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::GROUP_MODEL_ORDER_COMPONENT_GROUP_ID.id(),
        property_default_any(LayeredOptions::GROUP_MODEL_ORDER_COMPONENT_GROUP_ID),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::GROUP_MODEL_ORDER_CB_GROUP_ORDER_STRATEGY.id(),
        property_default_any(LayeredOptions::GROUP_MODEL_ORDER_CB_GROUP_ORDER_STRATEGY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY.id(),
        property_default_any(LayeredOptions::GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::GROUP_MODEL_ORDER_CM_ENFORCED_GROUP_ORDERS.id(),
        property_default_any(LayeredOptions::GROUP_MODEL_ORDER_CM_ENFORCED_GROUP_ORDERS),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::GROUP_MODEL_ORDER_CB_PREFERRED_SOURCE_ID.id(),
        property_default_any(LayeredOptions::GROUP_MODEL_ORDER_CB_PREFERRED_SOURCE_ID),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::GROUP_MODEL_ORDER_CB_PREFERRED_TARGET_ID.id(),
        property_default_any(LayeredOptions::GROUP_MODEL_ORDER_CB_PREFERRED_TARGET_ID),
    );
    registry.add_option_support(
        algo,
        LayeredOptions::GENERATE_POSITION_AND_LAYER_IDS.id(),
        property_default_any(LayeredOptions::GENERATE_POSITION_AND_LAYER_IDS),
    );
    // END GENERATED OPTION SUPPORTS
}

#[allow(clippy::too_many_arguments)]
fn register_option<T: Clone + Send + Sync + 'static>(
    registry: &mut dyn LayoutMetaDataRegistry,
    property: &'static LazyLock<Property<T>>,
    option_type: LayoutOptionType,
    name: &'static str,
    description: &'static str,
    targets: &'static [LayoutOptionTarget],
    visibility: LayoutOptionVisibility,
    group: Option<&'static str>,
    lower_bound: Option<Arc<dyn Any + Send + Sync>>,
) {
    let default_value = property_default_any(property);
    let legacy_ids = layered_legacy_ids(property.id());
    let mut builder = LayoutOptionData::builder()
        .id(property.id())
        .option_type(option_type)
        .default_value(default_value)
        .name(name)
        .description(description)
        .targets(targets.iter().copied().collect::<HashSet<_>>())
        .visibility(visibility)
        .value_type_id(TypeId::of::<T>());
    if option_type == LayoutOptionType::Enum {
        if let Some((choices, parser)) = layered_enum_support::<T>() {
            builder = builder.choices(choices).parser(parser);
        }
    }
    if !legacy_ids.is_empty() {
        builder = builder.legacy_ids(legacy_ids);
    }
    if let Some(group) = group {
        builder = builder.group(group);
    }
    if lower_bound.is_some() {
        builder = builder.lower_bound(lower_bound);
    }
    registry.register_option(builder.create());
}

fn layered_legacy_ids(id: &str) -> Vec<String> {
    id.strip_prefix("org.eclipse.elk.alg.layered.")
        .map(|suffix| format!("org.eclipse.elk.layered.{suffix}"))
        .into_iter()
        .collect()
}

fn layered_enum_support<T: Send + Sync + 'static>() -> Option<(Vec<String>, ParserFn)> {
    let type_id = TypeId::of::<T>();

    if type_id == TypeId::of::<CenterEdgeLabelPlacementStrategy>() {
        return Some(enum_support(&[
            CenterEdgeLabelPlacementStrategy::MedianLayer,
            CenterEdgeLabelPlacementStrategy::TailLayer,
            CenterEdgeLabelPlacementStrategy::HeadLayer,
            CenterEdgeLabelPlacementStrategy::SpaceEfficientLayer,
            CenterEdgeLabelPlacementStrategy::WidestLayer,
            CenterEdgeLabelPlacementStrategy::CenterLayer,
        ]));
    }
    if type_id == TypeId::of::<ComponentOrderingStrategy>() {
        return Some(enum_support(&[
            ComponentOrderingStrategy::None,
            ComponentOrderingStrategy::InsidePortSideGroups,
            ComponentOrderingStrategy::GroupModelOrder,
            ComponentOrderingStrategy::ModelOrder,
        ]));
    }
    if type_id == TypeId::of::<ConstraintCalculationStrategy>() {
        return Some(enum_support(&[
            ConstraintCalculationStrategy::Quadratic,
            ConstraintCalculationStrategy::Scanline,
        ]));
    }
    if type_id == TypeId::of::<CrossingMinimizationStrategy>() {
        return Some(enum_support(&[
            CrossingMinimizationStrategy::LayerSweep,
            CrossingMinimizationStrategy::MedianLayerSweep,
            CrossingMinimizationStrategy::Interactive,
            CrossingMinimizationStrategy::None,
        ]));
    }
    if type_id == TypeId::of::<CuttingStrategy>() {
        return Some(enum_support(&[
            CuttingStrategy::Ard,
            CuttingStrategy::Msd,
            CuttingStrategy::Manual,
        ]));
    }
    if type_id == TypeId::of::<CycleBreakingStrategy>() {
        return Some(enum_support(&[
            CycleBreakingStrategy::Greedy,
            CycleBreakingStrategy::DepthFirst,
            CycleBreakingStrategy::Interactive,
            CycleBreakingStrategy::ModelOrder,
            CycleBreakingStrategy::GreedyModelOrder,
            CycleBreakingStrategy::SccConnectivity,
            CycleBreakingStrategy::SccNodeType,
            CycleBreakingStrategy::DfsNodeOrder,
            CycleBreakingStrategy::BfsNodeOrder,
        ]));
    }
    if type_id == TypeId::of::<DirectionCongruency>() {
        return Some(enum_support(&[
            DirectionCongruency::ReadingDirection,
            DirectionCongruency::Rotation,
        ]));
    }
    if type_id == TypeId::of::<EdgeLabelSideSelection>() {
        return Some(enum_support(&[
            EdgeLabelSideSelection::AlwaysUp,
            EdgeLabelSideSelection::AlwaysDown,
            EdgeLabelSideSelection::DirectionUp,
            EdgeLabelSideSelection::DirectionDown,
            EdgeLabelSideSelection::SmartUp,
            EdgeLabelSideSelection::SmartDown,
        ]));
    }
    if type_id == TypeId::of::<EdgeStraighteningStrategy>() {
        return Some(enum_support(&[
            EdgeStraighteningStrategy::None,
            EdgeStraighteningStrategy::ImproveStraightness,
        ]));
    }
    if type_id == TypeId::of::<FixedAlignment>() {
        return Some(enum_support(&[
            FixedAlignment::None,
            FixedAlignment::LeftUp,
            FixedAlignment::RightUp,
            FixedAlignment::LeftDown,
            FixedAlignment::RightDown,
            FixedAlignment::Balanced,
        ]));
    }
    if type_id == TypeId::of::<GraphCompactionStrategy>() {
        return Some(enum_support(&[
            GraphCompactionStrategy::None,
            GraphCompactionStrategy::Left,
            GraphCompactionStrategy::Right,
            GraphCompactionStrategy::LeftRightConstraintLocking,
            GraphCompactionStrategy::LeftRightConnectionLocking,
            GraphCompactionStrategy::EdgeLength,
        ]));
    }
    if type_id == TypeId::of::<GreedySwitchType>() {
        return Some(enum_support(&[
            GreedySwitchType::OneSided,
            GreedySwitchType::TwoSided,
            GreedySwitchType::Off,
        ]));
    }
    if type_id == TypeId::of::<GroupOrderStrategy>() {
        return Some(enum_support(&[
            GroupOrderStrategy::OnlyWithinGroup,
            GroupOrderStrategy::ModelOrder,
            GroupOrderStrategy::Enforced,
        ]));
    }
    if type_id == TypeId::of::<InteractiveReferencePoint>() {
        return Some(enum_support(&[
            InteractiveReferencePoint::Center,
            InteractiveReferencePoint::TopLeft,
        ]));
    }
    if type_id == TypeId::of::<LayerConstraint>() {
        return Some(enum_support(&[
            LayerConstraint::None,
            LayerConstraint::First,
            LayerConstraint::FirstSeparate,
            LayerConstraint::Last,
            LayerConstraint::LastSeparate,
        ]));
    }
    if type_id == TypeId::of::<LayeringStrategy>() {
        return Some(enum_support(&[
            LayeringStrategy::NetworkSimplex,
            LayeringStrategy::LongestPath,
            LayeringStrategy::LongestPathSource,
            LayeringStrategy::CoffmanGraham,
            LayeringStrategy::Interactive,
            LayeringStrategy::StretchWidth,
            LayeringStrategy::MinWidth,
            LayeringStrategy::BfModelOrder,
            LayeringStrategy::DfModelOrder,
        ]));
    }
    if type_id == TypeId::of::<LayerUnzippingStrategy>() {
        return Some(enum_support(&[
            LayerUnzippingStrategy::None,
            LayerUnzippingStrategy::Alternating,
        ]));
    }
    if type_id == TypeId::of::<LongEdgeOrderingStrategy>() {
        return Some(enum_support(&[
            LongEdgeOrderingStrategy::DummyNodeOver,
            LongEdgeOrderingStrategy::DummyNodeUnder,
            LongEdgeOrderingStrategy::Equal,
        ]));
    }
    if type_id == TypeId::of::<NodeFlexibility>() {
        return Some(enum_support(&[
            NodeFlexibility::None,
            NodeFlexibility::PortPosition,
            NodeFlexibility::NodeSizeWhereSpacePermits,
            NodeFlexibility::NodeSize,
        ]));
    }
    if type_id == TypeId::of::<NodePlacementStrategy>() {
        return Some(enum_support(&[
            NodePlacementStrategy::Simple,
            NodePlacementStrategy::Interactive,
            NodePlacementStrategy::LinearSegments,
            NodePlacementStrategy::BrandesKoepf,
            NodePlacementStrategy::NetworkSimplex,
        ]));
    }
    if type_id == TypeId::of::<NodePromotionStrategy>() {
        return Some(enum_support(&[
            NodePromotionStrategy::None,
            NodePromotionStrategy::Nikolov,
            NodePromotionStrategy::NikolovPixel,
            NodePromotionStrategy::NikolovImproved,
            NodePromotionStrategy::NikolovImprovedPixel,
            NodePromotionStrategy::DummynodePercentage,
            NodePromotionStrategy::NodecountPercentage,
            NodePromotionStrategy::NoBoundary,
            NodePromotionStrategy::ModelOrderLeftToRight,
            NodePromotionStrategy::ModelOrderRightToLeft,
        ]));
    }
    if type_id == TypeId::of::<OrderingStrategy>() {
        return Some(enum_support(&[
            OrderingStrategy::None,
            OrderingStrategy::NodesAndEdges,
            OrderingStrategy::PreferEdges,
            OrderingStrategy::PreferNodes,
        ]));
    }
    if type_id == TypeId::of::<PortSortingStrategy>() {
        return Some(enum_support(&[
            PortSortingStrategy::InputOrder,
            PortSortingStrategy::PortDegree,
        ]));
    }
    if type_id == TypeId::of::<SelfLoopDistributionStrategy>() {
        return Some(enum_support(&[
            SelfLoopDistributionStrategy::Equally,
            SelfLoopDistributionStrategy::North,
            SelfLoopDistributionStrategy::NorthSouth,
        ]));
    }
    if type_id == TypeId::of::<SelfLoopOrderingStrategy>() {
        return Some(enum_support(&[
            SelfLoopOrderingStrategy::Stacked,
            SelfLoopOrderingStrategy::ReverseStacked,
            SelfLoopOrderingStrategy::Sequenced,
        ]));
    }
    if type_id == TypeId::of::<SplineRoutingMode>() {
        return Some(enum_support(&[
            SplineRoutingMode::Conservative,
            SplineRoutingMode::ConservativeSoft,
            SplineRoutingMode::Sloppy,
        ]));
    }
    if type_id == TypeId::of::<ValidifyStrategy>() {
        return Some(enum_support(&[
            ValidifyStrategy::No,
            ValidifyStrategy::Greedy,
            ValidifyStrategy::LookBack,
        ]));
    }
    if type_id == TypeId::of::<WrappingStrategy>() {
        return Some(enum_support(&[
            WrappingStrategy::Off,
            WrappingStrategy::SingleEdge,
            WrappingStrategy::MultiEdge,
        ]));
    }

    None
}

fn enum_support<T: Copy + fmt::Debug + Send + Sync + 'static>(
    variants: &'static [T],
) -> (Vec<String>, ParserFn) {
    (
        enum_choices(variants),
        Arc::new(move |value| {
            parse_enum_value(value, variants)
                .map(|parsed| Arc::new(parsed) as Arc<dyn Any + Send + Sync>)
        }),
    )
}

fn enum_choices<T: fmt::Debug>(variants: &'static [T]) -> Vec<String> {
    variants
        .iter()
        .map(|variant| to_upper_snake(&format!("{:?}", variant)))
        .collect()
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

fn to_upper_snake(value: &str) -> String {
    let mut out = String::new();
    let mut prev: Option<char> = None;
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        let next = chars.peek().copied();
        if let Some(prev_ch) = prev {
            if ch.is_uppercase()
                && (prev_ch.is_lowercase() || next.map(|n| n.is_lowercase()).unwrap_or(false))
            {
                out.push('_');
            }
        }
        out.push(ch.to_ascii_uppercase());
        prev = Some(ch);
    }
    out
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

fn explicit_default_any<T: Send + Sync + 'static>(value: T) -> Option<Arc<dyn Any + Send + Sync>> {
    Some(Arc::new(value))
}

fn bound_f64(value: f64) -> Arc<dyn Any + Send + Sync> {
    Arc::new(value)
}

fn bound_i32(value: i32) -> Arc<dyn Any + Send + Sync> {
    Arc::new(value)
}

fn init_reflect() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        ElkReflect::register(Some(Vec::<i32>::new), Some(|v: &Vec<i32>| v.clone()));
        ElkReflect::register(
            Some(|| WrappingStrategy::Off),
            Some(|v: &WrappingStrategy| *v),
        );
        ElkReflect::register(
            Some(|| CuttingStrategy::Msd),
            Some(|v: &CuttingStrategy| *v),
        );
        ElkReflect::register(
            Some(|| ValidifyStrategy::Greedy),
            Some(|v: &ValidifyStrategy| *v),
        );
        ElkReflect::register(
            Some(|| OrderingStrategy::None),
            Some(|v: &OrderingStrategy| *v),
        );
        ElkReflect::register(
            Some(|| ComponentOrderingStrategy::None),
            Some(|v: &ComponentOrderingStrategy| *v),
        );
        ElkReflect::register(
            Some(|| LongEdgeOrderingStrategy::DummyNodeOver),
            Some(|v: &LongEdgeOrderingStrategy| *v),
        );
        ElkReflect::register(
            Some(|| GroupOrderStrategy::OnlyWithinGroup),
            Some(|v: &GroupOrderStrategy| *v),
        );
        ElkReflect::register(
            Some(|| LayerConstraint::None),
            Some(|v: &LayerConstraint| *v),
        );
        ElkReflect::register(
            Some(|| InLayerConstraint::None),
            Some(|v: &InLayerConstraint| *v),
        );
        ElkReflect::register(
            Some(|| InteractiveReferencePoint::Center),
            Some(|v: &InteractiveReferencePoint| *v),
        );
        ElkReflect::register(
            Some(|| LayerUnzippingStrategy::None),
            Some(|v: &LayerUnzippingStrategy| *v),
        );
        ElkReflect::register(
            Some(|| EdgeLabelSideSelection::SmartDown),
            Some(|v: &EdgeLabelSideSelection| *v),
        );
        ElkReflect::register(
            Some(|| CenterEdgeLabelPlacementStrategy::MedianLayer),
            Some(|v: &CenterEdgeLabelPlacementStrategy| *v),
        );
        ElkReflect::register(
            Some(|| CycleBreakingStrategy::Greedy),
            Some(|v: &CycleBreakingStrategy| *v),
        );
        ElkReflect::register(
            Some(|| LayeringStrategy::NetworkSimplex),
            Some(|v: &LayeringStrategy| *v),
        );
        ElkReflect::register(
            Some(|| NodePromotionStrategy::None),
            Some(|v: &NodePromotionStrategy| *v),
        );
        ElkReflect::register(
            Some(|| CrossingMinimizationStrategy::LayerSweep),
            Some(|v: &CrossingMinimizationStrategy| *v),
        );
        ElkReflect::register(
            Some(|| GreedySwitchType::Off),
            Some(|v: &GreedySwitchType| *v),
        );
        ElkReflect::register(
            Some(|| NodePlacementStrategy::BrandesKoepf),
            Some(|v: &NodePlacementStrategy| *v),
        );
        ElkReflect::register(
            Some(|| EdgeStraighteningStrategy::ImproveStraightness),
            Some(|v: &EdgeStraighteningStrategy| *v),
        );
        ElkReflect::register(Some(|| FixedAlignment::None), Some(|v: &FixedAlignment| *v));
        ElkReflect::register(
            Some(|| NodeFlexibility::None),
            Some(|v: &NodeFlexibility| *v),
        );
        ElkReflect::register(
            Some(|| SplineRoutingMode::Sloppy),
            Some(|v: &SplineRoutingMode| *v),
        );
        ElkReflect::register(
            Some(|| SelfLoopDistributionStrategy::North),
            Some(|v: &SelfLoopDistributionStrategy| *v),
        );
        ElkReflect::register(
            Some(|| SelfLoopOrderingStrategy::Stacked),
            Some(|v: &SelfLoopOrderingStrategy| *v),
        );
        ElkReflect::register(
            Some(|| GraphCompactionStrategy::None),
            Some(|v: &GraphCompactionStrategy| *v),
        );
        ElkReflect::register(
            Some(|| ConstraintCalculationStrategy::Scanline),
            Some(|v: &ConstraintCalculationStrategy| *v),
        );
        ElkReflect::register(
            Some(|| DirectionCongruency::ReadingDirection),
            Some(|v: &DirectionCongruency| *v),
        );
        ElkReflect::register(
            Some(|| PortSortingStrategy::InputOrder),
            Some(|v: &PortSortingStrategy| *v),
        );
    });
}
