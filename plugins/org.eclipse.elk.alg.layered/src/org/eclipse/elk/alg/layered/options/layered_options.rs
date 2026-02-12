use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

use super::{
    CenterEdgeLabelPlacementStrategy, ConstraintCalculationStrategy, CuttingStrategy,
    CycleBreakingStrategy, CrossingMinimizationStrategy, DirectionCongruency,
    EdgeLabelSideSelection, EdgeStraighteningStrategy, FixedAlignment, GraphCompactionStrategy,
    GreedySwitchType, GroupOrderStrategy, InteractiveReferencePoint, LayerConstraint,
    LayerUnzippingStrategy, LayeringStrategy, LongEdgeOrderingStrategy, NodeFlexibility,
    NodePlacementStrategy, NodePromotionStrategy, OrderingStrategy, PortSortingStrategy,
    SelfLoopDistributionStrategy, SelfLoopOrderingStrategy, SplineRoutingMode, ValidifyStrategy,
    WrappingStrategy,
};
use crate::org::eclipse::elk::alg::layered::components::ComponentOrderingStrategy;
use org_eclipse_elk_core::org::eclipse::elk::core::options::alignment::Alignment;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::topdown_node_types::TopdownNodeTypes;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IndividualSpacings;
use org_eclipse_elk_core::org::eclipse::elk::core::math::elk_margin::ElkMargin;
use org_eclipse_elk_core::org::eclipse::elk::core::math::elk_padding::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::node_label_placement::NodeLabelPlacement;

pub struct LayeredOptions;

pub static SPACING_BASE_VALUE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.alg.layered.spacing.baseValue"));

pub static SPACING_EDGE_NODE_BETWEEN_LAYERS_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.spacing.edgeNodeBetweenLayers",
        10.0,
    )
});

pub static SPACING_EDGE_EDGE_BETWEEN_LAYERS_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.spacing.edgeEdgeBetweenLayers",
        10.0,
    )
});

pub static SPACING_NODE_NODE_BETWEEN_LAYERS_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.spacing.nodeNodeBetweenLayers",
        20.0,
    )
});

pub static PRIORITY_DIRECTION_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.layered.priority.direction", 0));

pub static PRIORITY_SHORTNESS_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.layered.priority.shortness", 0));

pub static PRIORITY_STRAIGHTNESS_PROPERTY: LazyLock<Property<i32>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.alg.layered.priority.straightness", 0)
});

pub static WRAPPING_STRATEGY_PROPERTY: LazyLock<Property<WrappingStrategy>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.layered.wrapping.strategy", WrappingStrategy::Off));

pub static WRAPPING_ADDITIONAL_EDGE_SPACING_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.wrapping.additionalEdgeSpacing",
        10.0,
    )
});

pub static WRAPPING_CORRECTION_FACTOR_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.alg.layered.wrapping.correctionFactor", 1.0)
});

pub static WRAPPING_CUTTING_STRATEGY_PROPERTY: LazyLock<Property<CuttingStrategy>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.wrapping.cutting.strategy",
        CuttingStrategy::Msd,
    )
});

pub static WRAPPING_CUTTING_CUTS_PROPERTY: LazyLock<Property<Vec<i32>>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.alg.layered.wrapping.cutting.cuts"));

pub static WRAPPING_CUTTING_MSD_FREEDOM_PROPERTY: LazyLock<Property<i32>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.alg.layered.wrapping.cutting.msd.freedom", 1)
});

pub static WRAPPING_VALIDIFY_STRATEGY_PROPERTY: LazyLock<Property<ValidifyStrategy>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.wrapping.validify.strategy",
        ValidifyStrategy::Greedy,
    )
});

pub static WRAPPING_VALIDIFY_FORBIDDEN_INDICES_PROPERTY: LazyLock<Property<Vec<i32>>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.alg.layered.wrapping.validify.forbiddenIndices"));

pub static WRAPPING_MULTI_EDGE_IMPROVE_CUTS_PROPERTY: LazyLock<Property<bool>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.alg.layered.wrapping.multiEdge.improveCuts", true)
});

pub static WRAPPING_MULTI_EDGE_DISTANCE_PENALTY_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.wrapping.multiEdge.distancePenalty",
        2.0,
    )
});

pub static WRAPPING_MULTI_EDGE_IMPROVE_WRAPPED_EDGES_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.wrapping.multiEdge.improveWrappedEdges",
            true,
        )
    });

pub static LAYER_UNZIPPING_STRATEGY_PROPERTY: LazyLock<Property<LayerUnzippingStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.layerUnzipping.strategy",
            LayerUnzippingStrategy::None,
        )
    });

pub static LAYER_UNZIPPING_MINIMIZE_EDGE_LENGTH_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.layerUnzipping.minimizeEdgeLength",
            false,
        )
    });

pub static LAYER_UNZIPPING_LAYER_SPLIT_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.layered.layerUnzipping.layerSplit", 2));

pub static LAYER_UNZIPPING_RESET_ON_LONG_EDGES_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.layerUnzipping.resetOnLongEdges",
            true,
        )
    });

pub static CYCLE_BREAKING_STRATEGY_PROPERTY: LazyLock<Property<CycleBreakingStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.cycleBreaking.strategy",
            CycleBreakingStrategy::Greedy,
        )
    });

