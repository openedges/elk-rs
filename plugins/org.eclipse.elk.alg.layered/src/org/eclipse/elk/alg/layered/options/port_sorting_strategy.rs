#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum PortSortingStrategy {
    #[default]
    InputOrder,
    PortDegree,
}
