use crate::org::eclipse::elk::core::math::elk_rectangle::ElkRectangle;
use crate::org::eclipse::elk::core::math::kvector::KVector;
use crate::org::eclipse::elk::core::math::kvector_chain::KVectorChain;

pub struct ElkMath;

pub trait IntersectsArgs {
    fn intersects(self) -> bool;
}

pub trait ContainsArgs {
    fn contains(self) -> bool;
}

const FACT_TABLE: [i64; 21] = [
    1,
    1,
    2,
    6,
    24,
    120,
    720,
    5040,
    40320,
    362880,
    3628800,
    39916800,
    479001600,
    6227020800,
    87178291200,
    1307674368000,
    20922789888000,
    355687428096000,
    6402373705728000,
    121645100408832000,
    2432902008176640000,
];

const W_DEGREE: usize = 5;
const DEGREE: usize = 3;
const MAXDEPTH: usize = 64;
const EPSILON: f64 = 1.0 / (1u128 << (MAXDEPTH + 1)) as f64;
const DOUBLE_EQ_EPSILON: f64 = 0.00001;

const ZERO_VECTOR: KVector = KVector { x: 0.0, y: 0.0 };

const CUBIC_Z: [[f64; 4]; 3] = [
    [1.0, 0.6, 0.3, 0.1],
    [0.4, 0.6, 0.6, 0.4],
    [0.1, 0.3, 0.6, 1.0],
];

impl ElkMath {
    pub fn factl(x: i32) -> i64 {
        if x < 0 || (x as usize) >= FACT_TABLE.len() {
            panic!("The input must be between 0 and {}", FACT_TABLE.len());
        }
        FACT_TABLE[x as usize]
    }

    pub fn factd(x: i32) -> f64 {
        if x < 0 {
            panic!("The input must be positive");
        }
        if (x as usize) < FACT_TABLE.len() {
            FACT_TABLE[x as usize] as f64
        } else {
            let xf = x as f64;
            let pow = Self::powf(x as f32, x) as f64;
            (2.0 * std::f64::consts::PI * xf).sqrt() * (pow / Self::powd(std::f64::consts::E, x))
        }
    }

    pub fn binomiall(n: i32, k: i32) -> i64 {
        if n < 0 || k < 0 {
            panic!("k and n must be positive");
        } else if k > n {
            panic!("k must be smaller than n");
        } else if k == 0 || k == n {
            1
        } else if n == 0 {
            0
        } else if (n as usize) < FACT_TABLE.len() {
            Self::factl(n) / (Self::factl(k) * Self::factl(n - k))
        } else {
            Self::binomiall(n - 1, k - 1) + Self::binomiall(n - 1, k)
        }
    }

    pub fn binomiald(n: i32, k: i32) -> f64 {
        if n < 0 || k < 0 {
            panic!("k and n must be positive");
        } else if k > n {
            panic!("k must be smaller than n");
        } else if k == 0 || k == n {
            1.0
        } else if n == 0 {
            0.0
        } else {
            Self::factd(n) / (Self::factd(k) * Self::factd(n - k))
        }
    }

    pub fn powd(a: f64, b: i32) -> f64 {
        let mut result = 1.0;
        let mut base = a;
        let mut exp = if b >= 0 { b } else { -b };
        while exp > 0 {
            if exp % 2 == 0 {
                base *= base;
                exp /= 2;
            } else {
                result *= base;
                exp -= 1;
            }
        }
        if b < 0 {
            1.0 / result
        } else {
            result
        }
    }

    pub fn powf(a: f32, b: i32) -> f32 {
        let mut result = 1.0f32;
        let mut base = a;
        let mut exp = if b >= 0 { b } else { -b };
        while exp > 0 {
            if exp % 2 == 0 {
                base *= base;
                exp /= 2;
            } else {
                result *= base;
                exp -= 1;
            }
        }
        if b < 0 {
            1.0f32 / result
        } else {
            result
        }
    }

    pub fn approximate_bezier_segment(result_size: usize, control_points: &[KVector]) -> Vec<KVector> {
        if result_size == 0 || control_points.is_empty() {
            return Vec::new();
        }
        let mut result = Vec::with_capacity(result_size);
        let dt = 1.0 / result_size as f64;
        let mut t = 0.0;
        for _ in 0..result_size {
            t += dt;
            result.push(Self::get_point_on_bezier_segment(t, control_points));
        }
        result
    }

