#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum FixedAlignment {
    #[default]
    None,
    LeftUp,
    RightUp,
    LeftDown,
    RightDown,
    Balanced,
}
