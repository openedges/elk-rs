use std::sync::{Arc, LazyLock};

use org_eclipse_elk_core::org::eclipse::elk::core::alg::AlgorithmAssembler;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::labels::LabelManagementOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, Random};

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LGraphRef, LGraphUtil, LNodeRef};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{
    CrossingMinimizationStrategy, GraphCompactionStrategy, GraphProperties, GreedySwitchType,
    InternalProperties, LayerUnzippingStrategy, LayeredOptions, NodePromotionStrategy,
    OrderingStrategy, Spacings, WrappingStrategy,
};
use crate::org::eclipse::elk::alg::layered::p5edges::edge_router_factory::EdgeRouterFactory;
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

const MIN_EDGE_SPACING: f64 = 2.0;

static DISABLE_GREEDY_SWITCH: LazyLock<bool> =
    LazyLock::new(|| std::env::var("ELK_DISABLE_GREEDY_SWITCH").is_ok());

static BASELINE_PROCESSING_CONFIGURATION: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .add_before(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::InnermostNodeMarginCalculator),
        )
        .add_before(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::LabelAndNodeSizeProcessor),
        )
        .add_before(
            LayeredPhases::P5EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::LayerSizeAndGraphHeightCalculator),
        )
        .add_after(
            LayeredPhases::P5EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::EndLabelSorter),
        );
    config
});

static LABEL_MANAGEMENT_ADDITIONS: LazyLock<LayoutProcessorConfiguration<LayeredPhases, LGraph>> =
    LazyLock::new(|| {
        let mut config = LayoutProcessorConfiguration::create();
        config
            .add_before(
                LayeredPhases::P4NodePlacement,
                Arc::new(IntermediateProcessorStrategy::CenterLabelManagementProcessor),
            )
            .add_before(
                LayeredPhases::P4NodePlacement,
                Arc::new(IntermediateProcessorStrategy::EndNodePortLabelManagementProcessor),
            );
        config
    });

static HIERARCHICAL_ADDITIONS: LazyLock<LayoutProcessorConfiguration<LayeredPhases, LGraph>> =
    LazyLock::new(|| {
        let mut config = LayoutProcessorConfiguration::create();
        config.add_after(
            LayeredPhases::P5EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::HierarchicalNodeResizer),
        );
        config
    });

pub struct GraphConfigurator {
    algorithm_assembler: AlgorithmAssembler<LayeredPhases, LGraph>,
}

impl GraphConfigurator {
    pub fn new() -> Self {
        GraphConfigurator {
            algorithm_assembler: AlgorithmAssembler::create(),
        }
    }

    pub fn prepare_graph_for_layout(&mut self, graph: &LGraphRef) {
        self.configure_graph_properties(graph);

        let mut graph_guard = match graph.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        let cycle_breaking = graph_guard
            .get_property(LayeredOptions::CYCLE_BREAKING_STRATEGY)
            .unwrap_or_default();
        let layering = graph_guard
            .get_property(LayeredOptions::LAYERING_STRATEGY)
            .unwrap_or_default();
        let crossing_minimization = graph_guard
            .get_property(LayeredOptions::CROSSING_MINIMIZATION_STRATEGY)
            .unwrap_or_default();
        let node_placement = graph_guard
            .get_property(LayeredOptions::NODE_PLACEMENT_STRATEGY)
            .unwrap_or_default();
        let edge_routing = graph_guard
            .get_property(LayeredOptions::EDGE_ROUTING)
            .unwrap_or(EdgeRouting::Undefined);

        self.algorithm_assembler.reset();
        self.algorithm_assembler
            .set_phase(LayeredPhases::P1CycleBreaking, Arc::new(cycle_breaking));
        self.algorithm_assembler
            .set_phase(LayeredPhases::P2Layering, Arc::new(layering));
        self.algorithm_assembler.set_phase(
            LayeredPhases::P3NodeOrdering,
            Arc::new(crossing_minimization),
        );
        self.algorithm_assembler
            .set_phase(LayeredPhases::P4NodePlacement, Arc::new(node_placement));
        self.algorithm_assembler.set_phase(
            LayeredPhases::P5EdgeRouting,
            EdgeRouterFactory::factory_for(edge_routing),
        );

        let processor_config =
            self.get_phase_independent_layout_processor_configuration(&mut graph_guard);
        self.algorithm_assembler
            .add_processor_configuration(&processor_config);

        let processors = self.algorithm_assembler.build(&*graph_guard);
        graph_guard.set_property(InternalProperties::PROCESSORS, Some(processors));
    }

