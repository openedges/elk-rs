use crate::org::eclipse::elk::core::math::elk_math::ElkMath;
use crate::org::eclipse::elk::core::math::kvector::KVector;

#[derive(Clone, Debug)]
pub struct BezierSpline {
    curves: Vec<BezierCurve>,
}

impl BezierSpline {
    pub fn new() -> Self {
        BezierSpline { curves: Vec::new() }
    }

    pub fn add_curve(&mut self, curve: BezierCurve) {
        self.curves.push(curve);
    }

    pub fn add_spline(&mut self, spline: &BezierSpline, beginning: bool) {
        if beginning {
            let mut new_curves = spline.curves.clone();
            new_curves.extend(self.curves.clone());
            self.curves = new_curves;
        } else {
            self.curves.extend(spline.curves.clone());
        }
    }

    pub fn add_curve_points(&mut self, start: KVector, first: KVector, second: KVector, end: KVector) {
        self.curves.push(BezierCurve::new(start, first, second, end));
    }

    pub fn get_start_point(&self) -> KVector {
        self.curves.first().expect("no curves").start
    }

    pub fn get_end_point(&self) -> KVector {
        self.curves.last().expect("no curves").end
    }

    pub fn get_inner_points(&self) -> Vec<KVector> {
        if self.curves.is_empty() {
            return Vec::new();
        }
        let mut points = Vec::with_capacity(self.curves.len() * 3 - 1);
        for (idx, curve) in self.curves.iter().enumerate() {
            points.push(curve.first_control);
            points.push(curve.second_control);
            if idx + 1 < self.curves.len() {
                points.push(curve.end);
            }
        }
        points
    }

    pub fn get_base_points(&self) -> Vec<KVector> {
        if self.curves.is_empty() {
            return Vec::new();
        }
        let mut points = Vec::with_capacity(self.curves.len() + 1);
        points.push(self.curves.first().unwrap().start);
        for curve in &self.curves {
            points.push(curve.end);
        }
        points
    }

    pub fn get_polyline_apprx(&self, accuracy: usize) -> Vec<KVector> {
        if self.curves.is_empty() {
            return Vec::new();
        }
        let mut apprx = Vec::with_capacity(self.curves.len() * accuracy + 1);
        apprx.push(self.curves.first().unwrap().start);
        for curve in &self.curves {
            let segment = ElkMath::approximate_bezier_segment(accuracy, &curve.as_array());
            for point in segment {
                apprx.push(point);
            }
        }
        apprx
    }

    pub fn get_curves(&self) -> &Vec<BezierCurve> {
        &self.curves
    }
}

impl Default for BezierSpline {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BezierSpline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for curve in &self.curves {
            writeln!(
                f,
                "{} -> {} -> {} -> {}",
                curve.start, curve.first_control, curve.second_control, curve.end
            )?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct BezierCurve {
    pub start: KVector,
    pub first_control: KVector,
    pub second_control: KVector,
    pub end: KVector,
}

impl BezierCurve {
    pub fn new(start: KVector, first_control: KVector, second_control: KVector, end: KVector) -> Self {
        BezierCurve {
            start,
            first_control,
            second_control,
            end,
        }
    }

    pub fn as_vector_list(&self) -> Vec<KVector> {
        vec![self.start, self.first_control, self.second_control, self.end]
    }

    pub fn as_array(&self) -> [KVector; 4] {
        [self.start, self.first_control, self.second_control, self.end]
    }
}

