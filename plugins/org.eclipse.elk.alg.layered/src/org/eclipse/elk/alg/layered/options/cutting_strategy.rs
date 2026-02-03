#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum CuttingStrategy {
    Ard,
    Msd,
    Manual,
}
