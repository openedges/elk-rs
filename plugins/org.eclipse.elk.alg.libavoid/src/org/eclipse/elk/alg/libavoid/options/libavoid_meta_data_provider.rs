use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::sync::{Arc, LazyLock, OnceLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutAlgorithmData, LayoutCategoryData, LayoutMetaDataRegistry,
    LayoutOptionData, LayoutOptionTarget, LayoutOptionType, LayoutOptionVisibility,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{EdgeRouting, PortConstraints};

use crate::org::eclipse::elk::alg::libavoid::options::LibavoidOptions;

pub struct LibavoidMetaDataProvider;

impl ILayoutMetaDataProvider for LibavoidMetaDataProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        init_reflect();
        register_category(registry);
        register_algorithm(registry);
        register_options(registry);
        register_supports(registry);
    }
}

const TARGET_PARENTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Parents];
const TARGET_NODES: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Nodes];

fn register_category(registry: &mut dyn LayoutMetaDataRegistry) {
    registry.register_category(
        LayoutCategoryData::builder()
            .id("org.eclipse.elk.alg.libavoid.edge")
            .name("Edge Routing")
            .description("Only route the edges without touching the node's positions.")
            .create(),
    );
}

fn register_algorithm(registry: &mut dyn LayoutMetaDataRegistry) {
    let mut data = LayoutAlgorithmData::new(LibavoidOptions::ALGORITHM_ID);
    data.set_name("Libavoid")
        .set_description(
            "libavoid is a cross-platform C++ library providing fast, object-avoiding orthogonal and polyline connector routing for use in interactive diagram editors.",
        )
        .set_category_id(Some("org.eclipse.elk.alg.libavoid.edge"));
    registry.register_algorithm(data);
}

