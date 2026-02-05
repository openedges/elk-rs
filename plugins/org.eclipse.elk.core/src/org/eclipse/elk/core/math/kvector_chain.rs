use crate::org::eclipse::elk::core::math::kvector::KVector;
use crate::org::eclipse::elk::core::util::IDataObject;

#[derive(Clone, Debug, PartialEq)]
pub struct KVectorChain {
    vectors: Vec<KVector>,
}

impl KVectorChain {
    pub fn new() -> Self {
        KVectorChain { vectors: Vec::new() }
    }

    pub fn from_collection(collection: &[KVector]) -> Self {
        KVectorChain {
            vectors: collection.to_vec(),
        }
    }

    pub fn from_vectors(vectors: &[KVector]) -> Self {
        Self::from_collection(vectors)
    }

    pub fn add_vector(&mut self, vector: KVector) {
        self.vectors.push(vector);
    }

    pub fn add(&mut self) {
        self.vectors.push(KVector::new());
    }

    pub fn add_values(&mut self, x: f64, y: f64) {
        self.vectors.push(KVector::with_values(x, y));
    }

    pub fn add_first(&mut self) {
        self.vectors.insert(0, KVector::new());
    }

    pub fn add_first_values(&mut self, x: f64, y: f64) {
        self.vectors.insert(0, KVector::with_values(x, y));
    }

    pub fn add_last(&mut self) {
        self.vectors.push(KVector::new());
    }

    pub fn add_last_values(&mut self, x: f64, y: f64) {
        self.vectors.push(KVector::with_values(x, y));
    }

    pub fn add_all(&mut self, vectors: &[KVector]) {
        self.vectors.extend_from_slice(vectors);
    }

    pub fn insert(&mut self, index: usize, vector: KVector) {
        if index > self.vectors.len() {
            panic!("Index out of bounds");
        }
        self.vectors.insert(index, vector);
    }

    pub fn set(&mut self, index: usize, vector: KVector) {
        if index >= self.vectors.len() {
            panic!("Index out of bounds");
        }
        self.vectors[index] = vector;
    }

