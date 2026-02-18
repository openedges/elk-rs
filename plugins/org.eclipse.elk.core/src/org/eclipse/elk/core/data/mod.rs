use std::any::Any;
use std::collections::{BTreeSet, HashMap, HashSet, LinkedList};
use std::sync::{Arc, Mutex, OnceLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::GraphFeature;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

pub mod deprecated_layout_option_replacer;
mod i_layout_meta_data;
mod i_layout_meta_data_provider;
mod layout_algorithm_data;
pub mod layout_algorithm_resolver;
mod layout_category_data;
mod layout_data_content_assist;
mod layout_option_data;

pub use deprecated_layout_option_replacer::DeprecatedLayoutOptionReplacer;
pub use i_layout_meta_data::ILayoutMetaData;
pub use i_layout_meta_data_provider::{ILayoutMetaDataProvider, LayoutMetaDataRegistry};
pub use layout_algorithm_data::LayoutAlgorithmData;
pub use layout_algorithm_resolver::LayoutAlgorithmResolver;
pub use layout_category_data::LayoutCategoryData;
pub use layout_data_content_assist::{LayoutDataContentAssist, Proposal};
pub use layout_option_data::{
    LayoutOptionData, LayoutOptionDependency, LayoutOptionTarget, LayoutOptionType,
    LayoutOptionVisibility,
};

use crate::org::eclipse::elk::core::math::{ElkMargin, ElkPadding, KVector, KVectorChain};
use crate::org::eclipse::elk::core::options::{
    Alignment, BoxLayouterOptions, ContentAlignment, CoreOptions, Direction, EdgeCoords,
    EdgeLabelPlacement, EdgeRouting, EdgeType, FixedLayouterOptions, HierarchyHandling, LabelSide,
    NodeLabelPlacement, PackingMode, PortAlignment, PortConstraints, PortLabelPlacement, PortSide,
    RandomLayouterOptions, ShapeCoords, SizeConstraint, SizeOptions, TopdownNodeTypes,
};
use crate::org::eclipse::elk::core::util::{
    AlgorithmFactory, BoxLayoutProvider, EnumSet, FixedLayoutProvider, IndividualSpacings,
    InstancePool, LinkedHashSet, RandomLayoutProvider,
};

type LayoutProviderPool = Arc<
    InstancePool<
        Box<dyn crate::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider>,
    >,
>;

pub struct LayoutMetaDataService {
    storage: Mutex<LayoutMetaDataStorage>,
}

struct LayoutMetaDataStorage {
    algorithms: HashMap<String, LayoutAlgorithmData>,
    options: HashMap<String, LayoutOptionData>,
    legacy_options: HashMap<String, LayoutOptionData>,
    categories: HashMap<String, LayoutCategoryData>,
    algorithm_suffix_map: HashMap<String, LayoutAlgorithmData>,
    option_suffix_map: HashMap<String, LayoutOptionData>,
}

impl LayoutMetaDataStorage {
    fn new() -> Self {
        LayoutMetaDataStorage {
            algorithms: HashMap::new(),
            options: HashMap::new(),
            legacy_options: HashMap::new(),
            categories: HashMap::new(),
            algorithm_suffix_map: HashMap::new(),
            option_suffix_map: HashMap::new(),
        }
    }
}

static INSTANCE: OnceLock<LayoutMetaDataService> = OnceLock::new();

impl LayoutMetaDataService {
    pub fn get_instance() -> &'static LayoutMetaDataService {
        INSTANCE.get_or_init(|| {
            LayoutMetaDataService::init_elk_reflect();
            let service = LayoutMetaDataService {
                storage: Mutex::new(LayoutMetaDataStorage::new()),
            };
            service.register_core_algorithms();
            service.register_layout_meta_data_provider(
                &crate::org::eclipse::elk::core::options::CoreOptions,
            );
            service.register_layout_meta_data_provider(
                &crate::org::eclipse::elk::core::options::BoxLayouterOptions,
            );
            service.register_layout_meta_data_provider(
                &crate::org::eclipse::elk::core::labels::LabelManagementOptions,
            );
            service
        })
    }

    pub fn unload() {
        // OnceLock cannot be reset safely without unsafe; keep as no-op for now.
    }

    pub fn register_layout_meta_data_providers(&self, providers: &[&dyn ILayoutMetaDataProvider]) {
        for provider in providers {
            let mut registry = Registry::new(self);
            provider.apply(&mut registry);
            registry.apply_dependencies();
        }
        let mut storage = self.storage.lock().unwrap();
        storage.option_suffix_map.clear();
    }

    pub fn register_layout_meta_data_provider(&self, provider: &dyn ILayoutMetaDataProvider) {
        self.register_layout_meta_data_providers(&[provider]);
    }

    pub fn override_algorithm_provider_pool(&self, algorithm_id: &str, pool: LayoutProviderPool) {
        let mut storage = self.storage.lock().unwrap();
        if let Some(algorithm_data) = storage.algorithms.get_mut(algorithm_id) {
            algorithm_data.set_provider_pool(Some(pool));
        }
    }

    fn register_layout_algorithm(&self, algorithm: LayoutAlgorithmData) {
        let mut storage = self.storage.lock().unwrap();
        storage
            .algorithms
            .insert(algorithm.id().to_string(), algorithm);
    }

    fn register_layout_option(&self, option: LayoutOptionData) {
        let mut storage = self.storage.lock().unwrap();
        let id = option.id().to_string();
        storage.options.insert(id, option.clone());
        for legacy_id in option.legacy_ids() {
            storage
                .legacy_options
                .insert(legacy_id.to_string(), option.clone());
        }
    }

    fn register_layout_category(&self, category: LayoutCategoryData) {
        let mut storage = self.storage.lock().unwrap();
        storage
            .categories
            .insert(category.id().to_string(), category);
    }

    pub fn get_algorithm_data(&self, algorithm_id: &str) -> Option<LayoutAlgorithmData> {
        let storage = self.storage.lock().unwrap();
        storage.algorithms.get(algorithm_id).cloned()
    }

    pub fn get_algorithm_data_list(&self) -> Vec<LayoutAlgorithmData> {
        let storage = self.storage.lock().unwrap();
        storage.algorithms.values().cloned().collect()
    }

    pub fn get_algorithm_data_by_suffix(&self, suffix: &str) -> Option<LayoutAlgorithmData> {
        if suffix.trim().is_empty() {
            return None;
        }

        let mut storage = self.storage.lock().unwrap();
        if let Some(data) = storage.algorithm_suffix_map.get(suffix) {
            return Some(data.clone());
        }

        let mut match_data: Option<LayoutAlgorithmData> = None;
        for data in storage.algorithms.values() {
            let id = data.id();
            if id_matches_suffix(id, suffix) {
                if match_data.is_some() {
                    return None;
                }
                match_data = Some(data.clone());
            }
        }

        if let Some(data) = match_data.as_ref() {
            storage
                .algorithm_suffix_map
                .insert(suffix.to_string(), data.clone());
        }

        match_data
    }

    pub fn get_algorithm_data_by_suffix_or_default(
        &self,
        algorithm_id: Option<&str>,
        default_id: Option<&str>,
    ) -> Option<LayoutAlgorithmData> {
        if let Some(id) = algorithm_id {
            let trimmed = id.trim();
            if !trimmed.is_empty() {
                if let Some(data) = self.get_algorithm_data_by_suffix(trimmed) {
                    return Some(data);
                }
            }
        }

        if let Some(default_id) = default_id {
            let trimmed = default_id.trim();
            if !trimmed.is_empty() {
                if let Some(data) = self.get_algorithm_data_by_suffix(trimmed) {
                    return Some(data);
                }
            }
        }

        None
    }

    pub fn get_option_data(&self, option_id: &str) -> Option<LayoutOptionData> {
        let storage = self.storage.lock().unwrap();
        storage
            .options
            .get(option_id)
            .cloned()
            .or_else(|| storage.legacy_options.get(option_id).cloned())
    }

    pub fn get_option_data_list(&self) -> Vec<LayoutOptionData> {
        let storage = self.storage.lock().unwrap();
        storage.options.values().cloned().collect()
    }

    pub fn get_option_data_by_suffix(&self, suffix: &str) -> Option<LayoutOptionData> {
        if suffix.trim().is_empty() {
            return None;
        }

        let mut storage = self.storage.lock().unwrap();
        if let Some(data) = storage.option_suffix_map.get(suffix) {
            return Some(data.clone());
        }

        let mut match_data: Option<LayoutOptionData> = None;
        for data in storage.options.values() {
            let id = data.id();
            if id_matches_suffix(id, suffix) {
                if match_data.is_some() {
                    return None;
                }
                match_data = Some(data.clone());
            }
        }

        if match_data.is_none() {
            for data in storage.options.values() {
                for legacy_id in data.legacy_ids() {
                    if id_matches_suffix(legacy_id, suffix) {
                        if match_data.is_some() {
                            return None;
                        }
                        match_data = Some(data.clone());
                    }
                }
            }
        }

        if let Some(data) = match_data.as_ref() {
            storage
                .option_suffix_map
                .insert(suffix.to_string(), data.clone());
        }

        match_data
    }

    pub fn get_option_data_for_algorithm(
        &self,
        algorithm_data: &LayoutAlgorithmData,
        target_type: LayoutOptionTarget,
    ) -> Vec<LayoutOptionData> {
        let storage = self.storage.lock().unwrap();
        let algorithm_option_id =
            crate::org::eclipse::elk::core::options::CoreOptions::ALGORITHM.id();

        storage
            .options
            .values()
            .filter(|option_data| {
                algorithm_data.knows_option(option_data.id())
                    || option_data.id() == algorithm_option_id
            })
            .filter(|option_data| option_data.targets().contains(&target_type))
            .cloned()
            .collect()
    }

    pub fn get_category_data(&self, category_id: &str) -> Option<LayoutCategoryData> {
        let storage = self.storage.lock().unwrap();
        storage.categories.get(category_id).cloned()
    }

    pub fn get_category_data_list(&self) -> Vec<LayoutCategoryData> {
        let storage = self.storage.lock().unwrap();
        storage.categories.values().cloned().collect()
    }

    pub fn init_elk_reflect() {
        ElkReflect::register(Some(KVector::new), Some(|v: &KVector| *v));
        ElkReflect::register(
            Some(KVectorChain::new),
            Some(|vc: &KVectorChain| vc.clone()),
        );
        ElkReflect::register(Some(ElkMargin::new), Some(|m: &ElkMargin| m.clone()));
        ElkReflect::register(Some(ElkPadding::new), Some(|p: &ElkPadding| p.clone()));
        ElkReflect::register(
            Some(IndividualSpacings::new),
            Some(|s: &IndividualSpacings| IndividualSpacings::from_other(s)),
        );

        ElkReflect::register(Some(|| 0_i32), Some(|v: &i32| *v));
        ElkReflect::register(Some(|| 0_f32), Some(|v: &f32| *v));
        ElkReflect::register(Some(|| 0_f64), Some(|v: &f64| *v));
        ElkReflect::register(Some(|| false), Some(|v: &bool| *v));
        ElkReflect::register(Some(String::new), Some(|v: &String| v.clone()));
        ElkReflect::register(Some(|| Alignment::Automatic), Some(|v: &Alignment| *v));
        ElkReflect::register(
            Some(|| HierarchyHandling::Inherit),
            Some(|v: &HierarchyHandling| *v),
        );
        ElkReflect::register_default_clone::<LayoutAlgorithmData>();
        ElkReflect::register(Some(|| LabelSide::Unknown), Some(|v: &LabelSide| *v));
        ElkReflect::register(Some(|| PortSide::Undefined), Some(|v: &PortSide| *v));
        ElkReflect::register(
            Some(|| PortAlignment::Distributed),
            Some(|v: &PortAlignment| *v),
        );
        ElkReflect::register(Some(|| Direction::Undefined), Some(|v: &Direction| *v));
        ElkReflect::register(Some(|| EdgeRouting::Undefined), Some(|v: &EdgeRouting| *v));
        ElkReflect::register(Some(|| EdgeCoords::Inherit), Some(|v: &EdgeCoords| *v));
        ElkReflect::register(Some(|| ShapeCoords::Inherit), Some(|v: &ShapeCoords| *v));
        ElkReflect::register(Some(|| EdgeType::None), Some(|v: &EdgeType| *v));
        ElkReflect::register(
            Some(|| PortConstraints::Undefined),
            Some(|v: &PortConstraints| *v),
        );
        ElkReflect::register(
            Some(|| EdgeLabelPlacement::Center),
            Some(|v: &EdgeLabelPlacement| *v),
        );
        ElkReflect::register(
            Some(|| TopdownNodeTypes::ParallelNode),
            Some(|v: &TopdownNodeTypes| *v),
        );
        ElkReflect::register(Some(|| PackingMode::Simple), Some(|v: &PackingMode| *v));

        ElkReflect::register(
            Some(Vec::<KVector>::new),
            Some(|v: &Vec<KVector>| v.clone()),
        );
        ElkReflect::register(
            Some(LinkedList::<KVector>::new),
            Some(|v: &LinkedList<KVector>| v.clone()),
        );
        ElkReflect::register(
            Some(HashSet::<KVector>::new),
            Some(|v: &HashSet<KVector>| v.clone()),
        );
        ElkReflect::register(
            Some(LinkedHashSet::<KVector>::new),
            Some(|v: &LinkedHashSet<KVector>| v.clone()),
        );
        ElkReflect::register(
            Some(BTreeSet::<i32>::new),
            Some(|v: &BTreeSet<i32>| v.clone()),
        );
        ElkReflect::register(
            Some(EnumSet::<SizeConstraint>::none_of),
            Some(|v: &EnumSet<SizeConstraint>| v.clone()),
        );
        ElkReflect::register(
            Some(EnumSet::<ContentAlignment>::none_of),
            Some(|v: &EnumSet<ContentAlignment>| v.clone()),
        );
        ElkReflect::register(
            Some(EnumSet::<SizeOptions>::none_of),
            Some(|v: &EnumSet<SizeOptions>| v.clone()),
        );
        ElkReflect::register(
            Some(EnumSet::<NodeLabelPlacement>::none_of),
            Some(|v: &EnumSet<NodeLabelPlacement>| v.clone()),
        );
        ElkReflect::register(
            Some(EnumSet::<PortLabelPlacement>::none_of),
            Some(|v: &EnumSet<PortLabelPlacement>| v.clone()),
        );
    }

    fn register_core_algorithms(&self) {
        let storage = self.storage.lock().unwrap();
        if storage
            .algorithms
            .contains_key(FixedLayouterOptions::ALGORITHM_ID)
        {
            return;
        }
        drop(storage);

        fn arc_any<T: Any + Send + Sync>(value: T) -> Option<Arc<dyn Any + Send + Sync>> {
            Some(Arc::new(value))
        }

        let factory = AlgorithmFactory::new(|| Box::new(FixedLayoutProvider::new()));
        let pool = InstancePool::new(Box::new(factory));
        let mut data = LayoutAlgorithmData::new(FixedLayouterOptions::ALGORITHM_ID)
            .with_provider_pool(Arc::new(pool));
        data.set_name("ELK Fixed")
            .set_description(
                "Keeps the current layout as it is, without any automatic modification. Optional coordinates can be given for nodes and edge bend points.",
            )
            .set_bundle_name(Some("ELK"))
            .set_defining_bundle_id(Some("org.eclipse.elk.core"));
        data.add_known_option_default(
            CoreOptions::PADDING.id(),
            arc_any(ElkPadding::with_any(15.0)),
        );
        data.add_known_option_default(CoreOptions::POSITION.id(), None);
        data.add_known_option_default(CoreOptions::BEND_POINTS.id(), None);
        data.add_known_option_default(CoreOptions::NODE_SIZE_CONSTRAINTS.id(), None);
        data.add_known_option_default(CoreOptions::NODE_SIZE_MINIMUM.id(), None);
        data.add_known_option_default(CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE.id(), None);
        self.register_layout_algorithm(data);

        let random_factory = AlgorithmFactory::new(|| Box::new(RandomLayoutProvider::new()));
        let random_pool = InstancePool::new(Box::new(random_factory));
        let mut random_data = LayoutAlgorithmData::new(RandomLayouterOptions::ALGORITHM_ID)
            .with_provider_pool(Arc::new(random_pool));
        random_data
            .set_name("ELK Randomizer")
            .set_description(
                "Distributes the nodes randomly on the plane, leading to very obfuscating layouts. Can be useful to demonstrate the power of \"real\" layout algorithms.",
            )
            .set_bundle_name(Some("ELK"))
            .set_defining_bundle_id(Some("org.eclipse.elk.core"))
            .set_preview_image_path(Some("images/random_layout.png"));
        random_data.add_known_option_default(
            CoreOptions::PADDING.id(),
            arc_any(ElkPadding::with_any(15.0)),
        );
        random_data
            .add_known_option_default(CoreOptions::SPACING_NODE_NODE.id(), arc_any(15.0_f64));
        random_data.add_known_option_default(CoreOptions::RANDOM_SEED.id(), arc_any(0_i32));
        random_data.add_known_option_default(CoreOptions::ASPECT_RATIO.id(), arc_any(1.6_f64));
        self.register_layout_algorithm(random_data);

        let box_factory = AlgorithmFactory::new(|| Box::new(BoxLayoutProvider::new()));
        let box_pool = InstancePool::new(Box::new(box_factory));
        let mut box_data = LayoutAlgorithmData::new(BoxLayouterOptions::ALGORITHM_ID)
            .with_provider_pool(Arc::new(box_pool));
        box_data
            .set_name("ELK Box")
            .set_description(
                "Algorithm for packing of unconnected boxes, i.e. graphs without edges.",
            )
            .set_bundle_name(Some("ELK"))
            .set_defining_bundle_id(Some("org.eclipse.elk.core"))
            .set_preview_image_path(Some("images/box_layout.png"));
        box_data.add_known_option_default(
            CoreOptions::PADDING.id(),
            arc_any(ElkPadding::with_any(15.0)),
        );
        box_data.add_known_option_default(CoreOptions::SPACING_NODE_NODE.id(), arc_any(15.0_f64));
        box_data.add_known_option_default(CoreOptions::PRIORITY.id(), arc_any(0_i32));
        box_data.add_known_option_default(CoreOptions::EXPAND_NODES.id(), None);
        box_data.add_known_option_default(CoreOptions::NODE_SIZE_CONSTRAINTS.id(), None);
        box_data.add_known_option_default(CoreOptions::NODE_SIZE_OPTIONS.id(), None);
        box_data.add_known_option_default(CoreOptions::ASPECT_RATIO.id(), arc_any(1.3_f64));
        box_data.add_known_option_default(CoreOptions::INTERACTIVE.id(), None);
        box_data.add_known_option_default(CoreOptions::NODE_SIZE_MINIMUM.id(), None);
        box_data.add_known_option_default(BoxLayouterOptions::BOX_PACKING_MODE.id(), None);
        box_data.add_known_option_default(CoreOptions::CONTENT_ALIGNMENT.id(), None);
        self.register_layout_algorithm(box_data);

        let layered_factory = AlgorithmFactory::new(|| Box::new(BoxLayoutProvider::new()));
        let layered_pool = InstancePool::new(Box::new(layered_factory));
        let mut layered_data = LayoutAlgorithmData::new("org.eclipse.elk.layered")
            .with_provider_pool(Arc::new(layered_pool));
        layered_data
            .set_name("ELK Layered")
            .set_description(
                "Layer-based algorithm provided by the Eclipse Layout Kernel. Arranges as many edges as possible into one direction by placing nodes into subsequent layers. This implementation supports different routing styles (straight, orthogonal, splines); if orthogonal routing is selected, arbitrary port constraints are respected, thus enabling the layout of block diagrams such as actor-oriented models or circuit schematics. Furthermore, full layout of compound graphs with cross-hierarchy edges is supported when the respective option is activated on the top level.",
            )
            .set_category_id(Some("org.eclipse.elk.layered"))
            .set_defining_bundle_id(Some("org.eclipse.elk.alg.layered"))
            .set_preview_image_path(Some("images/layered_layout.png"));
        layered_data
            .add_supported_feature(GraphFeature::SelfLoops)
            .add_supported_feature(GraphFeature::InsideSelfLoops)
            .add_supported_feature(GraphFeature::MultiEdges)
            .add_supported_feature(GraphFeature::EdgeLabels)
            .add_supported_feature(GraphFeature::Ports)
            .add_supported_feature(GraphFeature::Compound)
            .add_supported_feature(GraphFeature::Clusters);
        layered_data.add_known_option_default(CoreOptions::SPACING_COMMENT_COMMENT.id(), None);
        layered_data.add_known_option_default(CoreOptions::SPACING_COMMENT_NODE.id(), None);
        layered_data.add_known_option_default(CoreOptions::SPACING_COMPONENT_COMPONENT.id(), None);
        layered_data.add_known_option_default(CoreOptions::SPACING_EDGE_EDGE.id(), None);
        layered_data.add_known_option_default(CoreOptions::SPACING_EDGE_LABEL.id(), None);
        layered_data.add_known_option_default(CoreOptions::SPACING_EDGE_NODE.id(), None);
        layered_data.add_known_option_default(CoreOptions::SPACING_LABEL_LABEL.id(), None);
        layered_data
            .add_known_option_default(CoreOptions::SPACING_LABEL_PORT_HORIZONTAL.id(), None);
        layered_data.add_known_option_default(CoreOptions::SPACING_LABEL_PORT_VERTICAL.id(), None);
        layered_data.add_known_option_default(CoreOptions::SPACING_LABEL_NODE.id(), None);
        layered_data.add_known_option_default(CoreOptions::SPACING_NODE_NODE.id(), None);
        layered_data.add_known_option_default(CoreOptions::SPACING_NODE_SELF_LOOP.id(), None);
        layered_data.add_known_option_default(CoreOptions::SPACING_PORT_PORT.id(), None);
        layered_data.add_known_option_default(CoreOptions::SPACING_INDIVIDUAL.id(), None);
        layered_data.add_known_option_default(CoreOptions::PRIORITY.id(), arc_any(0_i32));
        layered_data.add_known_option_default(
            CoreOptions::SEPARATE_CONNECTED_COMPONENTS.id(),
            arc_any(true),
        );
        layered_data.add_known_option_default(
            CoreOptions::PORT_ALIGNMENT_DEFAULT.id(),
            arc_any(PortAlignment::Justified),
        );
        layered_data.add_known_option_default(CoreOptions::TOPDOWN_LAYOUT.id(), None);
        layered_data.add_known_option_default(CoreOptions::TOPDOWN_SCALE_FACTOR.id(), None);
        layered_data
            .add_known_option_default(CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH.id(), None);
        layered_data.add_known_option_default(
            CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO.id(),
            None,
        );
        layered_data.add_known_option_default(
            CoreOptions::TOPDOWN_NODE_TYPE.id(),
            arc_any(TopdownNodeTypes::HierarchicalNode),
        );
        layered_data.add_known_option_default(
            CoreOptions::PADDING.id(),
            arc_any(ElkPadding::with_any(12.0)),
        );
        layered_data.add_known_option_default(
            CoreOptions::EDGE_ROUTING.id(),
            arc_any(EdgeRouting::Orthogonal),
        );
        layered_data
            .add_known_option_default(CoreOptions::PORT_BORDER_OFFSET.id(), arc_any(0.0_f64));
        layered_data.add_known_option_default(CoreOptions::RANDOM_SEED.id(), arc_any(1_i32));
        layered_data.add_known_option_default(CoreOptions::ASPECT_RATIO.id(), arc_any(1.6_f64));
        layered_data.add_known_option_default(CoreOptions::NO_LAYOUT.id(), None);
        self.register_layout_algorithm(layered_data);
    }
}