    pub fn approximate_bezier_segment_default(control_points: &[KVector]) -> Vec<KVector> {
        let approximation_count = control_points.len() + 1;
        Self::approximate_bezier_segment(approximation_count, control_points)
    }

    pub fn get_point_on_bezier_segment(t: f64, control_points: &[KVector]) -> KVector {
        if control_points.is_empty() {
            return KVector::new();
        }
        let n = control_points.len() - 1;
        let mut px = 0.0;
        let mut py = 0.0;
        for (j, p) in control_points.iter().enumerate() {
            let p = *p;
            let factor = Self::binomiald(n as i32, j as i32)
                * Self::powd(1.0 - t, (n - j) as i32)
                * Self::powd(t, j as i32);
            px += p.x * factor;
            py += p.y * factor;
        }
        KVector::with_values(px, py)
    }

    pub fn approximate_bezier_spline(control_points: &KVectorChain) -> KVectorChain {
        let ctrl_pt_count = control_points.size();
        let mut spline = KVectorChain::new();
        if ctrl_pt_count == 0 {
            return spline;
        }
        let mut index = 1usize;
        let mut current_point = control_points.get(0);
        spline.add_vector(current_point);
        while index < ctrl_pt_count {
            let remaining_points = ctrl_pt_count - index;
            if remaining_points == 1 {
                spline.add_vector(control_points.get(index));
                break;
            } else if remaining_points == 2 {
                let control1 = control_points.get(index);
                let control2 = control_points.get(index + 1);
                let segment = Self::approximate_bezier_segment_default(&[current_point, control1, control2]);
                spline.add_all(&segment);
                break;
            } else {
                let control1 = control_points.get(index);
                let control2 = control_points.get(index + 1);
                let next_point = control_points.get(index + 2);
                let segment = Self::approximate_bezier_segment_default(&[
                    current_point,
                    control1,
                    control2,
                    next_point,
                ]);
                spline.add_all(&segment);
                current_point = next_point;
                index += 3;
            }
        }
        spline
    }

    pub fn distance_from_bezier_segment(
        start: KVector,
        c1: KVector,
        c2: KVector,
        end: KVector,
        needle: KVector,
    ) -> f64 {
        let mut t_candidate = [0.0; W_DEGREE + 1];
        let v = [start, c1, c2, end];

        let w = Self::convert_to_bezier_form(&v, &needle);
        let n_solutions = Self::find_roots(&w, W_DEGREE, &mut t_candidate, 0);

        let mut min_distance = needle.distance(&start);
        let mut t = 0.0;

        for &candidate in t_candidate[..n_solutions].iter() {
            let p = Self::bezier(&v, DEGREE, candidate, None, None);
            let distance = needle.distance(&p);
            if distance < min_distance {
                min_distance = distance;
                t = candidate;
            }
        }

        let distance = needle.distance(&end);
        if distance < min_distance {
            t = 1.0;
        }

        let pn = Self::bezier(&v, DEGREE, t, None, None);
        (pn.distance(&needle)).sqrt()
    }

    pub fn maxi(values: &[i32]) -> i32 {
        let mut max = i32::MIN;
        for value in values {
            if *value > max {
                max = *value;
            }
        }
        max
    }

    pub fn mini(values: &[i32]) -> i32 {
        let mut min = i32::MAX;
        for value in values {
            if *value < min {
                min = *value;
            }
        }
        min
    }

    pub fn averagei(values: &[i32]) -> i32 {
        let mut avg = 0i32;
        for value in values {
            avg += *value;
        }
        avg / values.len() as i32
    }

    pub fn maxl(values: &[i64]) -> i64 {
        let mut max = i64::from(i32::MIN);
        for value in values {
            if *value > max {
                max = *value;
            }
        }
        max
    }

    pub fn minl(values: &[i64]) -> i64 {
        let mut min = i64::from(i32::MAX);
        for value in values {
            if *value < min {
                min = *value;
            }
        }
        min
    }

    pub fn averagel(values: &[i64]) -> i64 {
        let mut avg = 0i64;
        for value in values {
            avg += *value;
        }
        avg / values.len() as i64
    }

    pub fn maxf(values: &[f32]) -> f32 {
        let mut max = -f32::MAX;
        for value in values {
            if *value > max {
                max = *value;
            }
        }
        max
    }

    pub fn minf(values: &[f32]) -> f32 {
        let mut min = f32::MAX;
        for value in values {
            if *value < min {
                min = *value;
            }
        }
        min
    }

