#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum OrderWeighting {
    ModelOrder,
    Descendants,
    Fan,
    Constraint,
}
