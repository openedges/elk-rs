use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;

use crate::org::eclipse::elk::alg::layered::p5edges::splines::rectangle::Rectangle;
use crate::org::eclipse::elk::alg::layered::p5edges::splines::splines_math::SplinesMath;

#[derive(Clone, Debug)]
pub struct NubSpline {
    knot_vector: Vec<f64>,
    control_points: Vec<PolarCP>,
    dim_nubs: usize,
    is_uniform: bool,
    is_clamped: bool,
    outer_box: Option<Rectangle>,
    is_bezier: bool,
    min_knot: f64,
    max_knot: f64,
}

impl NubSpline {
    pub const DIM: usize = 3;
    const EPSILON: f64 = 0.000001;

    pub fn new(clamped: bool, dimension: usize, k_vectors: KVectorChain) -> Self {
        Self::new_from_vec(clamped, dimension, k_vectors.to_array())
    }

    pub fn new_from_vec(clamped: bool, dimension: usize, mut k_vectors: Vec<KVector>) -> Self {
        if dimension < 1 {
            panic!("The dimension must be at least 1!");
        }
        if k_vectors.is_empty() {
            panic!("At least one control point is required");
        }
        let mut i = k_vectors.len().saturating_sub(1);
        while i < dimension {
            let first = k_vectors[0];
            k_vectors.insert(0, first);
            i += 1;
        }

        if k_vectors.len() < (dimension + 1) {
            panic!("At (least dimension + 1) control points are necessary!");
        }

        let mut spline = NubSpline {
            knot_vector: Vec::new(),
            control_points: Vec::new(),
            dim_nubs: dimension,
            is_uniform: true,
            is_clamped: clamped,
            outer_box: None,
            is_bezier: false,
            min_knot: 0.0,
            max_knot: 0.0,
        };

        spline.create_uniform_knot_vector(clamped, k_vectors.len() + spline.dim_nubs - 1);

        let mut polar_coordinate: Vec<f64> = Vec::new();
        let mut knot_iter = spline.knot_vector.iter();
        for _ in 0..(spline.dim_nubs - 1) {
            if let Some(knot) = knot_iter.next() {
                polar_coordinate.push(*knot);
            }
        }

        for k_vector in k_vectors {
            if let Some(knot) = knot_iter.next() {
                polar_coordinate.push(*knot);
            }
            spline
                .control_points
                .push(PolarCP::new(&k_vector, &polar_coordinate));
            if !polar_coordinate.is_empty() {
                polar_coordinate.remove(0);
            }
        }

        spline
    }

    fn new_with_params(
        clamped: bool,
        uniform: bool,
        bezier: bool,
        dim: usize,
        knot_vec: Vec<f64>,
        polar_vectors: Vec<PolarCP>,
    ) -> Self {
        let min_knot = knot_vec.first().copied().unwrap_or(0.0);
        let max_knot = knot_vec.last().copied().unwrap_or(0.0);
        NubSpline {
            is_clamped: clamped,
            is_uniform: uniform,
            is_bezier: bezier,
            dim_nubs: dim,
            knot_vector: knot_vec,
            control_points: polar_vectors,
            min_knot,
            max_knot,
            outer_box: None,
        }
    }

