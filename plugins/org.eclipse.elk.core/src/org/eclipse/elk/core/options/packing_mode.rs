#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum PackingMode {
    Simple,
    GroupDec,
    GroupMixed,
    GroupInc,
}
