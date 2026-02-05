#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum EdgeRoutingMode {
    None,
    MiddleToMiddle,
    AvoidOverlap,
}