    pub fn generate_derived_nubs(nub_spline: &NubSpline) -> NubSpline {
        let new_clamped = nub_spline.is_clamped;
        let new_uniform = nub_spline.is_uniform;
        let new_bezier = nub_spline.is_bezier;
        let old_dim = nub_spline.dim_nubs;
        let new_dim = old_dim - 1;
        let old_knot_vector = &nub_spline.knot_vector;
        let new_knot_vector = nub_spline.knot_vector[1..nub_spline.knot_vector.len() - 1].to_vec();
        let mut new_control_points: Vec<KVector> = Vec::new();

        for i in 0..(nub_spline.control_points.len() - 1) {
            let mut new_cp = nub_spline.control_points[i + 1].cp;
            let prev = nub_spline.control_points[i].cp;
            new_cp.sub(&prev);
            let denom = old_knot_vector[i + old_dim] - old_knot_vector[i];
            new_cp.scale(old_dim as f64 / denom);
            new_control_points.push(new_cp);
        }

        let mut polar_coordinate: Vec<f64> = Vec::new();
        let mut knot_iter = new_knot_vector.iter();
        let mut new_polar_vectors: Vec<PolarCP> = Vec::new();

        for _ in 0..(new_dim - 1) {
            if let Some(knot) = knot_iter.next() {
                polar_coordinate.push(*knot);
            }
        }

        for k_vector in new_control_points {
            if let Some(knot) = knot_iter.next() {
                polar_coordinate.push(*knot);
            }
            new_polar_vectors.push(PolarCP::new(&k_vector, &polar_coordinate));
            if !polar_coordinate.is_empty() {
                polar_coordinate.remove(0);
            }
        }

        NubSpline::new_with_params(
            new_clamped,
            new_uniform,
            new_bezier,
            new_dim,
            new_knot_vector,
            new_polar_vectors,
        )
    }

    pub fn generate_inverted_nubs(nub_spline: &NubSpline) -> NubSpline {
        let max_vector = *nub_spline.knot_vector.last().unwrap_or(&0.0);
        let mut new_knot_vector: Vec<f64> = Vec::with_capacity(nub_spline.knot_vector.len());
        for vector in nub_spline.knot_vector.iter().rev() {
            new_knot_vector.push(max_vector - vector);
        }

        let reversed_chain = KVectorChain::reverse(&nub_spline.get_control_points());
        let new_control_points = reversed_chain.to_array();

        let mut polar_coordinate: Vec<f64> = Vec::new();
        let mut knot_iter = new_knot_vector.iter();
        let mut new_polar_vectors: Vec<PolarCP> = Vec::new();

        for _ in 0..(nub_spline.dim_nubs - 1) {
            if let Some(knot) = knot_iter.next() {
                polar_coordinate.push(*knot);
            }
        }

        for k_vector in new_control_points {
            if let Some(knot) = knot_iter.next() {
                polar_coordinate.push(*knot);
            }
            new_polar_vectors.push(PolarCP::new(&k_vector, &polar_coordinate));
            if !polar_coordinate.is_empty() {
                polar_coordinate.remove(0);
            }
        }

        NubSpline::new_with_params(
            nub_spline.is_clamped,
            nub_spline.is_uniform,
            nub_spline.is_bezier,
            nub_spline.dim_nubs,
            new_knot_vector,
            new_polar_vectors,
        )
    }

    pub fn get_dim(&self) -> usize {
        self.dim_nubs
    }

    pub fn get_outer_box(&mut self) -> Rectangle {
        if self.outer_box.is_none() {
            self.calculate_outer_box();
        }
        self.outer_box.unwrap()
    }

    pub fn set_outer_box(&mut self, outer_rectangle: Rectangle) {
        self.outer_box = Some(outer_rectangle);
    }

    pub fn get_control_points(&self) -> KVectorChain {
        let mut ret_val = KVectorChain::new();
        for polar_cp in &self.control_points {
            ret_val.add_vector(polar_cp.cp);
        }
        ret_val
    }

    pub fn get_control_point(&self, i: usize) -> KVector {
        self.control_points[i].cp
    }

    pub fn get_control_points_size(&self) -> usize {
        self.control_points.len()
    }

    pub fn get_knot_vector(&self) -> Vec<f64> {
        self.knot_vector.clone()
    }

    fn get_index_in_knot_vector(&self, knot: f64) -> usize {
        for (index, value) in self.knot_vector.iter().enumerate() {
            if (*value - knot).abs() < Self::EPSILON {
                return index;
            }
        }
        self.knot_vector.len()
    }

    fn get_multiplicity(&self, knot_to_check: f64) -> usize {
        let mut count = 0;
        for current_knot in &self.knot_vector {
            let diff = current_knot - knot_to_check;
            if diff > Self::EPSILON {
                return count;
            } else if diff > -Self::EPSILON {
                count += 1;
            }
        }
        count
    }

