use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::sync::{Arc, LazyLock, OnceLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{GraphFeature, Property};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutAlgorithmData, LayoutMetaDataRegistry, LayoutOptionData,
    LayoutOptionTarget, LayoutOptionType, LayoutOptionVisibility,
};
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::topdown_node_types::TopdownNodeTypes;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;

use super::{EdgeRoutingMode, GraphProperties, MrTreeOptions, OrderWeighting, TreeifyingOrder};

pub struct MrTreeMetaDataProvider;

impl ILayoutMetaDataProvider for MrTreeMetaDataProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        init_reflect();
        register_algorithm(registry);
        register_options(registry);
        register_supports(registry);
    }
}

const TARGET_PARENTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Parents];
const TARGET_NODES: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Nodes];

fn register_algorithm(registry: &mut dyn LayoutMetaDataRegistry) {
    let mut data = LayoutAlgorithmData::new(MrTreeOptions::ALGORITHM_ID);
    data.set_name("ELK Mr. Tree")
        .set_description(concat!(
            "Tree-based algorithm provided by the Eclipse Layout Kernel. Computes a spanning tree of ",
            "the input graph and arranges all nodes according to the resulting parent-children hierarchy. ",
            "I pity the fool who doesn't use Mr. Tree Layout."
        ))
        .set_category_id(Some("org.eclipse.elk.tree"))
        .set_preview_image_path(Some("images/mrtree_layout.png"));
    data.add_supported_feature(GraphFeature::Disconnected);
    registry.register_algorithm(data);
}

fn register_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        MrTreeOptions::COMPACTION,
        LayoutOptionType::Boolean,
        "Compaction",
        "Turns on tree compaction which decreases the size of the whole tree by placing nodes of multiple levels in one large level.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        MrTreeOptions::EDGE_END_TEXTURE_LENGTH,
        LayoutOptionType::Double,
        "Edge End Texture Length",
        "Should be set to the length of the texture at the end of an edge. This value can be used to improve the edge routing.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        Some(Arc::new(0.0_f64)),
    );
    register_option(
        registry,
        MrTreeOptions::TREE_LEVEL,
        LayoutOptionType::Int,
        "Tree Level",
        "The index for the tree level the node is in.",
        &TARGET_NODES,
        LayoutOptionVisibility::Visible,
        None,
        Some(Arc::new(0_i32)),
    );
    register_option(
        registry,
        MrTreeOptions::POSITION_CONSTRAINT,
        LayoutOptionType::Int,
        "Position Constraint",
        "When set to a positive number this option will force the algorithm to place the node to the specified position within the tree layer if weighting is set to constraint.",
        &TARGET_NODES,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        MrTreeOptions::WEIGHTING,
        LayoutOptionType::Enum,
        "Weighting of Nodes",
        "Which weighting to use when computing a node order.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        MrTreeOptions::EDGE_ROUTING_MODE,
        LayoutOptionType::Enum,
        "Edge Routing Mode",
        "Chooses an edge routing algorithm.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        MrTreeOptions::SEARCH_ORDER,
        LayoutOptionType::Enum,
        "Search Order",
        "Which search order to use when computing a spanning tree.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        MrTreeOptions::GRAPH_PROPERTIES,
        LayoutOptionType::Object,
        "Graph Properties",
        "Properties of the graph that can be used to skip unnecessary processing steps.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );
}

fn register_supports(registry: &mut dyn LayoutMetaDataRegistry) {
    fn arc_any<T: Any + Send + Sync>(value: T) -> Option<Arc<dyn Any + Send + Sync>> {
        Some(Arc::new(value))
    }

    let algo = MrTreeOptions::ALGORITHM_ID;
    registry.add_option_support(
        algo,
        CoreOptions::PADDING.id(),
        arc_any(ElkPadding::with_any(20.0)),
    );
    registry.add_option_support(algo, CoreOptions::SPACING_NODE_NODE.id(), arc_any(20.0_f64));
    registry.add_option_support(algo, CoreOptions::SPACING_EDGE_NODE.id(), arc_any(3.0_f64));
    registry.add_option_support(algo, CoreOptions::ASPECT_RATIO.id(), arc_any(1.6_f64));
    registry.add_option_support(algo, CoreOptions::PRIORITY.id(), arc_any(1_i32));
    registry.add_option_support(
        algo,
        CoreOptions::SEPARATE_CONNECTED_COMPONENTS.id(),
        arc_any(true),
    );
    registry.add_option_support(algo, CoreOptions::DEBUG_MODE.id(), None);
    registry.add_option_support(
        algo,
        CoreOptions::DIRECTION.id(),
        arc_any(Direction::Undefined),
    );
    registry.add_option_support(algo, CoreOptions::INTERACTIVE.id(), None);
    registry.add_option_support(algo, CoreOptions::INTERACTIVE_LAYOUT.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_CONSTRAINTS.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_MINIMUM.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_OPTIONS.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_LABELS_PLACEMENT.id(), None);
    registry.add_option_support(algo, CoreOptions::OMIT_NODE_MICRO_LAYOUT.id(), None);
    registry.add_option_support(algo, CoreOptions::PORT_LABELS_PLACEMENT.id(), None);
    registry.add_option_support(algo, CoreOptions::TOPDOWN_LAYOUT.id(), None);
    registry.add_option_support(algo, CoreOptions::TOPDOWN_SCALE_FACTOR.id(), None);
    registry.add_option_support(algo, CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH.id(), None);
    registry.add_option_support(
        algo,
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO.id(),
        None,
    );
    registry.add_option_support(
        algo,
        CoreOptions::TOPDOWN_NODE_TYPE.id(),
        arc_any(TopdownNodeTypes::HierarchicalNode),
    );
    registry.add_option_support(algo, MrTreeOptions::WEIGHTING.id(), None);
    registry.add_option_support(algo, MrTreeOptions::SEARCH_ORDER.id(), None);
    registry.add_option_support(algo, MrTreeOptions::EDGE_ROUTING_MODE.id(), None);
    registry.add_option_support(algo, MrTreeOptions::POSITION_CONSTRAINT.id(), None);
    registry.add_option_support(algo, MrTreeOptions::TREE_LEVEL.id(), None);
    registry.add_option_support(algo, MrTreeOptions::COMPACTION.id(), None);
    registry.add_option_support(algo, MrTreeOptions::EDGE_END_TEXTURE_LENGTH.id(), None);
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

fn init_reflect() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        ElkReflect::register(Some(EnumSet::<GraphProperties>::none_of), Some(|v: &EnumSet<GraphProperties>| v.clone()));
        ElkReflect::register(Some(|| OrderWeighting::ModelOrder), Some(|v: &OrderWeighting| *v));
        ElkReflect::register(Some(|| TreeifyingOrder::Dfs), Some(|v: &TreeifyingOrder| *v));
        ElkReflect::register(Some(|| EdgeRoutingMode::AvoidOverlap), Some(|v: &EdgeRoutingMode| *v));
    });
}
