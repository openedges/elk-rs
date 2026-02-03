use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::sync::{Arc, LazyLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutMetaDataRegistry, LayoutOptionData, LayoutOptionTarget,
    LayoutOptionType, LayoutOptionVisibility,
};

use super::LayeredOptions;

pub struct LayeredMetaDataProvider;

impl ILayoutMetaDataProvider for LayeredMetaDataProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        register_spacing_options(registry);
        register_priority_options(registry);
    }
}

const TARGET_PARENTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Parents];
const TARGET_EDGES: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Edges];

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
