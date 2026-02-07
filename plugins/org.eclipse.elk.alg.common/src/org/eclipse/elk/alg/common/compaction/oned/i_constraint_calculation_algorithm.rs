use super::OneDimensionalCompactor;

pub trait IConstraintCalculationAlgorithm {
    fn calculate_constraints(&self, compactor: &mut OneDimensionalCompactor);
}
