#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum CompactionStrategy {
    #[default]
    Polyomino,
}
