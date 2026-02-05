#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum RoutingDirection {
    WestToEast,
    NorthToSouth,
    SouthToNorth,
}
