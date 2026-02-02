use crate::org::eclipse::elk::core::comments::i_decider::{IDecider, NormalizedHeuristics};

type Aggregator = dyn Fn(&[f64]) -> f64;

pub struct AggregatedMatchDecider<T> {
    aggregator: Box<Aggregator>,
    lower_boundary: f64,
    include_lower_boundary: bool,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: 'static> AggregatedMatchDecider<T> {
    pub fn new() -> Self {
        AggregatedMatchDecider {
            aggregator: Box::new(Self::max),
            lower_boundary: 0.0,
            include_lower_boundary: false,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_aggregator<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn(&[f64]) -> f64 + 'static,
    {
        self.aggregator = Box::new(f);
        self
    }

    pub fn with_lower_attachment_boundary(&mut self, lower: f64) -> &mut Self {
        if lower < 0.0 {
            panic!("Lower boundary must be >= 0.");
        }
        self.lower_boundary = lower;
        self
    }

    pub fn with_lower_boundary_included(&mut self, include: bool) -> &mut Self {
        self.include_lower_boundary = include;
        self
    }

    pub fn max(values: &[f64]) -> f64 {
        if values.is_empty() {
            panic!("Aggregator requires at least one value.");
        }
        values
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max)
    }

    pub fn min(values: &[f64]) -> f64 {
        if values.is_empty() {
            panic!("Aggregator requires at least one value.");
        }
        values.iter().cloned().fold(f64::INFINITY, f64::min)
    }

    pub fn avg(values: &[f64]) -> f64 {
        if values.is_empty() {
            panic!("Aggregator requires at least one value.");
        }
        let sum: f64 = values.iter().sum();
        sum / values.len() as f64
    }

    pub fn sum(values: &[f64]) -> f64 {
        if values.is_empty() {
            panic!("Aggregator requires at least one value.");
        }
        values.iter().sum()
    }
}

impl<T: 'static> Default for AggregatedMatchDecider<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> IDecider<T> for AggregatedMatchDecider<T> {
    fn make_attachment_decision(&self, heuristics: &[NormalizedHeuristics<T>]) -> Option<T> {
        let mut max = f64::NEG_INFINITY;
        let mut max_target: Option<T> = None;

        for entry in heuristics {
            let values: Vec<f64> = entry.values.values().copied().collect();
            let aggregate = (self.aggregator)(&values);
            if aggregate < 0.0 {
                panic!("The aggregator provided a value < 0.");
            }

            if aggregate > max {
                max = aggregate;
                max_target = Some(entry.target.clone());
            }
        }

        let target = max_target?;

        if self.include_lower_boundary {
            if max >= self.lower_boundary {
                Some(target)
            } else {
                None
            }
        } else if max > self.lower_boundary {
            Some(target)
        } else {
            None
        }
    }
}