    pub fn averagef(values: &[f32]) -> f32 {
        let mut avg = 0f32;
        for value in values {
            avg += *value;
        }
        avg / values.len() as f32
    }

    pub fn maxd(values: &[f64]) -> f64 {
        let mut max = -f64::MAX;
        for value in values {
            if *value > max {
                max = *value;
            }
        }
        max
    }

    pub fn mind(values: &[f64]) -> f64 {
        let mut min = f64::MAX;
        for value in values {
            if *value < min {
                min = *value;
            }
        }
        min
    }

    pub fn averaged(values: &[f64]) -> f64 {
        let mut avg = 0f64;
        for value in values {
            avg += *value;
        }
        avg / values.len() as f64
    }

    pub fn boundi(x: i32, lower: i32, upper: i32) -> i32 {
        if x <= lower {
            lower
        } else if x >= upper {
            upper
        } else {
            x
        }
    }

    pub fn boundl(x: i64, lower: i64, upper: i64) -> i64 {
        if x <= lower {
            lower
        } else if x >= upper {
            upper
        } else {
            x
        }
    }

    pub fn boundf(x: f32, lower: f32, upper: f32) -> f32 {
        if x <= lower {
            lower
        } else if x >= upper {
            upper
        } else {
            x
        }
    }

    pub fn boundd(x: f64, lower: f64, upper: f64) -> f64 {
        if x <= lower {
            lower
        } else if x >= upper {
            upper
        } else {
            x
        }
    }

    pub fn clip_vector(v: &mut KVector, width: f64, height: f64) -> &mut KVector {
        let wh = width / 2.0;
        let hh = height / 2.0;
        let absx = v.x.abs();
        let absy = v.y.abs();
        let mut xscale = 1.0;
        let mut yscale = 1.0;
        if absx > wh {
            xscale = wh / absx;
        }
        if absy > hh {
            yscale = hh / absy;
        }
        v.scale(xscale.min(yscale));
        v
    }

    pub fn signum(x: f64) -> i32 {
        if x < 0.0 {
            -1
        } else if x > 0.0 {
            1
        } else {
            0
        }
    }

    pub fn intersects<A: IntersectsArgs>(args: A) -> bool {
        args.intersects()
    }

