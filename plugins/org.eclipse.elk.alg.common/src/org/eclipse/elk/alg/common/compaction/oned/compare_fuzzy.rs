pub const TOLERANCE: f64 = 0.0001;

pub fn eq(d1: f64, d2: f64) -> bool {
    (d1 - d2).abs() <= TOLERANCE
}

pub fn gt(d1: f64, d2: f64) -> bool {
    d1 - d2 > TOLERANCE
}

pub fn lt(d1: f64, d2: f64) -> bool {
    d2 - d1 > TOLERANCE
}

pub fn ge(d1: f64, d2: f64) -> bool {
    d1 > d2 || eq(d1, d2)
}

pub fn le(d1: f64, d2: f64) -> bool {
    d1 < d2 || eq(d1, d2)
}