pub static LAYERING_STRATEGY_PROPERTY: LazyLock<Property<LayeringStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.layering.strategy",
            LayeringStrategy::NetworkSimplex,
        )
    });

pub static LAYERING_LAYER_CONSTRAINT_PROPERTY: LazyLock<Property<LayerConstraint>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.layering.layerConstraint",
            LayerConstraint::None,
        )
    });

pub static LAYERING_LAYER_CHOICE_CONSTRAINT_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.alg.layered.layering.layerChoiceConstraint"));

pub static LAYERING_LAYER_ID_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.layered.layering.layerId", -1));

pub static LAYERING_MIN_WIDTH_UPPER_BOUND_ON_WIDTH_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.layering.minWidth.upperBoundOnWidth",
            4,
        )
    });

pub static LAYERING_MIN_WIDTH_UPPER_LAYER_ESTIMATION_SCALING_FACTOR_PROPERTY: LazyLock<
    Property<i32>,
> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.layering.minWidth.upperLayerEstimationScalingFactor",
        2,
    )
});

pub static LAYERING_NODE_PROMOTION_STRATEGY_PROPERTY: LazyLock<Property<NodePromotionStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.layering.nodePromotion.strategy",
            NodePromotionStrategy::None,
        )
    });

pub static LAYERING_NODE_PROMOTION_MAX_ITERATIONS_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.layering.nodePromotion.maxIterations",
            0,
        )
    });

pub static LAYERING_COFFMAN_GRAHAM_LAYER_BOUND_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.layering.coffmanGraham.layerBound",
            i32::MAX,
        )
    });

pub static CROSSING_MINIMIZATION_STRATEGY_PROPERTY: LazyLock<
    Property<CrossingMinimizationStrategy>,
> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.crossingMinimization.strategy",
        CrossingMinimizationStrategy::LayerSweep,
    )
});

pub static CROSSING_MINIMIZATION_FORCE_NODE_MODEL_ORDER_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.crossingMinimization.forceNodeModelOrder",
            false,
        )
    });

pub static CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.crossingMinimization.hierarchicalSweepiness",
            0.1,
        )
    });

pub static CROSSING_MINIMIZATION_GREEDY_SWITCH_ACTIVATION_THRESHOLD_PROPERTY: LazyLock<
    Property<i32>,
> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.crossingMinimization.greedySwitch.activationThreshold",
        40,
    )
});

pub static CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE_PROPERTY: LazyLock<Property<GreedySwitchType>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.crossingMinimization.greedySwitch.type",
            GreedySwitchType::TwoSided,
        )
    });

pub static CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE_PROPERTY: LazyLock<
    Property<GreedySwitchType>,
> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.crossingMinimization.greedySwitchHierarchical.type",
        GreedySwitchType::Off,
    )
});

pub static CROSSING_MINIMIZATION_SEMI_INTERACTIVE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.crossingMinimization.semiInteractive",
            false,
        )
    });

pub static CROSSING_MINIMIZATION_IN_LAYER_PRED_OF_PROPERTY: LazyLock<Property<String>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.alg.layered.crossingMinimization.inLayerPredOf"));

pub static CROSSING_MINIMIZATION_IN_LAYER_SUCC_OF_PROPERTY: LazyLock<Property<String>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.alg.layered.crossingMinimization.inLayerSuccOf"));

pub static CROSSING_MINIMIZATION_POSITION_CHOICE_CONSTRAINT_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| {
        Property::new("org.eclipse.elk.alg.layered.crossingMinimization.positionChoiceConstraint")
    });

pub static CROSSING_MINIMIZATION_POSITION_ID_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.crossingMinimization.positionId",
            -1,
        )
    });

pub static NODE_PLACEMENT_STRATEGY_PROPERTY: LazyLock<Property<NodePlacementStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.nodePlacement.strategy",
            NodePlacementStrategy::BrandesKoepf,
        )
    });

pub static NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.alg.layered.nodePlacement.favorStraightEdges"));

pub static NODE_PLACEMENT_BK_EDGE_STRAIGHTENING_PROPERTY: LazyLock<
    Property<EdgeStraighteningStrategy>,
> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.nodePlacement.bk.edgeStraightening",
        EdgeStraighteningStrategy::ImproveStraightness,
    )
});

pub static NODE_PLACEMENT_BK_FIXED_ALIGNMENT_PROPERTY: LazyLock<Property<FixedAlignment>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.nodePlacement.bk.fixedAlignment",
            FixedAlignment::None,
        )
    });

pub static NODE_PLACEMENT_LINEAR_SEGMENTS_DEFLECTION_DAMPENING_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.nodePlacement.linearSegments.deflectionDampening",
            0.3,
        )
    });

pub static NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_PROPERTY: LazyLock<
    Property<NodeFlexibility>,
> = LazyLock::new(|| {
    Property::new("org.eclipse.elk.alg.layered.nodePlacement.networkSimplex.nodeFlexibility")
});

pub static NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT_PROPERTY: LazyLock<
    Property<NodeFlexibility>,
> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.nodePlacement.networkSimplex.nodeFlexibility.default",
        NodeFlexibility::None,
    )
});

pub static EDGE_ROUTING_SPLINES_MODE_PROPERTY: LazyLock<Property<SplineRoutingMode>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.edgeRouting.splines.mode",
            SplineRoutingMode::Sloppy,
        )
    });