struct Registry<'a> {
    service: &'a LayoutMetaDataService,
    option_dependencies: Vec<Triple>,
    option_support: Vec<Triple>,
}

impl<'a> Registry<'a> {
    fn new(service: &'a LayoutMetaDataService) -> Self {
        Registry {
            service,
            option_dependencies: Vec::new(),
            option_support: Vec::new(),
        }
    }

    fn apply_dependencies(&mut self) {
        let mut storage = self.service.storage.lock().unwrap();

        let algorithms: Vec<LayoutAlgorithmData> = storage.algorithms.values().cloned().collect();
        for algorithm in algorithms {
            let category_id = algorithm.category_id().unwrap_or("");
            let category = if category_id.is_empty() {
                retrieve_backup_category(&mut storage)
            } else {
                storage.categories.get_mut(category_id)
            };

            if let Some(category) = category {
                if !category
                    .layouters()
                    .iter()
                    .any(|existing| existing == &algorithm)
                {
                    category.layouters_mut().push(algorithm.clone());
                }
            }
        }

        for dep in &self.option_dependencies {
            let target = storage.options.get(&dep.second_id).cloned();
            if let (Some(source), Some(target)) = (storage.options.get_mut(&dep.first_id), target) {
                source.dependencies_mut().push(LayoutOptionDependency::new(
                    target,
                    dep.value.as_ref().map(Arc::clone),
                ));
            }
        }
        self.option_dependencies.clear();

        for sup in &self.option_support {
            let option_id = storage
                .options
                .get(&sup.second_id)
                .map(|option| option.id().to_string());
            if let (Some(algorithm), Some(option_id)) =
                (storage.algorithms.get_mut(&sup.first_id), option_id)
            {
                algorithm.add_known_option_default(option_id, sup.value.as_ref().map(Arc::clone));
            }
        }
        self.option_support.clear();
    }
}

