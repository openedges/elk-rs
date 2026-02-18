use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::org::eclipse::elk::graph::util::ElkReflect;

pub trait IPropertyValueProxy: Send + Sync {
    fn resolve_value(&self, property_id: &str) -> Option<Arc<dyn Any + Send + Sync>>;
}

#[derive(Clone)]
pub enum PropertyValue {
    Resolved(Arc<dyn Any + Send + Sync>),
    Proxy(Arc<dyn IPropertyValueProxy>),
}

#[derive(Clone)]
pub enum Bound<T> {
    NegativeInfinity,
    PositiveInfinity,
    Value(T),
}

impl<T> Bound<T> {
    pub fn negative_infinity() -> Self {
        Bound::NegativeInfinity
    }

    pub fn positive_infinity() -> Self {
        Bound::PositiveInfinity
    }
}

impl<T: PartialOrd> Bound<T> {
    pub fn compare(&self, value: &T) -> std::cmp::Ordering {
        match self {
            Bound::NegativeInfinity => std::cmp::Ordering::Less,
            Bound::PositiveInfinity => std::cmp::Ordering::Greater,
            Bound::Value(bound) => bound
                .partial_cmp(value)
                .unwrap_or(std::cmp::Ordering::Equal),
        }
    }
}

impl<T: fmt::Display> fmt::Display for Bound<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Bound::NegativeInfinity => write!(f, "-inf"),
            Bound::PositiveInfinity => write!(f, "+inf"),
            Bound::Value(value) => write!(f, "{value}"),
        }
    }
}

pub struct Property<T: Clone + Send + Sync + 'static> {
    id: String,
    default_value: Option<T>,
    lower_bound: Bound<T>,
    upper_bound: Bound<T>,
}

impl<T: Clone + Send + Sync + 'static> Property<T> {
    pub fn new(id: impl Into<String>) -> Self {
        Property {
            id: id.into(),
            default_value: None,
            lower_bound: Bound::negative_infinity(),
            upper_bound: Bound::positive_infinity(),
        }
    }

    pub fn with_default(id: impl Into<String>, default_value: T) -> Self {
        let mut property = Property::new(id);
        property.default_value = Some(default_value);
        property
    }

    pub fn from_property(other: &Property<T>, default_value: T) -> Self {
        let mut property = Property::new(other.id.clone());
        property.default_value = Some(default_value);
        property
    }

    pub fn with_default_and_lower(
        id: impl Into<String>,
        default_value: T,
        lower_bound: Bound<T>,
    ) -> Self {
        let mut property = Property::with_default(id, default_value);
        property.lower_bound = lower_bound;
        property
    }

    pub fn with_default_and_bounds(
        id: impl Into<String>,
        default_value: T,
        lower_bound: Bound<T>,
        upper_bound: Bound<T>,
    ) -> Self {
        let mut property = Property::with_default_and_lower(id, default_value, lower_bound);
        property.upper_bound = upper_bound;
        property
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn get_default(&self) -> Option<T> {
        let value = self.default_value.as_ref()?;
        if !ElkReflect::has_clone::<T>() {
            panic!(
                "Couldn't clone property '{}'. Make sure its type is registered with ElkReflect.",
                self.id
            );
        }
        ElkReflect::clone_value(value).or_else(|| {
            panic!(
                "Couldn't clone property '{}'. Make sure its type is registered with ElkReflect.",
                self.id
            );
        })
    }

    pub fn get_lower_bound(&self) -> &Bound<T> {
        &self.lower_bound
    }

    pub fn get_upper_bound(&self) -> &Bound<T> {
        &self.upper_bound
    }

    pub fn is_cloneable(&self) -> bool {
        ElkReflect::has_clone::<T>()
    }
}

impl<T: Clone + Send + Sync + 'static> fmt::Debug for Property<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Property").field("id", &self.id).finish()
    }
}

impl<T: Clone + Send + Sync + 'static> fmt::Display for Property<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl<T: Clone + Send + Sync + 'static> PartialEq for Property<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Clone + Send + Sync + 'static> Eq for Property<T> {}

impl<T: Clone + Send + Sync + 'static> Hash for Property<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Clone)]
pub struct MapPropertyHolder {
    property_map: HashMap<String, PropertyValue>,
}

impl MapPropertyHolder {
    pub fn new() -> Self {
        MapPropertyHolder {
            property_map: HashMap::new(),
        }
    }