fn register_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        LibavoidOptions::SEGMENT_PENALTY,
        LayoutOptionType::Double,
        "Segment Penalty",
        concat!(
            "This penalty is applied for each segment in the connector path beyond the first. ",
            "This should always normally be set when doing orthogonal routing to prevent ",
            "step-like connector paths."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::ANGLE_PENALTY,
        LayoutOptionType::Double,
        "Angle Penalty",
        concat!(
            "This penalty is applied in its full amount to tight acute bends in the connector path. ",
            "A smaller portion of the penalty is applied for slight bends, i.e., where the bend is close ",
            "to 180 degrees. This is useful for polyline routing where there is some evidence that tighter ",
            "corners are worse for readability, but that slight bends might not be so bad, ",
            "especially when smoothed by curves."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::CROSSING_PENALTY,
        LayoutOptionType::Double,
        "Crossing Penalty",
        concat!(
            "This penalty is applied whenever a connector path crosses another connector path. ",
            "It takes shared paths into consideration and the penalty is only applied ",
            "if there is an actual crossing."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::CLUSTER_CROSSING_PENALTY,
        LayoutOptionType::Double,
        "Cluster Crossing Penalty",
        "This penalty is applied whenever a connector path crosses a cluster boundary.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::FIXED_SHARED_PATH_PENALTY,
        LayoutOptionType::Double,
        "Fixed Shared Path Penalty",
        concat!(
            "This penalty is applied whenever a connector path shares some segments with an immovable ",
            "portion of an existing connector route (such as the first or last segment of a connector)."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::PORT_DIRECTION_PENALTY,
        LayoutOptionType::Double,
        "Port Direction Penalty",
        concat!(
            "This penalty is applied to port selection choice when the other end of the connector ",
            "being routed does not appear in any of the 90 degree visibility cones centered on the ",
            "visibility directions for the port."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::SHAPE_BUFFER_DISTANCE,
        LayoutOptionType::Double,
        "Shape Buffer Distance",
        concat!(
            "This parameter defines the spacing distance that will be added to the sides of each ",
            "shape when determining obstacle sizes for routing. This controls how closely connectors ",
            "pass shapes, and can be used to prevent connectors overlapping with shape boundaries."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::IDEAL_NUDGING_DISTANCE,
        LayoutOptionType::Double,
        "Ideal Nudging Distance",
        concat!(
            "This parameter defines the spacing distance that will be used for nudging apart ",
            "overlapping corners and line segments of connectors."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::REVERSE_DIRECTION_PENALTY,
        LayoutOptionType::Double,
        "Reverse Direction Penalty",
        concat!(
            "This penalty is applied whenever a connector path travels in the direction opposite ",
            "of the destination from the source endpoint. By default this penalty is set to zero. ",
            "This shouldn't be needed in most cases but can be useful if you use penalties such as ",
            "crossingPenalty which cause connectors to loop around obstacles."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::NUDGE_ORTHOGONAL_SEGMENTS_CONNECTED_TO_SHAPES,
        LayoutOptionType::Boolean,
        "Nudge Orthogonal Segments",
        concat!(
            "This option causes the final segments of connectors, which are attached to shapes, ",
            "to be nudged apart. Usually these segments are fixed, since they are considered to be ",
            "attached to ports."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::IMPROVE_HYPEREDGE_ROUTES_MOVING_JUNCTIONS,
        LayoutOptionType::Boolean,
        "Improve Hyperedge Routes",
        concat!(
            "This option causes hyperedge routes to be locally improved fixing obviously bad paths. ",
            "As part of this process libavoid will effectively move junctions, setting new ideal positions ",
            "for each junction."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::PENALISE_ORTHOGONAL_SHARED_PATHS_AT_CONN_ENDS,
        LayoutOptionType::Boolean,
        "Penalise Orthogonal Shared Paths",
        concat!(
            "This option penalises and attempts to reroute orthogonal shared connector paths terminating ",
            "at a common junction or shape connection pin. When multiple connector paths enter or leave ",
            "the same side of a junction (or shape pin), the router will attempt to reroute these to ",
            "different sides of the junction or different shape pins. This option depends on the ",
            "fixedSharedPathPenalty penalty having been set."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::NUDGE_ORTHOGONAL_TOUCHING_COLINEAR_SEGMENTS,
        LayoutOptionType::Boolean,
        "Nudge Orthogonal Touching Colinear Segments",
        concat!(
            "This option can be used to control whether colinear line segments that touch just at ",
            "their ends will be nudged apart. The overlap will usually be resolved in the other dimension, ",
            "so this is not usually required."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::PERFORM_UNIFYING_NUDGING_PREPROCESSING_STEP,
        LayoutOptionType::Boolean,
        "Perform Unifying Nudging Preprocessing",
        concat!(
            "This option can be used to control whether the router performs a preprocessing step before ",
            "orthogonal nudging where is tries to unify segments and centre them in free space. ",
            "This generally results in better quality ordering and nudging."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::IMPROVE_HYPEREDGE_ROUTES_MOVING_ADDING_AND_DELETING_JUNCTIONS,
        LayoutOptionType::Boolean,
        "Improve Hyperedge Routes Add/Delete",
        concat!(
            "This option causes hyperedge routes to be locally improved fixing obviously bad paths. ",
            "It can cause junctions and connectors to be added or removed from hyperedges. As part of ",
            "this process libavoid will effectively move junctions by setting new ideal positions for ",
            "each remaining or added junction. If set, this option overrides the ",
            "improveHyperedgeRoutesMovingJunctions option."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::NUDGE_SHARED_PATHS_WITH_COMMON_END_POINT,
        LayoutOptionType::Boolean,
        "Nudge Shared Paths With Common Endpoint",
        concat!(
            "This option determines whether intermediate segments of connectors that are attached to ",
            "common endpoints will be nudged apart. Usually these segments get nudged apart, but you ",
            "may want to turn this off if you would prefer that entire shared paths terminating at a ",
            "common end point should overlap."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::ENABLE_HYPEREDGES_FROM_COMMON_SOURCE,
        LayoutOptionType::Boolean,
        "Enable Hyperedges From Common Source",
        concat!(
            "This option enables a post-processing step that creates hyperedges for all edges with the same source. ",
            "Be aware that this step will significantly decrease performance."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::IS_CLUSTER,
        LayoutOptionType::Boolean,
        "Marks a node as a cluster",
        concat!(
            "This option marks a node as a cluster, resulting in its children being handled as ",
            "relative to the graph itself while the marked node is only added as a cluster. ",
            "Note that clusters are experimental and can therefore have a negative impact on performance. ",
            "The cluster node cannot have: ",
            "- clusters as children ",
            "- outgoing or incoming connections (directly to the node) ",
            "- ports ",
            "Edges into or out of the cluster must be added across the cluster's borders, without the use of hierarchical ports."
        ),
        &TARGET_NODES,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );

    register_option(
        registry,
        LibavoidOptions::PROCESS_TIMEOUT,
        LayoutOptionType::Int,
        "Default process timeout.",
        concat!(
            "Default timeout for waiting for the libavoid server to give some output. This option is read from ",
            "the root of the graph."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
    );
}

fn register_supports(registry: &mut dyn LayoutMetaDataRegistry) {
    let algo = LibavoidOptions::ALGORITHM_ID;

    registry.add_option_support(algo, CoreOptions::DEBUG_MODE.id(), Some(Arc::new(false)));
    registry.add_option_support(algo, CoreOptions::PORT_SIDE.id(), None);
    registry.add_option_support(algo, CoreOptions::DIRECTION.id(), None);
    registry.add_option_support(
        algo,
        CoreOptions::EDGE_ROUTING.id(),
        Some(Arc::new(EdgeRouting::Orthogonal)),
    );
    registry.add_option_support(
        algo,
        CoreOptions::PORT_CONSTRAINTS.id(),
        Some(Arc::new(PortConstraints::Free)),
    );
    registry.add_option_support(algo, CoreOptions::OMIT_NODE_MICRO_LAYOUT.id(), None);

    registry.add_option_support(
        algo,
        LibavoidOptions::SEGMENT_PENALTY.id(),
        property_default_any(LibavoidOptions::SEGMENT_PENALTY),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::ANGLE_PENALTY.id(),
        property_default_any(LibavoidOptions::ANGLE_PENALTY),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::CROSSING_PENALTY.id(),
        property_default_any(LibavoidOptions::CROSSING_PENALTY),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::CLUSTER_CROSSING_PENALTY.id(),
        property_default_any(LibavoidOptions::CLUSTER_CROSSING_PENALTY),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::FIXED_SHARED_PATH_PENALTY.id(),
        property_default_any(LibavoidOptions::FIXED_SHARED_PATH_PENALTY),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::PORT_DIRECTION_PENALTY.id(),
        property_default_any(LibavoidOptions::PORT_DIRECTION_PENALTY),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::SHAPE_BUFFER_DISTANCE.id(),
        property_default_any(LibavoidOptions::SHAPE_BUFFER_DISTANCE),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::IDEAL_NUDGING_DISTANCE.id(),
        property_default_any(LibavoidOptions::IDEAL_NUDGING_DISTANCE),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::REVERSE_DIRECTION_PENALTY.id(),
        property_default_any(LibavoidOptions::REVERSE_DIRECTION_PENALTY),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::NUDGE_ORTHOGONAL_SEGMENTS_CONNECTED_TO_SHAPES.id(),
        property_default_any(LibavoidOptions::NUDGE_ORTHOGONAL_SEGMENTS_CONNECTED_TO_SHAPES),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::IMPROVE_HYPEREDGE_ROUTES_MOVING_JUNCTIONS.id(),
        property_default_any(LibavoidOptions::IMPROVE_HYPEREDGE_ROUTES_MOVING_JUNCTIONS),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::PENALISE_ORTHOGONAL_SHARED_PATHS_AT_CONN_ENDS.id(),
        property_default_any(LibavoidOptions::PENALISE_ORTHOGONAL_SHARED_PATHS_AT_CONN_ENDS),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::NUDGE_ORTHOGONAL_TOUCHING_COLINEAR_SEGMENTS.id(),
        property_default_any(LibavoidOptions::NUDGE_ORTHOGONAL_TOUCHING_COLINEAR_SEGMENTS),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::PERFORM_UNIFYING_NUDGING_PREPROCESSING_STEP.id(),
        property_default_any(LibavoidOptions::PERFORM_UNIFYING_NUDGING_PREPROCESSING_STEP),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::IMPROVE_HYPEREDGE_ROUTES_MOVING_ADDING_AND_DELETING_JUNCTIONS.id(),
        property_default_any(
            LibavoidOptions::IMPROVE_HYPEREDGE_ROUTES_MOVING_ADDING_AND_DELETING_JUNCTIONS,
        ),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::NUDGE_SHARED_PATHS_WITH_COMMON_END_POINT.id(),
        property_default_any(LibavoidOptions::NUDGE_SHARED_PATHS_WITH_COMMON_END_POINT),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::ENABLE_HYPEREDGES_FROM_COMMON_SOURCE.id(),
        property_default_any(LibavoidOptions::ENABLE_HYPEREDGES_FROM_COMMON_SOURCE),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::IS_CLUSTER.id(),
        property_default_any(LibavoidOptions::IS_CLUSTER),
    );
    registry.add_option_support(
        algo,
        LibavoidOptions::PROCESS_TIMEOUT.id(),
        property_default_any(LibavoidOptions::PROCESS_TIMEOUT),
    );
}

#[allow(clippy::too_many_arguments)]
fn register_option<T: Clone + Send + Sync + 'static>(
    registry: &mut dyn LayoutMetaDataRegistry,
    property: &'static LazyLock<Property<T>>,
    option_type: LayoutOptionType,
    name: &'static str,
    description: &'static str,
    targets: &[LayoutOptionTarget],
    visibility: LayoutOptionVisibility,
    category: Option<&'static str>,
    default_value: Option<Arc<dyn Any + Send + Sync>>,
) {
    let mut targets_set = HashSet::new();
    for target in targets {
        targets_set.insert(*target);
    }
    let mut builder = LayoutOptionData::builder()
        .id(property.id())
        .option_type(option_type)
        .name(name)
        .description(description)
        .targets(targets_set)
        .visibility(visibility)
        .default_value(default_value.or_else(|| property_default_any(property)))
        .value_type_id(TypeId::of::<T>());
    if let Some(category) = category {
        builder = builder.group(category);
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
        ElkReflect::register(Some(|| 0_i32), Some(|v: &i32| *v));
        ElkReflect::register(Some(|| 0_f64), Some(|v: &f64| *v));
        ElkReflect::register(Some(|| false), Some(|v: &bool| *v));
    });
}
