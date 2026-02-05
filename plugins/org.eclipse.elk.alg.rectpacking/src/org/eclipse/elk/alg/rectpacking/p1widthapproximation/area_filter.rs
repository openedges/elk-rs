use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;

use crate::org::eclipse::elk::alg::rectpacking::p1widthapproximation::best_candidate_filter::BestCandidateFilter;
use crate::org::eclipse::elk::alg::rectpacking::util::DrawingData;

pub struct AreaFilter;

impl BestCandidateFilter for AreaFilter {
    fn filter_list(
        &self,
        candidates: Vec<DrawingData>,
        _aspect_ratio: f64,
        padding: &ElkPadding,
    ) -> Vec<DrawingData> {
        let mut min_area = f64::INFINITY;
        for opt in &candidates {
            let area = (opt.drawing_width() + padding.left + padding.right)
                * (opt.drawing_height() + padding.top + padding.bottom);
            min_area = min_area.min(area);
        }
        candidates
            .into_iter()
            .filter(|candidate| {
                let area = (candidate.drawing_width() + padding.left + padding.right)
                    * (candidate.drawing_height() + padding.top + padding.bottom);
                area == min_area
            })
            .collect()
    }
}
