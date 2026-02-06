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

use crate::org::eclipse::elk::alg::force::stress::Dimension;

use super::StressOptions;

pub struct StressMetaDataProvider;

impl ILayoutMetaDataProvider for StressMetaDataProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        init_reflect();
        register_algorithm(registry);
        register_options(registry);
        register_supports(registry);
    }
}

const TARGET_PARENTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Parents];
const TARGET_NODES: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Nodes];
const TARGET_PARENTS_EDGES: [LayoutOptionTarget; 2] = [LayoutOptionTarget::Parents, LayoutOptionTarget::Edges];

fn register_algorithm(registry: &mut dyn LayoutMetaDataRegistry) {
    let mut data = LayoutAlgorithmData::new(StressOptions::ALGORITHM_ID);
    data.set_name("ELK Stress")
        .set_description(concat!(
            "Minimizes the stress within a layout using stress majorization. ",
            "Stress exists if the euclidean distance between a pair of ",
            "nodes doesn't match their graph theoretic distance, that is, ",
            "the shortest path between the two nodes. ",
            "The method allows to specify individual edge lengths."
        ))
        .set_category_id(Some("org.eclipse.elk.force"))
        .set_preview_image_path(Some("images/stress_layout.png"));
    registry.register_algorithm(data);
}

fn register_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        StressOptions::FIXED,
        LayoutOptionType::Boolean,
        "Fixed Position",
        "Prevent that the node is moved by the layout algorithm.",
        &TARGET_NODES,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        StressOptions::DESIRED_EDGE_LENGTH,
        LayoutOptionType::Double,
        "Desired Edge Length",
        "Either specified for parent nodes or for individual edges, where the latter takes higher precedence.",
        &TARGET_PARENTS_EDGES,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        StressOptions::DIMENSION,
        LayoutOptionType::Enum,
        "Layout Dimension",
        "Dimensions that are permitted to be altered during layout.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        StressOptions::EPSILON,
        LayoutOptionType::Double,
        "Stress Epsilon",
        "Termination criterion for the iterative process.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        StressOptions::ITERATION_LIMIT,
        LayoutOptionType::Int,
        "Iteration Limit",
        "Maximum number of performed iterations. Takes higher precedence than 'epsilon'.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
}

fn register_supports(registry: &mut dyn LayoutMetaDataRegistry) {
    fn arc_any<T: Any + Send + Sync>(value: T) -> Option<Arc<dyn Any + Send + Sync>> {
        Some(Arc::new(value))
    }

    let algo = StressOptions::ALGORITHM_ID;
    registry.add_option_support(algo, CoreOptions::INTERACTIVE.id(), None);
    registry.add_option_support(algo, CoreOptions::EDGE_LABELS_INLINE.id(), arc_any(true));
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_CONSTRAINTS.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_MINIMUM.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_SIZE_OPTIONS.id(), None);
    registry.add_option_support(algo, CoreOptions::NODE_LABELS_PLACEMENT.id(), None);
    registry.add_option_support(algo, CoreOptions::OMIT_NODE_MICRO_LAYOUT.id(), None);
    registry.add_option_support(algo, CoreOptions::PORT_LABELS_PLACEMENT.id(), None);
    registry.add_option_support(algo, StressOptions::FIXED.id(), None);
    registry.add_option_support(algo, StressOptions::DIMENSION.id(), None);
    registry.add_option_support(algo, StressOptions::EPSILON.id(), None);
    registry.add_option_support(algo, StressOptions::ITERATION_LIMIT.id(), None);
    registry.add_option_support(algo, StressOptions::DESIRED_EDGE_LENGTH.id(), None);
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
        ElkReflect::register(Some(|| Dimension::XY), Some(|v: &Dimension| *v));
    });
}
