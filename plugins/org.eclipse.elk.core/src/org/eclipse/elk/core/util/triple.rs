use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Triple<F, S, T> {
    first: F,
    second: S,
    third: T,
}

impl<F, S, T> Triple<F, S, T> {
    pub fn new(first: F, second: S, third: T) -> Self {
        Triple {
            first,
            second,
            third,
        }
    }

    pub fn first(&self) -> &F {
        &self.first
    }

    pub fn second(&self) -> &S {
        &self.second
    }

    pub fn third(&self) -> &T {
        &self.third
    }
}

impl<F: fmt::Display, S: fmt::Display, T: fmt::Display> fmt::Display for Triple<F, S, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.first, self.second, self.third)
    }
}
