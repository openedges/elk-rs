#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum GreedySwitchType {
    OneSided,
    TwoSided,
    Off,
}

impl Default for GreedySwitchType {
    fn default() -> Self {
        GreedySwitchType::Off
    }
}
