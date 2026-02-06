#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum GreedySwitchType {
    OneSided,
    TwoSided,
    #[default]
    Off,
}
