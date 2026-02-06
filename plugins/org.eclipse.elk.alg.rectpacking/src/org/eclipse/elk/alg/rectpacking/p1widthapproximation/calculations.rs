use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::util::DrawingData;

pub struct Calculations;

impl Calculations {
    pub fn width_lpr_or_lpb(drawing_width: f64, x: f64, width: f64) -> f64 {
        drawing_width.max(x + width)
    }

    pub fn height_lpr_or_lpb(drawing_height: f64, y: f64, height: f64) -> f64 {
        drawing_height.max(y + height)
    }

    pub fn calculate_y_for_lpr(
        x: f64,
        placed_rects: &[ElkNodeRef],
        last_placed: &ElkNodeRef,
        node_node_spacing: f64,
    ) -> f64 {
        let mut closest_upper_neighbor: Option<ElkNodeRef> = None;
        let mut closest_neighbor_bottom_border = 0.0;
        let last_placed_y = last_placed.borrow_mut().connectable().shape().y();
        for placed_rect in placed_rects {
            let placed_rect_bottom_border = {
                let mut rect_mut = placed_rect.borrow_mut();
                rect_mut.connectable().shape().y() + rect_mut.connectable().shape().height()
            };
            if Self::vertical_order_constraint(placed_rect, x, node_node_spacing)
                && (closest_upper_neighbor.is_none()
                    || last_placed_y - placed_rect_bottom_border
                        < last_placed_y - closest_neighbor_bottom_border)
            {
                closest_upper_neighbor = Some(placed_rect.clone());
                closest_neighbor_bottom_border = placed_rect_bottom_border;
            }
        }

        if closest_upper_neighbor.is_none() {
            0.0
        } else {
            closest_neighbor_bottom_border + node_node_spacing
        }
    }

    pub fn calculate_x_for_lpb(
        y: f64,
        placed_rects: &[ElkNodeRef],
        last_placed: &ElkNodeRef,
        node_node_spacing: f64,
    ) -> f64 {
        let mut closest_left_neighbor: Option<ElkNodeRef> = None;
        let mut closest_neighbor_right_border = 0.0;
        let last_placed_x = last_placed.borrow_mut().connectable().shape().x();
        for placed_rect in placed_rects {
            let placed_rect_right_border = {
                let mut rect_mut = placed_rect.borrow_mut();
                rect_mut.connectable().shape().x() + rect_mut.connectable().shape().width()
            };
            if Self::horizontal_order_constraint(placed_rect, y, node_node_spacing)
                && (closest_left_neighbor.is_none()
                    || last_placed_x - placed_rect_right_border
                        < last_placed_x - closest_neighbor_right_border)
            {
                closest_left_neighbor = Some(placed_rect.clone());
                closest_neighbor_right_border = placed_rect_right_border;
            }
        }

        if closest_left_neighbor.is_none() {
            0.0
        } else {
            closest_neighbor_right_border + node_node_spacing
        }
    }

    pub fn calculate_area_lpr(
        last_placed: &ElkNodeRef,
        to_place: &ElkNodeRef,
        lpr_opt: &DrawingData,
    ) -> f64 {
        let last_placed_bottom_border = {
            let mut rect_mut = last_placed.borrow_mut();
            rect_mut.connectable().shape().y() + rect_mut.connectable().shape().height()
        };
        let to_place_bottom_border = lpr_opt.next_y_coordinate()
            + to_place.borrow_mut().connectable().shape().height();
        let max_y_lpr = last_placed_bottom_border.max(to_place_bottom_border);
        let last_placed_y = last_placed.borrow_mut().connectable().shape().y();
        let height_lpr = max_y_lpr - last_placed_y.min(lpr_opt.next_y_coordinate());
        let width_lpr = lpr_opt.next_x_coordinate()
            + to_place.borrow_mut().connectable().shape().width()
            - last_placed.borrow_mut().connectable().shape().x();
        width_lpr * height_lpr
    }

    pub fn calculate_area_lpb(
        last_placed: &ElkNodeRef,
        to_place: &ElkNodeRef,
        lpb_opt: &DrawingData,
    ) -> f64 {
        let last_placed_right_border = {
            let mut rect_mut = last_placed.borrow_mut();
            rect_mut.connectable().shape().x() + rect_mut.connectable().shape().width()
        };
        let to_place_right_border = lpb_opt.next_x_coordinate()
            + to_place.borrow_mut().connectable().shape().width();
        let max_x_lpb = last_placed_right_border.max(to_place_right_border);
        let last_placed_x = last_placed.borrow_mut().connectable().shape().x();
        let width_lpb = max_x_lpb - last_placed_x.min(lpb_opt.next_x_coordinate());
        let height_lpb = lpb_opt.next_y_coordinate()
            + to_place.borrow_mut().connectable().shape().height()
            - last_placed.borrow_mut().connectable().shape().y();
        width_lpb * height_lpb
    }

    fn vertical_order_constraint(placed_rect: &ElkNodeRef, x: f64, node_node_spacing: f64) -> bool {
        let mut rect_mut = placed_rect.borrow_mut();
        x < rect_mut.connectable().shape().x() + rect_mut.connectable().shape().width() + node_node_spacing
    }

    fn horizontal_order_constraint(placed_rect: &ElkNodeRef, y: f64, node_node_spacing: f64) -> bool {
        let mut rect_mut = placed_rect.borrow_mut();
        y < rect_mut.connectable().shape().y() + rect_mut.connectable().shape().height() + node_node_spacing
    }
}
