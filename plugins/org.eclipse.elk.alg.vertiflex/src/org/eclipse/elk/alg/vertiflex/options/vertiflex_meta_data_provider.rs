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

use crate::org::eclipse::elk::alg::vertiflex::EdgeRoutingStrategy;
use crate::org::eclipse::elk::alg::vertiflex::options::VertiFlexOptions;

pub struct VertiFlexMetaDataProvider;

impl ILayoutMetaDataProvider for VertiFlexMetaDataProvider {
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
    let mut data = LayoutAlgorithmData::new(VertiFlexOptions::ALGORITHM_ID);
    data.set_name("ELK VertiFlex")
        .set_description(concat!(
            "Tree layout algorithm that allows defining set vertical positions for nodes ",
            "rather than automatically placing nodes on levels according to their topology."
        ))
        .set_category_id(Some("org.eclipse.elk.tree"));
    data.add_supported_feature(GraphFeature::MultiEdges);
    data.add_supported_feature(GraphFeature::EdgeLabels);
    registry.register_algorithm(data);
}

fn register_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        VertiFlexOptions::VERTICAL_CONSTRAINT,
        LayoutOptionType::Double,
        "Fixed vertical position",
        "The Y position that the node should be fixed at.",
        &TARGET_NODES,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        VertiFlexOptions::LAYOUT_STRATEGY,
        LayoutOptionType::Enum,
        "Edge layout strategy",
        concat!(
            "Strategy for the layout of the children. 'straight' for straight line drawings, 'bend' for a possible bend. ",
            "When straight edges are prioritized the nodes will be reordered in order to guarantee that straight edges are ",
            "possible. If bend points are enabled on the other hand, the given model order of the nodes is maintained and ",
            "bend points are introduced to prevent edge node overlaps."
        ),
        &TARGET_NODES,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        VertiFlexOptions::LAYER_DISTANCE,
        LayoutOptionType::Double,
        "Layer distance",
        "The distance to use between nodes of different layers if no vertical constraints are set.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );
    register_option(
        registry,
        VertiFlexOptions::CONSIDER_NODE_MODEL_ORDER,
        LayoutOptionType::Boolean,
        "Consider node model order",
        "Consider node model as a secondary criterion when using straight line routing.",
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

    let algo = VertiFlexOptions::ALGORITHM_ID;
    registry.add_option_support(algo, CoreOptions::SPACING_NODE_NODE.id(), arc_any(20.0_f64));
    registry.add_option_support(
        algo,
        CoreOptions::PADDING.id(),
        arc_any(ElkPadding::with_any(5.0)),
    );
    registry.add_option_support(algo, CoreOptions::INTERACTIVE.id(), None);
    registry.add_option_support(algo, CoreOptions::PORT_CONSTRAINTS.id(), None);
    registry.add_option_support(algo, CoreOptions::EDGE_LABELS_INLINE.id(), arc_any(false));
    registry.add_option_support(algo, CoreOptions::OMIT_NODE_MICRO_LAYOUT.id(), None);
    registry.add_option_support(algo, CoreOptions::MARGINS.id(), None);

    registry.add_option_support(algo, VertiFlexOptions::VERTICAL_CONSTRAINT.id(), None);
    registry.add_option_support(algo, VertiFlexOptions::LAYOUT_STRATEGY.id(), None);
    registry.add_option_support(algo, VertiFlexOptions::LAYER_DISTANCE.id(), None);
    registry.add_option_support(algo, VertiFlexOptions::CONSIDER_NODE_MODEL_ORDER.id(), None);
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
        ElkReflect::register(
            Some(|| EdgeRoutingStrategy::Straight),
            Some(|v: &EdgeRoutingStrategy| *v),
        );
    });
}
