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
    CenterEdgeLabelPlacementStrategy, CuttingStrategy, EdgeLabelSideSelection,
    LayerUnzippingStrategy, LayeredOptions, ValidifyStrategy, WrappingStrategy,
};

pub struct LayeredMetaDataProvider;

impl ILayoutMetaDataProvider for LayeredMetaDataProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        init_reflect();
        register_spacing_options(registry);
        register_priority_options(registry);
        register_wrapping_options(registry);
        register_layer_unzipping_options(registry);
        register_edge_label_options(registry);
    }
}

const TARGET_PARENTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Parents];
const TARGET_EDGES: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Edges];
const TARGET_NODES: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Nodes];
const TARGET_PARENTS_LABELS: [LayoutOptionTarget; 2] =
    [LayoutOptionTarget::Parents, LayoutOptionTarget::Labels];

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
        ElkReflect::register(Some(|| WrappingStrategy::Off), Some(|v: &WrappingStrategy| *v));
        ElkReflect::register(Some(|| CuttingStrategy::Msd), Some(|v: &CuttingStrategy| *v));
        ElkReflect::register(Some(|| ValidifyStrategy::Greedy), Some(|v: &ValidifyStrategy| *v));
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
    });
}
