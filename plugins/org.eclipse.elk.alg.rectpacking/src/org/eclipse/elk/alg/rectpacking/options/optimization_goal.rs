#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum OptimizationGoal {
    AspectRatioDriven,
    #[default]
    MaxScaleDriven,
    AreaDriven,
}
