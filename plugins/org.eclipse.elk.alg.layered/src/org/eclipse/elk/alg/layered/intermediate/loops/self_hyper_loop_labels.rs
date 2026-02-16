use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::labels::ILabelManager;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::LLabelRef;
use crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfLoopPortRef;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
    Center,
    Left,
    Right,
    Top,
}

pub struct SelfHyperLoopLabels {
    l_labels: Vec<LLabelRef>,
    size: KVector,
    position: KVector,
    side: PortSide,
    alignment: Alignment,
    alignment_reference_sl_port: Option<SelfLoopPortRef>,
    label_label_spacing: f64,
    layout_direction_horizontal: bool,
}

impl SelfHyperLoopLabels {
    pub fn new() -> Self {
        SelfHyperLoopLabels {
            l_labels: Vec::new(),
            size: KVector::new(),
            position: KVector::new(),
            side: PortSide::Undefined,
            alignment: Alignment::Center,
            alignment_reference_sl_port: None,
            label_label_spacing: 0.0,
            layout_direction_horizontal: true,
        }
    }

    pub fn set_label_label_spacing(&mut self, spacing: f64) {
        self.label_label_spacing = spacing;
        self.recalculate_size();
    }

    pub fn set_layout_direction_horizontal(&mut self, horizontal: bool) {
        self.layout_direction_horizontal = horizontal;
        self.recalculate_size();
    }

    fn recalculate_size(&mut self) {
        let mut new_size = KVector::new();
        let labels: Vec<_> = self.l_labels.clone();
        for (i, l_label) in labels.iter().enumerate() {
            if let Ok(mut label_guard) = l_label.lock() {
                let width = label_guard.shape().size_ref().x;
                let height = label_guard.shape().size_ref().y;
                if self.layout_direction_horizontal {
                    new_size.x = new_size.x.max(width);
                    new_size.y += height;
                    if i > 0 {
                        new_size.y += self.label_label_spacing;
                    }
                } else {
                    new_size.x += width;
                    new_size.y = new_size.y.max(height);
                    if i > 0 {
                        new_size.x += self.label_label_spacing;
                    }
                }
            }
        }
        self.size = new_size;
    }

    pub fn add_l_labels(&mut self, labels: &[LLabelRef]) {
        for label in labels {
            if self.l_labels.iter().any(|existing| std::sync::Arc::ptr_eq(existing, label)) {
                continue;
            }
            self.l_labels.push(label.clone());
            self.update_size(label);
        }
    }

    pub fn l_labels(&self) -> &Vec<LLabelRef> {
        &self.l_labels
    }

    pub fn size(&self) -> &KVector {
        &self.size
    }

    pub fn size_mut(&mut self) -> &mut KVector {
        &mut self.size
    }

    pub fn position(&self) -> &KVector {
        &self.position
    }

    pub fn position_mut(&mut self) -> &mut KVector {
        &mut self.position
    }

    pub fn side(&self) -> PortSide {
        self.side
    }

    pub fn set_side(&mut self, side: PortSide) {
        self.side = side;
    }

    pub fn alignment(&self) -> Alignment {
        self.alignment
    }

    pub fn set_alignment(&mut self, alignment: Alignment) {
        self.alignment = alignment;
    }

    pub fn alignment_reference_sl_port(&self) -> Option<SelfLoopPortRef> {
        self.alignment_reference_sl_port.clone()
    }

    pub fn set_alignment_reference_sl_port(&mut self, sl_port: Option<SelfLoopPortRef>) {
        self.alignment_reference_sl_port = sl_port;
    }

    /// Applies placement to individual labels, matching Java's applyPlacement.
    /// `offset` is added to each label's position after computing it from self.position.
    pub fn apply_placement(&mut self, offset: KVector) {
        if self.layout_direction_horizontal {
            self.apply_placement_for_horizontal_layout(offset);
        } else {
            self.apply_placement_for_vertical_layout(offset);
        }
    }

    fn apply_placement_for_horizontal_layout(&mut self, offset: KVector) {
        let x = self.position.x;
        let mut y = self.position.y;

        for l_label in &self.l_labels {
            if let Ok(mut label_guard) = l_label.lock() {
                let label_width = label_guard.shape().size_ref().x;
                let label_height = label_guard.shape().size_ref().y;

                // X depends on alignment and/or side
                let label_x = if self.alignment == Alignment::Left || self.side == PortSide::East {
                    x
                } else if self.alignment == Alignment::Right || self.side == PortSide::West {
                    x + self.size.x - label_width
                } else {
                    // Center
                    x + (self.size.x - label_width) / 2.0
                };

                label_guard.shape().position().x = label_x + offset.x;
                label_guard.shape().position().y = y + offset.y;
                y += label_height + self.label_label_spacing;
            }
        }
    }

    fn apply_placement_for_vertical_layout(&mut self, offset: KVector) {
        let mut x = self.position.x;
        let y = self.position.y;

        for l_label in &self.l_labels {
            if let Ok(mut label_guard) = l_label.lock() {
                let label_width = label_guard.shape().size_ref().x;
                let label_height = label_guard.shape().size_ref().y;

                label_guard.shape().position().x = x + offset.x;

                // Always top-align, except for northern side
                let label_y = if self.side == PortSide::North {
                    y + self.size.y - label_height
                } else {
                    y
                };

                label_guard.shape().position().y = label_y + offset.y;
                x += label_width + self.label_label_spacing;
            }
        }
    }

    pub fn apply_label_management(
        &mut self,
        label_manager: &Arc<dyn ILabelManager>,
        target_width: f64,
        label_label_spacing: f64,
    ) {
        let mut new_size = KVector::new();

        for (index, l_label) in self.l_labels.iter().enumerate() {
            if let Ok(mut label_guard) = l_label.lock() {
                if let Some(updated_size) =
                    label_manager.manage_label_size(&*label_guard, target_width)
                {
                    label_guard.shape().size().x = updated_size.x;
                    label_guard.shape().size().y = updated_size.y;
                }

                new_size.x = new_size.x.max(label_guard.shape().size_ref().x);
                new_size.y += label_guard.shape().size_ref().y;
                if index > 0 {
                    new_size.y += label_label_spacing;
                }
            }
        }

        self.size = new_size;
    }

    fn update_size(&mut self, l_label: &LLabelRef) {
        if let Ok(mut label_guard) = l_label.lock() {
            let width = label_guard.shape().size_ref().x;
            let height = label_guard.shape().size_ref().y;
            if self.layout_direction_horizontal {
                self.size.x = self.size.x.max(width);
                self.size.y += height;
                if self.l_labels.len() > 1 {
                    self.size.y += self.label_label_spacing;
                }
            } else {
                self.size.x += width;
                self.size.y = self.size.y.max(height);
                if self.l_labels.len() > 1 {
                    self.size.x += self.label_label_spacing;
                }
            }
        }
    }
}

impl Default for SelfHyperLoopLabels {
    fn default() -> Self {
        Self::new()
    }
}