    fn configure_graph_properties(&mut self, graph: &LGraphRef) {
        let (edge_spacing, direction, random_seed, edge_routing, favor_straightness) =
            if let Ok(mut graph_guard) = graph.lock() {
                (
                    graph_guard
                        .get_property(LayeredOptions::SPACING_EDGE_EDGE)
                        .unwrap_or(0.0),
                    graph_guard
                        .get_property(LayeredOptions::DIRECTION)
                        .unwrap_or(Direction::Undefined),
                    graph_guard
                        .get_property(LayeredOptions::RANDOM_SEED)
                        .unwrap_or(1),
                    graph_guard
                        .get_property(LayeredOptions::EDGE_ROUTING)
                        .unwrap_or(EdgeRouting::Undefined),
                    graph_guard.get_property(LayeredOptions::NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES),
                )
            } else {
                return;
            };

        if edge_spacing < MIN_EDGE_SPACING {
            if let Ok(mut graph_guard) = graph.lock() {
                graph_guard.set_property(LayeredOptions::SPACING_EDGE_EDGE, Some(MIN_EDGE_SPACING));
            }
        }

        if direction == Direction::Undefined {
            let inferred = LGraphUtil::get_direction(graph);
            if let Ok(mut graph_guard) = graph.lock() {
                graph_guard.set_property(LayeredOptions::DIRECTION, Some(inferred));
            }
        }

        let random = if random_seed == 0 {
            Random::default()
        } else {
            Random::new(random_seed as u64)
        };
        if let Ok(mut graph_guard) = graph.lock() {
            graph_guard.set_property(InternalProperties::RANDOM, Some(random));
        }

        if favor_straightness.is_none() {
            // Java layered metadata defaults edge routing to ORTHOGONAL for this algorithm.
            // In Rust, missing values are often still read as Undefined from copied graph properties,
            // while phase selection already falls back to Orthogonal for Undefined.
            // Treat Undefined as Orthogonal here as well so the BK favor-straight default is consistent.
            let favor = matches!(edge_routing, EdgeRouting::Orthogonal | EdgeRouting::Undefined);
            if let Ok(mut graph_guard) = graph.lock() {
                graph_guard.set_property(
                    LayeredOptions::NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES,
                    Some(favor),
                );
            }
        }

        self.copy_port_constraints(graph);

        let spacings = Spacings::new(graph);
        if let Ok(mut graph_guard) = graph.lock() {
            graph_guard.set_property(InternalProperties::SPACINGS, Some(spacings));
        }
    }

    fn copy_port_constraints(&self, graph: &LGraphRef) {
        let nodes: Vec<LNodeRef> = if let Ok(graph_guard) = graph.lock() {
            let mut result = graph_guard.layerless_nodes().clone();
            for layer in graph_guard.layers() {
                if let Ok(layer_guard) = layer.lock() {
                    result.extend(layer_guard.nodes().clone());
                }
            }
            result
        } else {
            return;
        };

        for node in nodes {
            self.copy_port_constraints_node(&node);
        }
    }

    fn copy_port_constraints_node(&self, node: &LNodeRef) {
        let nested = if let Ok(mut node_guard) = node.lock() {
            let original = node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS);
            node_guard.set_property(InternalProperties::ORIGINAL_PORT_CONSTRAINTS, original);
            node_guard.nested_graph()
        } else {
            None
        };

