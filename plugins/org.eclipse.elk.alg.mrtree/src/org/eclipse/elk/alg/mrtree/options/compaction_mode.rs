#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum CompactionMode {
    None,
    LevelPreserving,
    Aggressive,
}