    pub fn set_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
        value: Option<T>,
    ) -> &mut Self {
        match value {
            Some(value) => {
                self.property_map.insert(
                    property.id().to_owned(),
                    PropertyValue::Resolved(Arc::new(value)),
                );
            }
            None => {
                self.property_map.remove(property.id());
            }
        }
        self
    }

    pub fn set_property_proxy(
        &mut self,
        property_id: impl Into<String>,
        proxy: Arc<dyn IPropertyValueProxy>,
    ) -> &mut Self {
        self.property_map
            .insert(property_id.into(), PropertyValue::Proxy(proxy));
        self
    }

    pub fn set_property_any(
        &mut self,
        property_id: impl Into<String>,
        value: Option<Arc<dyn Any + Send + Sync>>,
    ) -> &mut Self {
        match value {
            Some(value) => {
                self.property_map
                    .insert(property_id.into(), PropertyValue::Resolved(value));
            }
            None => {
                self.property_map.remove(&property_id.into());
            }
        }
        self
    }

    pub fn get_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
    ) -> Option<T> {
        if let Some(value) = self.property_map.get(property.id()) {
            match value {
                PropertyValue::Resolved(value) => {
                    let typed = value.clone().downcast::<T>().ok()?;
                    return Some((*typed).clone());
                }
                PropertyValue::Proxy(proxy) => {
                    if let Some(resolved) = proxy.resolve_value(property.id()) {
                        let typed = resolved.clone().downcast::<T>().ok()?;
                        self.property_map
                            .insert(property.id().to_owned(), PropertyValue::Resolved(resolved));
                        return Some((*typed).clone());
                    }
                }
            }
        }

        let default_value = property.get_default();
        if property.is_cloneable() {
            if let Some(default_value) = default_value.clone() {
                self.set_property(property, Some(default_value));
            }
        }
        default_value
    }

    pub fn has_property<T: Clone + Send + Sync + 'static>(&self, property: &Property<T>) -> bool {
        self.property_map.contains_key(property.id())
    }

    pub fn copy_properties(&mut self, other: &MapPropertyHolder) -> &mut Self {
        if self.property_map.is_empty() {
            self.property_map = other.property_map.clone();
        } else {
            self.property_map.extend(other.property_map.clone());
        }
        self
    }

    pub fn get_all_properties(&self) -> &HashMap<String, PropertyValue> {
        &self.property_map
    }

    pub fn has_property_id(&self, property_id: &str) -> bool {
        self.property_map.contains_key(property_id)
    }

    pub fn clear(&mut self) {
        self.property_map.clear();
    }
}

impl Default for MapPropertyHolder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum GraphFeature {
    SelfLoops,
    InsideSelfLoops,
    MultiEdges,
    EdgeLabels,
    Ports,
    Compound,
    Clusters,
    Disconnected,
}

impl GraphFeature {
    pub fn description(&self) -> &'static str {
        match self {
            GraphFeature::SelfLoops => "Edges connecting a node with itself.",
            GraphFeature::InsideSelfLoops => "Self-loops routed through a node instead of around it.",
            GraphFeature::MultiEdges => "Multiple edges with the same source and target node.",
            GraphFeature::EdgeLabels => "Labels that are associated with edges.",
            GraphFeature::Ports => "Edges are connected to nodes over ports.",
            GraphFeature::Compound => {
                "Edges that connect nodes from different hierarchy levels and are incident to compound nodes."
            }
            GraphFeature::Clusters => {
                "Edges that connect nodes from different clusters, but not the cluster parent nodes."
            }
            GraphFeature::Disconnected => "Multiple connected components.",
        }
    }
}

pub struct PropertyHolderComparator<T: Ord + Clone + Send + Sync + 'static> {
    property: Property<T>,
}

impl<T: Ord + Clone + Send + Sync + 'static> PropertyHolderComparator<T> {
    pub fn with(property: &Property<T>) -> Self {
        PropertyHolderComparator {
            property: Property::from_property(property, property.get_default().expect("default")),
        }
    }

    pub fn compare(
        &self,
        holder1: &mut MapPropertyHolder,
        holder2: &mut MapPropertyHolder,
    ) -> std::cmp::Ordering {
        let p1 = holder1.get_property(&self.property);
        let p2 = holder2.get_property(&self.property);
        match (p1, p2) {
            (Some(v1), Some(v2)) => v1.cmp(&v2),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    }
}
