use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkPadding, ElkRectangle, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{GraphElementAdapter, LabelAdapter};

// =====================================================================================
// Enums: HorizontalLabelAlignment, VerticalLabelAlignment (unchanged)
// =====================================================================================

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

// =====================================================================================
// Cell struct (unchanged)
// =====================================================================================

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

// =====================================================================================
// LabelCell<A, L> (unchanged)
// =====================================================================================

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

// =====================================================================================
// NEW: ContainerArea enum
// =====================================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerArea {
    Begin,
    Center,
    End,
}

impl ContainerArea {
    pub const VALUES: [ContainerArea; 3] = [
        ContainerArea::Begin,
        ContainerArea::Center,
        ContainerArea::End,
    ];

    pub fn values() -> &'static [ContainerArea; 3] {
        &Self::VALUES
    }

    pub fn index(self) -> usize {
        match self {
            ContainerArea::Begin => 0,
            ContainerArea::Center => 1,
            ContainerArea::End => 2,
        }
    }
}

// =====================================================================================
// NEW: Strip enum
// =====================================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strip {
    Vertical,
    Horizontal,
}

// =====================================================================================
// NEW: AtomicCell struct
// =====================================================================================

#[derive(Clone, Debug)]
pub struct AtomicCell {
    cell: Cell,
    minimum_content_area_size: KVector,
}

impl AtomicCell {
    pub fn new() -> Self {
        AtomicCell {
            cell: Cell::new(),
            minimum_content_area_size: KVector::new(),
        }
    }

    pub fn minimum_content_area_size(&self) -> &KVector {
        &self.minimum_content_area_size
    }

    pub fn minimum_content_area_size_mut(&mut self) -> &mut KVector {
        &mut self.minimum_content_area_size
    }

    pub fn set_minimum_content_area_size(&mut self, size: KVector, includes_padding: bool) {
        if includes_padding {
            let padding = &self.cell.padding;
            self.minimum_content_area_size.x = size.x - padding.left - padding.right;
            self.minimum_content_area_size.y = size.y - padding.top - padding.bottom;
        } else {
            self.minimum_content_area_size = size;
        }
    }

    pub fn minimum_width(&self) -> f64 {
        let padding = self.cell.padding_ref();
        self.minimum_content_area_size.x + padding.left + padding.right
    }

    pub fn minimum_height(&self) -> f64 {
        let padding = self.cell.padding_ref();
        self.minimum_content_area_size.y + padding.top + padding.bottom
    }

    // Cell delegate methods
    pub fn padding(&mut self) -> &mut ElkPadding {
        self.cell.padding()
    }

    pub fn padding_ref(&self) -> &ElkPadding {
        self.cell.padding_ref()
    }

    pub fn cell_rectangle(&mut self) -> &mut ElkRectangle {
        self.cell.cell_rectangle()
    }

    pub fn cell_rectangle_ref(&self) -> &ElkRectangle {
        self.cell.cell_rectangle_ref()
    }

    pub fn contributes_to_minimum_width(&self) -> bool {
        self.cell.contributes_to_minimum_width()
    }

    pub fn set_contributes_to_minimum_width(&mut self, contributes: bool) {
        self.cell.set_contributes_to_minimum_width(contributes);
    }

    pub fn contributes_to_minimum_height(&self) -> bool {
        self.cell.contributes_to_minimum_height()
    }

    pub fn set_contributes_to_minimum_height(&mut self, contributes: bool) {
        self.cell.set_contributes_to_minimum_height(contributes);
    }
}

impl Default for AtomicCell {
    fn default() -> Self {
        Self::new()
    }
}

// =====================================================================================
// NEW: DynLabelOps trait and DynLabel type-erased wrapper
// =====================================================================================

pub trait DynLabelOps {
    fn dyn_get_size(&self) -> KVector;
    fn dyn_set_position(&self, pos: KVector);
}

// Note: Blanket impls cannot be used because T is unconstrained. Instead, callers
// construct DynLabel via DynLabel::new() which wraps the concrete adapter.

pub struct DynLabel {
    inner: Box<dyn DynLabelOps>,
}

/// Wrapper that adapts a GraphElementAdapter into DynLabelOps.
struct DynLabelWrapper<T, L: GraphElementAdapter<T>> {
    label: L,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, L: GraphElementAdapter<T>> DynLabelOps for DynLabelWrapper<T, L> {
    fn dyn_get_size(&self) -> KVector {
        self.label.get_size()
    }

    fn dyn_set_position(&self, pos: KVector) {
        self.label.set_position(pos);
    }
}

impl DynLabel {
    pub fn new<T: 'static, L: GraphElementAdapter<T> + 'static>(label: L) -> Self {
        DynLabel {
            inner: Box::new(DynLabelWrapper {
                label,
                _phantom: std::marker::PhantomData,
            }),
        }
    }

    pub fn get_size(&self) -> KVector {
        self.inner.dyn_get_size()
    }

    pub fn set_position(&self, pos: KVector) {
        self.inner.dyn_set_position(pos);
    }
}

impl std::fmt::Debug for DynLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynLabel").finish()
    }
}

// =====================================================================================
// NEW: DynLabelCell struct
// =====================================================================================