    fn create_uniform_knot_vector(&mut self, clamped: bool, size: usize) {
        if size < (2 * self.dim_nubs) {
            panic!("The knot vector must have at least two time the dimension elements.");
        }
        let my_size: usize;

        if clamped {
            self.min_knot = 0.0;
            self.max_knot = 1.0;
            for _ in 0..self.dim_nubs {
                self.knot_vector.push(0.0);
            }
            my_size = size + 1 - 2 * self.dim_nubs;
        } else {
            my_size = size + 1;
            let ddim = self.dim_nubs as f64;
            self.min_knot = ddim / (my_size as f64 + 1.0);
            self.max_knot = (my_size as f64 - ddim) / my_size as f64;
        }

        let fraction = my_size as f64;
        for i in 1..my_size {
            self.knot_vector.push(i as f64 / fraction);
        }

        if self.is_clamped {
            for _ in 0..self.dim_nubs {
                self.knot_vector.push(1.0);
            }
        }
    }

    fn calculate_outer_box(&mut self) {
        self.outer_box = Some(Rectangle::from_iter(self.get_control_points().iter()));
    }

    fn get_t_from_polar(polar: &[f64]) -> f64 {
        let sum: f64 = polar.iter().copied().sum();
        sum / polar.len() as f64
    }

    fn insert_knot(&mut self, knot_to_insert: f64, insertions: usize) {
        let mut knot_index = if self.is_clamped {
            self.dim_nubs
        } else {
            self.dim_nubs - 1
        };

        let mut cp_index = 0usize;
        let mut current_knot = self.knot_vector[knot_index];

        while current_knot - knot_to_insert < Self::EPSILON {
            knot_index += 1;
            cp_index += 1;
            current_knot = self.knot_vector[knot_index];
        }
        if knot_index > 0 {
            knot_index -= 1;
        }

        self.insert_knot_at_current_position(insertions, knot_to_insert, cp_index, knot_index);
    }

    fn insert_knot_at_current_position(
        &mut self,
        insertions: usize,
        knot_to_insert: f64,
        mut cp_index: usize,
        mut knot_index: usize,
    ) {
        let multiplicity = self.get_multiplicity(knot_to_insert);
        for i in 0..insertions {
            self.knot_vector.insert(knot_index, knot_to_insert);
            knot_index += 1;

            let mut new_cps: Vec<PolarCP> = Vec::new();
            let mut second_cp = self.control_points[cp_index].clone();
            cp_index += 1;

            for _ in (multiplicity + i)..self.dim_nubs {
                let first_cp = second_cp;
                second_cp = self.control_points[cp_index].clone();
                cp_index += 1;
                new_cps.push(PolarCP::from_two(&first_cp, &second_cp, knot_to_insert));
            }

            for j in (multiplicity + i)..self.dim_nubs {
                if cp_index == 0 {
                    break;
                }
                cp_index -= 1;
                if j > multiplicity + i {
                    self.control_points.remove(cp_index);
                }
            }

            for cp in new_cps {
                self.control_points.insert(cp_index, cp);
                cp_index += 1;
            }

            if i < insertions - 1 {
                for _ in (multiplicity + i)..self.dim_nubs {
                    if cp_index == 0 {
                        break;
                    }
                    cp_index -= 1;
                }
            }
        }
    }

    pub fn get_first_vertical_point(nub_spline: &NubSpline, accuracy: f64, max_recursion: usize) -> KVector {
        let mut first_derive = NubSpline::generate_derived_nubs(nub_spline);
        let mut current_accuracy = f64::INFINITY;
        let mut current_vector: Option<KVector> = None;
        let mut loop_count = 0usize;
        let mut knot = 0.0;

        while current_accuracy > accuracy && loop_count < max_recursion {
            knot = NubSpline::get_zero_x_of_control_polygon(&first_derive);
            let vector = first_derive.get_point_on_curve(knot, true);
            current_accuracy = vector.x.abs();
            current_vector = Some(vector);
            loop_count += 1;
        }
        let _ = current_vector;
        let mut copy = nub_spline.clone();
        copy.get_point_on_curve(knot, false)
    }

