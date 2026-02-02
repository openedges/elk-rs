#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum PortAlignment {
    Distributed,
    Justified,
    Begin,
    Center,
    End,
}
