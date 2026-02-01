use std::collections::{BTreeSet, HashSet};
use std::hash::Hash;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef;

use crate::org::eclipse::elk::core::validation::GraphIssue;

pub mod elk_util;

pub use elk_util::ElkUtil;

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
}

impl IDataObject for IndividualSpacings {}
