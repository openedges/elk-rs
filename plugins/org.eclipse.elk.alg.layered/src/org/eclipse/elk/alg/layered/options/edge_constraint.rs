#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum EdgeConstraint {
    None,
    IncomingOnly,
    OutgoingOnly,
}
