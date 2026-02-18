use std::rc::Rc;

use crate::org::eclipse::elk::core::comments::abstract_normalized_matcher::{
    NormalizationConfig, NormalizationFunction,
};
use crate::org::eclipse::elk::core::comments::i_bounds_provider::IBoundsProvider;
use crate::org::eclipse::elk::core::comments::i_matcher::IMatcher;
use crate::org::eclipse::elk::core::comments::rectangle_utils::{
    outcode, BOTTOM, BOTTOM_LEFT, BOTTOM_RIGHT, LEFT, RIGHT, TOP, TOP_LEFT, TOP_RIGHT,
};
use crate::org::eclipse::elk::core::math::ElkRectangle;

pub struct AlignmentMatcher<C, T> {
    config: NormalizationConfig,
    bounds_provider: Option<Rc<dyn IBoundsProvider<C, T>>>,
}

impl<C, T> AlignmentMatcher<C, T> {
    pub fn new() -> Self {
        AlignmentMatcher {
            config: NormalizationConfig::new(),
            bounds_provider: None,
        }
    }

    pub fn with_maximum_alignment_offset(&mut self, offset: f64) -> &mut Self {
        if offset <= 0.0 {
            panic!("Maximum alignment offset must be > 0.");
        }
        self.config.with_bounds(offset, 0.0);
        self
    }

    pub fn with_bounds_provider(&mut self, provider: Rc<dyn IBoundsProvider<C, T>>) -> &mut Self {
        self.bounds_provider = Some(provider);
        self
    }

    pub fn with_normalization_function(
        &mut self,
        normalization_function: NormalizationFunction,
    ) -> &mut Self {
        self.config
            .with_normalization_function(normalization_function);
        self
    }

    pub fn alignment(bounds1: &ElkRectangle, bounds2: &ElkRectangle) -> f64 {
        alignment(bounds1, bounds2)
    }

    fn check_configuration(&self) {
        if self.bounds_provider.is_none() {
            panic!("A bounds provider is required.");
        }
    }
}

impl<C, T> Default for AlignmentMatcher<C, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: 'static, T: 'static> IMatcher<C, T> for AlignmentMatcher<C, T> {
    fn raw(&self, comment: &C, target: &T) -> f64 {
        let Some(provider) = self.bounds_provider.as_ref() else {
            return self.config.worst_raw_value();
        };
        let Some(comment_bounds) = provider.bounds_for_comment(comment) else {
            return self.config.worst_raw_value();
        };
        let Some(target_bounds) = provider.bounds_for_target(target) else {
            return self.config.worst_raw_value();
        };

        let alignment = alignment(&comment_bounds, &target_bounds);
        if alignment < 0.0 {
            self.config.worst_raw_value()
        } else {
            alignment
        }
    }

    fn normalized(&self, comment: &C, target: &T) -> f64 {
        self.config.normalize(self.raw(comment, target))
    }

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
}

pub fn alignment(bounds1: &ElkRectangle, bounds2: &ElkRectangle) -> f64 {
    let top_left_outcode = outcode(bounds2, bounds1.x, bounds1.y);
    let bottom_right_outcode = outcode(
        bounds2,
        bounds1.x + bounds1.width,
        bounds1.y + bounds1.height,
    );

    if (bottom_right_outcode & top_left_outcode & TOP_LEFT) == TOP_LEFT
        || (bottom_right_outcode & top_left_outcode & TOP_RIGHT) == TOP_RIGHT
        || (bottom_right_outcode & top_left_outcode & BOTTOM_LEFT) == BOTTOM_LEFT
        || (bottom_right_outcode & top_left_outcode & BOTTOM_RIGHT) == BOTTOM_RIGHT
    {
        return -1.0;
    }

    let horizontal_alignment_offset = (bounds2.x - bounds1.x)
        .abs()
        .min((bounds2.x + bounds2.width - bounds1.x - bounds1.width).abs());
    let vertical_alignment_offset = (bounds2.y - bounds1.y)
        .abs()
        .min((bounds2.y + bounds2.height - bounds1.y - bounds1.height).abs());

    if bounds2.intersects(bounds1) {
        return horizontal_alignment_offset.min(vertical_alignment_offset);
    }

    if (bottom_right_outcode & TOP) != 0 || (top_left_outcode & BOTTOM) != 0 {
        return horizontal_alignment_offset;
    } else if (bottom_right_outcode & LEFT) != 0 || (top_left_outcode & RIGHT) != 0 {
        return vertical_alignment_offset;
    } else if bounds1.y == bounds2.y + bounds2.height || bounds1.y + bounds1.height == bounds2.y {
        return horizontal_alignment_offset;
    } else if bounds1.x == bounds2.x + bounds2.width || bounds1.x + bounds1.width == bounds2.x {
        return vertical_alignment_offset;
    }

    -1.0
}