    pub fn get_last_vertical_point(nub_spline: &NubSpline, accuracy: f64, max_recursion: usize) -> KVector {
        let mut first_derive = NubSpline::generate_inverted_nubs(&NubSpline::generate_derived_nubs(nub_spline));
        let mut current_accuracy = f64::INFINITY;
        let mut current_vector: Option<KVector> = None;
        let mut loop_count = 0usize;
        let mut knot = 0.0;

        while current_accuracy > accuracy && loop_count < max_recursion {
            knot = NubSpline::get_zero_x_of_control_polygon(&first_derive);
            let vector = first_derive.get_point_on_curve(knot, true);
            current_accuracy = vector.x.abs();
            current_vector = Some(vector);
            loop_count += 1;
        }
        let _ = current_vector;
        let max_val = *nub_spline.knot_vector.last().unwrap_or(&0.0);
        let mut copy = nub_spline.clone();
        copy.get_point_on_curve(max_val - knot, false)
    }

    pub fn get_first_horizontal_point(nub_spline: &NubSpline, accuracy: f64, max_recursion: usize) -> KVector {
        let mut first_derive = NubSpline::generate_derived_nubs(nub_spline);
        let mut current_accuracy = f64::INFINITY;
        let mut current_vector: Option<KVector> = None;
        let mut loop_count = 0usize;
        let mut knot = 0.0;

        while current_accuracy > accuracy && loop_count < max_recursion {
            knot = NubSpline::get_zero_y_of_control_polygon(&first_derive);
            let vector = first_derive.get_point_on_curve(knot, true);
            current_accuracy = vector.y.abs();
            current_vector = Some(vector);
            loop_count += 1;
        }
        let _ = current_vector;
        let mut copy = nub_spline.clone();
        copy.get_point_on_curve(knot, false)
    }

    pub fn get_last_horizontal_point(nub_spline: &NubSpline, accuracy: f64, max_recursion: usize) -> KVector {
        let mut first_derive = NubSpline::generate_inverted_nubs(&NubSpline::generate_derived_nubs(nub_spline));
        let mut current_accuracy = f64::INFINITY;
        let mut current_vector: Option<KVector> = None;
        let mut loop_count = 0usize;
        let mut knot = 0.0;

        while current_accuracy > accuracy && loop_count < max_recursion {
            knot = NubSpline::get_zero_y_of_control_polygon(&first_derive);
            let vector = first_derive.get_point_on_curve(knot, true);
            current_accuracy = vector.y.abs();
            current_vector = Some(vector);
            loop_count += 1;
        }
        let _ = current_vector;
        let max_val = *nub_spline.knot_vector.last().unwrap_or(&0.0);
        let mut copy = nub_spline.clone();
        copy.get_point_on_curve(max_val - knot, false)
    }

    pub fn get_point_on_curve(&mut self, t: f64, modify: bool) -> KVector {
        let multiplicity = self.get_multiplicity(t);
        if multiplicity == self.dim_nubs {
            return self.get_control_point(self.get_index_in_knot_vector(t));
        }
        if modify {
            self.insert_knot(t, self.dim_nubs - multiplicity);
            return self.get_control_point(self.get_index_in_knot_vector(t));
        }
        let mut copy = self.clone();
        copy.insert_knot(t, self.dim_nubs - multiplicity);
        copy.get_control_point(copy.get_index_in_knot_vector(t))
    }

    fn get_zero_y_of_control_polygon(nub_spline: &NubSpline) -> f64 {
        let mut iter = nub_spline.control_points.iter();
        let mut second_cp = iter.next().unwrap();
        let mut second_y = second_cp.cp.y;
        let mut second_is_positive = second_y > Self::EPSILON;
        let mut second_is_negative = second_y < -Self::EPSILON;

        while let Some(next_cp) = iter.next() {
            let first_cp = second_cp;
            let first_y = second_y;
            let first_is_positive = second_is_positive;
            let first_is_negative = second_is_negative;

            second_cp = next_cp;
            second_y = second_cp.cp.y;
            second_is_positive = second_y > Self::EPSILON;
            second_is_negative = second_y < -Self::EPSILON;

            if !(second_is_positive || second_is_negative) {
                return NubSpline::get_t_from_polar(&second_cp.polar_coordinate);
            }

            if (first_is_positive && second_is_negative) || (first_is_negative && second_is_positive) {
                let factor = first_y / (first_y - second_y);
                let t1 = NubSpline::get_t_from_polar(&first_cp.polar_coordinate);
                let t2 = NubSpline::get_t_from_polar(&second_cp.polar_coordinate);
                return factor * t1 + (1.0 - factor) * t2;
            }
        }
        0.0
    }

