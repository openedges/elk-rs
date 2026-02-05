#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum CompactionStrategy {
    Polyomino,
}

impl Default for CompactionStrategy {
    fn default() -> Self {
        CompactionStrategy::Polyomino
    }
}
