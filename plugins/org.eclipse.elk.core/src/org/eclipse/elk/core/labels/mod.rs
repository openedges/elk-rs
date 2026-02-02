use std::any::Any;

use crate::org::eclipse::elk::core::math::KVector;

pub trait ILabelManager: Send + Sync {
    fn manage_label_size(&self, label: &dyn Any, target_width: f64) -> Option<KVector>;
}
