use std::rc::Rc;

use crate::org::eclipse::elk::core::comments::i_bounds_provider::IBoundsProvider;
use crate::org::eclipse::elk::core::comments::i_filter::IFilter;

pub struct SizeFilter<C, T> {
    bounds_provider: Option<Rc<dyn IBoundsProvider<C, T>>>,
    max_area: f64,
}

impl<C, T> SizeFilter<C, T> {
    pub fn new() -> Self {
        SizeFilter {
            bounds_provider: None,
            max_area: -1.0,
        }
    }

    pub fn with_maximum_area(&mut self, area: f64) -> &mut Self {
        if area < 0.0 {
            panic!("Maximum area must be >= 0.");
        }
        self.max_area = area;
        self
    }

    pub fn with_bounds_provider(&mut self, provider: Rc<dyn IBoundsProvider<C, T>>) -> &mut Self {
        self.bounds_provider = Some(provider);
        self
    }

    fn check_configuration(&self) {
        if self.bounds_provider.is_none() {
            panic!("A bounds provider is required.");
        }
    }
}

impl<C, T> Default for SizeFilter<C, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: 'static, T: 'static> IFilter<C, T> for SizeFilter<C, T> {
    fn preprocess(
        &self,
        data_provider: &dyn crate::org::eclipse::elk::core::comments::i_data_provider::IDataProvider<
            C,
            T,
        >,
        include_hierarchy: bool,
    ) {
        let _ = (data_provider, include_hierarchy);
        self.check_configuration();
    }

    fn eligible_for_attachment(&self, comment: &C) -> bool {
        let provider = self
            .bounds_provider
            .as_ref()
            .expect("A bounds provider is required.");
        let Some(bounds) = provider.bounds_for_comment(comment) else {
            return false;
        };
        bounds.width * bounds.height <= self.max_area
    }
}
