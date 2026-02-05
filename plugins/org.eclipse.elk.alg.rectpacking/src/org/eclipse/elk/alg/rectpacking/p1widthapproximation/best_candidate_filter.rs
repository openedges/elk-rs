use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;

use crate::org::eclipse::elk::alg::rectpacking::util::DrawingData;

pub trait BestCandidateFilter {
    fn filter_list(
        &self,
        candidates: Vec<DrawingData>,
        aspect_ratio: f64,
        padding: &ElkPadding,
    ) -> Vec<DrawingData>;
}
