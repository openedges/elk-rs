use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::options::OptimizationGoal;
use crate::org::eclipse::elk::alg::rectpacking::p1widthapproximation::calculations::Calculations;
use crate::org::eclipse::elk::alg::rectpacking::p1widthapproximation::{
    AreaFilter, AspectRatioFilter, BestCandidateFilter, ScaleMeasureFilter,
};
use crate::org::eclipse::elk::alg::rectpacking::util::{
    DrawingData, DrawingDataDescriptor,
};

pub struct AreaApproximation {
    aspect_ratio: f64,
    goal: OptimizationGoal,
    lp_shift: bool,
}

impl AreaApproximation {
    pub fn new(aspect_ratio: f64, goal: OptimizationGoal, lp_shift: bool) -> Self {
        AreaApproximation {
            aspect_ratio,
            goal,
            lp_shift,
        }
    }

    pub fn approx_bounding_box(
        &self,
        rectangles: &[ElkNodeRef],
        node_node_spacing: f64,
        padding: &ElkPadding,
    ) -> DrawingData {
        let first_rect = rectangles
            .first()
            .expect("at least one rectangle needed")
            .clone();
        {
            let mut first_mut = first_rect.borrow_mut();
            first_mut.connectable().shape().set_location(0.0, 0.0);
        }
        let mut placed_rects: Vec<ElkNodeRef> = Vec::new();
        placed_rects.push(first_rect.clone());
        let mut last_placed = first_rect.clone();
        let mut current_values = DrawingData::new(
            self.aspect_ratio,
            last_placed.borrow_mut().connectable().shape().width(),
            last_placed.borrow_mut().connectable().shape().height(),
            DrawingDataDescriptor::WholeDrawing,
        );

        for rectangle_idx in 1..rectangles.len() {
            let to_place = rectangles[rectangle_idx].clone();

            let opt1 = self.calc_values_for_opt(
                DrawingDataDescriptor::CandidatePositionLastPlacedRight,
                &to_place,
                &last_placed,
                &current_values,
                &placed_rects,
                node_node_spacing,
            );
            let opt2 = self.calc_values_for_opt(
                DrawingDataDescriptor::CandidatePositionLastPlacedBelow,
                &to_place,
                &last_placed,
                &current_values,
                &placed_rects,
                node_node_spacing,
            );
            let opt3 = self.calc_values_for_opt(
                DrawingDataDescriptor::CandidatePositionWholeDrawingRight,
                &to_place,
                &last_placed,
                &current_values,
                &placed_rects,
                node_node_spacing,
            );
            let opt4 = self.calc_values_for_opt(
                DrawingDataDescriptor::CandidatePositionWholeDrawingBelow,
                &to_place,
                &last_placed,
                &current_values,
                &placed_rects,
                node_node_spacing,
            );

            let best_opt = self.find_best_candidate(opt1, opt2, opt3, opt4, &to_place, &last_placed, padding);

            {
                let mut to_place_mut = to_place.borrow_mut();
                let shape = to_place_mut.connectable().shape();
                shape.set_x(best_opt.next_x_coordinate());
                shape.set_y(best_opt.next_y_coordinate());
            }
            let mut updated = best_opt;
            updated.set_placement_option(DrawingDataDescriptor::WholeDrawing);
            current_values = updated;
            last_placed = to_place.clone();
            placed_rects.push(to_place);
        }

        current_values
    }

    fn find_best_candidate(
        &self,
        opt1: DrawingData,
        opt2: DrawingData,
        opt3: DrawingData,
        opt4: DrawingData,
        to_place: &ElkNodeRef,
        last_placed: &ElkNodeRef,
        padding: &ElkPadding,
    ) -> DrawingData {
        let mut candidates = vec![opt1, opt2, opt3, opt4];
        let filters: Vec<Box<dyn BestCandidateFilter>> = match self.goal {
            OptimizationGoal::MaxScaleDriven => vec![
                Box::new(ScaleMeasureFilter),
                Box::new(AreaFilter),
                Box::new(AspectRatioFilter),
            ],
            OptimizationGoal::AspectRatioDriven => vec![
                Box::new(AspectRatioFilter),
                Box::new(AreaFilter),
                Box::new(ScaleMeasureFilter),
            ],
            OptimizationGoal::AreaDriven => vec![
                Box::new(AreaFilter),
                Box::new(ScaleMeasureFilter),
                Box::new(AspectRatioFilter),
            ],
        };

        for filter in filters {
            if candidates.len() > 1 {
                candidates = filter.filter_list(candidates, self.aspect_ratio, padding);
            }
        }

        if candidates.len() == 1 {
            return candidates.pop().expect("candidate exists");
        }
        if candidates.len() == 2 {
            return self.check_special_cases(&candidates[0], &candidates[1], last_placed, to_place);
        }
        candidates
            .into_iter()
            .next()
            .expect("candidate exists")
    }

