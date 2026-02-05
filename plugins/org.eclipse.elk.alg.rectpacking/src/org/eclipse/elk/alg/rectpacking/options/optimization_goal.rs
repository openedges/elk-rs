#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum OptimizationGoal {
    AspectRatioDriven,
    MaxScaleDriven,
    AreaDriven,
}

impl Default for OptimizationGoal {
    fn default() -> Self {
        OptimizationGoal::MaxScaleDriven
    }
}