    pub fn clear(&mut self) {
        self.vectors.clear();
    }

    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    pub fn size(&self) -> usize {
        self.vectors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    pub fn get(&self, index: usize) -> KVector {
        self.vectors[index]
    }

    pub fn get_first(&self) -> KVector {
        if self.vectors.is_empty() {
            panic!("Cannot get first element of empty vector chain.");
        }
        self.vectors[0]
    }

    pub fn get_last(&self) -> KVector {
        if self.vectors.is_empty() {
            panic!("Cannot get last element of empty vector chain.");
        }
        self.vectors[self.vectors.len() - 1]
    }

    pub fn iter(&self) -> impl Iterator<Item = &KVector> {
        self.vectors.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut KVector> {
        self.vectors.iter_mut()
    }

    pub fn to_array(&self) -> Vec<KVector> {
        self.vectors.clone()
    }

    pub fn to_array_from(&self, begin_index: usize) -> Vec<KVector> {
        if begin_index > self.vectors.len() {
            panic!("Index out of bounds");
        }
        self.vectors[begin_index..].to_vec()
    }

    pub fn parse(&mut self, string: &str) {
        let tokens: Vec<&str> = string
            .split(&[',', ';', '(', ')', '[', ']', '{', '}', ' ', '\t', '\n'][..])
            .collect();
        self.vectors.clear();
        let mut xy = 0;
        let mut x = 0.0;
        for token in tokens {
            if !token.trim().is_empty() {
                let value: f64 = token.trim().parse().unwrap_or_else(|_| {
                    panic!("The given string does not match the expected format for vectors.")
                });
                if xy % 2 == 0 {
                    x = value;
                } else {
                    self.vectors.push(KVector::with_values(x, value));
                }
                xy += 1;
            }
        }
    }

    pub fn offset_non_mutating(&self, offset: &KVector) -> KVectorChain {
        let mut result = KVectorChain::new();
        for vector in &self.vectors {
            let mut sum = KVector::from_vector(vector);
            sum.add(offset);
            result.add_vector(sum);
        }
        result
    }

    pub fn offset(&mut self, dx: f64, dy: f64) -> &mut Self {
        for vector in &mut self.vectors {
            vector.add_values(dx, dy);
        }
        self
    }

    pub fn remove_last(&mut self) -> Option<KVector> {
        self.vectors.pop()
    }

    pub fn total_length(&self) -> f64 {
        let mut length = 0.0;
        if self.vectors.len() >= 2 {
            for i in 0..self.vectors.len() - 1 {
                length += self.vectors[i].distance(&self.vectors[i + 1]);
            }
        }
        length
    }

    pub fn has_nan(&self) -> bool {
        self.vectors.iter().any(|v| v.is_nan())
    }

    pub fn has_infinite(&self) -> bool {
        self.vectors.iter().any(|v| v.is_infinite())
    }

    pub fn point_on_line(&self, dist: f64) -> KVector {
        if self.vectors.is_empty() {
            panic!("Cannot determine a point on an empty vector chain.");
        }
        if self.vectors.len() == 1 {
            return self.vectors[0];
        }
        let abs_distance = dist.abs();
        let mut distance_sum = 0.0;
        if dist >= 0.0 {
            for i in 0..self.vectors.len() - 1 {
                let current = self.vectors[i];
                let next = self.vectors[i + 1];
                let additional = current.distance(&next);
                if additional > 0.0 {
                    let old_sum = distance_sum;
                    distance_sum += additional;
                    if distance_sum >= abs_distance {
                        let relative = (abs_distance - old_sum) / additional;
                        let mut result = KVector::from_vector(&next);
                        result.sub(&current);
                        result.scale(relative);
                        result.add(&current);
                        return result;
                    }
                }
            }
            return self.vectors[self.vectors.len() - 1];
        }
        for i in (1..self.vectors.len()).rev() {
            let current = self.vectors[i];
            let next = self.vectors[i - 1];
            let additional = current.distance(&next);
            if additional > 0.0 {
                let old_sum = distance_sum;
                distance_sum += additional;
                if distance_sum >= abs_distance {
                    let relative = (abs_distance - old_sum) / additional;
                    let mut result = KVector::from_vector(&next);
                    result.sub(&current);
                    result.scale(relative);
                    result.add(&current);
                    return result;
                }
            }
        }
        self.vectors[0]
    }

    pub fn angle_on_line(&self, dist: f64) -> f64 {
        if self.vectors.len() < 2 {
            panic!("Need at least two points to determine an angle.");
        }
        let abs_distance = dist.abs();
        let mut distance_sum = 0.0;
        if dist >= 0.0 {
            for i in 0..self.vectors.len() - 1 {
                let current = self.vectors[i];
                let next = self.vectors[i + 1];
                let additional = current.distance(&next);
                if additional > 0.0 {
                    distance_sum += additional;
                    if distance_sum >= abs_distance {
                        let mut diff = KVector::from_vector(&next);
                        diff.sub(&current);
                        return diff.to_radians();
                    }
                }
            }
            let mut diff = KVector::from_vector(&self.vectors[self.vectors.len() - 1]);
            diff.sub(&self.vectors[self.vectors.len() - 2]);
            return diff.to_radians();
        }
        for i in (1..self.vectors.len()).rev() {
            let current = self.vectors[i];
            let next = self.vectors[i - 1];
            let additional = current.distance(&next);
            if additional > 0.0 {
                distance_sum += additional;
                if distance_sum >= abs_distance {
                    let mut diff = KVector::from_vector(&next);
                    diff.sub(&current);
                    return diff.to_radians();
                }
            }
        }
        let mut diff = KVector::from_vector(&self.vectors[0]);
        diff.sub(&self.vectors[1]);
        diff.to_radians()
    }

    pub fn reverse(chain: &KVectorChain) -> KVectorChain {
        let mut result = KVectorChain::new();
        for vector in chain.vectors.iter().rev() {
            result.vectors.push(KVector::from_vector(vector));
        }
        result
    }
}

impl Default for KVectorChain {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for KVectorChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        for vector in &self.vectors {
            parts.push(format!("{},{}", vector.x, vector.y));
        }
        write!(f, "({})", parts.join("; "))
    }
}

impl IDataObject for KVectorChain {}
