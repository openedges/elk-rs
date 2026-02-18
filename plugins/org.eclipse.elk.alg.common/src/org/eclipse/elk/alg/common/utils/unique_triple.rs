use std::fmt;

#[derive(Clone, Debug)]
pub struct UniqueTriple<F, S, T> {
    first: F,
    second: S,
    third: T,
}

impl<F, S, T> UniqueTriple<F, S, T> {
    pub fn new(first: F, second: S, third: T) -> Self {
        UniqueTriple {
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

impl<F: fmt::Display, S: fmt::Display, T: fmt::Display> fmt::Display for UniqueTriple<F, S, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.first, self.second, self.third)
    }
}