    fn get_zero_x_of_control_polygon(nub_spline: &NubSpline) -> f64 {
        let mut iter = nub_spline.control_points.iter();
        let mut second_cp = iter.next().unwrap();
        let mut second_x = second_cp.cp.x;
        let mut second_is_positive = second_x > Self::EPSILON;
        let mut second_is_negative = second_x < -Self::EPSILON;

        while let Some(next_cp) = iter.next() {
            let first_cp = second_cp;
            let first_x = second_x;
            let first_is_positive = second_is_positive;
            let first_is_negative = second_is_negative;

            second_cp = next_cp;
            second_x = second_cp.cp.x;
            second_is_positive = second_x > Self::EPSILON;
            second_is_negative = second_x < -Self::EPSILON;

            if !(second_is_positive || second_is_negative) {
                return NubSpline::get_t_from_polar(&second_cp.polar_coordinate);
            }

            if (first_is_positive && second_is_negative) || (first_is_negative && second_is_positive) {
                let factor = first_x / (first_x - second_x);
                let t1 = NubSpline::get_t_from_polar(&first_cp.polar_coordinate);
                let t2 = NubSpline::get_t_from_polar(&second_cp.polar_coordinate);
                return factor * t1 + (1.0 - factor) * t2;
            }
        }
        0.0
    }

    pub fn to_bezier(&mut self) {
        let mut knot_index = if self.is_clamped {
            self.dim_nubs
        } else {
            for _ in 0..(self.dim_nubs - 1) {
                if !self.knot_vector.is_empty() {
                    self.knot_vector.remove(0);
                }
            }
            self.dim_nubs - 1
        };

        let mut cp_index = 0usize;
        let mut current_knot = self.knot_vector[knot_index];
        while self.max_knot - current_knot > Self::EPSILON {
            let knot_to_count = current_knot;
            let mut occurrence = 0usize;
            while (current_knot - knot_to_count).abs() < Self::EPSILON {
                occurrence += 1;
                knot_index += 1;
                cp_index += 1;
                current_knot = self.knot_vector[knot_index];
            }

            if occurrence < self.dim_nubs {
                if knot_index > 0 {
                    knot_index -= 1;
                }
                self.insert_knot_at_current_position(
                    self.dim_nubs - occurrence,
                    knot_to_count,
                    cp_index,
                    knot_index,
                );
                knot_index += 1;
            }

            if cp_index > 0 {
                cp_index -= 1;
            }
        }

        if !self.is_clamped {
            for _ in 0..(self.dim_nubs - 1) {
                if knot_index < self.knot_vector.len() {
                    self.knot_vector.remove(knot_index);
                }
            }
        }
        self.is_clamped = true;
        self.is_bezier = true;
    }

    pub fn get_bezier_cp(&mut self, with_source_vector: bool, with_target_vector: bool) -> KVectorChain {
        if !self.is_bezier {
            self.to_bezier();
        }
        let mut ret_val = KVectorChain::new();
        let mut iter = self.control_points.iter();

        if !with_source_vector {
            let _ = iter.next();
        }

        for cp in iter.by_ref() {
            ret_val.add_vector(cp.cp);
        }

        if !with_target_vector {
            ret_val.remove_last();
        }
        ret_val
    }

    pub fn get_bezier_cp_default(&mut self) -> KVectorChain {
        self.get_bezier_cp(false, false)
    }

