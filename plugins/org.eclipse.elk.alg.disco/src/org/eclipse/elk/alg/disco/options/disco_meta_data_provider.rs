use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::sync::{Arc, LazyLock, OnceLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutAlgorithmData, LayoutMetaDataRegistry, LayoutOptionData,
    LayoutOptionTarget, LayoutOptionType, LayoutOptionVisibility,
};
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::compaction::options::PolyominoOptions;

use crate::org::eclipse::elk::alg::disco::options::{CompactionStrategy, DisCoOptions};
use crate::org::eclipse::elk::alg::disco::structures::DCPolyomino;
use crate::org::eclipse::elk::alg::disco::graph::DCGraph;

pub struct DisCoMetaDataProvider;

impl ILayoutMetaDataProvider for DisCoMetaDataProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        init_reflect();
        register_algorithm(registry);
        register_options(registry);
        register_supports(registry);
    }
}

const TARGET_PARENTS: [LayoutOptionTarget; 1] = [LayoutOptionTarget::Parents];

fn register_algorithm(registry: &mut dyn LayoutMetaDataRegistry) {
    let mut data = LayoutAlgorithmData::new(DisCoOptions::ALGORITHM_ID);
    data.set_name("ELK DisCo")
        .set_description(
            "Layouter for arranging unconnected subgraphs. The subgraphs themselves are, by default, not laid out.",
        )
        .set_preview_image_path(Some("images/disco_layout.png"));
    registry.register_algorithm(data);
}

fn register_options(registry: &mut dyn LayoutMetaDataRegistry) {
    register_option(
        registry,
        DisCoOptions::COMPONENT_COMPACTION_STRATEGY,
        LayoutOptionType::Enum,
        "Connected Components Compaction Strategy",
        "Strategy for packing different connected components in order to save space and enhance readability of a graph.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("componentCompaction"),
        None,
    );
    register_option(
        registry,
        DisCoOptions::COMPONENT_COMPACTION_COMPONENT_LAYOUT_ALGORITHM,
        LayoutOptionType::String,
        "Connected Components Layout Algorithm",
        "A layout algorithm that is to be applied to each connected component before the components themselves are compacted.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Visible,
        Some("componentCompaction"),
        None,
    );
    register_option(
        registry,
        DisCoOptions::DEBUG_DISCO_GRAPH,
        LayoutOptionType::Object,
        "DCGraph",
        "Access to the DCGraph is intended for the debug view.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Hidden,
        Some("debug"),
        None,
    );
    register_option(
        registry,
        DisCoOptions::DEBUG_DISCO_POLYS,
        LayoutOptionType::Object,
        "List of Polyominoes",
        "Access to the polyominoes is intended for the debug view.",
        &TARGET_PARENTS,
        LayoutOptionVisibility::Hidden,
        Some("debug"),
        None,
    );
}

fn register_supports(registry: &mut dyn LayoutMetaDataRegistry) {
    fn arc_any<T: Any + Send + Sync>(value: T) -> Option<Arc<dyn Any + Send + Sync>> {
        Some(Arc::new(value))
    }

    let algo = DisCoOptions::ALGORITHM_ID;
    registry.add_option_support(
        algo,
        CoreOptions::SPACING_COMPONENT_COMPONENT.id(),
        arc_any(20.0_f64),
    );
    registry.add_option_support(algo, CoreOptions::EDGE_THICKNESS.id(), arc_any(1.0_f64));
    registry.add_option_support(algo, CoreOptions::ASPECT_RATIO.id(), arc_any(1.0_f64));
    registry.add_option_support(
        algo,
        CoreOptions::PADDING.id(),
        arc_any(ElkPadding::with_any(0.0)),
    );

    registry.add_option_support(
        algo,
        PolyominoOptions::POLYOMINO_LOW_LEVEL_SORT.id(),
        property_default_any(PolyominoOptions::POLYOMINO_LOW_LEVEL_SORT),
    );
    registry.add_option_support(
        algo,
        PolyominoOptions::POLYOMINO_HIGH_LEVEL_SORT.id(),
        property_default_any(PolyominoOptions::POLYOMINO_HIGH_LEVEL_SORT),
    );
    registry.add_option_support(
        algo,
        PolyominoOptions::POLYOMINO_TRAVERSAL_STRATEGY.id(),
        property_default_any(PolyominoOptions::POLYOMINO_TRAVERSAL_STRATEGY),
    );
    registry.add_option_support(
        algo,
        PolyominoOptions::POLYOMINO_FILL.id(),
        property_default_any(PolyominoOptions::POLYOMINO_FILL),
    );

    registry.add_option_support(
        algo,
        DisCoOptions::COMPONENT_COMPACTION_STRATEGY.id(),
        property_default_any(DisCoOptions::COMPONENT_COMPACTION_STRATEGY),
    );
    registry.add_option_support(
        algo,
        DisCoOptions::COMPONENT_COMPACTION_COMPONENT_LAYOUT_ALGORITHM.id(),
        None,
    );
    registry.add_option_support(algo, DisCoOptions::DEBUG_DISCO_GRAPH.id(), None);
    registry.add_option_support(algo, DisCoOptions::DEBUG_DISCO_POLYS.id(), None);
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
        ElkReflect::register(
            Some(|| CompactionStrategy::Polyomino),
            Some(|v: &CompactionStrategy| *v),
        );
        ElkReflect::register_default_clone::<DCGraph>();
        ElkReflect::register_default_clone::<Vec<DCPolyomino>>();
    });
}
