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
use org_eclipse_elk_core::org::eclipse::elk::core::options::{Direction, EdgeRouting};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::ExclusiveBounds;

use org_eclipse_elk_alg_graphviz_dot::org::eclipse::elk::alg::graphviz::dot::transform::{
    NeatoModel, OverlapMode,
};

use crate::org::eclipse::elk::alg::graphviz::layouter::options::{
    CircoOptions, DotOptions, FdpOptions, GraphvizOptions, NeatoOptions, TwopiOptions,
};

pub struct GraphvizMetaDataProvider;

impl ILayoutMetaDataProvider for GraphvizMetaDataProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        init_reflect();
        register_algorithms(registry);
        register_options(registry);
        register_supports(registry);
    }
}

const TARGET_PARENTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Parents];
const TARGET_EDGES: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Edges];

fn register_algorithms(registry: &mut dyn LayoutMetaDataRegistry) {
    let mut dot = LayoutAlgorithmData::new(DotOptions::ALGORITHM_ID);
    dot.set_name("Graphviz Dot")
        .set_description(concat!(
            "Layered drawings of directed graphs. The algorithm aims edges in the same direction (top ",
            "to bottom, or left to right) and then attempts to avoid edge crossings and reduce edge ",
            "length. Edges are routed as spline curves and are thus drawn very smoothly. This algorithm ",
            "is very suitable for state machine and activity diagrams, where the direction of edges has ",
            "an important role."
        ))
        .set_preview_image_path(Some("images/dot_layout.png"))
        .set_category_id(Some("org.eclipse.elk.layered"))
        .set_bundle_name(Some("Graphviz"))
        .set_defining_bundle_id(Some("org.eclipse.elk.alg.graphviz.layouter"))
        .add_supported_feature(GraphFeature::SelfLoops)
        .add_supported_feature(GraphFeature::MultiEdges)
        .add_supported_feature(GraphFeature::EdgeLabels)
        .add_supported_feature(GraphFeature::Compound)
        .add_supported_feature(GraphFeature::Clusters);
    registry.register_algorithm(dot);

    let mut neato = LayoutAlgorithmData::new(NeatoOptions::ALGORITHM_ID);
    neato
        .set_name("Graphviz Neato")
        .set_description(concat!(
            "Spring model layouts. Neato attempts to minimize a global energy function, which is ",
            "equivalent to statistical multi-dimensional scaling. The solution is achieved using ",
            "stress majorization, though the older Kamada-Kawai algorithm, using steepest descent, is ",
            "also available."
        ))
        .set_preview_image_path(Some("images/neato_layout.png"))
        .set_category_id(Some("org.eclipse.elk.force"))
        .set_bundle_name(Some("Graphviz"))
        .set_defining_bundle_id(Some("org.eclipse.elk.alg.graphviz.layouter"))
        .add_supported_feature(GraphFeature::SelfLoops)
        .add_supported_feature(GraphFeature::MultiEdges)
        .add_supported_feature(GraphFeature::EdgeLabels);
    registry.register_algorithm(neato);

    let mut fdp = LayoutAlgorithmData::new(FdpOptions::ALGORITHM_ID);
    fdp.set_name("Graphviz FDP")
        .set_description(concat!(
            "Spring model layouts similar to those of Neato, but does this by reducing forces rather ",
            "than working with energy. Fdp implements the Fruchterman-Reingold heuristic including a ",
            "multigrid solver that handles larger graphs and clustered undirected graphs."
        ))
        .set_preview_image_path(Some("images/fdp_layout.png"))
        .set_category_id(Some("org.eclipse.elk.force"))
        .set_bundle_name(Some("Graphviz"))
        .set_defining_bundle_id(Some("org.eclipse.elk.alg.graphviz.layouter"))
        .add_supported_feature(GraphFeature::SelfLoops)
        .add_supported_feature(GraphFeature::MultiEdges)
        .add_supported_feature(GraphFeature::EdgeLabels)
        .add_supported_feature(GraphFeature::Clusters);
    registry.register_algorithm(fdp);

    let mut twopi = LayoutAlgorithmData::new(TwopiOptions::ALGORITHM_ID);
    twopi
        .set_name("Graphviz Twopi")
        .set_description(concat!(
            "Radial layouts, after Wills '97. The nodes are placed on concentric circles depending on ",
            "their distance from a given root node. The algorithm is designed to handle not only small ",
            "graphs, but also very large ones."
        ))
        .set_preview_image_path(Some("images/twopi_layout.png"))
        .set_category_id(Some("org.eclipse.elk.radial"))
        .set_bundle_name(Some("Graphviz"))
        .set_defining_bundle_id(Some("org.eclipse.elk.alg.graphviz.layouter"))
        .add_supported_feature(GraphFeature::SelfLoops)
        .add_supported_feature(GraphFeature::MultiEdges)
        .add_supported_feature(GraphFeature::EdgeLabels);
    registry.register_algorithm(twopi);

    let mut circo = LayoutAlgorithmData::new(CircoOptions::ALGORITHM_ID);
    circo
        .set_name("Graphviz Circo")
        .set_description(concat!(
            "Circular layout, after Six and Tollis '99, Kaufmann and Wiese '02. The algorithm finds ",
            "biconnected components and arranges each component in a circle, trying to minimize the ",
            "number of crossings inside the circle. This is suitable for certain diagrams of multiple ",
            "cyclic structures such as certain telecommunications networks."
        ))
        .set_preview_image_path(Some("images/circo_layout.png"))
        .set_category_id(Some("org.eclipse.elk.circle"))
        .set_bundle_name(Some("Graphviz"))
        .set_defining_bundle_id(Some("org.eclipse.elk.alg.graphviz.layouter"))
        .add_supported_feature(GraphFeature::SelfLoops)
        .add_supported_feature(GraphFeature::MultiEdges)
        .add_supported_feature(GraphFeature::EdgeLabels);
    registry.register_algorithm(circo);
}