impl LayoutMetaDataRegistry for Registry<'_> {
    fn register_algorithm(&mut self, algorithm_data: LayoutAlgorithmData) {
        self.service.register_layout_algorithm(algorithm_data);
    }

    fn register_option(&mut self, option_data: LayoutOptionData) {
        self.service.register_layout_option(option_data);
    }

    fn register_category(&mut self, category_data: LayoutCategoryData) {
        self.service.register_layout_category(category_data);
    }

    fn add_dependency(
        &mut self,
        source_option: &str,
        target_option: &str,
        required_value: Option<Arc<dyn Any + Send + Sync>>,
    ) {
        self.option_dependencies.push(Triple {
            first_id: source_option.to_string(),
            second_id: target_option.to_string(),
            value: required_value,
        });
    }

    fn add_option_support(
        &mut self,
        algorithm: &str,
        option: &str,
        default_value: Option<Arc<dyn Any + Send + Sync>>,
    ) {
        self.option_support.push(Triple {
            first_id: algorithm.to_string(),
            second_id: option.to_string(),
            value: default_value,
        });
    }
}

struct Triple {
    first_id: String,
    second_id: String,
    value: Option<Arc<dyn Any + Send + Sync>>,
}

fn retrieve_backup_category(
    storage: &mut LayoutMetaDataStorage,
) -> Option<&mut LayoutCategoryData> {
    if !storage.categories.contains_key("") {
        let other_category = LayoutCategoryData::builder().id("").name("Other").create();
        storage.categories.insert("".to_string(), other_category);
    }
    storage.categories.get_mut("")
}

fn id_matches_suffix(id: &str, suffix: &str) -> bool {
    if !id.ends_with(suffix) {
        return false;
    }
    if id.len() == suffix.len() {
        return true;
    }
    id.as_bytes()
        .get(id.len().saturating_sub(suffix.len() + 1))
        .map(|value| *value == b'.')
        .unwrap_or(false)
}