    pub fn generate_nice_curve() -> NubSpline {
        let mut nice_chain = KVectorChain::new();
        let vector1 = KVector::with_values(5.5, 23.0);
        let vector2 = KVector::with_values(2.5, 12.0);
        let vector3 = KVector::with_values(5.0, 10.0);
        let vector4 = KVector::with_values(5.0, 9.0);
        let vector5 = KVector::with_values(3.8, 5.5);
        let vector6 = KVector::with_values(7.0, 4.0);
        let vector7 = KVector::with_values(7.0, 3.5);
        let vector8 = KVector::with_values(6.0, 2.2);
        let vector9 = KVector::with_values(8.0, 0.5);

        nice_chain.add_vector(vector1);
        nice_chain.add_vector(vector1);
        nice_chain.add_vector(vector1);
        nice_chain.add_vector(vector2);
        nice_chain.add_vector(vector3);
        nice_chain.add_vector(vector4);
        nice_chain.add_vector(vector5);
        nice_chain.add_vector(vector6);
        nice_chain.add_vector(vector7);
        nice_chain.add_vector(vector8);
        nice_chain.add_vector(vector9);
        nice_chain.add_vector(mirror_on_x(&vector8, vector9.x));
        nice_chain.add_vector(mirror_on_x(&vector7, vector9.x));
        nice_chain.add_vector(mirror_on_x(&vector6, vector9.x));
        nice_chain.add_vector(mirror_on_x(&vector5, vector9.x));
        nice_chain.add_vector(mirror_on_x(&vector4, vector9.x));
        nice_chain.add_vector(mirror_on_x(&vector3, vector9.x));
        nice_chain.add_vector(mirror_on_x(&vector2, vector9.x));
        nice_chain.add_vector(mirror_on_x(&vector1, vector9.x));
        nice_chain.add_vector(mirror_on_x(&vector1, vector9.x));
        nice_chain.add_vector(mirror_on_x(&vector1, vector9.x));

        for vector in nice_chain.iter_mut() {
            vector.scale(5.0);
        }
        nice_chain.offset(-50.0, -80.0);

        NubSpline::new(true, 3, nice_chain)
    }
}

fn mirror_on_x(original: &KVector, x_val: f64) -> KVector {
    KVector::with_values(x_val + x_val - original.x, original.y)
}

#[derive(Clone, Debug, PartialEq)]
struct PolarCP {
    cp: KVector,
    polar_coordinate: Vec<f64>,
}

impl PolarCP {
    fn new(control_point: &KVector, polar_coordinate: &[f64]) -> PolarCP {
        PolarCP {
            cp: *control_point,
            polar_coordinate: polar_coordinate.to_vec(),
        }
    }

    fn from_two(first_cp: &PolarCP, second_cp: &PolarCP, new_knot: f64) -> PolarCP {
        let first_factor = first_cp.polar_coordinate.first().copied().unwrap_or(0.0);
        let second_factor = second_cp.polar_coordinate.last().copied().unwrap_or(0.0);

        let mut a_scaled = first_cp.cp;
        a_scaled.scale(second_factor - new_knot);
        let mut b_scaled = second_cp.cp;
        b_scaled.scale(new_knot - first_factor);
        let mut total = a_scaled;
        total.add(&b_scaled);
        total.scale(1.0 / (second_factor - first_factor));

        let mut polar_coordinate: Vec<f64> = Vec::new();
        let mut needs_to_be_added = true;
        let mut iter = first_cp.polar_coordinate.iter();
        let _ = iter.next();
        for next_knot in iter {
            if needs_to_be_added && (*next_knot - new_knot) > NubSpline::EPSILON {
                polar_coordinate.push(new_knot);
                needs_to_be_added = false;
            }
            polar_coordinate.push(*next_knot);
        }
        if needs_to_be_added {
            polar_coordinate.push(new_knot);
        }

        PolarCP {
            cp: total,
            polar_coordinate,
        }
    }
}

impl std::fmt::Display for PolarCP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} {}",
            self.polar_coordinate,
            SplinesMath::convert_kvector_to_string(Some(&self.cp))
        )
    }
}

impl std::fmt::Display for NubSpline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.control_points)
    }
}
