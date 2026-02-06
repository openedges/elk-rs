use std::cmp::Ordering;

use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkRectangle;

use crate::org::eclipse::elk::alg::common::compaction::{Scanline, ScanlineEventHandler};
use crate::org::eclipse::elk::alg::common::point::{Point, Quadrant};

pub struct RectilinearConvexHull {
    hull: Vec<Point>,
    x_min1: Option<Point>,
    x_min2: Option<Point>,
    x_max1: Option<Point>,
    x_max2: Option<Point>,
    y_min1: Option<Point>,
    y_min2: Option<Point>,
    y_max1: Option<Point>,
    y_max2: Option<Point>,
}

impl RectilinearConvexHull {
    pub fn of(points: impl IntoIterator<Item = Point>) -> Self {
        let points: Vec<Point> = points.into_iter().collect();
        let mut hull = RectilinearConvexHull {
            hull: Vec::new(),
            x_min1: None,
            x_min2: None,
            x_max1: None,
            x_max2: None,
            y_min1: None,
            y_min2: None,
            y_max1: None,
            y_max2: None,
        };

        for p in &points {
            if hull.x_max1.is_none_or(|v| p.x >= v.x) {
                hull.x_max2 = hull.x_max1;
                hull.x_max1 = Some(*p);
            }
            if hull.x_min1.is_none_or(|v| p.x <= v.x) {
                hull.x_min2 = hull.x_min1;
                hull.x_min1 = Some(*p);
            }
            if hull.y_max1.is_none_or(|v| p.y >= v.y) {
                hull.y_max2 = hull.y_max1;
                hull.y_max1 = Some(*p);
            }
            if hull.y_min1.is_none_or(|v| p.y <= v.y) {
                hull.y_min2 = hull.y_min1;
                hull.y_min1 = Some(*p);
            }
        }

        let mut q1 = MaximalElementsEventHandler::new(Quadrant::Q1);
        Scanline::execute(points.clone(), right_low_first, &mut q1);

        let mut q4 = MaximalElementsEventHandler::new(Quadrant::Q4);
        Scanline::execute(points.clone(), right_high_first, &mut q4);

        let mut q2 = MaximalElementsEventHandler::new(Quadrant::Q2);
        Scanline::execute(points.clone(), left_low_first, &mut q2);

        let mut q3 = MaximalElementsEventHandler::new(Quadrant::Q3);
        Scanline::execute(points, left_high_first, &mut q3);

        add_concave_corners(&mut q1.points, Quadrant::Q1);
        add_concave_corners(&mut q2.points, Quadrant::Q2);
        add_concave_corners(&mut q3.points, Quadrant::Q3);
        add_concave_corners(&mut q4.points, Quadrant::Q4);

        hull.hull.clear();
        hull.hull.extend(q1.points);
        hull.hull.extend(q2.points.into_iter().rev());
        hull.hull.extend(q3.points);
        hull.hull.extend(q4.points.into_iter().rev());

        hull
    }

    pub fn get_hull(&self) -> &Vec<Point> {
        &self.hull
    }

    pub fn split_into_rectangles(&self) -> Vec<ElkRectangle> {
        let mut handler = RectangleEventHandler::new(self.x_min1, self.x_min2);
        Scanline::execute(self.hull.clone(), right_special_order, &mut handler);

        if let Some(queued) = handler.queued.take() {
            handler.rects.push(queued);
        }

        handler.rects
    }
}

fn add_concave_corners(points: &mut Vec<Point>, quadrant: Quadrant) {
    if points.is_empty() {
        return;
    }
    let mut index = 1usize;
    let mut last = points[0];
    while index < points.len() {
        let next = points[index];
        let mut p = Point::with_quadrant(next.x, last.y, quadrant);
        p.convex = false;
        points.insert(index, p);
        index += 2;
        last = next;
    }
}

fn right_high_first(p1: &Point, p2: &Point) -> Ordering {
    if p1.x == p2.x {
        p2.y.partial_cmp(&p1.y).unwrap_or(Ordering::Equal)
    } else {
        p1.x.partial_cmp(&p2.x).unwrap_or(Ordering::Equal)
    }
}

fn right_low_first(p1: &Point, p2: &Point) -> Ordering {
    if p1.x == p2.x {
        p1.y.partial_cmp(&p2.y).unwrap_or(Ordering::Equal)
    } else {
        p1.x.partial_cmp(&p2.x).unwrap_or(Ordering::Equal)
    }
}