    fn check_special_cases(
        &self,
        drawing1: &DrawingData,
        drawing2: &DrawingData,
        last_placed: &ElkNodeRef,
        to_place: &ElkNodeRef,
    ) -> DrawingData {
        let first_opt = drawing1.placement_option();
        let second_opt = drawing2.placement_option();

        let first_opt_lpb_or_wdb = matches!(
            first_opt,
            DrawingDataDescriptor::CandidatePositionLastPlacedBelow
                | DrawingDataDescriptor::CandidatePositionWholeDrawingBelow
        );
        let second_opt_lpb_or_wdb = matches!(
            second_opt,
            DrawingDataDescriptor::CandidatePositionLastPlacedBelow
                | DrawingDataDescriptor::CandidatePositionWholeDrawingBelow
        );
        let first_opt_lpr_or_wdr = matches!(
            first_opt,
            DrawingDataDescriptor::CandidatePositionLastPlacedRight
                | DrawingDataDescriptor::CandidatePositionWholeDrawingRight
        );
        let second_opt_lpr_or_wdr = matches!(
            second_opt,
            DrawingDataDescriptor::CandidatePositionLastPlacedRight
                | DrawingDataDescriptor::CandidatePositionWholeDrawingRight
        );
        let first_opt_lpr_or_lpb = matches!(
            first_opt,
            DrawingDataDescriptor::CandidatePositionLastPlacedRight
                | DrawingDataDescriptor::CandidatePositionLastPlacedBelow
        );
        let second_opt_lpr_or_lpb = matches!(
            second_opt,
            DrawingDataDescriptor::CandidatePositionLastPlacedRight
                | DrawingDataDescriptor::CandidatePositionLastPlacedBelow
        );

        if first_opt_lpb_or_wdb && second_opt_lpb_or_wdb {
            return if drawing1.placement_option()
                == DrawingDataDescriptor::CandidatePositionWholeDrawingBelow
            {
                drawing1.clone()
            } else {
                drawing2.clone()
            };
        } else if first_opt_lpr_or_wdr && second_opt_lpr_or_wdr {
            return if drawing1.placement_option()
                == DrawingDataDescriptor::CandidatePositionWholeDrawingRight
            {
                drawing1.clone()
            } else {
                drawing2.clone()
            };
        } else if first_opt_lpr_or_lpb && second_opt_lpr_or_lpb {
            let (lpr_opt, lpb_opt) = if first_opt
                == DrawingDataDescriptor::CandidatePositionLastPlacedRight
            {
                (drawing1, drawing2)
            } else {
                (drawing2, drawing1)
            };
            let area_lpr = Calculations::calculate_area_lpr(last_placed, to_place, lpr_opt);
            let area_lpb = Calculations::calculate_area_lpb(last_placed, to_place, lpb_opt);
            if area_lpr <= area_lpb {
                return if drawing1.placement_option()
                    == DrawingDataDescriptor::CandidatePositionLastPlacedRight
                {
                    drawing1.clone()
                } else {
                    drawing2.clone()
                };
            } else {
                return if drawing1.placement_option()
                    == DrawingDataDescriptor::CandidatePositionLastPlacedBelow
                {
                    drawing1.clone()
                } else {
                    drawing2.clone()
                };
            }
        }

        drawing1.clone()
    }

    fn calc_values_for_opt(
        &self,
        option: DrawingDataDescriptor,
        to_place: &ElkNodeRef,
        last_placed: &ElkNodeRef,
        drawing: &DrawingData,
        placed_rects: &[ElkNodeRef],
        node_node_spacing: f64,
    ) -> DrawingData {
        let mut x = 0.0;
        let mut y = 0.0;
        let drawing_width = drawing.drawing_width();
        let drawing_height = drawing.drawing_height();
        let height_to_place = to_place.borrow_mut().connectable().shape().height();
        let width_to_place = to_place.borrow_mut().connectable().shape().width();
        let width;
        let height;

        match option {
            DrawingDataDescriptor::CandidatePositionLastPlacedRight => {
                let last_width = last_placed.borrow_mut().connectable().shape().width();
                let last_x = last_placed.borrow_mut().connectable().shape().x();
                x = last_x + last_width + node_node_spacing;
                if self.lp_shift {
                    y = Calculations::calculate_y_for_lpr(x, placed_rects, last_placed, node_node_spacing);
                } else {
                    y = last_placed.borrow_mut().connectable().shape().y();
                }
                width = Calculations::width_lpr_or_lpb(drawing_width, x, width_to_place);
                height = Calculations::height_lpr_or_lpb(drawing_height, y, height_to_place);
            }
            DrawingDataDescriptor::CandidatePositionLastPlacedBelow => {
                let last_height = last_placed.borrow_mut().connectable().shape().height();
                let last_y = last_placed.borrow_mut().connectable().shape().y();
                y = last_y + last_height + node_node_spacing;
                if self.lp_shift {
                    x = Calculations::calculate_x_for_lpb(y, placed_rects, last_placed, node_node_spacing);
                } else {
                    x = last_placed.borrow_mut().connectable().shape().x();
                }
                width = Calculations::width_lpr_or_lpb(drawing_width, x, width_to_place);
                height = Calculations::height_lpr_or_lpb(drawing_height, y, height_to_place);
            }
            DrawingDataDescriptor::CandidatePositionWholeDrawingRight => {
                x = drawing_width + node_node_spacing;
                y = 0.0;
                width = drawing_width + node_node_spacing + width_to_place;
                height = drawing_height.max(height_to_place);
            }
            DrawingDataDescriptor::CandidatePositionWholeDrawingBelow => {
                x = 0.0;
                y = drawing_height + node_node_spacing;
                width = drawing_width.max(width_to_place);
                height = drawing_height + node_node_spacing + height_to_place;
            }
            DrawingDataDescriptor::WholeDrawing => {
                width = drawing_width;
                height = drawing_height;
            }
        }

        DrawingData::with_coordinates(self.aspect_ratio, width, height, option, x, y)
    }
}
