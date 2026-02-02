#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalizationFunction {
    Linear,
    Binary,
}

#[derive(Clone, Copy, Debug)]
pub struct NormalizationConfig {
    worst_raw_value: f64,
    best_raw_value: f64,
    normalization_function: NormalizationFunction,
}

impl NormalizationConfig {
    pub fn new() -> Self {
        NormalizationConfig {
            worst_raw_value: 0.0,
            best_raw_value: 1.0,
            normalization_function: NormalizationFunction::Linear,
        }
    }

    pub fn with_bounds(&mut self, worst_raw_value: f64, best_raw_value: f64) -> &mut Self {
        self.worst_raw_value = worst_raw_value;
        self.best_raw_value = best_raw_value;
        self
    }

    pub fn with_normalization_function(
        &mut self,
        normalization_function: NormalizationFunction,
    ) -> &mut Self {
        self.normalization_function = normalization_function;
        self
    }

    pub fn worst_raw_value(&self) -> f64 {
        self.worst_raw_value
    }

    pub fn best_raw_value(&self) -> f64 {
        self.best_raw_value
    }

    pub fn normalize(&self, raw: f64) -> f64 {
        match self.normalization_function {
            NormalizationFunction::Linear => self.normalize_linear(raw),
            NormalizationFunction::Binary => self.normalize_binary(raw),
        }
    }

    pub fn normalize_linear(&self, raw: f64) -> f64 {
        if self.worst_raw_value < self.best_raw_value {
            if raw <= self.worst_raw_value {
                0.0
            } else if raw >= self.best_raw_value {
                1.0
            } else {
                (raw - self.worst_raw_value) / (self.best_raw_value - self.worst_raw_value)
            }
        } else if self.best_raw_value < self.worst_raw_value {
            if raw <= self.best_raw_value {
                1.0
            } else if raw >= self.worst_raw_value {
                0.0
            } else {
                1.0 - (raw - self.best_raw_value) / (self.worst_raw_value - self.best_raw_value)
            }
        } else if raw == self.best_raw_value {
            1.0
        } else {
            0.0
        }
    }

    pub fn normalize_binary(&self, raw: f64) -> f64 {
        if self.worst_raw_value < self.best_raw_value {
            if raw <= self.worst_raw_value {
                0.0
            } else {
                1.0
            }
        } else if self.best_raw_value < self.worst_raw_value {
            if raw >= self.worst_raw_value {
                0.0
            } else {
                1.0
            }
        } else if raw == self.best_raw_value {
            1.0
        } else {
            0.0
        }
    }
}

impl Default for NormalizationConfig {
    fn default() -> Self {
        Self::new()
    }
}
