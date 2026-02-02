use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Quadruple<A, B, C, D> {
    first: A,
    second: B,
    third: C,
    fourth: D,
}

impl<A, B, C, D> Quadruple<A, B, C, D> {
    pub fn new(first: A, second: B, third: C, fourth: D) -> Self {
        Quadruple {
            first,
            second,
            third,
            fourth,
        }
    }

    pub fn first(&self) -> &A {
        &self.first
    }

    pub fn second(&self) -> &B {
        &self.second
    }

    pub fn third(&self) -> &C {
        &self.third
    }

    pub fn fourth(&self) -> &D {
        &self.fourth
    }
}

impl<A: fmt::Display, B: fmt::Display, C: fmt::Display, D: fmt::Display> fmt::Display
    for Quadruple<A, B, C, D>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}, {}, {}, {})",
            self.first, self.second, self.third, self.fourth
        )
    }
}
