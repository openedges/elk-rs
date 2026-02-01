#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Alignment {
    Automatic,
    Left,
    Right,
    Top,
    Bottom,
    Center,
}
