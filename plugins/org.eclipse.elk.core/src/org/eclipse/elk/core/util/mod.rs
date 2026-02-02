use std::collections::{BTreeSet, HashSet};
use std::hash::Hash;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkGraphElementRef, ElkNodeRef};

use crate::org::eclipse::elk::core::data::LayoutMetaDataService;
use crate::org::eclipse::elk::core::options::CoreOptions;
use crate::org::eclipse::elk::core::util::adapters::{GraphElementAdapter, NodeAdapter};
use crate::org::eclipse::elk::core::validation::GraphIssue;

pub mod elk_util;
pub mod adapters;
pub mod abstract_random_list_accessor;
pub mod exclusive_bounds;
pub mod elk_spacings;
pub mod property_constants_delegator;
pub mod progress_monitor;
pub mod fixed_layout_provider;
pub mod box_layout_provider;
pub mod random_layout_provider;
pub mod instance_pool;
pub mod algorithm_factory;
pub mod pair;
pub mod default_factory;
pub mod maybe;
pub mod triple;
pub mod quadruple;
pub mod wrapped_exception;
pub mod selection;
pub mod internal;
pub mod persistence;

pub use elk_util::ElkUtil;
pub use abstract_random_list_accessor::AbstractRandomListAccessor;
pub use exclusive_bounds::ExclusiveBounds;
pub use elk_spacings::ElkSpacings;
pub use property_constants_delegator::PropertyConstantsDelegator;
pub use progress_monitor::{
    BasicProgressMonitor, IElkCancelIndicator, IElkProgressMonitor, LoggedGraph, LoggedGraphType,
    NullElkProgressMonitor,
};
pub use fixed_layout_provider::FixedLayoutProvider;
pub use box_layout_provider::BoxLayoutProvider;
pub use random_layout_provider::RandomLayoutProvider;
pub use instance_pool::{IFactory, InstancePool};
pub use algorithm_factory::AlgorithmFactory;
pub use pair::{FirstComparator, Pair, SecondComparator};
pub use default_factory::DefaultFactory;
pub use maybe::Maybe;
pub use triple::Triple;
pub use quadruple::Quadruple;
pub use wrapped_exception::WrappedException;
pub use selection::{DefaultSelectionIterator, SelectionIterator};
pub use persistence::{ElkGraphResource, ElkGraphResourceFactory, ElkGraphXMIHelper, ElkGraphXMISave};

pub trait IDataObject: Clone + Send + Sync + 'static {}

pub trait IGraphElementVisitor {
    fn visit(&mut self, element: &ElkGraphElementRef);
    fn issues(&self) -> Option<&[GraphIssue]> {
        None
    }
}

impl<F> IGraphElementVisitor for F
where
    F: FnMut(&ElkGraphElementRef),
{
    fn visit(&mut self, element: &ElkGraphElementRef) {
        self(element);
    }
}

#[derive(Clone, Debug)]
pub struct Random {
    seed: u64,
}

impl Random {
    const MULTIPLIER: u64 = 0x5DEECE66D;
    const ADDEND: u64 = 0xB;
    const MASK: u64 = (1u64 << 48) - 1;

    pub fn new(seed: u64) -> Self {
        let scrambled = (seed ^ Self::MULTIPLIER) & Self::MASK;
        Random { seed: scrambled }
    }

    pub fn next_double(&mut self) -> f64 {
        let high = self.next(26) as u64;
        let low = self.next(27) as u64;
        let value = (high << 27) + low;
        (value as f64) / ((1u64 << 53) as f64)
    }

    pub fn next_float(&mut self) -> f64 {
        let value = self.next(24) as u64;
        (value as f64) / ((1u64 << 24) as f64)
    }

    pub fn next_int(&mut self, bound: i32) -> i32 {
        if bound <= 0 {
            panic!("bound must be positive");
        }
        if (bound & -bound) == bound {
            return (((bound as i64) * (self.next(31) as i64)) >> 31) as i32;
        }
        loop {
            let bits = self.next(31) as i32;
            let val = bits % bound;
            if bits - val + (bound - 1) >= 0 {
                return val;
            }
        }
    }

    fn next(&mut self, bits: u32) -> u32 {
        self.seed = (self.seed.wrapping_mul(Self::MULTIPLIER).wrapping_add(Self::ADDEND)) & Self::MASK;
        (self.seed >> (48 - bits)) as u32
    }
}

impl Default for Random {
    fn default() -> Self {
        Random::new(0)
    }
}

#[derive(Clone, Debug)]
pub struct LinkedHashSet<T: Eq + Hash + Clone> {
    order: Vec<T>,
    set: HashSet<T>,
}

impl<T: Eq + Hash + Clone> LinkedHashSet<T> {
    pub fn new() -> Self {
        LinkedHashSet {
            order: Vec::new(),
            set: HashSet::new(),
        }
    }

    pub fn insert(&mut self, value: T) {
        if self.set.insert(value.clone()) {
            self.order.push(value);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.order.iter()
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}

impl<T: Eq + Hash + Clone> PartialEq for LinkedHashSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.set == other.set
    }
}

impl<T: Eq + Hash + Clone> Eq for LinkedHashSet<T> {}

impl<T: Eq + Hash + Clone> Default for LinkedHashSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Eq + Hash + Clone> std::iter::FromIterator<T> for LinkedHashSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut set = LinkedHashSet::new();
        for item in iter {
            set.insert(item);
        }
        set
    }
}

pub trait EnumSetType: Copy + Ord + 'static {
    fn variants() -> &'static [Self];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumSet<T: EnumSetType> {
    inner: BTreeSet<T>,
}