fn register_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        GraphvizOptions::ADAPT_PORT_POSITIONS,
        LayoutOptionType::Boolean,
        "Adapt Port Positions",
        "Whether ports should be moved to the point where edges cross the node's bounds.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
        None,
    );

    register_option(
        registry,
        GraphvizOptions::CONCENTRATE,
        LayoutOptionType::Boolean,
        "Concentrate Edges",
        concat!(
            "Merges multiedges into a single edge and causes partially parallel edges to share part of ",
            "their paths."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
        None,
    );

    register_option(
        registry,
        GraphvizOptions::EPSILON,
        LayoutOptionType::Double,
        "Epsilon",
        concat!(
            "Terminating condition. If the length squared of all energy gradients are less than ",
            "epsilon, the algorithm stops."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        Some(Arc::new(ExclusiveBounds::greater_than(0.0_f64))),
        None,
    );

    register_option(
        registry,
        GraphvizOptions::ITERATIONS_FACTOR,
        LayoutOptionType::Double,
        "Iterations Factor",
        concat!(
            "Multiplicative scale factor for the maximal number of iterations used during crossing ",
            "minimization, node ranking, and node positioning."
        ),
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        Some(Arc::new(ExclusiveBounds::greater_than(0.0_f64))),
        None,
    );

    register_option(
        registry,
        GraphvizOptions::LABEL_ANGLE,
        LayoutOptionType::Double,
        "Label Angle",
        "Angle between head / tail positioned edge labels and the corresponding edge.",
        &TARGET_EDGES,
        LayoutOptionVisibility::Visible,
        None,
        None,
        None,
    );

    register_option(
        registry,
        GraphvizOptions::LABEL_DISTANCE,
        LayoutOptionType::Double,
        "Label Distance",
        "Distance of head / tail positioned edge labels to the source or target node.",
        &TARGET_EDGES,
        LayoutOptionVisibility::Visible,
        None,
        Some(Arc::new(0.0_f64)),
        None,
    );

    register_option(
        registry,
        GraphvizOptions::LAYER_SPACING_FACTOR,
        LayoutOptionType::Double,
        "Layer Spacing Factor",
        "Factor for the spacing of different layers (ranks).",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        Some(Arc::new(ExclusiveBounds::greater_than(0.0_f64))),
        None,
    );

    register_option(
        registry,
        GraphvizOptions::MAXITER,
        LayoutOptionType::Int,
        "Max. Iterations",
        "The maximum number of iterations.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        Some(Arc::new(1_i32)),
        None,
    );

    register_option(
        registry,
        GraphvizOptions::NEATO_MODEL,
        LayoutOptionType::Enum,
        "Distance Model",
        "Specifies how the distance matrix is computed for the input graph.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Advanced,
        None,
        None,
        None,
    );

    register_option(
        registry,
        GraphvizOptions::OVERLAP_MODE,
        LayoutOptionType::Enum,
        "Overlap Removal",
        "Determines if and how node overlaps should be removed.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        None,
        None,
        None,
    );
}

fn register_supports(registry: &mut dyn LayoutMetaDataRegistry) {
    fn arc_any<T: Any + Send + Sync>(value: T) -> Option<Arc<dyn Any + Send + Sync>> {
        Some(Arc::new(value))
    }

    // Dot
    let dot = DotOptions::ALGORITHM_ID;
    registry.add_option_support(dot, CoreOptions::PADDING.id(), arc_any(ElkPadding::with_any(10.0)));
    registry.add_option_support(dot, CoreOptions::DIRECTION.id(), arc_any(Direction::Down));
    registry.add_option_support(dot, CoreOptions::SPACING_NODE_NODE.id(), arc_any(20.0_f64));
    registry.add_option_support(dot, CoreOptions::SPACING_EDGE_LABEL.id(), None);
    registry.add_option_support(dot, CoreOptions::NODE_SIZE_CONSTRAINTS.id(), None);
    registry.add_option_support(dot, CoreOptions::NODE_SIZE_OPTIONS.id(), None);
    registry.add_option_support(dot, CoreOptions::EDGE_ROUTING.id(), arc_any(EdgeRouting::Splines));
    registry.add_option_support(dot, CoreOptions::DEBUG_MODE.id(), None);
    registry.add_option_support(dot, CoreOptions::HIERARCHY_HANDLING.id(), None);
    registry.add_option_support(dot, GraphvizOptions::ITERATIONS_FACTOR.id(), arc_any(1.0_f64));
    registry.add_option_support(dot, GraphvizOptions::CONCENTRATE.id(), property_default_any(GraphvizOptions::CONCENTRATE));
    registry.add_option_support(dot, GraphvizOptions::LABEL_DISTANCE.id(), property_default_any(GraphvizOptions::LABEL_DISTANCE));
    registry.add_option_support(dot, GraphvizOptions::LABEL_ANGLE.id(), property_default_any(GraphvizOptions::LABEL_ANGLE));
    registry.add_option_support(dot, GraphvizOptions::LAYER_SPACING_FACTOR.id(), property_default_any(GraphvizOptions::LAYER_SPACING_FACTOR));
    registry.add_option_support(dot, GraphvizOptions::ADAPT_PORT_POSITIONS.id(), property_default_any(GraphvizOptions::ADAPT_PORT_POSITIONS));

    // Neato
    let neato = NeatoOptions::ALGORITHM_ID;
    registry.add_option_support(neato, CoreOptions::PADDING.id(), arc_any(ElkPadding::with_any(10.0)));
    registry.add_option_support(neato, CoreOptions::SPACING_NODE_NODE.id(), arc_any(40.0_f64));
    registry.add_option_support(neato, CoreOptions::SPACING_EDGE_LABEL.id(), None);
    registry.add_option_support(neato, CoreOptions::NODE_SIZE_CONSTRAINTS.id(), None);
    registry.add_option_support(neato, CoreOptions::NODE_SIZE_OPTIONS.id(), None);
    registry.add_option_support(neato, CoreOptions::RANDOM_SEED.id(), arc_any(1_i32));
    registry.add_option_support(neato, CoreOptions::INTERACTIVE.id(), None);
    registry.add_option_support(neato, CoreOptions::EDGE_ROUTING.id(), arc_any(EdgeRouting::Splines));
    registry.add_option_support(neato, CoreOptions::DEBUG_MODE.id(), None);
    registry.add_option_support(neato, CoreOptions::SEPARATE_CONNECTED_COMPONENTS.id(), arc_any(false));
    registry.add_option_support(neato, GraphvizOptions::CONCENTRATE.id(), property_default_any(GraphvizOptions::CONCENTRATE));
    registry.add_option_support(neato, GraphvizOptions::EPSILON.id(), arc_any(0.0001_f64));
    registry.add_option_support(neato, GraphvizOptions::LABEL_DISTANCE.id(), property_default_any(GraphvizOptions::LABEL_DISTANCE));
    registry.add_option_support(neato, GraphvizOptions::LABEL_ANGLE.id(), property_default_any(GraphvizOptions::LABEL_ANGLE));
    registry.add_option_support(neato, GraphvizOptions::MAXITER.id(), arc_any(200_i32));
    registry.add_option_support(neato, GraphvizOptions::NEATO_MODEL.id(), property_default_any(GraphvizOptions::NEATO_MODEL));
    registry.add_option_support(neato, GraphvizOptions::OVERLAP_MODE.id(), property_default_any(GraphvizOptions::OVERLAP_MODE));
    registry.add_option_support(neato, GraphvizOptions::ADAPT_PORT_POSITIONS.id(), property_default_any(GraphvizOptions::ADAPT_PORT_POSITIONS));

    // FDP
    let fdp = FdpOptions::ALGORITHM_ID;
    registry.add_option_support(fdp, CoreOptions::PADDING.id(), arc_any(ElkPadding::with_any(10.0)));
    registry.add_option_support(fdp, CoreOptions::SPACING_NODE_NODE.id(), arc_any(40.0_f64));
    registry.add_option_support(fdp, CoreOptions::SPACING_EDGE_LABEL.id(), None);
    registry.add_option_support(fdp, CoreOptions::NODE_SIZE_CONSTRAINTS.id(), None);
    registry.add_option_support(fdp, CoreOptions::NODE_SIZE_OPTIONS.id(), None);
    registry.add_option_support(fdp, CoreOptions::INTERACTIVE.id(), None);
    registry.add_option_support(fdp, CoreOptions::EDGE_ROUTING.id(), arc_any(EdgeRouting::Splines));
    registry.add_option_support(fdp, CoreOptions::DEBUG_MODE.id(), None);
    registry.add_option_support(fdp, CoreOptions::SEPARATE_CONNECTED_COMPONENTS.id(), arc_any(false));
    registry.add_option_support(fdp, GraphvizOptions::CONCENTRATE.id(), property_default_any(GraphvizOptions::CONCENTRATE));
    registry.add_option_support(fdp, GraphvizOptions::LABEL_DISTANCE.id(), property_default_any(GraphvizOptions::LABEL_DISTANCE));
    registry.add_option_support(fdp, GraphvizOptions::LABEL_ANGLE.id(), property_default_any(GraphvizOptions::LABEL_ANGLE));
    registry.add_option_support(fdp, GraphvizOptions::MAXITER.id(), arc_any(600_i32));
    registry.add_option_support(fdp, GraphvizOptions::OVERLAP_MODE.id(), property_default_any(GraphvizOptions::OVERLAP_MODE));
    registry.add_option_support(fdp, GraphvizOptions::ADAPT_PORT_POSITIONS.id(), property_default_any(GraphvizOptions::ADAPT_PORT_POSITIONS));

    // Twopi
    let twopi = TwopiOptions::ALGORITHM_ID;
    registry.add_option_support(twopi, CoreOptions::PADDING.id(), arc_any(ElkPadding::with_any(10.0)));
    registry.add_option_support(twopi, CoreOptions::SPACING_NODE_NODE.id(), arc_any(60.0_f64));
    registry.add_option_support(twopi, CoreOptions::SPACING_EDGE_LABEL.id(), None);
    registry.add_option_support(twopi, CoreOptions::NODE_SIZE_CONSTRAINTS.id(), None);
    registry.add_option_support(twopi, CoreOptions::NODE_SIZE_OPTIONS.id(), None);
    registry.add_option_support(twopi, CoreOptions::EDGE_ROUTING.id(), arc_any(EdgeRouting::Splines));
    registry.add_option_support(twopi, CoreOptions::DEBUG_MODE.id(), None);
    registry.add_option_support(twopi, GraphvizOptions::CONCENTRATE.id(), property_default_any(GraphvizOptions::CONCENTRATE));
    registry.add_option_support(twopi, GraphvizOptions::LABEL_DISTANCE.id(), property_default_any(GraphvizOptions::LABEL_DISTANCE));
    registry.add_option_support(twopi, GraphvizOptions::LABEL_ANGLE.id(), property_default_any(GraphvizOptions::LABEL_ANGLE));
    registry.add_option_support(twopi, GraphvizOptions::OVERLAP_MODE.id(), property_default_any(GraphvizOptions::OVERLAP_MODE));
    registry.add_option_support(twopi, GraphvizOptions::ADAPT_PORT_POSITIONS.id(), property_default_any(GraphvizOptions::ADAPT_PORT_POSITIONS));

    // Circo
    let circo = CircoOptions::ALGORITHM_ID;
    registry.add_option_support(circo, CoreOptions::PADDING.id(), arc_any(ElkPadding::with_any(10.0)));
    registry.add_option_support(circo, CoreOptions::SPACING_NODE_NODE.id(), arc_any(40.0_f64));
    registry.add_option_support(circo, CoreOptions::SPACING_EDGE_LABEL.id(), None);
    registry.add_option_support(circo, CoreOptions::NODE_SIZE_CONSTRAINTS.id(), None);
    registry.add_option_support(circo, CoreOptions::NODE_SIZE_OPTIONS.id(), None);
    registry.add_option_support(circo, CoreOptions::EDGE_ROUTING.id(), arc_any(EdgeRouting::Splines));
    registry.add_option_support(circo, CoreOptions::DEBUG_MODE.id(), None);
    registry.add_option_support(circo, CoreOptions::SEPARATE_CONNECTED_COMPONENTS.id(), arc_any(false));
    registry.add_option_support(circo, GraphvizOptions::CONCENTRATE.id(), property_default_any(GraphvizOptions::CONCENTRATE));
    registry.add_option_support(circo, GraphvizOptions::LABEL_DISTANCE.id(), property_default_any(GraphvizOptions::LABEL_DISTANCE));
    registry.add_option_support(circo, GraphvizOptions::LABEL_ANGLE.id(), property_default_any(GraphvizOptions::LABEL_ANGLE));
    registry.add_option_support(circo, GraphvizOptions::OVERLAP_MODE.id(), property_default_any(GraphvizOptions::OVERLAP_MODE));
    registry.add_option_support(circo, GraphvizOptions::ADAPT_PORT_POSITIONS.id(), property_default_any(GraphvizOptions::ADAPT_PORT_POSITIONS));
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
    lower_bound: Option<Arc<dyn Any + Send + Sync>>,
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
            Some(|| NeatoModel::Shortpath),
            Some(|v: &NeatoModel| *v),
        );
        ElkReflect::register(
            Some(|| OverlapMode::Prism),
            Some(|v: &OverlapMode| *v),
        );
    });
}