#[derive(Debug)]
pub struct DynLabelCell {
    cell: Cell,
    horizontal_layout_mode: bool,
    horizontal_alignment: HorizontalLabelAlignment,
    vertical_alignment: VerticalLabelAlignment,
    gap: f64,
    labels: Vec<DynLabel>,
    minimum_content_area_size: KVector,
}

impl DynLabelCell {
    pub fn new(gap: f64) -> Self {
        DynLabelCell {
            cell: Cell::new(),
            horizontal_layout_mode: true,
            horizontal_alignment: HorizontalLabelAlignment::Center,
            vertical_alignment: VerticalLabelAlignment::Center,
            gap,
            labels: Vec::with_capacity(2),
            minimum_content_area_size: KVector::new(),
        }
    }

    pub fn new_with_layout_mode(gap: f64, horizontal_layout_mode: bool) -> Self {
        DynLabelCell {
            horizontal_layout_mode,
            ..Self::new(gap)
        }
    }

    pub fn add_label(&mut self, label: DynLabel) {
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

    pub fn has_labels(&self) -> bool {
        !self.labels.is_empty()
    }

    pub fn labels(&self) -> &Vec<DynLabel> {
        &self.labels
    }

    pub fn minimum_width(&self) -> f64 {
        let padding = self.cell.padding_ref();
        self.minimum_content_area_size.x + padding.left + padding.right
    }

    pub fn minimum_height(&self) -> f64 {
        let padding = self.cell.padding_ref();
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
        let cell_rect = *self.cell.cell_rectangle_ref();
        let padding = self.cell.padding_ref().clone();

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
        let cell_rect = *self.cell.cell_rectangle_ref();
        let padding = self.cell.padding_ref().clone();

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

    pub fn set_horizontal_alignment(&mut self, alignment: HorizontalLabelAlignment) {
        self.horizontal_alignment = alignment;
    }

    pub fn set_vertical_alignment(&mut self, alignment: VerticalLabelAlignment) {
        self.vertical_alignment = alignment;
    }

    pub fn horizontal_alignment(&self) -> HorizontalLabelAlignment {
        self.horizontal_alignment
    }

    pub fn vertical_alignment(&self) -> VerticalLabelAlignment {
        self.vertical_alignment
    }

    // Cell delegate methods
    pub fn padding(&mut self) -> &mut ElkPadding {
        self.cell.padding()
    }

    pub fn padding_ref(&self) -> &ElkPadding {
        self.cell.padding_ref()
    }

    pub fn cell_rectangle(&mut self) -> &mut ElkRectangle {
        self.cell.cell_rectangle()
    }

    pub fn cell_rectangle_ref(&self) -> &ElkRectangle {
        self.cell.cell_rectangle_ref()
    }

    pub fn contributes_to_minimum_width(&self) -> bool {
        self.cell.contributes_to_minimum_width()
    }

    pub fn set_contributes_to_minimum_width(&mut self, contributes: bool) {
        self.cell.set_contributes_to_minimum_width(contributes);
    }

    pub fn contributes_to_minimum_height(&self) -> bool {
        self.cell.contributes_to_minimum_height()
    }

    pub fn set_contributes_to_minimum_height(&mut self, contributes: bool) {
        self.cell.set_contributes_to_minimum_height(contributes);
    }
}

// =====================================================================================
// NEW: CellChild enum
// =====================================================================================

/// A type-erased cell that can hold any cell variant, matching Java's Cell hierarchy
/// where ContainerCell children can be AtomicCell, LabelCell, StripContainerCell, or
/// GridContainerCell.
#[derive(Debug)]
pub enum CellChild {
    None,
    Atomic(AtomicCell),
    Label(DynLabelCell),
    Strip(Box<StripContainerCell>),
    Grid(Box<GridContainerCell>),
}

impl CellChild {
    pub fn is_none(&self) -> bool {
        matches!(self, CellChild::None)
    }

    pub fn is_atomic(&self) -> bool {
        matches!(self, CellChild::Atomic(_))
    }

    pub fn as_atomic(&self) -> Option<&AtomicCell> {
        match self {
            CellChild::Atomic(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_atomic_mut(&mut self) -> Option<&mut AtomicCell> {
        match self {
            CellChild::Atomic(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_label(&self) -> Option<&DynLabelCell> {
        match self {
            CellChild::Label(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_label_mut(&mut self) -> Option<&mut DynLabelCell> {
        match self {
            CellChild::Label(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_strip(&self) -> Option<&StripContainerCell> {
        match self {
            CellChild::Strip(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_strip_mut(&mut self) -> Option<&mut StripContainerCell> {
        match self {
            CellChild::Strip(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_grid(&self) -> Option<&GridContainerCell> {
        match self {
            CellChild::Grid(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_grid_mut(&mut self) -> Option<&mut GridContainerCell> {
        match self {
            CellChild::Grid(c) => Some(c),
            _ => None,
        }
    }

    pub fn minimum_width(&self) -> f64 {
        match self {
            CellChild::None => 0.0,
            CellChild::Atomic(c) => c.minimum_width(),
            CellChild::Label(c) => c.minimum_width(),
            CellChild::Strip(c) => c.minimum_width(),
            CellChild::Grid(c) => c.minimum_width(),
        }
    }

    pub fn minimum_height(&self) -> f64 {
        match self {
            CellChild::None => 0.0,
            CellChild::Atomic(c) => c.minimum_height(),
            CellChild::Label(c) => c.minimum_height(),
            CellChild::Strip(c) => c.minimum_height(),
            CellChild::Grid(c) => c.minimum_height(),
        }
    }

    pub fn is_contributing_to_minimum_width(&self) -> bool {
        match self {
            CellChild::None => false,
            CellChild::Atomic(c) => c.contributes_to_minimum_width(),
            CellChild::Label(c) => c.contributes_to_minimum_width(),
            CellChild::Strip(c) => c.contributes_to_minimum_width(),
            CellChild::Grid(c) => c.contributes_to_minimum_width(),
        }
    }

    pub fn is_contributing_to_minimum_height(&self) -> bool {
        match self {
            CellChild::None => false,
            CellChild::Atomic(c) => c.contributes_to_minimum_height(),
            CellChild::Label(c) => c.contributes_to_minimum_height(),
            CellChild::Strip(c) => c.contributes_to_minimum_height(),
            CellChild::Grid(c) => c.contributes_to_minimum_height(),
        }
    }

    pub fn set_contributes_to_minimum_width(&mut self, val: bool) {
        match self {
            CellChild::None => {}
            CellChild::Atomic(c) => c.set_contributes_to_minimum_width(val),
            CellChild::Label(c) => c.set_contributes_to_minimum_width(val),
            CellChild::Strip(c) => c.set_contributes_to_minimum_width(val),
            CellChild::Grid(c) => c.set_contributes_to_minimum_width(val),
        }
    }

    pub fn set_contributes_to_minimum_height(&mut self, val: bool) {
        match self {
            CellChild::None => {}
            CellChild::Atomic(c) => c.set_contributes_to_minimum_height(val),
            CellChild::Label(c) => c.set_contributes_to_minimum_height(val),
            CellChild::Strip(c) => c.set_contributes_to_minimum_height(val),
            CellChild::Grid(c) => c.set_contributes_to_minimum_height(val),
        }
    }

    pub fn cell_rectangle(&self) -> &ElkRectangle {
        match self {
            CellChild::None => &ZERO_RECTANGLE,
            CellChild::Atomic(c) => c.cell_rectangle_ref(),
            CellChild::Label(c) => c.cell_rectangle_ref(),
            CellChild::Strip(c) => c.cell_rectangle_ref(),
            CellChild::Grid(c) => c.cell_rectangle_ref(),
        }
    }

    pub fn cell_rectangle_mut(&mut self) -> &mut ElkRectangle {
        match self {
            CellChild::None => panic!("Cannot get mutable cell_rectangle on CellChild::None"),
            CellChild::Atomic(c) => c.cell_rectangle(),
            CellChild::Label(c) => c.cell_rectangle(),
            CellChild::Strip(c) => c.cell_rectangle(),
            CellChild::Grid(c) => c.cell_rectangle(),
        }
    }

    pub fn padding(&self) -> &ElkPadding {
        match self {
            CellChild::None => &ZERO_PADDING,
            CellChild::Atomic(c) => c.padding_ref(),
            CellChild::Label(c) => c.padding_ref(),
            CellChild::Strip(c) => c.padding_ref(),
            CellChild::Grid(c) => c.padding_ref(),
        }
    }

    pub fn padding_mut(&mut self) -> &mut ElkPadding {
        match self {
            CellChild::None => panic!("Cannot get mutable padding on CellChild::None"),
            CellChild::Atomic(c) => c.padding(),
            CellChild::Label(c) => c.padding(),
            CellChild::Strip(c) => c.padding(),
            CellChild::Grid(c) => c.padding(),
        }
    }

    pub fn layout_children_horizontally(&mut self) {
        match self {
            CellChild::Strip(c) => c.layout_children_horizontally(),
            CellChild::Grid(c) => c.layout_children_horizontally(),
            _ => {}
        }
    }

    pub fn layout_children_vertically(&mut self) {
        match self {
            CellChild::Strip(c) => c.layout_children_vertically(),
            CellChild::Grid(c) => c.layout_children_vertically(),
            _ => {}
        }
    }
}

/// Static zero rectangle for CellChild::None references.
static ZERO_RECTANGLE: ElkRectangle = ElkRectangle {
    x: 0.0,
    y: 0.0,
    width: 0.0,
    height: 0.0,
};

/// Static zero padding for CellChild::None references.
static ZERO_PADDING: ElkPadding = ElkPadding {
    top: 0.0,
    right: 0.0,
    bottom: 0.0,
    left: 0.0,
};

// =====================================================================================
// Helper functions matching Java's ContainerCell.minWidthOfCell / minHeightOfCell
// =====================================================================================

/// Returns the minimum width of the given CellChild, matching Java's
/// ContainerCell.minWidthOfCell logic.
fn min_width_of_cell(cell: &CellChild, respect_contribution_flag: bool) -> f64 {
    // If there's no cell, there's no minimum width
    if cell.is_none() {
        return 0.0;
    }

    // If the cell doesn't have its contribution flag activated, there's no minimum width
    if respect_contribution_flag && !cell.is_contributing_to_minimum_width() {
        return 0.0;
    }

    // If the cell is an atomic cell with a content area of no width, there's no minimum width
    if let CellChild::Atomic(atomic) = cell {
        if atomic.minimum_content_area_size().x == 0.0 {
            return 0.0;
        }
    }

    cell.minimum_width()
}

/// Returns the minimum height of the given CellChild, matching Java's
/// ContainerCell.minHeightOfCell logic.
fn min_height_of_cell(cell: &CellChild, respect_contribution_flag: bool) -> f64 {
    // If there's no cell, there's no minimum height
    if cell.is_none() {
        return 0.0;
    }

    // If the cell doesn't have its contribution flag activated, there's no minimum height
    if respect_contribution_flag && !cell.is_contributing_to_minimum_height() {
        return 0.0;
    }

    // If the cell is an atomic cell with a content area of no height, there's no minimum height
    if let CellChild::Atomic(atomic) = cell {
        if atomic.minimum_content_area_size().y == 0.0 {
            return 0.0;
        }
    }

    cell.minimum_height()
}

/// Applies horizontal layout information to a CellChild (does nothing for None).
fn apply_horizontal_layout(cell: &mut CellChild, x: f64, width: f64) {
    if !cell.is_none() {
        let rect = cell.cell_rectangle_mut();
        rect.x = x;
        rect.width = width;
    }
}

/// Applies vertical layout information to a CellChild (does nothing for None).
fn apply_vertical_layout(cell: &mut CellChild, y: f64, height: f64) {
    if !cell.is_none() {
        let rect = cell.cell_rectangle_mut();
        rect.y = y;
        rect.height = height;
    }
}

// =====================================================================================
// NEW: StripContainerCell struct
// =====================================================================================

/// A container cell that lays its children out along a strip. The strip can be
/// horizontal (children are columns) or vertical (children are rows). Faithfully
/// ports Java's StripContainerCell.
#[derive(Debug)]
pub struct StripContainerCell {
    cell: Cell,
    container_mode: Strip,
    symmetrical: bool,
    gap: f64,
    cells: [CellChild; 3],
}

impl StripContainerCell {
    pub fn new(mode: Strip, symmetrical: bool, gap: f64) -> Self {
        StripContainerCell {
            cell: Cell::new(),
            container_mode: mode,
            symmetrical,
            gap,
            cells: [CellChild::None, CellChild::None, CellChild::None],
        }
    }

    pub fn container_mode(&self) -> Strip {
        self.container_mode
    }

    pub fn gap(&self) -> f64 {
        self.gap
    }

    pub fn get_cell(&self, area: ContainerArea) -> &CellChild {
        &self.cells[area.index()]
    }

    pub fn get_cell_mut(&mut self, area: ContainerArea) -> &mut CellChild {
        &mut self.cells[area.index()]
    }

    pub fn set_cell(&mut self, area: ContainerArea, cell: CellChild) {
        self.cells[area.index()] = cell;
    }

    /// Returns the minimum width of the strip, matching Java's StripContainerCell.getMinimumWidth.
    pub fn minimum_width(&self) -> f64 {
        let mut width: f64 = 0.0;

        if self.container_mode == Strip::Vertical {
            // Take the maximum of the child cells
            for cell in &self.cells {
                if !cell.is_none() && cell.is_contributing_to_minimum_width() {
                    width = width.max(cell.minimum_width());
                }
            }
        } else {
            // Minimum widths of the different cells
            let cell_widths = self.min_cell_widths(true);

            // Keep track of how many cells we have
            let mut active_cells = 0;
            for cell_width in &cell_widths {
                if *cell_width > 0.0 {
                    width += cell_width;
                    active_cells += 1;
                }
            }

            // If there is more than a single cell, add necessary gaps
            if active_cells > 1 {
                width += self.gap * (active_cells - 1) as f64;
            }
        }

        // If we don't have cells, we don't have width
        if width > 0.0 {
            width + self.cell.padding_ref().left + self.cell.padding_ref().right
        } else {
            0.0
        }
    }

    /// Returns the minimum height of the strip, matching Java's StripContainerCell.getMinimumHeight.
    pub fn minimum_height(&self) -> f64 {
        let mut height = 0.0;

        if self.container_mode == Strip::Vertical {
            // Minimum heights of the different cells
            let cell_heights = self.min_cell_heights(true);

            // Keep track of how many cells we have
            let mut active_cells = 0;
            for cell_height in &cell_heights {
                if *cell_height > 0.0 {
                    height += cell_height;
                    active_cells += 1;
                }
            }

            // If there is more than a single cell, add necessary gaps
            if active_cells > 1 {
                height += self.gap * (active_cells - 1) as f64;
            }
        } else {
            // Take the maximum of the child cells
            for cell in &self.cells {
                if !cell.is_none() && cell.is_contributing_to_minimum_height() {
                    height = height.max(cell.minimum_height());
                }
            }
        }

        // If we don't have cells, we don't have height
        if height > 0.0 {
            height + self.cell.padding_ref().top + self.cell.padding_ref().bottom
        } else {
            0.0
        }
    }

    /// Compute x coordinates and widths of children, matching Java's
    /// StripContainerCell.layoutChildrenHorizontally.
    pub fn layout_children_horizontally(&mut self) {
        let cell_rectangle = *self.cell.cell_rectangle_ref();
        let cell_padding = self.cell.padding_ref().clone();

        if self.container_mode == Strip::Vertical {
            // Each child cell begins at our left border (plus padding) and is as large as our
            // content area
            let x_pos = cell_rectangle.x + cell_padding.left;
            let width = cell_rectangle.width - cell_padding.left - cell_padding.right;

            for child_cell in &mut self.cells {
                apply_horizontal_layout(child_cell, x_pos, width);
            }
        } else {
            let mut cell_widths = self.min_cell_widths(false);

            // Left cell is left-aligned with our content area, right cell is right-aligned
            apply_horizontal_layout(
                &mut self.cells[0],
                cell_rectangle.x + cell_padding.left,
                cell_widths[0],
            );
            apply_horizontal_layout(
                &mut self.cells[2],
                cell_rectangle.x + cell_rectangle.width - cell_padding.right - cell_widths[2],
                cell_widths[2],
            );

            // Size of the content area and size of the available space in the content area
            let mut free_content_area_width =
                cell_rectangle.width - cell_padding.left - cell_padding.right;

            if cell_widths[0] > 0.0 {
                free_content_area_width -= cell_widths[0] + self.gap;
                // We add the gap here because that will spare us to check if cell_widths[0] is
                // zero later on
                cell_widths[0] += self.gap;
            }

            if cell_widths[2] > 0.0 {
                free_content_area_width -= cell_widths[2] + self.gap;
            }

            // If the available space is larger than the current size of the center cell, enlarge
            // that thing
            cell_widths[1] = cell_widths[1].max(free_content_area_width);

            // Place the center cell, possibly enlarging it in the process
            apply_horizontal_layout(
                &mut self.cells[1],
                cell_rectangle.x + cell_padding.left + cell_widths[0]
                    - (cell_widths[1] - free_content_area_width) / 2.0,
                cell_widths[1],
            );
        }

        // Layout container cells recursively
        for child_cell in &mut self.cells {
            child_cell.layout_children_horizontally();
        }
    }

    /// Compute y coordinates and heights of children, matching Java's
    /// StripContainerCell.layoutChildrenVertically.
    pub fn layout_children_vertically(&mut self) {
        let cell_rectangle = *self.cell.cell_rectangle_ref();
        let cell_padding = self.cell.padding_ref().clone();

        if self.container_mode == Strip::Vertical {
            let mut cell_heights = self.min_cell_heights(false);

            // Top cell is top-aligned with our content area, bottom cell is bottom-aligned
            apply_vertical_layout(
                &mut self.cells[0],
                cell_rectangle.y + cell_padding.top,
                cell_heights[0],
            );
            apply_vertical_layout(
                &mut self.cells[2],
                cell_rectangle.y + cell_rectangle.height - cell_padding.bottom - cell_heights[2],
                cell_heights[2],
            );

            // Size of the content area and size of the available space in the content area
            let mut content_area_free_height =
                cell_rectangle.height - cell_padding.top - cell_padding.bottom;

            if cell_heights[0] > 0.0 {
                // We add the gap here because that will spare us to check if cell_heights[0] is
                // zero later on
                cell_heights[0] += self.gap;
                content_area_free_height -= cell_heights[0];
            }

            if cell_heights[2] > 0.0 {
                content_area_free_height -= cell_heights[2] + self.gap;
            }

            // If the available space is larger than the current size of the center cell, enlarge
            // that thing
            cell_heights[1] = cell_heights[1].max(content_area_free_height);

            // Place the center cell, possibly enlarging it in the process
            apply_vertical_layout(
                &mut self.cells[1],
                cell_rectangle.y + cell_padding.top + cell_heights[0]
                    - (cell_heights[1] - content_area_free_height) / 2.0,
                cell_heights[1],
            );
        } else {
            // Each child cell begins at our top border (plus padding) and is as large as our
            // content area
            let y_pos = cell_rectangle.y + cell_padding.top;
            let height = cell_rectangle.height - cell_padding.top - cell_padding.bottom;

            for child_cell in &mut self.cells {
                apply_vertical_layout(child_cell, y_pos, height);
            }
        }

        // Layout container cells recursively
        for child_cell in &mut self.cells {
            child_cell.layout_children_vertically();
        }
    }

    /// Returns an array containing the width of each cell, with symmetry applied if activated.
    fn min_cell_widths(&self, respect_contribution_flag: bool) -> [f64; 3] {
        let mut cell_widths = [
            min_width_of_cell(&self.cells[0], respect_contribution_flag),
            min_width_of_cell(&self.cells[1], respect_contribution_flag),
            min_width_of_cell(&self.cells[2], respect_contribution_flag),
        ];

        // If we are to be symmetrical, the outer cells need to be the same size
        if self.symmetrical {
            cell_widths[0] = cell_widths[0].max(cell_widths[2]);
            cell_widths[2] = cell_widths[0];
        }

        cell_widths
    }

    /// Returns an array containing the height of each cell, with symmetry applied if activated.
    fn min_cell_heights(&self, respect_contribution_flag: bool) -> [f64; 3] {
        let mut cell_heights = [
            min_height_of_cell(&self.cells[0], respect_contribution_flag),
            min_height_of_cell(&self.cells[1], respect_contribution_flag),
            min_height_of_cell(&self.cells[2], respect_contribution_flag),
        ];

        // If we are to be symmetrical, the outer cells need to be the same size
        if self.symmetrical {
            cell_heights[0] = cell_heights[0].max(cell_heights[2]);
            cell_heights[2] = cell_heights[0];
        }

        cell_heights
    }

    // Cell delegate methods
    pub fn padding(&mut self) -> &mut ElkPadding {
        self.cell.padding()
    }

    pub fn padding_ref(&self) -> &ElkPadding {
        self.cell.padding_ref()
    }

    pub fn cell_rectangle(&mut self) -> &mut ElkRectangle {
        self.cell.cell_rectangle()
    }

    pub fn cell_rectangle_ref(&self) -> &ElkRectangle {
        self.cell.cell_rectangle_ref()
    }

    pub fn contributes_to_minimum_width(&self) -> bool {
        self.cell.contributes_to_minimum_width()
    }

    pub fn set_contributes_to_minimum_width(&mut self, contributes: bool) {
        self.cell.set_contributes_to_minimum_width(contributes);
    }

    pub fn contributes_to_minimum_height(&self) -> bool {
        self.cell.contributes_to_minimum_height()
    }

    pub fn set_contributes_to_minimum_height(&mut self, contributes: bool) {
        self.cell.set_contributes_to_minimum_height(contributes);
    }
}

// =====================================================================================
// NEW: GridContainerCell struct
// =====================================================================================

/// A container that lays out its child cells in a 3x3 grid. Faithfully ports Java's
/// GridContainerCell. Can operate in tabular mode (column widths locked across rows)
/// or non-tabular mode (each row has independent column widths).
#[derive(Debug)]
pub struct GridContainerCell {
    cell: Cell,
    gap: f64,
    tabular: bool,
    symmetrical: bool,
    cells: [[CellChild; 3]; 3],
    center_cell_minimum_size: Option<KVector>,
    only_center_cell_contributes: bool,
    center_cell_rect: ElkRectangle,
}

impl GridContainerCell {
    pub fn new(tabular: bool, symmetrical: bool, gap: f64) -> Self {
        GridContainerCell {
            cell: Cell::new(),
            gap,
            tabular,
            symmetrical,
            cells: [
                [CellChild::None, CellChild::None, CellChild::None],
                [CellChild::None, CellChild::None, CellChild::None],
                [CellChild::None, CellChild::None, CellChild::None],
            ],
            center_cell_minimum_size: None,
            only_center_cell_contributes: false,
            center_cell_rect: ElkRectangle::new(),
        }
    }

    pub fn gap(&self) -> f64 {
        self.gap
    }

    pub fn get_cell(&self, row: ContainerArea, col: ContainerArea) -> &CellChild {
        &self.cells[row.index()][col.index()]
    }

    pub fn get_cell_mut(&mut self, row: ContainerArea, col: ContainerArea) -> &mut CellChild {
        &mut self.cells[row.index()][col.index()]
    }

    pub fn set_cell(&mut self, row: ContainerArea, col: ContainerArea, cell: CellChild) {
        self.cells[row.index()][col.index()] = cell;
    }

    pub fn set_center_cell_minimum_size(&mut self, size: KVector) {
        self.center_cell_minimum_size = Some(size);
    }

    pub fn set_only_center_cell_contributes(&mut self, val: bool) {
        self.only_center_cell_contributes = val;
    }

    /// Returns a copy of the center cell rectangle, matching Java's getCenterCellRectangle.
    pub fn center_cell_rectangle(&self) -> ElkRectangle {
        self.center_cell_rect
    }

    /// Returns the minimum width, matching Java's GridContainerCell.getMinimumWidth.
    pub fn minimum_width(&self) -> f64 {
        let mut width = 0.0;

        if self.only_center_cell_contributes {
            // Check if an explicit minimum size for the center cell was set
            if let Some(ref min_size) = self.center_cell_minimum_size {
                width = min_size.x;
            } else if !self.cells[1][1].is_none() {
                width = self.cells[1][1].minimum_width();
            }
        } else if self.tabular {
            // Use aggregated widths
            width = self.sum_with_gaps(&self.min_column_widths(None, true));
        } else {
            // Use maximum width over each row
            for area in ContainerArea::values() {
                width = width.max(self.sum_with_gaps(&self.min_column_widths(Some(*area), true)));
            }
        }

        // If we don't have cells, we don't have width
        if width > 0.0 {
            width + self.cell.padding_ref().left + self.cell.padding_ref().right
        } else {
            0.0
        }
    }

    /// Returns the minimum height, matching Java's GridContainerCell.getMinimumHeight.
    pub fn minimum_height(&self) -> f64 {
        let mut height = 0.0;

        if self.only_center_cell_contributes {
            // Check if an explicit minimum size for the center cell was set
            if let Some(ref min_size) = self.center_cell_minimum_size {
                height = min_size.y;
            } else if !self.cells[1][1].is_none() {
                height = self.cells[1][1].minimum_height();
            }
        } else {
            // Minimum height of the different rows (independent of tabular mode)
            height = self.sum_with_gaps(&self.min_row_heights(true));
        }

        // If we don't have cells, we don't have height
        if height > 0.0 {
            height + self.cell.padding_ref().top + self.cell.padding_ref().bottom
        } else {
            0.0
        }
    }

    /// Compute x coordinates and widths of children, matching Java's
    /// GridContainerCell.layoutChildrenHorizontally.
    pub fn layout_children_horizontally(&mut self) {
        if self.tabular {
            let col_widths = self.min_column_widths(None, false);
            for area in ContainerArea::values() {
                self.apply_widths_to_row(*area, col_widths);
            }
        } else {
            for area in ContainerArea::values() {
                let col_widths = self.min_column_widths(Some(*area), false);
                self.apply_widths_to_row(*area, col_widths);
            }
        }
    }

    /// Compute y coordinates and heights of children, matching Java's
    /// GridContainerCell.layoutChildrenVertically.
    pub fn layout_children_vertically(&mut self) {
        let cell_rectangle = *self.cell.cell_rectangle_ref();
        let cell_padding = self.cell.padding_ref().clone();

        let mut row_heights = self.min_row_heights(false);

        // Top row is top-aligned with our content area, bottom row is bottom-aligned
        self.apply_height_to_row(
            ContainerArea::Begin,
            cell_rectangle.y + cell_padding.top,
            &row_heights,
        );
        self.apply_height_to_row(
            ContainerArea::End,
            cell_rectangle.y + cell_rectangle.height - cell_padding.bottom - row_heights[2],
            &row_heights,
        );

        // Size of the content area and size of the available space in the content area
        let mut free_content_area_height =
            cell_rectangle.height - cell_padding.top - cell_padding.bottom;

        if row_heights[0] > 0.0 {
            row_heights[0] += self.gap;
            free_content_area_height -= row_heights[0];
        }

        if row_heights[2] > 0.0 {
            row_heights[2] += self.gap;
            free_content_area_height -= row_heights[2];
        }

        // Compute the center cell rectangle
        self.center_cell_rect.height = 0.0_f64.max(free_content_area_height);
        self.center_cell_rect.y = cell_rectangle.y + cell_padding.top
            + (self.center_cell_rect.height - free_content_area_height) / 2.0;

        // If the available space is larger than the current size of the center cell, enlarge
        row_heights[1] = row_heights[1].max(free_content_area_height);

        // Place the center cell, possibly enlarging it in the process
        self.apply_height_to_row(
            ContainerArea::Center,
            cell_rectangle.y + cell_padding.top + row_heights[0]
                - (row_heights[1] - free_content_area_height) / 2.0,
            &row_heights,
        );
    }

    /// Returns an array of minimum column widths. If `row` is Some, only that row's widths
    /// are used. If None, the maximum across all rows is used (tabular mode).
    fn min_column_widths(
        &self,
        row: Option<ContainerArea>,
        respect_contribution_flag: bool,
    ) -> [f64; 3] {
        let mut col_widths = [
            self.min_width_of_column(ContainerArea::Begin, row, respect_contribution_flag),
            self.min_width_of_column(ContainerArea::Center, row, respect_contribution_flag),
            self.min_width_of_column(ContainerArea::End, row, respect_contribution_flag),
        ];

        // If we are to be symmetrical, the outer cells need to be the same size
        if self.symmetrical {
            col_widths[0] = col_widths[0].max(col_widths[2]);
            col_widths[2] = col_widths[0];
        }

        col_widths
    }

    /// Returns the minimum width of the given column.
    fn min_width_of_column(
        &self,
        column: ContainerArea,
        row: Option<ContainerArea>,
        respect_contribution_flag: bool,
    ) -> f64 {
        let mut max_min_width: f64 = 0.0;

        match row {
            None => {
                // Aggregate values for all rows
                for row_index in 0..3 {
                    max_min_width = max_min_width.max(min_width_of_cell(
                        &self.cells[row_index][column.index()],
                        respect_contribution_flag,
                    ));
                }
            }
            Some(r) => {
                // Only concentrate on the specified row
                max_min_width =
                    min_width_of_cell(&self.cells[r.index()][column.index()], respect_contribution_flag);
            }
        }

        // If this is the center column, we might have an explicit minimal width for that
        if column == ContainerArea::Center {
            if let Some(ref min_size) = self.center_cell_minimum_size {
                max_min_width = max_min_width.max(min_size.x);
            }
        }

        max_min_width
    }

    /// Returns an array containing the height of each row. Symmetry is applied if activated.
    fn min_row_heights(&self, respect_contribution_flag: bool) -> [f64; 3] {
        let mut row_heights = [
            self.min_height_of_row(ContainerArea::Begin, respect_contribution_flag),
            self.min_height_of_row(ContainerArea::Center, respect_contribution_flag),
            self.min_height_of_row(ContainerArea::End, respect_contribution_flag),
        ];

        // If we are to be symmetrical, the outer cells need to be the same size
        if self.symmetrical {
            row_heights[0] = row_heights[0].max(row_heights[2]);
            row_heights[2] = row_heights[0];
        }

        row_heights
    }

    /// Returns the minimum height of the given row.
    fn min_height_of_row(
        &self,
        row: ContainerArea,
        respect_contribution_flag: bool,
    ) -> f64 {
        let mut max_min_height: f64 = 0.0;
        for column in 0..3 {
            max_min_height = max_min_height.max(min_height_of_cell(
                &self.cells[row.index()][column],
                respect_contribution_flag,
            ));
        }

        // If this is the center row, we might have an explicit minimal height for that
        if row == ContainerArea::Center {
            if let Some(ref min_size) = self.center_cell_minimum_size {
                max_min_height = max_min_height.max(min_size.y);
            }
        }

        max_min_height
    }

    /// Takes an array of values and sums them up, inserting gaps between each pair of values
    /// bigger than zero.
    fn sum_with_gaps(&self, values: &[f64; 3]) -> f64 {
        let mut sum = 0.0;
        let mut active_components = 0;
        for val in values {
            if *val > 0.0 {
                sum += val;
                active_components += 1;
            }
        }
        if active_components > 1 {
            sum += self.gap * (active_components - 1) as f64;
        }
        sum
    }

    /// Computes horizontal coordinates for all cells in the given row, based on the given
    /// array of column widths.
    fn apply_widths_to_row(&mut self, row: ContainerArea, mut col_widths: [f64; 3]) {
        let cell_rectangle = *self.cell.cell_rectangle_ref();
        let cell_padding = self.cell.padding_ref().clone();

        // Left column is left-aligned with our content area, right column is right-aligned
        self.apply_width_to_column(
            ContainerArea::Begin,
            cell_rectangle.x + cell_padding.left,
            &col_widths,
        );
        self.apply_width_to_column(
            ContainerArea::End,
            cell_rectangle.x + cell_rectangle.width - cell_padding.right - col_widths[2],
            &col_widths,
        );

        // Size of the content area and size of the available space in the content area
        let mut free_content_area_width =
            cell_rectangle.width - cell_padding.left - cell_padding.right;

        if col_widths[0] > 0.0 {
            col_widths[0] += self.gap;
            free_content_area_width -= col_widths[0];
        }

        if col_widths[2] > 0.0 {
            col_widths[2] += self.gap;
            free_content_area_width -= col_widths[2];
        }

        // Compute how wide the center cell can be
        let center_width = 0.0_f64.max(free_content_area_width);

        // If the available space is larger than the current size of the center cell, enlarge
        col_widths[1] = col_widths[1].max(free_content_area_width);

        // Place the center cell, possibly enlarging it in the process
        self.apply_width_to_column(
            ContainerArea::Center,
            cell_rectangle.x + cell_padding.left + col_widths[0]
                - (col_widths[1] - free_content_area_width) / 2.0,
            &col_widths,
        );

        // If this is the center row, remember the center cell's data for the center cell rectangle
        if row == ContainerArea::Center {
            self.center_cell_rect.width = center_width;
            self.center_cell_rect.x = cell_rectangle.x + cell_padding.left
                + (center_width - free_content_area_width) / 2.0;
        }
    }

    /// Applies horizontal layout to all cells in the given column.
    fn apply_width_to_column(&mut self, column: ContainerArea, x: f64, col_widths: &[f64; 3]) {
        let col_idx = column.index();
        let width = col_widths[col_idx];
        for row in 0..3 {
            apply_horizontal_layout(&mut self.cells[row][col_idx], x, width);
        }
    }

    /// Applies vertical layout to all cells in the given row.
    fn apply_height_to_row(&mut self, row: ContainerArea, y: f64, row_heights: &[f64; 3]) {
        let row_idx = row.index();
        let height = row_heights[row_idx];
        for column in 0..3 {
            apply_vertical_layout(&mut self.cells[row_idx][column], y, height);
        }
    }

    // Cell delegate methods
    pub fn padding(&mut self) -> &mut ElkPadding {
        self.cell.padding()
    }

    pub fn padding_ref(&self) -> &ElkPadding {
        self.cell.padding_ref()
    }

    pub fn cell_rectangle(&mut self) -> &mut ElkRectangle {
        self.cell.cell_rectangle()
    }

    pub fn cell_rectangle_ref(&self) -> &ElkRectangle {
        self.cell.cell_rectangle_ref()
    }

    pub fn contributes_to_minimum_width(&self) -> bool {
        self.cell.contributes_to_minimum_width()
    }

    pub fn set_contributes_to_minimum_width(&mut self, contributes: bool) {
        self.cell.set_contributes_to_minimum_width(contributes);
    }

    pub fn contributes_to_minimum_height(&self) -> bool {
        self.cell.contributes_to_minimum_height()
    }

    pub fn set_contributes_to_minimum_height(&mut self, contributes: bool) {
        self.cell.set_contributes_to_minimum_height(contributes);
    }
}
