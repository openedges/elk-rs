use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub struct ExclusiveBounds;

impl ExclusiveBounds {
    pub fn greater_than(exclusive_lower_bound: f64) -> ExclusiveLowerBound {
        ExclusiveLowerBound::new(exclusive_lower_bound)
    }

    pub fn less_than(exclusive_upper_bound: f64) -> ExclusiveUpperBound {
        ExclusiveUpperBound::new(exclusive_upper_bound)
    }
}

#[derive(Clone, Debug)]
pub struct ExclusiveLowerBound {
    exclusive_lower_bound: f64,
}

impl ExclusiveLowerBound {
    pub fn new(exclusive_lower_bound: f64) -> Self {
        ExclusiveLowerBound {
            exclusive_lower_bound,
        }
    }

    pub fn compare_to(&self, value: f64) -> Ordering {
        if self.exclusive_lower_bound < value {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

impl PartialEq for ExclusiveLowerBound {
    fn eq(&self, other: &Self) -> bool {
        self.exclusive_lower_bound == other.exclusive_lower_bound
    }
}

impl Eq for ExclusiveLowerBound {}

impl Hash for ExclusiveLowerBound {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.exclusive_lower_bound.to_bits().hash(state);
    }
}

impl std::fmt::Display for ExclusiveLowerBound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} (exclusive)", self.exclusive_lower_bound)
    }
}

#[derive(Clone, Debug)]
pub struct ExclusiveUpperBound {
    exclusive_upper_bound: f64,
}

impl ExclusiveUpperBound {
    pub fn new(exclusive_upper_bound: f64) -> Self {
        ExclusiveUpperBound {
            exclusive_upper_bound,
        }
    }

    pub fn compare_to(&self, value: f64) -> Ordering {
        if self.exclusive_upper_bound > value {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    }
}

impl PartialEq for ExclusiveUpperBound {
    fn eq(&self, other: &Self) -> bool {
        self.exclusive_upper_bound == other.exclusive_upper_bound
    }
}

impl Eq for ExclusiveUpperBound {}

impl Hash for ExclusiveUpperBound {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.exclusive_upper_bound.to_bits().hash(state);
    }
}

impl std::fmt::Display for ExclusiveUpperBound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} (exclusive)", self.exclusive_upper_bound)
    }
}