pub static EDGE_ROUTING_SPLINES_SLOPPY_LAYER_SPACING_FACTOR_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.edgeRouting.splines.sloppy.layerSpacingFactor",
            0.2,
        )
    });

pub static EDGE_ROUTING_POLYLINE_SLOPED_EDGE_ZONE_WIDTH_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.edgeRouting.polyline.slopedEdgeZoneWidth",
            2.0,
        )
    });

pub static EDGE_ROUTING_SELF_LOOP_DISTRIBUTION_PROPERTY: LazyLock<
    Property<SelfLoopDistributionStrategy>,
> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.edgeRouting.selfLoopDistribution",
        SelfLoopDistributionStrategy::North,
    )
});

pub static EDGE_ROUTING_SELF_LOOP_ORDERING_PROPERTY: LazyLock<Property<SelfLoopOrderingStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.edgeRouting.selfLoopOrdering",
            SelfLoopOrderingStrategy::Stacked,
        )
    });

pub static COMPACTION_POST_COMPACTION_STRATEGY_PROPERTY: LazyLock<Property<GraphCompactionStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.compaction.postCompaction.strategy",
            GraphCompactionStrategy::None,
        )
    });

pub static COMPACTION_POST_COMPACTION_CONSTRAINTS_PROPERTY: LazyLock<
    Property<ConstraintCalculationStrategy>,
> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.compaction.postCompaction.constraints",
        ConstraintCalculationStrategy::Scanline,
    )
});

pub static COMPACTION_CONNECTED_COMPONENTS_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.compaction.connectedComponents",
            false,
        )
    });

pub static HIGH_DEGREE_NODES_TREATMENT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.highDegreeNodes.treatment",
            false,
        )
    });

pub static HIGH_DEGREE_NODES_THRESHOLD_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.highDegreeNodes.threshold",
            16,
        )
    });

pub static HIGH_DEGREE_NODES_TREE_HEIGHT_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.highDegreeNodes.treeHeight",
            5,
        )
    });

pub static DIRECTION_CONGRUENCY_PROPERTY: LazyLock<Property<DirectionCongruency>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.directionCongruency",
            DirectionCongruency::ReadingDirection,
        )
    });

pub static FEEDBACK_EDGES_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.layered.feedbackEdges", false));

pub static MERGE_HIERARCHY_EDGES_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default("org.eclipse.elk.alg.layered.mergeHierarchyEdges", true)
    });

pub static ALLOW_NON_FLOW_PORTS_TO_SWITCH_SIDES_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.allowNonFlowPortsToSwitchSides",
            false,
        )
    });

pub static PORT_SORTING_STRATEGY_PROPERTY: LazyLock<Property<PortSortingStrategy>> =
    LazyLock::new(|| {
        ElkReflect::register(
            Some(|| PortSortingStrategy::InputOrder),
            Some(|v: &PortSortingStrategy| *v),
        );
        Property::with_default(
            "org.eclipse.elk.alg.layered.portSortingStrategy",
            PortSortingStrategy::InputOrder,
        )
    });

pub static THOROUGHNESS_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.layered.thoroughness", 7));

pub static UNNECESSARY_BENDPOINTS_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.layered.unnecessaryBendpoints", false));

pub static GENERATE_POSITION_AND_LAYER_IDS_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.generatePositionAndLayerIds",
            false,
        )
    });

pub static EDGE_LABELS_SIDE_SELECTION_PROPERTY: LazyLock<Property<EdgeLabelSideSelection>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.edgeLabels.sideSelection",
            EdgeLabelSideSelection::SmartDown,
        )
    });

pub static EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY_PROPERTY: LazyLock<
    Property<CenterEdgeLabelPlacementStrategy>,
> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.layered.edgeLabels.centerLabelPlacementStrategy",
        CenterEdgeLabelPlacementStrategy::MedianLayer,
    )
});

pub static MERGE_EDGES_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.layered.mergeEdges", false));

pub static INTERACTIVE_REFERENCE_POINT_PROPERTY: LazyLock<Property<InteractiveReferencePoint>> =
    LazyLock::new(|| {
        ElkReflect::register(
            Some(|| InteractiveReferencePoint::Center),
            Some(|v: &InteractiveReferencePoint| *v),
        );
        Property::with_default(
            "org.eclipse.elk.alg.layered.interactiveReferencePoint",
            InteractiveReferencePoint::Center,
        )
    });

pub static CONSIDER_MODEL_ORDER_STRATEGY_PROPERTY: LazyLock<Property<OrderingStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.considerModelOrder.strategy",
            OrderingStrategy::None,
        )
    });

pub static CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.considerModelOrder.portModelOrder",
            false,
        )
    });

pub static CONSIDER_MODEL_ORDER_NO_MODEL_ORDER_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.considerModelOrder.noModelOrder",
            false,
        )
    });

pub static CONSIDER_MODEL_ORDER_COMPONENTS_PROPERTY: LazyLock<Property<ComponentOrderingStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.considerModelOrder.components",
            ComponentOrderingStrategy::None,
        )
    });

pub static CONSIDER_MODEL_ORDER_LONG_EDGE_STRATEGY_PROPERTY: LazyLock<Property<LongEdgeOrderingStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.considerModelOrder.longEdgeStrategy",
            LongEdgeOrderingStrategy::DummyNodeOver,
        )
    });

