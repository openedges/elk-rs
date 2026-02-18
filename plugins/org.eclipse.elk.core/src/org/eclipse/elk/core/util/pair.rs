use std::cmp::Ordering;
use std::fmt;
use std::hash::Hash;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Pair<F, S> {
    pub first: F,
    pub second: S,
}

impl<F, S> Pair<F, S> {
    pub fn of(first: F, second: S) -> Self {
        Pair { first, second }
    }

    pub fn set_first(&mut self, first: F) {
        self.first = first;
    }

    pub fn set_second(&mut self, second: S) {
        self.second = second;
    }

    pub fn first(&self) -> &F {
        &self.first
    }

    pub fn second(&self) -> &S {
        &self.second
    }

    pub fn from_map<G: Clone, T: Clone>(map: &std::collections::HashMap<G, T>) -> Vec<Pair<G, T>> {
        map.iter()
            .map(|(key, value)| Pair::of(key.clone(), value.clone()))
            .collect()
    }
}

impl<F: Default, S: Default> Pair<F, S> {
    pub fn create() -> Self {
        Pair {
            first: F::default(),
            second: S::default(),
        }
    }
}

impl<F: fmt::Display, S: fmt::Display> fmt::Display for Pair<F, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pair({},{})", self.first, self.second)
    }
}

pub struct FirstComparator<F, S>(std::marker::PhantomData<(F, S)>);

impl<F, S> Default for FirstComparator<F, S> {
    fn default() -> Self {
        FirstComparator(std::marker::PhantomData)
    }
}

impl<F: Ord, S> FirstComparator<F, S> {
    pub fn compare(&self, left: &Pair<F, S>, right: &Pair<F, S>) -> Ordering {
        left.first.cmp(&right.first)
    }
}

pub struct SecondComparator<F, S>(std::marker::PhantomData<(F, S)>);

impl<F, S> Default for SecondComparator<F, S> {
    fn default() -> Self {
        SecondComparator(std::marker::PhantomData)
    }
}

impl<F, S: Ord> SecondComparator<F, S> {
    pub fn compare(&self, left: &Pair<F, S>, right: &Pair<F, S>) -> Ordering {
        left.second.cmp(&right.second)
    }
}