        if let Some(nested_graph) = nested {
            self.copy_port_constraints(&nested_graph);
        }
    }

    fn get_phase_independent_layout_processor_configuration(
        &self,
        graph: &mut LGraph,
    ) -> LayoutProcessorConfiguration<LayeredPhases, LGraph> {
        let graph_properties = graph
            .get_property(InternalProperties::GRAPH_PROPERTIES)
            .unwrap_or_else(EnumSet::none_of);

        let mut configuration =
            LayoutProcessorConfiguration::create_from(&BASELINE_PROCESSING_CONFIGURATION);

        if graph
            .get_property(LayeredOptions::HIERARCHY_HANDLING)
            .unwrap_or(HierarchyHandling::Inherit)
            == HierarchyHandling::IncludeChildren
        {
            configuration.add_all(&HIERARCHICAL_ADDITIONS);
        }

        if graph
            .get_property(LayeredOptions::FEEDBACK_EDGES)
            .unwrap_or(false)
        {
            configuration.add_before(
                LayeredPhases::P1CycleBreaking,
                Arc::new(IntermediateProcessorStrategy::PortSideProcessor),
            );
        } else {
            configuration.add_before(
                LayeredPhases::P3NodeOrdering,
                Arc::new(IntermediateProcessorStrategy::PortSideProcessor),
            );
        }

        if graph
            .get_property(LabelManagementOptions::LABEL_MANAGER)
            .is_some()
        {
            configuration.add_all(&LABEL_MANAGEMENT_ADDITIONS);
        }

        if graph
            .get_property(LayeredOptions::INTERACTIVE_LAYOUT)
            .unwrap_or(false)
            || graph
                .get_property(LayeredOptions::GENERATE_POSITION_AND_LAYER_IDS)
                .unwrap_or(false)
        {
            configuration.add_after(
                LayeredPhases::P5EdgeRouting,
                Arc::new(IntermediateProcessorStrategy::ConstraintsPostprocessor),
            );
        }

        match graph
            .get_property(LayeredOptions::DIRECTION)
            .unwrap_or(Direction::Right)
        {
            Direction::Left | Direction::Down | Direction::Up => {
                configuration
                    .add_before(
                        LayeredPhases::P1CycleBreaking,
                        Arc::new(IntermediateProcessorStrategy::DirectionPreprocessor),
                    )
                    .add_after(
                        LayeredPhases::P5EdgeRouting,
                        Arc::new(IntermediateProcessorStrategy::DirectionPostprocessor),
                    );
            }
            _ => {}
        }

        if graph_properties.contains(&GraphProperties::Comments) {
            configuration
                .add_before(
                    LayeredPhases::P1CycleBreaking,
                    Arc::new(IntermediateProcessorStrategy::CommentPreprocessor),
                )
                .add_before(
                    LayeredPhases::P4NodePlacement,
                    Arc::new(IntermediateProcessorStrategy::CommentNodeMarginCalculator),
                )
                .add_after(
                    LayeredPhases::P5EdgeRouting,
                    Arc::new(IntermediateProcessorStrategy::CommentPostprocessor),
                );
        }

        if graph
            .get_property(LayeredOptions::LAYERING_NODE_PROMOTION_STRATEGY)
            .unwrap_or(NodePromotionStrategy::None)
            != NodePromotionStrategy::None
        {
            configuration.add_before(
                LayeredPhases::P3NodeOrdering,
                Arc::new(IntermediateProcessorStrategy::NodePromotion),
            );
        }

        if graph_properties.contains(&GraphProperties::Partitions) {
            configuration
                .add_before(
                    LayeredPhases::P1CycleBreaking,
                    Arc::new(IntermediateProcessorStrategy::PartitionPreprocessor),
                )
                .add_before(
                    LayeredPhases::P2Layering,
                    Arc::new(IntermediateProcessorStrategy::PartitionMidprocessor),
                )
                .add_before(
                    LayeredPhases::P3NodeOrdering,
                    Arc::new(IntermediateProcessorStrategy::PartitionPostprocessor),
                );
        }

        if graph
            .get_property(LayeredOptions::COMPACTION_POST_COMPACTION_STRATEGY)
            .unwrap_or(GraphCompactionStrategy::None)
            != GraphCompactionStrategy::None
            && graph
                .get_property(LayeredOptions::EDGE_ROUTING)
                .unwrap_or(EdgeRouting::Undefined)
                != EdgeRouting::Polyline
        {
            configuration.add_after(
                LayeredPhases::P5EdgeRouting,
                Arc::new(IntermediateProcessorStrategy::HorizontalCompactor),
            );
        }

        if graph
            .get_property(LayeredOptions::HIGH_DEGREE_NODES_TREATMENT)
            .unwrap_or(false)
        {
            configuration.add_before(
                LayeredPhases::P3NodeOrdering,
                Arc::new(IntermediateProcessorStrategy::HighDegreeNodeLayerProcessor),
            );
        }

        if graph
            .get_property(LayeredOptions::CROSSING_MINIMIZATION_SEMI_INTERACTIVE)
            .unwrap_or(false)
        {
            configuration.add_before(
                LayeredPhases::P3NodeOrdering,
                Arc::new(IntermediateProcessorStrategy::SemiInteractiveCrossminProcessor),
            );
        }

        if Self::activate_greedy_switch_for(graph) && !*DISABLE_GREEDY_SWITCH {
            let greedy_type = if Self::is_hierarchical_layout(graph) {
                graph
                    .get_property(
                        LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE,
                    )
                    .unwrap_or(GreedySwitchType::Off)
            } else {
                graph
                    .get_property(LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE)
                    .unwrap_or(GreedySwitchType::Off)
            };
            let internal_strategy = if greedy_type == GreedySwitchType::OneSided {
                IntermediateProcessorStrategy::OneSidedGreedySwitch
            } else {
                IntermediateProcessorStrategy::TwoSidedGreedySwitch
            };
            configuration.add_before(LayeredPhases::P4NodePlacement, Arc::new(internal_strategy));
        }

        if graph
            .get_property(LayeredOptions::LAYER_UNZIPPING_STRATEGY)
            .unwrap_or(LayerUnzippingStrategy::None)
            == LayerUnzippingStrategy::Alternating
        {
            configuration.add_before(
                LayeredPhases::P4NodePlacement,
                Arc::new(IntermediateProcessorStrategy::AlternatingLayerUnzipper),
            );
        }

        match graph
            .get_property(LayeredOptions::WRAPPING_STRATEGY)
            .unwrap_or(WrappingStrategy::Off)
        {
            WrappingStrategy::SingleEdge => {
                configuration.add_before(
                    LayeredPhases::P4NodePlacement,
                    Arc::new(IntermediateProcessorStrategy::SingleEdgeGraphWrapper),
                );
            }
            WrappingStrategy::MultiEdge => {
                configuration
                    .add_before(
                        LayeredPhases::P3NodeOrdering,
                        Arc::new(IntermediateProcessorStrategy::BreakingPointInserter),
                    )
                    .add_before(
                        LayeredPhases::P4NodePlacement,
                        Arc::new(IntermediateProcessorStrategy::BreakingPointProcessor),
                    )
                    .add_after(
                        LayeredPhases::P5EdgeRouting,
                        Arc::new(IntermediateProcessorStrategy::BreakingPointRemover),
                    );
            }
            _ => {}
        }

        if graph
            .get_property(LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY)
            .unwrap_or(OrderingStrategy::None)
            != OrderingStrategy::None
        {
            configuration.add_before(
                LayeredPhases::P3NodeOrdering,
                Arc::new(IntermediateProcessorStrategy::SortByInputOrderOfModel),
            );
        }

        // Grid snap — always registered, processors self-disable when grid_size <= 0
        configuration.add_before(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::GridSnapSizeProcessor),
        );
        configuration.add_after(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::GridSnapPositionProcessor),
        );
        configuration.add_after(
            LayeredPhases::P5EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::GridSnapGraphSizeProcessor),
        );

        configuration
    }

    fn activate_greedy_switch_for(graph: &mut LGraph) -> bool {
        if Self::is_hierarchical_layout(graph) {
            let greedy_type = graph
                .get_property(LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE)
                .unwrap_or(GreedySwitchType::Off);
            return graph.parent_node().is_none() && greedy_type != GreedySwitchType::Off;
        }

        let greedy_type = graph
            .get_property(LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE)
            .unwrap_or(GreedySwitchType::Off);
        let interactive_crossmin = graph
            .get_property(LayeredOptions::CROSSING_MINIMIZATION_SEMI_INTERACTIVE)
            .unwrap_or(false)
            || graph
                .get_property(LayeredOptions::CROSSING_MINIMIZATION_STRATEGY)
                .unwrap_or(CrossingMinimizationStrategy::LayerSweep)
                == CrossingMinimizationStrategy::Interactive;
        let activation_threshold = graph
            .get_property(LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_ACTIVATION_THRESHOLD)
            .unwrap_or(0);
        let graph_size = graph.layerless_nodes().len() as i32;

        !interactive_crossmin
            && greedy_type != GreedySwitchType::Off
            && (activation_threshold == 0 || activation_threshold > graph_size)
    }

    fn is_hierarchical_layout(graph: &mut LGraph) -> bool {
        graph
            .get_property(LayeredOptions::HIERARCHY_HANDLING)
            .unwrap_or(HierarchyHandling::Inherit)
            == HierarchyHandling::IncludeChildren
    }
}

impl Default for GraphConfigurator {
    fn default() -> Self {
        Self::new()
    }
}

// Edge routing phase factory replaced by p5 edge router factory.