pub static CONSIDER_MODEL_ORDER_CROSSING_COUNTER_NODE_INFLUENCE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.considerModelOrder.crossingCounterNodeInfluence",
            0.0,
        )
    });

pub static CONSIDER_MODEL_ORDER_CROSSING_COUNTER_PORT_INFLUENCE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.considerModelOrder.crossingCounterPortInfluence",
            0.0,
        )
    });

pub static GROUP_MODEL_ORDER_CYCLE_BREAKING_ID_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.considerModelOrder.groupModelOrder.cycleBreakingId",
            0,
        )
    });

pub static GROUP_MODEL_ORDER_CROSSING_MINIMIZATION_ID_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.considerModelOrder.groupModelOrder.crossingMinimizationId",
            0,
        )
    });

pub static GROUP_MODEL_ORDER_COMPONENT_GROUP_ID_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.considerModelOrder.groupModelOrder.componentGroupId",
            0,
        )
    });

pub static GROUP_MODEL_ORDER_CB_GROUP_ORDER_STRATEGY_PROPERTY: LazyLock<Property<GroupOrderStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.considerModelOrder.groupModelOrder.cbGroupOrderStrategy",
            GroupOrderStrategy::OnlyWithinGroup,
        )
    });

pub static GROUP_MODEL_ORDER_CB_PREFERRED_SOURCE_ID_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| {
        Property::new(
            "org.eclipse.elk.alg.layered.considerModelOrder.groupModelOrder.cbPreferredSourceId",
        )
    });

pub static GROUP_MODEL_ORDER_CB_PREFERRED_TARGET_ID_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| {
        Property::new(
            "org.eclipse.elk.alg.layered.considerModelOrder.groupModelOrder.cbPreferredTargetId",
        )
    });

pub static GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY_PROPERTY: LazyLock<Property<GroupOrderStrategy>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.considerModelOrder.groupModelOrder.cmGroupOrderStrategy",
            GroupOrderStrategy::OnlyWithinGroup,
        )
    });

pub static GROUP_MODEL_ORDER_CM_ENFORCED_GROUP_ORDERS_PROPERTY: LazyLock<Property<Vec<i32>>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.layered.considerModelOrder.groupModelOrder.cmEnforcedGroupOrders",
            vec![1, 2, 6, 7, 10, 11],
        )
    });

