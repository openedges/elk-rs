use std::fmt;
use std::hash::{Hash, Hasher};

use crate::org::eclipse::elk::core::util::{IDataObject, Random};

#[derive(Clone, Copy, Debug)]
pub struct KVector {
    pub x: f64,
    pub y: f64,
}

impl KVector {
    pub const DEFAULT_FUZZYNESS: f64 = 0.05;

    pub fn new() -> Self {
        KVector { x: 0.0, y: 0.0 }
    }

    pub fn with_values(x: f64, y: f64) -> Self {
        KVector { x, y }
    }

    pub fn from_vector(other: &KVector) -> Self {
        KVector {
            x: other.x,
            y: other.y,
        }
    }

    pub fn from_points(start: &KVector, end: &KVector) -> Self {
        KVector {
            x: end.x - start.x,
            y: end.y - start.y,
        }
    }

    pub fn from_angle(angle: f64) -> Self {
        KVector {
            x: angle.cos(),
            y: angle.sin(),
        }
    }

    pub fn equals_fuzzily(&self, other: &KVector) -> bool {
        self.equals_fuzzily_with(other, Self::DEFAULT_FUZZYNESS)
    }

    pub fn equals_fuzzily_with(&self, other: &KVector, fuzzyness: f64) -> bool {
        (self.x - other.x).abs() <= fuzzyness && (self.y - other.y).abs() <= fuzzyness
    }

    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn square_length(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    pub fn reset(&mut self) -> &mut Self {
        self.x = 0.0;
        self.y = 0.0;
        self
    }

    pub fn set(&mut self, other: &KVector) -> &mut Self {
        self.x = other.x;
        self.y = other.y;
        self
    }

    pub fn set_values(&mut self, x: f64, y: f64) -> &mut Self {
        self.x = x;
        self.y = y;
        self
    }

    pub fn add(&mut self, other: &KVector) -> &mut Self {
        self.x += other.x;
        self.y += other.y;
        self
    }

    pub fn add_values(&mut self, dx: f64, dy: f64) -> &mut Self {
        self.x += dx;
        self.y += dy;
        self
    }

    pub fn sum(vectors: &[KVector]) -> KVector {
        let mut sum = KVector::new();
        for v in vectors {
            sum.x += v.x;
            sum.y += v.y;
        }
        sum
    }

    pub fn sub(&mut self, other: &KVector) -> &mut Self {
        self.x -= other.x;
        self.y -= other.y;
        self
    }

    pub fn sub_values(&mut self, dx: f64, dy: f64) -> &mut Self {
        self.x -= dx;
        self.y -= dy;
        self
    }

    pub fn diff(v1: &KVector, v2: &KVector) -> KVector {
        KVector {
            x: v1.x - v2.x,
            y: v1.y - v2.y,
        }
    }

    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.x *= scale;
        self.y *= scale;
        self
    }

    pub fn scale_values(&mut self, scalex: f64, scaley: f64) -> &mut Self {
        self.x *= scalex;
        self.y *= scaley;
        self
    }

    pub fn normalize(&mut self) -> &mut Self {
        let length = self.length();
        if length > 0.0 {
            self.x /= length;
            self.y /= length;
        }
        self
    }

    pub fn scale_to_length(&mut self, length: f64) -> &mut Self {
        self.normalize();
        self.scale(length);
        self
    }

    pub fn negate(&mut self) -> &mut Self {
        self.x = -self.x;
        self.y = -self.y;
        self
    }

    pub fn to_degrees(&self) -> f64 {
        self.to_radians().to_degrees()
    }

    pub fn to_radians(&self) -> f64 {
        let length = self.length();
        assert!(length > 0.0);
        if self.x >= 0.0 && self.y >= 0.0 {
            (self.y / length).asin()
        } else if self.x < 0.0 {
            std::f64::consts::PI - (self.y / length).asin()
        } else {
            2.0 * std::f64::consts::PI + (self.y / length).asin()
        }
    }

    pub fn wiggle(&mut self, random: &mut Random, amount: f64) {
        self.x += random.next_double() * amount - (amount / 2.0);
        self.y += random.next_double() * amount - (amount / 2.0);
    }

    pub fn distance(&self, other: &KVector) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn dot_product(&self, other: &KVector) -> f64 {
        self.x * other.x + self.y * other.y
    }

    pub fn cross_product(v: &KVector, w: &KVector) -> f64 {
        v.x * w.y - v.y * w.x
    }

    pub fn rotate(&mut self, angle: f64) -> &mut Self {
        let new_x = self.x * angle.cos() - self.y * angle.sin();
        self.y = self.x * angle.sin() + self.y * angle.cos();
        self.x = new_x;
        self
    }

    pub fn angle(&self, other: &KVector) -> f64 {
        (self.dot_product(other) / (self.length() * other.length())).acos()
    }

    pub fn bound(&mut self, lowx: f64, lowy: f64, highx: f64, highy: f64) -> &mut Self {
        if highx < lowx || highy < lowy {
            panic!("The highx must be bigger then lowx and the highy must be bigger then lowy");
        }
        if self.x < lowx {
            self.x = lowx;
        } else if self.x > highx {
            self.x = highx;
        }
        if self.y < lowy {
            self.y = lowy;
        } else if self.y > highy {
            self.y = highy;
        }
        self
    }

    pub fn is_nan(&self) -> bool {
        self.x.is_nan() || self.y.is_nan()
    }

    pub fn is_infinite(&self) -> bool {
        self.x.is_infinite() || self.y.is_infinite()
    }

    pub fn parse(&mut self, string: &str) {
        let mut start = 0usize;
        let chars: Vec<char> = string.chars().collect();
        while start < chars.len() && is_delim(chars[start], "([{'\" \t\r\n") {
            start += 1;
        }
        let mut end = chars.len();
        while end > 0 && is_delim(chars[end - 1], ")]}'\" \t\r\n") {
            end -= 1;
        }
        if start >= end {
            panic!("The given string does not contain any numbers.");
        }
        let slice: String = chars[start..end].iter().collect();
        let tokens: Vec<&str> = slice.split(&[',', ';', '\r', '\n'][..]).collect();
        if tokens.len() != 2 {
            panic!(
                "Exactly two numbers are expected, {} were found.",
                tokens.len()
            );
        }
        let x: f64 = tokens[0].trim().parse().unwrap_or_else(|_| {
            panic!("The given string contains parts that cannot be parsed as numbers.")
        });
        let y: f64 = tokens[1].trim().parse().unwrap_or_else(|_| {
            panic!("The given string contains parts that cannot be parsed as numbers.")
        });
        self.x = x;
        self.y = y;
    }
}

impl Default for KVector {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for KVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }
}

impl PartialEq for KVector {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl Eq for KVector {}

impl Hash for KVector {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.to_bits().hash(state);
        self.y.to_bits().hash(state);
    }
}

impl IDataObject for KVector {}

fn is_delim(c: char, delims: &str) -> bool {
    delims.chars().any(|d| d == c)
}
