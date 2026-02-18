use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkPadding, ElkRectangle, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::LabelAdapter;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorizontalLabelAlignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalLabelAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Debug)]
pub struct Cell {
    padding: ElkPadding,
    cell_rectangle: ElkRectangle,
    contributes_to_minimum_width: bool,
    contributes_to_minimum_height: bool,
}

impl Cell {
    pub fn new() -> Self {
        Cell {
            padding: ElkPadding::new(),
            cell_rectangle: ElkRectangle::new(),
            contributes_to_minimum_width: false,
            contributes_to_minimum_height: false,
        }
    }

    pub fn padding(&mut self) -> &mut ElkPadding {
        &mut self.padding
    }

    pub fn padding_ref(&self) -> &ElkPadding {
        &self.padding
    }

    pub fn cell_rectangle(&mut self) -> &mut ElkRectangle {
        &mut self.cell_rectangle
    }

    pub fn cell_rectangle_ref(&self) -> &ElkRectangle {
        &self.cell_rectangle
    }

    pub fn contributes_to_minimum_width(&self) -> bool {
        self.contributes_to_minimum_width
    }

    pub fn set_contributes_to_minimum_width(&mut self, contributes: bool) {
        self.contributes_to_minimum_width = contributes;
    }

    pub fn contributes_to_minimum_height(&self) -> bool {
        self.contributes_to_minimum_height
    }

    pub fn set_contributes_to_minimum_height(&mut self, contributes: bool) {
        self.contributes_to_minimum_height = contributes;
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct LabelCell<A, L>
where
    A: LabelAdapter<L> + Clone,
{
    cell: Cell,
    horizontal_layout_mode: bool,
    horizontal_alignment: HorizontalLabelAlignment,
    vertical_alignment: VerticalLabelAlignment,
    gap: f64,
    labels: Vec<A>,
    minimum_content_area_size: KVector,
    _marker: std::marker::PhantomData<L>,
}

impl<A, L> LabelCell<A, L>
where
    A: LabelAdapter<L> + Clone,
{
    pub fn new(gap: f64) -> Self {
        LabelCell {
            cell: Cell::new(),
            horizontal_layout_mode: true,
            horizontal_alignment: HorizontalLabelAlignment::Center,
            vertical_alignment: VerticalLabelAlignment::Center,
            gap,
            labels: Vec::with_capacity(2),
            minimum_content_area_size: KVector::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn new_with_layout_mode(gap: f64, horizontal_layout_mode: bool) -> Self {
        LabelCell {
            horizontal_layout_mode,
            ..Self::new(gap)
        }
    }

    pub fn cell_rectangle(&mut self) -> &mut ElkRectangle {
        self.cell.cell_rectangle()
    }

    pub fn cell_rectangle_ref(&self) -> &ElkRectangle {
        self.cell.cell_rectangle_ref()
    }

    pub fn padding(&mut self) -> &mut ElkPadding {
        self.cell.padding()
    }

    pub fn padding_ref(&self) -> &ElkPadding {
        self.cell.padding_ref()
    }

    pub fn horizontal_alignment(&self) -> HorizontalLabelAlignment {
        self.horizontal_alignment
    }

    pub fn set_horizontal_alignment(&mut self, alignment: HorizontalLabelAlignment) {
        self.horizontal_alignment = alignment;
    }

    pub fn vertical_alignment(&self) -> VerticalLabelAlignment {
        self.vertical_alignment
    }

    pub fn set_vertical_alignment(&mut self, alignment: VerticalLabelAlignment) {
        self.vertical_alignment = alignment;
    }

    pub fn labels(&self) -> &Vec<A> {
        &self.labels
    }

    pub fn has_labels(&self) -> bool {
        !self.labels.is_empty()
    }

    pub fn add_label(&mut self, label: A) {
        let label_size = label.get_size();
        self.labels.push(label);

        if self.horizontal_layout_mode {
            self.minimum_content_area_size.x = self.minimum_content_area_size.x.max(label_size.x);
            self.minimum_content_area_size.y += label_size.y;
            if self.labels.len() > 1 {
                self.minimum_content_area_size.y += self.gap;
            }
        } else {
            self.minimum_content_area_size.x += label_size.x;
            self.minimum_content_area_size.y = self.minimum_content_area_size.y.max(label_size.y);
            if self.labels.len() > 1 {
                self.minimum_content_area_size.x += self.gap;
            }
        }
    }

    pub fn minimum_width(&self) -> f64 {
        let padding = self.padding_ref();
        self.minimum_content_area_size.x + padding.left + padding.right
    }

    pub fn minimum_height(&self) -> f64 {
        let padding = self.padding_ref();
        self.minimum_content_area_size.y + padding.top + padding.bottom
    }

    pub fn apply_label_layout(&mut self) {
        if self.horizontal_layout_mode {
            self.apply_horizontal_mode_label_layout();
        } else {
            self.apply_vertical_mode_label_layout();
        }
    }

    fn apply_horizontal_mode_label_layout(&mut self) {
        let cell_rect = *self.cell_rectangle_ref();
        let padding = self.padding_ref();

        let mut y_pos = cell_rect.y;
        if self.vertical_alignment == VerticalLabelAlignment::Center {
            y_pos += (cell_rect.height - self.minimum_content_area_size.y) / 2.0;
        } else if self.vertical_alignment == VerticalLabelAlignment::Bottom {
            y_pos += cell_rect.height - self.minimum_content_area_size.y;
        }

        for label in &self.labels {
            let label_size = label.get_size();
            let mut label_pos = KVector::new();

            label_pos.y = y_pos;
            y_pos += label_size.y + self.gap;

            match self.horizontal_alignment {
                HorizontalLabelAlignment::Left => {
                    label_pos.x = cell_rect.x + padding.left;
                }
                HorizontalLabelAlignment::Center => {
                    label_pos.x =
                        cell_rect.x + padding.left + (cell_rect.width - label_size.x) / 2.0;
                }
                HorizontalLabelAlignment::Right => {
                    label_pos.x = cell_rect.x + cell_rect.width - padding.right - label_size.x;
                }
            }

            label.set_position(label_pos);
        }
    }

    fn apply_vertical_mode_label_layout(&mut self) {
        let cell_rect = *self.cell_rectangle_ref();
        let padding = self.padding_ref();

        let mut x_pos = cell_rect.x;
        if self.horizontal_alignment == HorizontalLabelAlignment::Center {
            x_pos += (cell_rect.width - self.minimum_content_area_size.x) / 2.0;
        } else if self.horizontal_alignment == HorizontalLabelAlignment::Right {
            x_pos += cell_rect.width - self.minimum_content_area_size.x;
        }

        for label in &self.labels {
            let label_size = label.get_size();
            let mut label_pos = KVector::new();

            label_pos.x = x_pos;
            x_pos += label_size.x + self.gap;

            match self.vertical_alignment {
                VerticalLabelAlignment::Top => {
                    label_pos.y = cell_rect.y + padding.top;
                }
                VerticalLabelAlignment::Center => {
                    label_pos.y =
                        cell_rect.y + padding.top + (cell_rect.height - label_size.y) / 2.0;
                }
                VerticalLabelAlignment::Bottom => {
                    label_pos.y = cell_rect.y + cell_rect.height - padding.bottom - label_size.y;
                }
            }

            label.set_position(label_pos);
        }
    }
}
