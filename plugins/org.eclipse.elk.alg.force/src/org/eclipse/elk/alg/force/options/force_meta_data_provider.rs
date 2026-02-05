use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::sync::{Arc, LazyLock, OnceLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{GraphFeature, Property};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutAlgorithmData, LayoutMetaDataRegistry, LayoutOptionData,
    LayoutOptionTarget, LayoutOptionType, LayoutOptionVisibility,
};
use org_eclipse_elk_core::org::eclipse::elk::core::math::elk_padding::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::topdown_node_types::TopdownNodeTypes;
use org_eclipse_elk_core::org::eclipse::elk::core::util::ExclusiveBounds;

use super::{ForceModelStrategy, ForceOptions};

pub struct ForceMetaDataProvider;

impl ILayoutMetaDataProvider for ForceMetaDataProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        init_reflect();
        register_algorithm(registry);
        register_options(registry);
        register_supports(registry);
    }
}

const TARGET_PARENTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Parents];
const TARGET_EDGES: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Edges];

fn register_algorithm(registry: &mut dyn LayoutMetaDataRegistry) {
    let mut data = LayoutAlgorithmData::new(ForceOptions::ALGORITHM_ID);
    data.set_name("ELK Force")
        .set_description(concat!(
            "Force-based algorithm provided by the Eclipse Layout Kernel. Implements methods that ",
            "follow physical analogies by simulating forces that move the nodes into a balanced ",
            "distribution. Currently the original Eades model and the Fruchterman - Reingold model are ",
            "supported."
        ))
        .set_category_id(Some("force"))
        .set_preview_image_path(Some("images/force_layout.png"));
    data.add_supported_feature(GraphFeature::MultiEdges);
    data.add_supported_feature(GraphFeature::EdgeLabels);
    registry.register_algorithm(data);
}

fn register_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        ForceOptions::MODEL,
        LayoutOptionType::Enum,
        "Force Model",
        "Determines the model for force calculation.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        ForceOptions::ITERATIONS,
        LayoutOptionType::Int,
        "Iterations",
        "The number of iterations on the force model.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        Some(Arc::new(1_i32)),
    );
    register_option(
        registry,
        ForceOptions::REPULSIVE_POWER,
        LayoutOptionType::Int,
        "Repulsive Power",
        concat!(
            "Determines how many bend points are added to the edge; such bend points are regarded as ",
            "repelling particles in the force model"
        ),
        &TARGET_EDGES,
        LayoutOptionVisibility::Visible,
        None,
        Some(Arc::new(0_i32)),
    );
    register_option(
        registry,
        ForceOptions::TEMPERATURE,
        LayoutOptionType::Double,
        "FR Temperature",
        "The temperature is used as a scaling factor for particle displacements.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        Some(Arc::new(ExclusiveBounds::greater_than(0.0_f64))),
    );
    register_option(
        registry,
        ForceOptions::REPULSION,
        LayoutOptionType::Double,
        "Eades Repulsion",
        "Factor for repulsive forces in Eades' model.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        Some(Arc::new(ExclusiveBounds::greater_than(0.0_f64))),
    );
}

fn register_supports(registry: &mut dyn LayoutMetaDataRegistry) {
    fn arc_any<T: Any + Send + Sync>(value: T) -> Option<Arc<dyn Any + Send + Sync>> {
        Some(Arc::new(value))
    }

    let algo = ForceOptions::ALGORITHM_ID;
    registry.add_option_support(algo, CoreOptions::PRIORITY.id(), arc_any(1_i32));
    registry.add_option_support(algo, CoreOptions::SPACING_NODE_NODE.id(), arc_any(80.0_f64));
    registry.add_option_support(algo, CoreOptions::SPACING_EDGE_LABEL.id(), arc_any(5.0_f64));
    registry.add_option_support(algo, CoreOptions::ASPECT_RATIO.id(), arc_any(1.6_f64));
    registry.add_option_support(algo, CoreOptions::RANDOM_SEED.id(), arc_any(1_i32));
    registry.add_option_support(algo, CoreOptions::SEPARATE_CONNECTED_COMPONENTS.id(), arc_any(true));
    registry.add_option_support(algo, CoreOptions::PADDING.id(), arc_any(ElkPadding::with_any(50.0)));
    registry.add_option_support(algo, CoreOptions::INTERACTIVE.id(), None);
    registry.add_option_support(algo, CoreOptions::PORT_CONSTRAINTS.id(), None);
    registry.add_option_support(algo, CoreOptions::EDGE_LABELS_INLINE.id(), arc_any(false));
    registry.add_option_support(algo, CoreOptions::OMIT_NODE_MICRO_LAYOUT.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_OPTIONS.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_CONSTRAINTS.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_LABELS_PLACEMENT.id(), None);
    registry.add_option_support(algo, CoreOptions::PORT_LABELS_PLACEMENT.id(), None);
    registry.add_option_support(algo, ForceOptions::MODEL.id(), None);
    registry.add_option_support(algo, ForceOptions::TEMPERATURE.id(), None);
    registry.add_option_support(algo, ForceOptions::ITERATIONS.id(), None);
    registry.add_option_support(algo, ForceOptions::REPULSION.id(), None);
    registry.add_option_support(algo, ForceOptions::REPULSIVE_POWER.id(), None);
    registry.add_option_support(algo, CoreOptions::TOPDOWN_LAYOUT.id(), None);
    registry.add_option_support(algo, CoreOptions::TOPDOWN_SCALE_FACTOR.id(), None);
    registry.add_option_support(algo, CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH.id(), None);
    registry.add_option_support(algo, CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO.id(), None);
    registry.add_option_support(
        algo,
        CoreOptions::TOPDOWN_NODE_TYPE.id(),
        arc_any(TopdownNodeTypes::HierarchicalNode),
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
    if let Some(lower_bound) = lower_bound {
        builder = builder.lower_bound(Some(lower_bound));
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
        ElkReflect::register(Some(|| ForceModelStrategy::FruchtermanReingold), Some(|v: &ForceModelStrategy| *v));
    });
}