impl LayeredOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.layered";

    pub const SPACING_BASE_VALUE: &'static LazyLock<Property<f64>> = &SPACING_BASE_VALUE_PROPERTY;
    pub const SPACING_EDGE_NODE_BETWEEN_LAYERS: &'static LazyLock<Property<f64>> =
        &SPACING_EDGE_NODE_BETWEEN_LAYERS_PROPERTY;
    pub const SPACING_EDGE_EDGE_BETWEEN_LAYERS: &'static LazyLock<Property<f64>> =
        &SPACING_EDGE_EDGE_BETWEEN_LAYERS_PROPERTY;
    pub const SPACING_NODE_NODE_BETWEEN_LAYERS: &'static LazyLock<Property<f64>> =
        &SPACING_NODE_NODE_BETWEEN_LAYERS_PROPERTY;
    pub const SPACING_COMMENT_COMMENT: &'static LazyLock<Property<f64>> =
        CoreOptions::SPACING_COMMENT_COMMENT;
    pub const SPACING_COMMENT_NODE: &'static LazyLock<Property<f64>> =
        CoreOptions::SPACING_COMMENT_NODE;
    pub const SPACING_COMPONENT_COMPONENT: &'static LazyLock<Property<f64>> =
        CoreOptions::SPACING_COMPONENT_COMPONENT;
    pub const SPACING_EDGE_EDGE: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_EDGE_EDGE;
    pub const SPACING_EDGE_LABEL: &'static LazyLock<Property<f64>> =
        CoreOptions::SPACING_EDGE_LABEL;
    pub const SPACING_EDGE_NODE: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_EDGE_NODE;
    pub const SPACING_LABEL_LABEL: &'static LazyLock<Property<f64>> =
        CoreOptions::SPACING_LABEL_LABEL;
    pub const SPACING_LABEL_NODE: &'static LazyLock<Property<f64>> =
        CoreOptions::SPACING_LABEL_NODE;
    pub const SPACING_LABEL_PORT_HORIZONTAL: &'static LazyLock<Property<f64>> =
        CoreOptions::SPACING_LABEL_PORT_HORIZONTAL;
    pub const SPACING_LABEL_PORT_VERTICAL: &'static LazyLock<Property<f64>> =
        CoreOptions::SPACING_LABEL_PORT_VERTICAL;
    pub const SPACING_NODE_NODE: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_NODE_NODE;
    pub const SPACING_NODE_SELF_LOOP: &'static LazyLock<Property<f64>> =
        CoreOptions::SPACING_NODE_SELF_LOOP;
    pub const SPACING_PORT_PORT: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_PORT_PORT;
    pub const SPACING_INDIVIDUAL: &'static LazyLock<Property<IndividualSpacings>> =
        CoreOptions::SPACING_INDIVIDUAL;
    pub const SPACING_PORTS_SURROUNDING: &'static LazyLock<Property<ElkMargin>> =
        CoreOptions::SPACING_PORTS_SURROUNDING;

    pub const PRIORITY: &'static LazyLock<Property<i32>> = CoreOptions::PRIORITY;
    pub const PRIORITY_DIRECTION: &'static LazyLock<Property<i32>> = &PRIORITY_DIRECTION_PROPERTY;
    pub const PRIORITY_SHORTNESS: &'static LazyLock<Property<i32>> = &PRIORITY_SHORTNESS_PROPERTY;
    pub const PRIORITY_STRAIGHTNESS: &'static LazyLock<Property<i32>> = &PRIORITY_STRAIGHTNESS_PROPERTY;

    pub const WRAPPING_STRATEGY: &'static LazyLock<Property<WrappingStrategy>> =
        &WRAPPING_STRATEGY_PROPERTY;
    pub const WRAPPING_ADDITIONAL_EDGE_SPACING: &'static LazyLock<Property<f64>> =
        &WRAPPING_ADDITIONAL_EDGE_SPACING_PROPERTY;
    pub const WRAPPING_CORRECTION_FACTOR: &'static LazyLock<Property<f64>> =
        &WRAPPING_CORRECTION_FACTOR_PROPERTY;
    pub const WRAPPING_CUTTING_STRATEGY: &'static LazyLock<Property<CuttingStrategy>> =
        &WRAPPING_CUTTING_STRATEGY_PROPERTY;
    pub const WRAPPING_CUTTING_CUTS: &'static LazyLock<Property<Vec<i32>>> =
        &WRAPPING_CUTTING_CUTS_PROPERTY;
    pub const WRAPPING_CUTTING_MSD_FREEDOM: &'static LazyLock<Property<i32>> =
        &WRAPPING_CUTTING_MSD_FREEDOM_PROPERTY;
    pub const WRAPPING_VALIDIFY_STRATEGY: &'static LazyLock<Property<ValidifyStrategy>> =
        &WRAPPING_VALIDIFY_STRATEGY_PROPERTY;
    pub const WRAPPING_VALIDIFY_FORBIDDEN_INDICES: &'static LazyLock<Property<Vec<i32>>> =
        &WRAPPING_VALIDIFY_FORBIDDEN_INDICES_PROPERTY;
    pub const WRAPPING_MULTI_EDGE_IMPROVE_CUTS: &'static LazyLock<Property<bool>> =
        &WRAPPING_MULTI_EDGE_IMPROVE_CUTS_PROPERTY;
    pub const WRAPPING_MULTI_EDGE_DISTANCE_PENALTY: &'static LazyLock<Property<f64>> =
        &WRAPPING_MULTI_EDGE_DISTANCE_PENALTY_PROPERTY;
    pub const WRAPPING_MULTI_EDGE_IMPROVE_WRAPPED_EDGES: &'static LazyLock<Property<bool>> =
        &WRAPPING_MULTI_EDGE_IMPROVE_WRAPPED_EDGES_PROPERTY;

    pub const LAYER_UNZIPPING_STRATEGY: &'static LazyLock<Property<LayerUnzippingStrategy>> =
        &LAYER_UNZIPPING_STRATEGY_PROPERTY;
    pub const LAYER_UNZIPPING_MINIMIZE_EDGE_LENGTH: &'static LazyLock<Property<bool>> =
        &LAYER_UNZIPPING_MINIMIZE_EDGE_LENGTH_PROPERTY;
    pub const LAYER_UNZIPPING_LAYER_SPLIT: &'static LazyLock<Property<i32>> =
        &LAYER_UNZIPPING_LAYER_SPLIT_PROPERTY;
    pub const LAYER_UNZIPPING_RESET_ON_LONG_EDGES: &'static LazyLock<Property<bool>> =
        &LAYER_UNZIPPING_RESET_ON_LONG_EDGES_PROPERTY;

    pub const CYCLE_BREAKING_STRATEGY: &'static LazyLock<Property<CycleBreakingStrategy>> =
        &CYCLE_BREAKING_STRATEGY_PROPERTY;

    pub const LAYERING_STRATEGY: &'static LazyLock<Property<LayeringStrategy>> =
        &LAYERING_STRATEGY_PROPERTY;
    pub const LAYERING_LAYER_CONSTRAINT: &'static LazyLock<Property<LayerConstraint>> =
        &LAYERING_LAYER_CONSTRAINT_PROPERTY;
    pub const LAYERING_LAYER_CHOICE_CONSTRAINT: &'static LazyLock<Property<i32>> =
        &LAYERING_LAYER_CHOICE_CONSTRAINT_PROPERTY;
    pub const LAYERING_LAYER_ID: &'static LazyLock<Property<i32>> = &LAYERING_LAYER_ID_PROPERTY;
    pub const LAYERING_MIN_WIDTH_UPPER_BOUND_ON_WIDTH: &'static LazyLock<Property<i32>> =
        &LAYERING_MIN_WIDTH_UPPER_BOUND_ON_WIDTH_PROPERTY;
    pub const LAYERING_MIN_WIDTH_UPPER_LAYER_ESTIMATION_SCALING_FACTOR: &'static LazyLock<
        Property<i32>,
    > = &LAYERING_MIN_WIDTH_UPPER_LAYER_ESTIMATION_SCALING_FACTOR_PROPERTY;
    pub const LAYERING_NODE_PROMOTION_STRATEGY: &'static LazyLock<Property<NodePromotionStrategy>> =
        &LAYERING_NODE_PROMOTION_STRATEGY_PROPERTY;
    pub const LAYERING_NODE_PROMOTION_MAX_ITERATIONS: &'static LazyLock<Property<i32>> =
        &LAYERING_NODE_PROMOTION_MAX_ITERATIONS_PROPERTY;
    pub const LAYERING_COFFMAN_GRAHAM_LAYER_BOUND: &'static LazyLock<Property<i32>> =
        &LAYERING_COFFMAN_GRAHAM_LAYER_BOUND_PROPERTY;

    pub const CROSSING_MINIMIZATION_STRATEGY: &'static LazyLock<
        Property<CrossingMinimizationStrategy>,
    > = &CROSSING_MINIMIZATION_STRATEGY_PROPERTY;
    pub const CROSSING_MINIMIZATION_FORCE_NODE_MODEL_ORDER: &'static LazyLock<Property<bool>> =
        &CROSSING_MINIMIZATION_FORCE_NODE_MODEL_ORDER_PROPERTY;
    pub const CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS: &'static LazyLock<Property<f64>> =
        &CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS_PROPERTY;
    pub const CROSSING_MINIMIZATION_GREEDY_SWITCH_ACTIVATION_THRESHOLD: &'static LazyLock<
        Property<i32>,
    > = &CROSSING_MINIMIZATION_GREEDY_SWITCH_ACTIVATION_THRESHOLD_PROPERTY;
    pub const CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE: &'static LazyLock<Property<GreedySwitchType>> =
        &CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE_PROPERTY;
    pub const CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE: &'static LazyLock<
        Property<GreedySwitchType>,
    > = &CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE_PROPERTY;
    pub const CROSSING_MINIMIZATION_SEMI_INTERACTIVE: &'static LazyLock<Property<bool>> =
        &CROSSING_MINIMIZATION_SEMI_INTERACTIVE_PROPERTY;
    pub const CROSSING_MINIMIZATION_IN_LAYER_PRED_OF: &'static LazyLock<Property<String>> =
        &CROSSING_MINIMIZATION_IN_LAYER_PRED_OF_PROPERTY;
    pub const CROSSING_MINIMIZATION_IN_LAYER_SUCC_OF: &'static LazyLock<Property<String>> =
        &CROSSING_MINIMIZATION_IN_LAYER_SUCC_OF_PROPERTY;
    pub const CROSSING_MINIMIZATION_POSITION_CHOICE_CONSTRAINT: &'static LazyLock<Property<i32>> =
        &CROSSING_MINIMIZATION_POSITION_CHOICE_CONSTRAINT_PROPERTY;
    pub const CROSSING_MINIMIZATION_POSITION_ID: &'static LazyLock<Property<i32>> =
        &CROSSING_MINIMIZATION_POSITION_ID_PROPERTY;

    pub const NODE_PLACEMENT_STRATEGY: &'static LazyLock<Property<NodePlacementStrategy>> =
        &NODE_PLACEMENT_STRATEGY_PROPERTY;
    pub const NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES: &'static LazyLock<Property<bool>> =
        &NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES_PROPERTY;
    pub const NODE_PLACEMENT_BK_EDGE_STRAIGHTENING: &'static LazyLock<
        Property<EdgeStraighteningStrategy>,
    > = &NODE_PLACEMENT_BK_EDGE_STRAIGHTENING_PROPERTY;
    pub const NODE_PLACEMENT_BK_FIXED_ALIGNMENT: &'static LazyLock<Property<FixedAlignment>> =
        &NODE_PLACEMENT_BK_FIXED_ALIGNMENT_PROPERTY;
    pub const NODE_PLACEMENT_LINEAR_SEGMENTS_DEFLECTION_DAMPENING: &'static LazyLock<Property<f64>> =
        &NODE_PLACEMENT_LINEAR_SEGMENTS_DEFLECTION_DAMPENING_PROPERTY;
    pub const NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY: &'static LazyLock<
        Property<NodeFlexibility>,
    > = &NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_PROPERTY;
    pub const NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT: &'static LazyLock<
        Property<NodeFlexibility>,
    > = &NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT_PROPERTY;

    pub const EDGE_ROUTING_SPLINES_MODE: &'static LazyLock<Property<SplineRoutingMode>> =
        &EDGE_ROUTING_SPLINES_MODE_PROPERTY;
    pub const EDGE_ROUTING_SPLINES_SLOPPY_LAYER_SPACING_FACTOR: &'static LazyLock<Property<f64>> =
        &EDGE_ROUTING_SPLINES_SLOPPY_LAYER_SPACING_FACTOR_PROPERTY;
    pub const EDGE_ROUTING_POLYLINE_SLOPED_EDGE_ZONE_WIDTH: &'static LazyLock<Property<f64>> =
        &EDGE_ROUTING_POLYLINE_SLOPED_EDGE_ZONE_WIDTH_PROPERTY;
    pub const EDGE_ROUTING_SELF_LOOP_DISTRIBUTION: &'static LazyLock<
        Property<SelfLoopDistributionStrategy>,
    > = &EDGE_ROUTING_SELF_LOOP_DISTRIBUTION_PROPERTY;
    pub const EDGE_ROUTING_SELF_LOOP_ORDERING: &'static LazyLock<
        Property<SelfLoopOrderingStrategy>,
    > = &EDGE_ROUTING_SELF_LOOP_ORDERING_PROPERTY;

    pub const COMPACTION_POST_COMPACTION_STRATEGY: &'static LazyLock<Property<GraphCompactionStrategy>> =
        &COMPACTION_POST_COMPACTION_STRATEGY_PROPERTY;
    pub const COMPACTION_POST_COMPACTION_CONSTRAINTS: &'static LazyLock<
        Property<ConstraintCalculationStrategy>,
    > = &COMPACTION_POST_COMPACTION_CONSTRAINTS_PROPERTY;
    pub const COMPACTION_CONNECTED_COMPONENTS: &'static LazyLock<Property<bool>> =
        &COMPACTION_CONNECTED_COMPONENTS_PROPERTY;

    pub const HIGH_DEGREE_NODES_TREATMENT: &'static LazyLock<Property<bool>> =
        &HIGH_DEGREE_NODES_TREATMENT_PROPERTY;
    pub const HIGH_DEGREE_NODES_THRESHOLD: &'static LazyLock<Property<i32>> =
        &HIGH_DEGREE_NODES_THRESHOLD_PROPERTY;
    pub const HIGH_DEGREE_NODES_TREE_HEIGHT: &'static LazyLock<Property<i32>> =
        &HIGH_DEGREE_NODES_TREE_HEIGHT_PROPERTY;

    pub const DIRECTION_CONGRUENCY: &'static LazyLock<Property<DirectionCongruency>> =
        &DIRECTION_CONGRUENCY_PROPERTY;
    pub const FEEDBACK_EDGES: &'static LazyLock<Property<bool>> = &FEEDBACK_EDGES_PROPERTY;
    pub const MERGE_HIERARCHY_EDGES: &'static LazyLock<Property<bool>> =
        &MERGE_HIERARCHY_EDGES_PROPERTY;
    pub const ALLOW_NON_FLOW_PORTS_TO_SWITCH_SIDES: &'static LazyLock<Property<bool>> =
        &ALLOW_NON_FLOW_PORTS_TO_SWITCH_SIDES_PROPERTY;
    pub const PORT_SORTING_STRATEGY: &'static LazyLock<Property<PortSortingStrategy>> =
        &PORT_SORTING_STRATEGY_PROPERTY;
    pub const THOROUGHNESS: &'static LazyLock<Property<i32>> = &THOROUGHNESS_PROPERTY;
    pub const UNNECESSARY_BENDPOINTS: &'static LazyLock<Property<bool>> =
        &UNNECESSARY_BENDPOINTS_PROPERTY;
    pub const GENERATE_POSITION_AND_LAYER_IDS: &'static LazyLock<Property<bool>> =
        &GENERATE_POSITION_AND_LAYER_IDS_PROPERTY;

    pub const EDGE_LABELS_SIDE_SELECTION: &'static LazyLock<Property<EdgeLabelSideSelection>> =
        &EDGE_LABELS_SIDE_SELECTION_PROPERTY;
    pub const EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY: &'static LazyLock<
        Property<CenterEdgeLabelPlacementStrategy>,
    > = &EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY_PROPERTY;
    pub const EDGE_LABELS_PLACEMENT: &'static LazyLock<Property<EdgeLabelPlacement>> =
        CoreOptions::EDGE_LABELS_PLACEMENT;
    pub const EDGE_LABELS_INLINE: &'static LazyLock<Property<bool>> = CoreOptions::EDGE_LABELS_INLINE;
    pub const INTERACTIVE_REFERENCE_POINT: &'static LazyLock<Property<InteractiveReferencePoint>> =
        &INTERACTIVE_REFERENCE_POINT_PROPERTY;
    pub const ALIGNMENT: &'static LazyLock<Property<Alignment>> = CoreOptions::ALIGNMENT;
    pub const ASPECT_RATIO: &'static LazyLock<Property<f64>> = CoreOptions::ASPECT_RATIO;
    pub const COMMENT_BOX: &'static LazyLock<Property<bool>> = CoreOptions::COMMENT_BOX;
    pub const DIRECTION: &'static LazyLock<Property<Direction>> = CoreOptions::DIRECTION;
    pub const EDGE_ROUTING: &'static LazyLock<Property<EdgeRouting>> = CoreOptions::EDGE_ROUTING;
    pub const HYPERNODE: &'static LazyLock<Property<bool>> = CoreOptions::HYPERNODE;
    pub const HIERARCHY_HANDLING: &'static LazyLock<Property<HierarchyHandling>> =
        CoreOptions::HIERARCHY_HANDLING;
    pub const INTERACTIVE_LAYOUT: &'static LazyLock<Property<bool>> = CoreOptions::INTERACTIVE_LAYOUT;
    pub const JUNCTION_POINTS: &'static LazyLock<Property<KVectorChain>> =
        CoreOptions::JUNCTION_POINTS;
    pub const NODE_LABELS_PADDING: &'static LazyLock<Property<ElkPadding>> =
        CoreOptions::NODE_LABELS_PADDING;
    pub const NODE_LABELS_PLACEMENT: &'static LazyLock<Property<EnumSet<NodeLabelPlacement>>> =
        CoreOptions::NODE_LABELS_PLACEMENT;
    pub const NODE_SIZE_CONSTRAINTS: &'static LazyLock<Property<EnumSet<SizeConstraint>>> =
        CoreOptions::NODE_SIZE_CONSTRAINTS;
    pub const NODE_SIZE_MINIMUM: &'static LazyLock<Property<KVector>> =
        CoreOptions::NODE_SIZE_MINIMUM;
    pub const POSITION: &'static LazyLock<Property<KVector>> = CoreOptions::POSITION;
    pub const SEPARATE_CONNECTED_COMPONENTS: &'static LazyLock<Property<bool>> =
        CoreOptions::SEPARATE_CONNECTED_COMPONENTS;
    pub const TOPDOWN_NODE_TYPE: &'static LazyLock<Property<TopdownNodeTypes>> =
        CoreOptions::TOPDOWN_NODE_TYPE;
    pub const PORT_ANCHOR: &'static LazyLock<
        Property<org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector>,
    > = CoreOptions::PORT_ANCHOR;
    pub const PORT_BORDER_OFFSET: &'static LazyLock<Property<f64>> = CoreOptions::PORT_BORDER_OFFSET;
    pub const PORT_CONSTRAINTS: &'static LazyLock<Property<PortConstraints>> =
        CoreOptions::PORT_CONSTRAINTS;
    pub const PORT_INDEX: &'static LazyLock<Property<i32>> = CoreOptions::PORT_INDEX;
    pub const PORT_SIDE: &'static LazyLock<
        Property<org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide>,
    > = CoreOptions::PORT_SIDE;
    pub const RANDOM_SEED: &'static LazyLock<Property<i32>> = CoreOptions::RANDOM_SEED;
    pub const MERGE_EDGES: &'static LazyLock<Property<bool>> = &MERGE_EDGES_PROPERTY;

    pub const CONSIDER_MODEL_ORDER_STRATEGY: &'static LazyLock<Property<OrderingStrategy>> =
        &CONSIDER_MODEL_ORDER_STRATEGY_PROPERTY;
    pub const CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER: &'static LazyLock<Property<bool>> =
        &CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER_PROPERTY;
    pub const CONSIDER_MODEL_ORDER_NO_MODEL_ORDER: &'static LazyLock<Property<bool>> =
        &CONSIDER_MODEL_ORDER_NO_MODEL_ORDER_PROPERTY;
    pub const CONSIDER_MODEL_ORDER_COMPONENTS: &'static LazyLock<
        Property<ComponentOrderingStrategy>,
    > = &CONSIDER_MODEL_ORDER_COMPONENTS_PROPERTY;
    pub const CONSIDER_MODEL_ORDER_LONG_EDGE_STRATEGY: &'static LazyLock<
        Property<LongEdgeOrderingStrategy>,
    > = &CONSIDER_MODEL_ORDER_LONG_EDGE_STRATEGY_PROPERTY;
    pub const CONSIDER_MODEL_ORDER_CROSSING_COUNTER_NODE_INFLUENCE: &'static LazyLock<Property<f64>> =
        &CONSIDER_MODEL_ORDER_CROSSING_COUNTER_NODE_INFLUENCE_PROPERTY;
    pub const CONSIDER_MODEL_ORDER_CROSSING_COUNTER_PORT_INFLUENCE: &'static LazyLock<Property<f64>> =
        &CONSIDER_MODEL_ORDER_CROSSING_COUNTER_PORT_INFLUENCE_PROPERTY;

    pub const GROUP_MODEL_ORDER_CYCLE_BREAKING_ID: &'static LazyLock<Property<i32>> =
        &GROUP_MODEL_ORDER_CYCLE_BREAKING_ID_PROPERTY;
    pub const GROUP_MODEL_ORDER_CROSSING_MINIMIZATION_ID: &'static LazyLock<Property<i32>> =
        &GROUP_MODEL_ORDER_CROSSING_MINIMIZATION_ID_PROPERTY;
    pub const GROUP_MODEL_ORDER_COMPONENT_GROUP_ID: &'static LazyLock<Property<i32>> =
        &GROUP_MODEL_ORDER_COMPONENT_GROUP_ID_PROPERTY;
    pub const GROUP_MODEL_ORDER_CB_GROUP_ORDER_STRATEGY: &'static LazyLock<Property<GroupOrderStrategy>> =
        &GROUP_MODEL_ORDER_CB_GROUP_ORDER_STRATEGY_PROPERTY;
    pub const GROUP_MODEL_ORDER_CB_PREFERRED_SOURCE_ID: &'static LazyLock<Property<i32>> =
        &GROUP_MODEL_ORDER_CB_PREFERRED_SOURCE_ID_PROPERTY;
    pub const GROUP_MODEL_ORDER_CB_PREFERRED_TARGET_ID: &'static LazyLock<Property<i32>> =
        &GROUP_MODEL_ORDER_CB_PREFERRED_TARGET_ID_PROPERTY;
    pub const GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY: &'static LazyLock<Property<GroupOrderStrategy>> =
        &GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY_PROPERTY;
    pub const GROUP_MODEL_ORDER_CM_ENFORCED_GROUP_ORDERS: &'static LazyLock<Property<Vec<i32>>> =
        &GROUP_MODEL_ORDER_CM_ENFORCED_GROUP_ORDERS_PROPERTY;
}
