use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::sync::{Arc, LazyLock, OnceLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutMetaDataRegistry, LayoutOptionData, LayoutOptionTarget,
    LayoutOptionType, LayoutOptionVisibility,
};

use super::{
    CenterEdgeLabelPlacementStrategy, ConstraintCalculationStrategy, CrossingMinimizationStrategy,
    CuttingStrategy, CycleBreakingStrategy, DirectionCongruency, EdgeLabelSideSelection,
    EdgeStraighteningStrategy, FixedAlignment, GraphCompactionStrategy, GreedySwitchType,
    GroupOrderStrategy, InteractiveReferencePoint, LayerUnzippingStrategy, LayeredOptions,
    LayeringStrategy, LongEdgeOrderingStrategy, NodeFlexibility, NodePlacementStrategy,
    NodePromotionStrategy, OrderingStrategy, PortSortingStrategy, SelfLoopDistributionStrategy,
    SelfLoopOrderingStrategy, SplineRoutingMode, ValidifyStrategy, WrappingStrategy,
};
use crate::org::eclipse::elk::alg::layered::components::ComponentOrderingStrategy;

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
    let mut builder = LayoutOptionData::builder()
        .id(property.id())
        .option_type(option_type)
        .default_value(default_value)
        .name(name)
        .description(description)
        .targets(targets.iter().copied().collect::<HashSet<_>>())
        .visibility(visibility)
        .value_type_id(TypeId::of::<T>());
    if let Some(group) = group {
        builder = builder.group(group);
    }
    if lower_bound.is_some() {
        builder = builder.lower_bound(lower_bound);
    }
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
        ElkReflect::register(Some(|| WrappingStrategy::Off), Some(|v: &WrappingStrategy| *v));
        ElkReflect::register(Some(|| CuttingStrategy::Msd), Some(|v: &CuttingStrategy| *v));
        ElkReflect::register(Some(|| ValidifyStrategy::Greedy), Some(|v: &ValidifyStrategy| *v));
        ElkReflect::register(Some(|| OrderingStrategy::None), Some(|v: &OrderingStrategy| *v));
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
        ElkReflect::register(
            Some(|| FixedAlignment::None),
            Some(|v: &FixedAlignment| *v),
        );
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