    pub fn intersects2(p: &KVector, r: &KVector, q: &KVector, s: &KVector) -> Option<KVector> {
        let mut pq = *q;
        pq.sub(p);
        let pq_x_r = KVector::cross_product(&pq, r);
        let r_x_s = KVector::cross_product(r, s);
        let t = KVector::cross_product(&pq, s) / r_x_s;
        let u = pq_x_r / r_x_s;

        if r_x_s == 0.0 {
            if pq_x_r == 0.0 {
                let mut center = *q;
                let mut half_s = *s;
                half_s.scale(0.5);
                center.add(&half_s);
                let d1 = p.distance(&center);
                let mut p_plus_r = *p;
                p_plus_r.add(r);
                let d2 = p_plus_r.distance(&center);
                let l = s.length() * 0.5;
                if d1 < d2 && d1 <= l {
                    return Some(*p);
                }
                if d2 <= l {
                    return Some(p_plus_r);
                }
                return None;
            }
            None
        } else if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
            let mut point = *p;
            let mut scaled = *r;
            scaled.scale(t);
            point.add(&scaled);
            Some(point)
        } else {
            None
        }
    }

    pub fn distance(a1: &KVector, a2: &KVector, b1: &KVector, b2: &KVector, v: &KVector) -> f64 {
        Self::trace_rays(a1, a2, b1, b2, v)
            .min(Self::trace_rays(b1, b2, a1, a2, &{
                let mut neg = *v;
                neg.negate();
                neg
            }))
    }

    pub fn contains<A: ContainsArgs>(args: A) -> bool {
        args.contains()
    }

    pub fn shortest_distance(r1: &ElkRectangle, r2: &ElkRectangle) -> f64 {
        let right_dist = r2.x - (r1.x + r1.width);
        let left_dist = r1.x - (r2.x + r2.width);
        let top_dist = r1.y - (r2.y + r2.height);
        let bottom_dist = r2.y - (r1.y + r1.height);
        let horz_dist = left_dist.max(right_dist);
        let vert_dist = top_dist.max(bottom_dist);
        if (Self::fuzzy_compare(horz_dist, 0.0, DOUBLE_EQ_EPSILON) >= 0)
            ^ (Self::fuzzy_compare(vert_dist, 0.0, DOUBLE_EQ_EPSILON) >= 0)
        {
            return vert_dist.max(horz_dist);
        }
        if Self::fuzzy_compare(horz_dist, 0.0, DOUBLE_EQ_EPSILON) > 0 {
            return (vert_dist * vert_dist + horz_dist * horz_dist).sqrt();
        }
        -(vert_dist * vert_dist + horz_dist * horz_dist).sqrt()
    }

    fn intersects_rect_path(rect: &ElkRectangle, path: &KVectorChain) -> bool {
        if path.size() < 2 {
            return false;
        }
        let mut iter = path.iter();
        let first = *iter.next().expect("path has at least one element");
        let mut p1 = first;
        for p2 in iter {
            if Self::intersects_rect_line(rect, &p1, p2) {
                return true;
            }
            p1 = *p2;
        }
        Self::intersects_rect_line(rect, &p1, &first)
    }

    fn intersects_rect_line(rect: &ElkRectangle, p1: &KVector, p2: &KVector) -> bool {
        if Self::contains_rect_line(rect, p1, p2) {
            return false;
        }
        let top_left = rect.get_top_left();
        let top_right = rect.get_top_right();
        let bottom_right = rect.get_bottom_right();
        let bottom_left = rect.get_bottom_left();
        Self::intersects_lines(&top_left, &top_right, p1, p2)
            || Self::intersects_lines(&top_right, &bottom_right, p1, p2)
            || Self::intersects_lines(&bottom_right, &bottom_left, p1, p2)
            || Self::intersects_lines(&bottom_left, &top_left, p1, p2)
    }

    fn intersects_lines(l11: &KVector, l12: &KVector, l21: &KVector, l22: &KVector) -> bool {
        let u0 = *l11;
        let mut v0 = *l12;
        v0.sub(l11);
        let u1 = *l21;
        let mut v1 = *l22;
        v1.sub(l21);
        let x00 = u0.x;
        let y00 = u0.y;
        let x10 = u1.x;
        let y10 = u1.y;
        let x01 = v0.x;
        let y01 = v0.y;
        let x11 = v1.x;
        let y11 = v1.y;

        let d = x11 * y01 - x01 * y11;
        if Self::fuzzy_equals(0.0, d, DOUBLE_EQ_EPSILON) {
            return false;
        }
        let s = (1.0 / d) * ((x00 - x10) * y01 - (y00 - y10) * x01);
        let t = (1.0 / d) * -(-(x00 - x10) * y11 + (y00 - y10) * x11);

        Self::fuzzy_compare(0.0, s, DOUBLE_EQ_EPSILON) < 0
            && Self::fuzzy_compare(s, 1.0, DOUBLE_EQ_EPSILON) < 0
            && Self::fuzzy_compare(0.0, t, DOUBLE_EQ_EPSILON) < 0
            && Self::fuzzy_compare(t, 1.0, DOUBLE_EQ_EPSILON) < 0
    }

    fn trace_rays(a1: &KVector, a2: &KVector, b1: &KVector, b2: &KVector, v: &KVector) -> f64 {
        let mut result = f64::INFINITY;
        let mut endpoint_hit = false;

        let mut b1_plus_v = *b1;
        b1_plus_v.add(v);
        let mut b_dir = *b2;
        b_dir.sub(b1);
        let mut a_dir = *a2;
        a_dir.sub(a1);
        let intersection = Self::intersects2(a1, &a_dir, &b1_plus_v, &b_dir);
        let edge_case = intersection
            .map(|point| !(point.equals_fuzzily(a1) || point.equals_fuzzily(a2)))
            .unwrap_or(false);

        let intersection = Self::intersects2(a1, &a_dir, b1, v);
        if let Some(point) = intersection {
            if point.equals_fuzzily(a1) == point.equals_fuzzily(a2) || edge_case {
                let mut diff = point;
                diff.sub(b1);
                result = result.min(diff.length());
            } else {
                endpoint_hit = true;
            }
        }

        let intersection = Self::intersects2(a1, &a_dir, b2, v);
        if let Some(point) = intersection {
            if endpoint_hit || point.equals_fuzzily(a1) == point.equals_fuzzily(a2) || edge_case {
                let mut diff = point;
                diff.sub(b2);
                result = result.min(diff.length());
            }
        }

        result
    }

    fn contains_rect_path(rect: &ElkRectangle, path: &KVectorChain) -> bool {
        if path.size() < 2 {
            return false;
        }
        let mut iter = path.iter();
        let first = *iter.next().expect("path has at least one element");
        let mut p1 = first;
        for p2 in iter {
            if !Self::contains_rect_line(rect, &p1, p2) {
                return false;
            }
            p1 = *p2;
        }
        Self::contains_rect_line(rect, &p1, &first)
    }

    fn contains_rect_line(rect: &ElkRectangle, p1: &KVector, p2: &KVector) -> bool {
        Self::contains_rect_point(rect, p1) && Self::contains_rect_point(rect, p2)
    }

    fn contains_rect_point(rect: &ElkRectangle, p: &KVector) -> bool {
        let min_x = rect.x;
        let max_x = rect.x + rect.width;
        let min_y = rect.y;
        let max_y = rect.y + rect.height;
        (p.x > min_x && p.x < max_x) && (p.y > min_y && p.y < max_y)
    }

    fn convert_to_bezier_form(v: &[KVector; DEGREE + 1], pa: &KVector) -> [KVector; W_DEGREE + 1] {
        let mut c = [ZERO_VECTOR; DEGREE + 1];
        let mut d = [ZERO_VECTOR; DEGREE];
        let mut cd_table = [[0.0; DEGREE + 1]; DEGREE];
        let mut w = [ZERO_VECTOR; W_DEGREE + 1];

        for i in 0..=DEGREE {
            c[i] = KVector::with_values(v[i].x - pa.x, v[i].y - pa.y);
        }

        let s = DEGREE as f64;
        for i in 0..DEGREE {
            d[i] = KVector::with_values(s * (v[i + 1].x - v[i].x), s * (v[i + 1].y - v[i].y));
        }

        for row in 0..DEGREE {
            for (column, c_column) in c.iter().enumerate() {
                cd_table[row][column] = d[row].x * c_column.x + d[row].y * c_column.y;
            }
        }

        for (i, value) in w.iter_mut().enumerate() {
            *value = KVector::with_values(i as f64 / W_DEGREE as f64, 0.0);
        }

        let n = DEGREE;
        let m = DEGREE - 1;
        for k in 0..=n + m {
            let lb = k.saturating_sub(m);
            let ub = if k <= n { k } else { n };
            for i in lb..=ub {
                let j = k - i;
                w[i + j].y += cd_table[j][i] * CUBIC_Z[j][i];
            }
        }

        w
    }

    fn find_roots(
        w: &[KVector; W_DEGREE + 1],
        degree: usize,
        t: &mut [f64; W_DEGREE + 1],
        depth: usize,
    ) -> usize {
        match Self::crossing_count(w, degree) {
            0 => return 0,
            1 => {
                if depth >= MAXDEPTH {
                    t[0] = (w[0].x + w[W_DEGREE].x) / 2.0;
                    return 1;
                }
                if Self::control_polygon_flat_enough(w, degree) {
                    t[0] = Self::compute_x_intercept(w, degree);
                    return 1;
                }
            }
            _ => {}
        }

        let mut left = [ZERO_VECTOR; W_DEGREE + 1];
        let mut right = [ZERO_VECTOR; W_DEGREE + 1];
        let mut left_t = [0.0; W_DEGREE + 1];
        let mut right_t = [0.0; W_DEGREE + 1];

        Self::bezier(w, degree, 0.5, Some(&mut left), Some(&mut right));
        let left_count = Self::find_roots(&left, degree, &mut left_t, depth + 1);
        let right_count = Self::find_roots(&right, degree, &mut right_t, depth + 1);

        t[..left_count].copy_from_slice(&left_t[..left_count]);
        t[left_count..(left_count + right_count)].copy_from_slice(&right_t[..right_count]);

        left_count + right_count
    }

    fn control_polygon_flat_enough(v: &[KVector; W_DEGREE + 1], degree: usize) -> bool {
        let a = v[0].y - v[degree].y;
        let b = v[degree].x - v[0].x;
        let c = v[0].x * v[degree].y - v[degree].x * v[0].y;

        let ab_squared = a * a + b * b;
        let mut distance = [0.0; W_DEGREE + 1];

        for i in 1..degree {
            distance[i] = a * v[i].x + b * v[i].y + c;
            if distance[i] > 0.0 {
                distance[i] = (distance[i] * distance[i]) / ab_squared;
            }
            if distance[i] < 0.0 {
                distance[i] = -((distance[i] * distance[i]) / ab_squared);
            }
        }

        let mut max_distance_above: f64 = 0.0;
        let mut max_distance_below: f64 = 0.0;
        for &value in distance.iter().take(degree).skip(1) {
            if value < 0.0 {
                max_distance_below = max_distance_below.min(value);
            }
            if value > 0.0 {
                max_distance_above = max_distance_above.max(value);
            }
        }

        let a1 = 0.0;
        let b1 = 1.0;
        let c1 = 0.0;

        let mut a2 = a;
        let mut b2 = b;
        let mut c2 = c + max_distance_above;
        let mut det = a1 * b2 - a2 * b1;
        let mut d_inv = 1.0 / det;
        let intercept1 = (b1 * c2 - b2 * c1) * d_inv;

        a2 = a;
        b2 = b;
        c2 = c + max_distance_below;
        det = a1 * b2 - a2 * b1;
        d_inv = 1.0 / det;
        let intercept2 = (b1 * c2 - b2 * c1) * d_inv;

        let left_intercept = intercept1.min(intercept2);
        let right_intercept = intercept1.max(intercept2);

        let error = (right_intercept - left_intercept) / 2.0;
        error < EPSILON
    }

    fn compute_x_intercept(v: &[KVector; W_DEGREE + 1], degree: usize) -> f64 {
        let xnm = v[degree].x - v[0].x;
        let ynm = v[degree].y - v[0].y;
        let xmk = v[0].x;
        let ymk = v[0].y;

        let det_inv = -1.0 / ynm;
        (xnm * ymk - ynm * xmk) * det_inv
    }

    fn crossing_count(v: &[KVector; W_DEGREE + 1], degree: usize) -> i32 {
        let mut n_crossings = 0;
        let mut sign = if v[0].y < 0.0 { -1 } else { 1 };
        let mut old_sign = sign;
        for point in v.iter().take(degree + 1).skip(1) {
            sign = if point.y < 0.0 { -1 } else { 1 };
            if sign != old_sign {
                n_crossings += 1;
            }
            old_sign = sign;
        }
        n_crossings
    }

    fn bezier(
        c: &[KVector],
        degree: usize,
        t: f64,
        left: Option<&mut [KVector; W_DEGREE + 1]>,
        right: Option<&mut [KVector; W_DEGREE + 1]>,
    ) -> KVector {
        let mut p = [[ZERO_VECTOR; W_DEGREE + 1]; W_DEGREE + 1];

        p[0][..(degree + 1)].copy_from_slice(&c[..(degree + 1)]);

        for i in 1..=degree {
            for j in 0..=degree - i {
                p[i][j] = KVector::with_values(
                    (1.0 - t) * p[i - 1][j].x + t * p[i - 1][j + 1].x,
                    (1.0 - t) * p[i - 1][j].y + t * p[i - 1][j + 1].y,
                );
            }
        }

        if let Some(left) = left {
            for j in 0..=degree {
                left[j] = p[j][0];
            }
        }

        if let Some(right) = right {
            for j in 0..=degree {
                right[j] = p[degree - j][j];
            }
        }

        p[degree][0]
    }

    fn fuzzy_equals(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    fn fuzzy_compare(a: f64, b: f64, eps: f64) -> i32 {
        if a < b - eps {
            -1
        } else if a > b + eps {
            1
        } else {
            0
        }
    }
}

impl IntersectsArgs for (&ElkRectangle, &KVectorChain) {
    fn intersects(self) -> bool {
        ElkMath::intersects_rect_path(self.0, self.1)
    }
}

impl IntersectsArgs for (&ElkRectangle, &KVector, &KVector) {
    fn intersects(self) -> bool {
        ElkMath::intersects_rect_line(self.0, self.1, self.2)
    }
}

impl IntersectsArgs for (&KVector, &KVector, &KVector, &KVector) {
    fn intersects(self) -> bool {
        ElkMath::intersects_lines(self.0, self.1, self.2, self.3)
    }
}

impl ContainsArgs for (&ElkRectangle, &KVectorChain) {
    fn contains(self) -> bool {
        ElkMath::contains_rect_path(self.0, self.1)
    }
}

impl ContainsArgs for (&ElkRectangle, &KVector, &KVector) {
    fn contains(self) -> bool {
        ElkMath::contains_rect_line(self.0, self.1, self.2)
    }
}

impl ContainsArgs for (&ElkRectangle, &KVector) {
    fn contains(self) -> bool {
        ElkMath::contains_rect_point(self.0, self.1)
    }
}