fn left_high_first(p1: &Point, p2: &Point) -> Ordering {
    if p1.x == p2.x {
        p2.y.partial_cmp(&p1.y).unwrap_or(Ordering::Equal)
    } else {
        p2.x.partial_cmp(&p1.x).unwrap_or(Ordering::Equal)
    }
}

fn left_low_first(p1: &Point, p2: &Point) -> Ordering {
    if p1.x == p2.x {
        p1.y.partial_cmp(&p2.y).unwrap_or(Ordering::Equal)
    } else {
        p2.x.partial_cmp(&p1.x).unwrap_or(Ordering::Equal)
    }
}

fn right_special_order(p1: &Point, p2: &Point) -> Ordering {
    if p1.x == p2.x {
        if p1.quadrant == p2.quadrant
            || Quadrant::is_both_left_or_both_right(p1.quadrant, p2.quadrant)
        {
            let val = if p1.quadrant.is_left() { Ordering::Greater } else { Ordering::Less };
            if p1.convex && !p2.convex {
                return val;
            } else if !p1.convex && p2.convex {
                return val.reverse();
            }
        }
        return p1.quadrant.cmp(&p2.quadrant);
    }
    p1.x.partial_cmp(&p2.x).unwrap_or(Ordering::Equal)
}

struct MaximalElementsEventHandler {
    quadrant: Quadrant,
    pub points: Vec<Point>,
    maximal_y: f64,
    reverse: bool,
}

impl MaximalElementsEventHandler {
    fn new(quadrant: Quadrant) -> Self {
        match quadrant {
            Quadrant::Q1 | Quadrant::Q2 => MaximalElementsEventHandler {
                quadrant,
                points: Vec::new(),
                maximal_y: f64::INFINITY,
                reverse: true,
            },
            Quadrant::Q3 | Quadrant::Q4 => MaximalElementsEventHandler {
                quadrant,
                points: Vec::new(),
                maximal_y: f64::NEG_INFINITY,
                reverse: false,
            },
        }
    }
}

impl ScanlineEventHandler<Point> for MaximalElementsEventHandler {
    fn handle(&mut self, point: &Point) {
        let mut cmp = point
            .y
            .partial_cmp(&self.maximal_y)
            .unwrap_or(Ordering::Equal);
        if self.reverse {
            cmp = cmp.reverse();
        }
        if cmp == Ordering::Greater {
            self.points.push(Point::with_quadrant(point.x, point.y, self.quadrant));
            self.maximal_y = point.y;
        }
    }
}

struct RectangleEventHandler {
    rects: Vec<ElkRectangle>,
    min_y: Option<Point>,
    max_y: Option<Point>,
    last_x: f64,
    queued: Option<ElkRectangle>,
    queued_point: Option<Point>,
}

impl RectangleEventHandler {
    fn new(x_min1: Option<Point>, x_min2: Option<Point>) -> Self {
        let mut last_x = 0.0;
        if let Some(p1) = x_min1 {
            last_x = p1.x;
        }
        if let Some(p2) = x_min2 {
            last_x = last_x.min(p2.x);
        }
        RectangleEventHandler {
            rects: Vec::new(),
            min_y: None,
            max_y: None,
            last_x,
            queued: None,
            queued_point: None,
        }
    }
}

impl ScanlineEventHandler<Point> for RectangleEventHandler {
    fn handle(&mut self, point: &Point) {
        if let (Some(queued), Some(queued_point)) = (&self.queued, &self.queued_point) {
            if point.x != queued_point.x
                || Quadrant::is_one_left_one_right(queued_point.quadrant, point.quadrant)
            {
                self.rects.push(*queued);
                self.last_x = queued.x + queued.width;
                self.queued = None;
                self.queued_point = None;
            }
        }

        if point.quadrant.is_upper() {
            self.min_y = Some(*point);
        } else {
            self.max_y = Some(*point);
        }

        let queue_rect = if point.convex {
            matches!(point.quadrant, Quadrant::Q2 | Quadrant::Q3)
        } else {
            matches!(point.quadrant, Quadrant::Q1 | Quadrant::Q4)
        };

        if queue_rect {
            if let (Some(min_y), Some(max_y)) = (self.min_y, self.max_y) {
                let rect = ElkRectangle::with_values(
                    self.last_x,
                    min_y.y,
                    point.x - self.last_x,
                    max_y.y - min_y.y,
                );
                self.queued = Some(rect);
                self.queued_point = Some(*point);
            }
        }
    }
}
