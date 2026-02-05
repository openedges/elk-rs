use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::sync::{Arc, LazyLock, OnceLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutAlgorithmData, LayoutMetaDataRegistry, LayoutOptionData,
    LayoutOptionTarget, LayoutOptionType, LayoutOptionVisibility,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::topdown_node_types::TopdownNodeTypes;

use crate::org::eclipse::elk::alg::topdownpacking::node_arrangement_strategy::NodeArrangementStrategy;
use crate::org::eclipse::elk::alg::topdownpacking::options::topdownpacking_options::TopdownpackingOptions;
use crate::org::eclipse::elk::alg::topdownpacking::whitespace_elimination_strategy::WhitespaceEliminationStrategy;

pub struct TopdownpackingMetaDataProvider;

impl ILayoutMetaDataProvider for TopdownpackingMetaDataProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        init_reflect();
        register_algorithm(registry);
        register_options(registry);
        register_supports(registry);
    }
}

const TARGET_PARENTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Parents];

fn register_algorithm(registry: &mut dyn LayoutMetaDataRegistry) {
    let mut data = LayoutAlgorithmData::new(TopdownpackingOptions::ALGORITHM_ID);
    data.set_name("ELK Top-down Packing")
        .set_description(concat!(
            "Places fixed-size boxes in a grid and expands them horizontally to fill whitespace. ",
            "Use as a standalone algorithm or as the layout for parallel top-down nodes.",
        ));
    registry.register_algorithm(data);
}

fn register_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        TopdownpackingOptions::NODE_ARRANGEMENT_STRATEGY,
        LayoutOptionType::Enum,
        "Node arrangement strategy",
        "Strategy for node arrangement. The strategy determines the size of the resulting graph.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("nodeArrangement"),
        None,
    );
    register_option(
        registry,
        TopdownpackingOptions::WHITESPACE_ELIMINATION_STRATEGY,
        LayoutOptionType::Enum,
        "Whitespace elimination strategy",
        "Strategy for whitespace elimination.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("whitespaceElimination"),
        None,
    );
}

fn register_supports(registry: &mut dyn LayoutMetaDataRegistry) {
    fn arc_any<T: Any + Send + Sync>(value: T) -> Option<Arc<dyn Any + Send + Sync>> {
        Some(Arc::new(value))
    }

    let algo = TopdownpackingOptions::ALGORITHM_ID;
    registry.add_option_support(algo, CoreOptions::PADDING.id(), None);
    registry.add_option_support(algo, CoreOptions::SPACING_NODE_NODE.id(), None);
    registry.add_option_support(algo, CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH.id(), None);
    registry.add_option_support(
        algo,
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO.id(),
        None,
    );
    registry.add_option_support(algo, CoreOptions::TOPDOWN_LAYOUT.id(), None);
    registry.add_option_support(
        algo,
        CoreOptions::TOPDOWN_NODE_TYPE.id(),
        arc_any(TopdownNodeTypes::ParallelNode),
    );
    registry.add_option_support(
        algo,
        TopdownpackingOptions::NODE_ARRANGEMENT_STRATEGY.id(),
        None,
    );
    registry.add_option_support(
        algo,
        TopdownpackingOptions::WHITESPACE_ELIMINATION_STRATEGY.id(),
        None,
    );
}

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
        ElkReflect::register(
            Some(|| NodeArrangementStrategy::LeftRightTopDownNodePlacer),
            Some(|v: &NodeArrangementStrategy| *v),
        );
        ElkReflect::register(
            Some(|| WhitespaceEliminationStrategy::BottomRowEqualWhitespaceEliminator),
            Some(|v: &WhitespaceEliminationStrategy| *v),
        );
    });
}
