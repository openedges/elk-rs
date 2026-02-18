use std::rc::Rc;

use crate::org::eclipse::elk::core::comments::abstract_normalized_matcher::{
    NormalizationConfig, NormalizationFunction,
};
use crate::org::eclipse::elk::core::comments::i_bounds_provider::IBoundsProvider;
use crate::org::eclipse::elk::core::comments::i_matcher::IMatcher;
use crate::org::eclipse::elk::core::comments::rectangle_utils::{
    outcode, BOTTOM_LEFT, BOTTOM_RIGHT, OUT_BOTTOM, OUT_LEFT, OUT_RIGHT, OUT_TOP, TOP_LEFT,
    TOP_RIGHT,
};
use crate::org::eclipse::elk::core::math::ElkRectangle;

pub struct DistanceMatcher<C, T> {
    config: NormalizationConfig,
    bounds_provider: Option<Rc<dyn IBoundsProvider<C, T>>>,
}

impl<C, T> DistanceMatcher<C, T> {
    pub fn new() -> Self {
        DistanceMatcher {
            config: NormalizationConfig::new(),
            bounds_provider: None,
        }
    }

    pub fn with_maximum_attachment_distance(&mut self, distance: f64) -> &mut Self {
        if distance < 0.0 {
            panic!("Maximum attachment distance must be >= 0.");
        }
        self.config.with_bounds(distance, 0.0);
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

    pub fn distance(bounds1: &ElkRectangle, bounds2: &ElkRectangle) -> f64 {
        distance(bounds1, bounds2)
    }

    fn check_configuration(&self) {
        if self.bounds_provider.is_none() {
            panic!("A bounds provider is required.");
        }
    }
}

impl<C, T> Default for DistanceMatcher<C, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: 'static, T: 'static> IMatcher<C, T> for DistanceMatcher<C, T> {
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

        let distance = distance(&comment_bounds, &target_bounds);
        if distance < 0.0 {
            self.config.worst_raw_value()
        } else {
            distance
        }
    }

    fn normalized(&self, comment: &C, target: &T) -> f64 {
        self.config.normalize(self.raw(comment, target))
    }

    fn preprocess(
        &self,
        data_provider: &dyn crate::org::eclipse::elk::core::comments::i_data_provider::IDataProvider<C, T>,
        include_hierarchy: bool,
    ) {
        let _ = (data_provider, include_hierarchy);
        self.check_configuration();
    }
}

pub fn distance(bounds1: &ElkRectangle, bounds2: &ElkRectangle) -> f64 {
    if bounds1.intersects(bounds2) {
        return 0.0;
    }

    let top_left_outcode = outcode(bounds2, bounds1.x, bounds1.y);
    let bottom_right_outcode = outcode(
        bounds2,
        bounds1.x + bounds1.width,
        bounds1.y + bounds1.height,
    );

    if (top_left_outcode & TOP_LEFT) == TOP_LEFT && (bottom_right_outcode & TOP_LEFT) == TOP_LEFT {
        return distance_points(
            bounds1.x + bounds1.width,
            bounds1.y + bounds1.height,
            bounds2.x,
            bounds2.y,
        );
    } else if (top_left_outcode & BOTTOM_LEFT) == BOTTOM_LEFT
        && (bottom_right_outcode & BOTTOM_LEFT) == BOTTOM_LEFT
    {
        return distance_points(
            bounds1.x + bounds1.width,
            bounds1.y,
            bounds2.x,
            bounds2.y + bounds2.height,
        );
    } else if (top_left_outcode & TOP_RIGHT) == TOP_RIGHT
        && (bottom_right_outcode & TOP_RIGHT) == TOP_RIGHT
    {
        return distance_points(
            bounds1.x,
            bounds1.y + bounds1.height,
            bounds2.x + bounds2.width,
            bounds2.y,
        );
    } else if (top_left_outcode & BOTTOM_RIGHT) == BOTTOM_RIGHT
        && (bottom_right_outcode & BOTTOM_RIGHT) == BOTTOM_RIGHT
    {
        return distance_points(
            bounds1.x,
            bounds1.y,
            bounds2.x + bounds2.width,
            bounds2.y + bounds2.height,
        );
    } else if (top_left_outcode & OUT_LEFT) != 0 && (bottom_right_outcode & OUT_LEFT) != 0 {
        return bounds2.x - bounds1.x + bounds1.width;
    } else if (top_left_outcode & OUT_RIGHT) != 0 && (bottom_right_outcode & OUT_RIGHT) != 0 {
        return bounds1.x - bounds2.x - bounds2.width;
    } else if (top_left_outcode & OUT_TOP) != 0 && (bottom_right_outcode & OUT_TOP) != 0 {
        return bounds2.y - bounds1.y + bounds1.height;
    } else if (top_left_outcode & OUT_BOTTOM) != 0 && (bottom_right_outcode & OUT_BOTTOM) != 0 {
        return bounds1.y - bounds2.y - bounds2.height;
    }

    if bounds2.x <= bounds1.x + bounds1.width && bounds2.x + bounds2.width >= bounds1.x {
        if bounds2.y + bounds2.height == bounds1.y || bounds2.y == bounds1.y + bounds1.height {
            return 0.0;
        }
    } else if bounds2.y <= bounds1.y + bounds1.height
        && bounds2.y + bounds2.height >= bounds1.y
        && (bounds2.x + bounds2.width == bounds1.x || bounds2.x == bounds1.x + bounds1.width)
    {
        return 0.0;
    }

    -1.0
}

fn distance_points(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let delta_x = x2 - x1;
    let delta_y = y2 - y1;
    (delta_x * delta_x + delta_y * delta_y).sqrt()
}
