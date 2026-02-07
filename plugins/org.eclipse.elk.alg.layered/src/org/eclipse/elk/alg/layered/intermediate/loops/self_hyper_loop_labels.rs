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
        }
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

    pub fn apply_vertical_stack(&mut self, absolute_position: KVector, spacing: f64) {
        let mut next_y = absolute_position.y;
        for l_label in &self.l_labels {
            if let Ok(mut label_guard) = l_label.lock() {
                let width = label_guard.shape().size_ref().x;
                let x = match self.alignment {
                    Alignment::Left => absolute_position.x,
                    Alignment::Right => absolute_position.x + self.size.x - width,
                    Alignment::Center | Alignment::Top => {
                        absolute_position.x + (self.size.x - width) / 2.0
                    }
                };
                label_guard.shape().position().x = x;
                label_guard.shape().position().y = next_y;
                next_y += label_guard.shape().size_ref().y + spacing;
            }
        }
    }

    pub fn apply_horizontal_stack(&mut self, absolute_position: KVector, spacing: f64) {
        let mut next_x = absolute_position.x;
        for l_label in &self.l_labels {
            if let Ok(mut label_guard) = l_label.lock() {
                label_guard.shape().position().x = next_x;
                label_guard.shape().position().y = absolute_position.y;
                next_x += label_guard.shape().size_ref().x + spacing;
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
            self.size.x = self.size.x.max(width);
            self.size.y += height;
            if self.l_labels.len() > 1 {
                self.size.y += 2.0;
            }
        }
    }
}

impl Default for SelfHyperLoopLabels {
    fn default() -> Self {
        Self::new()
    }
}