impl<T: EnumSetType> EnumSet<T> {
    pub fn none_of() -> Self {
        EnumSet {
            inner: BTreeSet::new(),
        }
    }

    pub fn all_of() -> Self {
        let mut set = BTreeSet::new();
        for value in T::variants() {
            set.insert(*value);
        }
        EnumSet { inner: set }
    }

    pub fn of(values: &[T]) -> Self {
        let mut set = BTreeSet::new();
        for value in values {
            set.insert(*value);
        }
        EnumSet { inner: set }
    }

    pub fn insert(&mut self, value: T) {
        self.inner.insert(value);
    }

    pub fn remove(&mut self, value: &T) {
        self.inner.remove(value);
    }

    pub fn contains(&self, value: &T) -> bool {
        self.inner.contains(value)
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.inner.iter()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<T: EnumSetType> Default for EnumSet<T> {
    fn default() -> Self {
        Self::none_of()
    }
}

#[derive(Clone, Default)]
pub struct IndividualSpacings {
    holder: MapPropertyHolder,
}

impl IndividualSpacings {
    const SERIALIZED_OPTION_SEPARATOR: &'static str = ";,;";

    pub fn new() -> Self {
        IndividualSpacings {
            holder: MapPropertyHolder::new(),
        }
    }

    pub fn from_other(other: &IndividualSpacings) -> Self {
        IndividualSpacings {
            holder: other.holder.clone(),
        }
    }

    pub fn properties(&self) -> &MapPropertyHolder {
        &self.holder
    }

    pub fn properties_mut(&mut self) -> &mut MapPropertyHolder {
        &mut self.holder
    }

    pub fn get_individual_or_inherited<T: Clone + Send + Sync + 'static>(
        node: &ElkNodeRef,
        property: &Property<T>,
    ) -> Option<T> {
        let mut result = with_node_properties_mut(node, |props| {
            let mut individual = props.get_property(CoreOptions::SPACING_INDIVIDUAL)?;
            if individual.properties().has_property(property) {
                individual.properties_mut().get_property(property)
            } else {
                None
            }
        });

        if result.is_none() {
            let parent = node.borrow().parent();
            if let Some(parent) = parent {
                result = with_node_properties_mut(&parent, |props| props.get_property(property));
            }
        }

        result
    }

    pub fn get_individual_or_inherited_adapter<T, N>(
        node: &N,
        property: &Property<T>,
    ) -> Option<T>
    where
        T: Clone + Send + Sync + 'static,
        N: NodeAdapter<ElkNodeRef>,
        N::Graph: GraphElementAdapter<ElkNodeRef>,
    {
        let mut result = None;

        if node.has_property(CoreOptions::SPACING_INDIVIDUAL) {
            if let Some(mut individual) = node.get_property(CoreOptions::SPACING_INDIVIDUAL) {
                if individual.properties().has_property(property) {
                    result = individual.properties_mut().get_property(property);
                }
            }
        }

        if result.is_none() {
            if let Some(graph) = node.get_graph() {
                result = graph.get_property(property);
            }
        }

        if result.is_none() {
            result = property.get_default();
        }

        result
    }

    pub fn parse(&mut self, value: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            return Ok(());
        }

        for option_string in value.split(Self::SERIALIZED_OPTION_SEPARATOR) {
            let mut parts = option_string.splitn(2, ':');
            let option_id = parts
                .next()
                .ok_or_else(|| "Invalid option entry.".to_string())?;
            let option_value = parts
                .next()
                .ok_or_else(|| "Invalid option entry.".to_string())?;

            let option_data = LayoutMetaDataService::get_instance()
                .get_option_data_by_suffix(option_id)
                .ok_or_else(|| format!("Invalid option id: {option_id}"))?;
            let parsed_value = option_data
                .parse_value(option_value)
                .ok_or_else(|| format!("Invalid option value: {option_value}"))?;
            self.holder
                .set_property_any(option_data.id(), Some(parsed_value));
        }

        Ok(())
    }
}

impl IDataObject for IndividualSpacings {}

impl std::fmt::Display for IndividualSpacings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let serialized = self
            .holder
            .get_all_properties()
            .iter()
            .filter_map(|(id, value)| {
                property_value_to_string(id, value).map(|serialized| format!("{id}:{serialized}"))
            })
            .collect::<Vec<_>>()
            .join(Self::SERIALIZED_OPTION_SEPARATOR);
        write!(f, "{serialized}")
    }
}

fn property_value_to_string(
    property_id: &str,
    value: &org_eclipse_elk_graph::org::eclipse::elk::graph::properties::PropertyValue,
) -> Option<String> {
    use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::PropertyValue;

    match value {
        PropertyValue::Resolved(value) => any_value_to_string(value),
        PropertyValue::Proxy(proxy) => proxy
            .resolve_value(property_id)
            .and_then(|resolved| any_value_to_string(&resolved)),
    }
}

fn any_value_to_string(value: &std::sync::Arc<dyn std::any::Any + Send + Sync>) -> Option<String> {
    if let Some(value) = value.downcast_ref::<f64>() {
        return Some(value.to_string());
    }
    if let Some(value) = value.downcast_ref::<i32>() {
        return Some(value.to_string());
    }
    if let Some(value) = value.downcast_ref::<bool>() {
        return Some(value.to_string());
    }
    if let Some(value) = value.downcast_ref::<String>() {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<crate::org::eclipse::elk::core::math::KVectorChain>() {
        return Some(value.to_string());
    }
    None
}

fn with_node_properties_mut<R>(node: &ElkNodeRef, f: impl FnOnce(&mut MapPropertyHolder) -> R) -> R {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    f(props)
}
