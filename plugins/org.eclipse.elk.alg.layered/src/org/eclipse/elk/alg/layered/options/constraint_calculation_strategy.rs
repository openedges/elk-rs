#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum ConstraintCalculationStrategy {
    Quadratic,
    #[default]
    Scanline,
}
