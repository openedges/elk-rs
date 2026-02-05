use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;

use crate::org::eclipse::elk::alg::rectpacking::p1widthapproximation::best_candidate_filter::BestCandidateFilter;
use crate::org::eclipse::elk::alg::rectpacking::util::{DrawingData, DrawingUtil};

pub struct ScaleMeasureFilter;

impl BestCandidateFilter for ScaleMeasureFilter {
    fn filter_list(
        &self,
        candidates: Vec<DrawingData>,
        _aspect_ratio: f64,
        padding: &ElkPadding,
    ) -> Vec<DrawingData> {
        let mut max_scale = f64::NEG_INFINITY;
        for opt in &candidates {
            let scale = DrawingUtil::compute_scale_measure(
                opt.drawing_width() + padding.left + padding.right,
                opt.drawing_height() + padding.top + padding.bottom,
                opt.desired_aspect_ratio(),
            );
            max_scale = max_scale.max(scale);
        }
        candidates
            .into_iter()
            .filter(|candidate| {
                let scale = DrawingUtil::compute_scale_measure(
                    candidate.drawing_width() + padding.left + padding.right,
                    candidate.drawing_height() + padding.top + padding.bottom,
                    candidate.desired_aspect_ratio(),
                );
                scale == max_scale
            })
            .collect()
    }
}
