use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;

use crate::org::eclipse::elk::alg::rectpacking::p1widthapproximation::best_candidate_filter::BestCandidateFilter;
use crate::org::eclipse::elk::alg::rectpacking::util::DrawingData;

pub struct AspectRatioFilter;

impl BestCandidateFilter for AspectRatioFilter {
    fn filter_list(
        &self,
        candidates: Vec<DrawingData>,
        aspect_ratio: f64,
        padding: &ElkPadding,
    ) -> Vec<DrawingData> {
        let mut smallest_deviation = f64::INFINITY;
        for opt in &candidates {
            let ratio = (opt.drawing_width() + padding.left + padding.right)
                / (opt.drawing_height() + padding.top + padding.bottom);
            smallest_deviation = smallest_deviation.min((ratio - aspect_ratio).abs());
        }
        candidates
            .into_iter()
            .filter(|candidate| {
                let ratio = (candidate.drawing_width() + padding.left + padding.right)
                    / (candidate.drawing_height() + padding.top + padding.bottom);
                (ratio - aspect_ratio).abs() == smallest_deviation
            })
            .collect()
    }
}
