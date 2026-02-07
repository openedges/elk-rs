use super::OneDimensionalCompactor;

pub trait ICompactionAlgorithm {
    fn compact(&self, compactor: &mut OneDimensionalCompactor);
}
