use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMargin, ElkPadding, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, Direction, NodeLabelPlacement, PortAlignment, PortConstraints, PortLabelPlacement,
    PortSide, SizeConstraint, SizeOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{
    GraphElementAdapter, LabelAdapter, NodeAdapter, PortAdapter,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, IndividualSpacings};
/// Type-erased label wrapper. Stores any label that implements GraphElementAdapter
/// so that LabelCellLayout can work with both ElkLabelAdapter and LLabelAdapter.
trait DynLabelOps {
    fn dyn_get_size(&self) -> KVector;
    fn dyn_set_position(&self, pos: KVector);
}

impl<T: 'static, L: GraphElementAdapter<T>> DynLabelOps for DynLabelWrapper<T, L> {
    fn dyn_get_size(&self) -> KVector {
        self.0.get_size()
    }
    fn dyn_set_position(&self, pos: KVector) {
        self.0.set_position(pos)
    }
}

struct DynLabelWrapper<T, L: GraphElementAdapter<T>>(L, std::marker::PhantomData<T>);

struct DynLabel {
    inner: Box<dyn DynLabelOps>,
}

impl DynLabel {
    fn new<T: 'static, L: GraphElementAdapter<T> + 'static>(label: L) -> Self {
        DynLabel {
            inner: Box::new(DynLabelWrapper(label, std::marker::PhantomData)),
        }
    }

    fn get_size(&self) -> KVector {
        self.inner.dyn_get_size()
    }

    fn set_position(&self, pos: KVector) {
        self.inner.dyn_set_position(pos)
    }
}

pub struct NodeLabelAndSizeCalculator;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContainerArea {
    Begin,
    Center,
    End,
}

impl ContainerArea {
    fn values() -> [ContainerArea; 3] {
        [
            ContainerArea::Begin,
            ContainerArea::Center,
            ContainerArea::End,
        ]
    }

    fn index(self) -> usize {
        match self {
            ContainerArea::Begin => 0,
            ContainerArea::Center => 1,
            ContainerArea::End => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HorizontalLabelAlignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VerticalLabelAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutsideSide {
    North,
    South,
    East,
    West,
}

#[derive(Clone, Copy)]
struct NodeLabelLocationInfo {
    inside: bool,
    row: ContainerArea,
    col: ContainerArea,
    outside_side: Option<OutsideSide>,
}

#[derive(Clone, Copy, Default)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

struct LabelCellLayout {
    labels: Vec<DynLabel>,
    minimum_content_area_size: CellMinSize,
    horizontal_alignment: HorizontalLabelAlignment,
    vertical_alignment: VerticalLabelAlignment,
    horizontal_layout_mode: bool,
    label_gap: f64,
}

impl LabelCellLayout {
    fn new(
        horizontal_alignment: HorizontalLabelAlignment,
        vertical_alignment: VerticalLabelAlignment,
        horizontal_layout_mode: bool,
        label_gap: f64,
    ) -> Self {
        LabelCellLayout {
            labels: Vec::new(),
            minimum_content_area_size: CellMinSize::default(),
            horizontal_alignment,
            vertical_alignment,
            horizontal_layout_mode,
            label_gap,
        }
    }

    fn add_label(&mut self, label: DynLabel) {
        self.minimum_content_area_size.add_label(
            label.get_size(),
            self.label_gap,
            self.horizontal_layout_mode,
        );
        self.labels.push(label);
    }

    fn min_width(&self) -> f64 {
        self.minimum_content_area_size.width
    }

    fn min_height(&self) -> f64 {
        self.minimum_content_area_size.height
    }

    fn apply_layout(&self, rect: Rect) {
        if self.labels.is_empty() {
            return;
        }

        if self.horizontal_layout_mode {
            self.apply_horizontal_layout(rect);
        } else {
            self.apply_vertical_layout(rect);
        }
    }

    fn apply_horizontal_layout(&self, rect: Rect) {
        let mut y_pos = rect.y;
        if self.vertical_alignment == VerticalLabelAlignment::Center {
            y_pos += (rect.height - self.minimum_content_area_size.height) / 2.0;
        } else if self.vertical_alignment == VerticalLabelAlignment::Bottom {
            y_pos += rect.height - self.minimum_content_area_size.height;
        }

        for label in &self.labels {
            let size = label.get_size();
            let mut pos = KVector::new();
            pos.y = y_pos;
            y_pos += size.y + self.label_gap;

            pos.x = match self.horizontal_alignment {
                HorizontalLabelAlignment::Left => rect.x,
                HorizontalLabelAlignment::Center => rect.x + (rect.width - size.x) / 2.0,
                HorizontalLabelAlignment::Right => rect.x + rect.width - size.x,
            };
            label.set_position(pos);
        }
    }

    fn apply_vertical_layout(&self, rect: Rect) {
        let mut x_pos = rect.x;
        if self.horizontal_alignment == HorizontalLabelAlignment::Center {
            x_pos += (rect.width - self.minimum_content_area_size.width) / 2.0;
        } else if self.horizontal_alignment == HorizontalLabelAlignment::Right {
            x_pos += rect.width - self.minimum_content_area_size.width;
        }

        for label in &self.labels {
            let size = label.get_size();
            let mut pos = KVector::new();
            pos.x = x_pos;
            x_pos += size.x + self.label_gap;

            pos.y = match self.vertical_alignment {
                VerticalLabelAlignment::Top => rect.y,
                VerticalLabelAlignment::Center => rect.y + (rect.height - size.y) / 2.0,
                VerticalLabelAlignment::Bottom => rect.y + rect.height - size.y,
            };
            label.set_position(pos);
        }
    }
}

#[derive(Clone, Copy, Default)]
struct CellMinSize {
    width: f64,
    height: f64,
    label_count: usize,
}

impl CellMinSize {
    fn add_label(&mut self, label_size: KVector, gap: f64, horizontal_layout_mode: bool) {
        if horizontal_layout_mode {
            self.width = self.width.max(label_size.x);
            self.height += label_size.y;
            if self.label_count > 0 {
                self.height += gap;
            }
        } else {
            self.width += label_size.x;
            if self.label_count > 0 {
                self.width += gap;
            }
            self.height = self.height.max(label_size.y);
        }
        self.label_count += 1;
    }
}

struct InsideNodeLabelGrid {
    cells: [[CellMinSize; 3]; 3],
    container_gap: f64,
    label_gap: f64,
    padding: ElkPadding,
    tabular: bool,
    symmetrical: bool,
    center_cell_min_size: Option<KVector>,
}

impl InsideNodeLabelGrid {
    fn new<N, T>(node: &N, layout_direction: Direction) -> Self
    where
        T: 'static,
        N: NodeAdapter<T>,
        N::Graph: GraphElementAdapter<T>,
        N::LabelAdapter: 'static,
    {
        let size_options = node
            .get_property(CoreOptions::NODE_SIZE_OPTIONS)
            .unwrap_or_default();
        let label_gap = IndividualSpacings::get_individual_or_inherited_adapter(
            node,
            CoreOptions::SPACING_LABEL_LABEL,
        )
        .unwrap_or(0.0);
        let container_gap = 2.0 * label_gap;
        let padding = IndividualSpacings::get_individual_or_inherited_adapter(
            node,
            CoreOptions::NODE_LABELS_PADDING,
        )
        .unwrap_or_default();

        let mut grid = InsideNodeLabelGrid {
            cells: [[CellMinSize::default(); 3]; 3],
            container_gap,
            label_gap,
            padding,
            tabular: size_options.contains(&SizeOptions::ForceTabularNodeLabels),
            symmetrical: !size_options.contains(&SizeOptions::Asymmetrical),
            center_cell_min_size: None,
        };

        let default_node_label_placement = node
            .get_property(CoreOptions::NODE_LABELS_PLACEMENT)
            .unwrap_or_default();
        let horizontal_layout_mode = !layout_direction.is_vertical();

        for label in node.get_labels() {
            let effective_placement = if label.has_property(CoreOptions::NODE_LABELS_PLACEMENT) {
                label
                    .get_property(CoreOptions::NODE_LABELS_PLACEMENT)
                    .unwrap_or_else(|| default_node_label_placement.clone())
            } else {
                default_node_label_placement.clone()
            };

            if let Some((row, col)) = inside_cell_for_placement(&effective_placement) {
                let size = label.get_size();
                grid.cells[row.index()][col.index()].add_label(
                    size,
                    grid.label_gap,
                    horizontal_layout_mode,
                );
            }
        }

        let size_constraints = node
            .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
            .unwrap_or_default();
        if size_constraints.contains(&SizeConstraint::MinimumSize)
            && size_options.contains(&SizeOptions::MinimumSizeAccountsForPadding)
        {
            grid.center_cell_min_size = Some(configured_minimum_size(node, &size_options));
        }

        grid
    }

    fn compute_inside_padding(&self) -> ElkPadding {
        let mut padding = ElkPadding::new();

        for col in ContainerArea::values() {
            let label_cell = self.cells[ContainerArea::Begin.index()][col.index()];
            padding.top = padding.top.max(label_cell.height);
        }

        for col in ContainerArea::values() {
            let label_cell = self.cells[ContainerArea::End.index()][col.index()];
            padding.bottom = padding.bottom.max(label_cell.height);
        }

        for row in ContainerArea::values() {
            let label_cell = self.cells[row.index()][ContainerArea::Begin.index()];
            padding.left = padding.left.max(label_cell.width);
        }

        for row in ContainerArea::values() {
            let label_cell = self.cells[row.index()][ContainerArea::End.index()];
            padding.right = padding.right.max(label_cell.width);
        }

        if padding.top > 0.0 {
            padding.top += self.padding.top + self.container_gap;
        }
        if padding.bottom > 0.0 {
            padding.bottom += self.padding.bottom + self.container_gap;
        }
        if padding.left > 0.0 {
            padding.left += self.padding.left + self.container_gap;
        }
        if padding.right > 0.0 {
            padding.right += self.padding.right + self.container_gap;
        }

        padding
    }

    fn compute_minimum_size(&self, only_center_cell_contributes: bool) -> KVector {
        let (mut min_width, mut min_height) = if only_center_cell_contributes {
            if let Some(center_min_size) = self.center_cell_min_size {
                (center_min_size.x, center_min_size.y)
            } else {
                let center =
                    self.cells[ContainerArea::Center.index()][ContainerArea::Center.index()];
                (center.width, center.height)
            }
        } else {
            let mut width = 0.0;
            if self.tabular {
                width = sum_with_gaps_with_gap(self.min_column_widths(None), self.container_gap);
            } else {
                for row in ContainerArea::values() {
                    let row_width = sum_with_gaps_with_gap(
                        self.min_column_widths(Some(row)),
                        self.container_gap,
                    );
                    width = width.max(row_width);
                }
            }

            (
                width,
                sum_with_gaps_with_gap(self.min_row_heights(), self.container_gap),
            )
        };

        if min_width > 0.0 {
            min_width += self.padding.left + self.padding.right;
        }
        if min_height > 0.0 {
            min_height += self.padding.top + self.padding.bottom;
        }

        KVector::with_values(min_width, min_height)
    }

    fn min_column_widths(&self, row: Option<ContainerArea>) -> [f64; 3] {
        let mut col_widths = [0.0, 0.0, 0.0];

        for col in ContainerArea::values() {
            let col_index = col.index();
            let max_width = if let Some(row) = row {
                self.cells[row.index()][col_index].width
            } else {
                ContainerArea::values()
                    .iter()
                    .map(|iter_row| self.cells[iter_row.index()][col_index].width)
                    .fold(0.0, f64::max)
            };

            col_widths[col_index] = max_width;
        }

        if let Some(center_min_size) = self.center_cell_min_size {
            col_widths[ContainerArea::Center.index()] =
                col_widths[ContainerArea::Center.index()].max(center_min_size.x);
        }

        if self.symmetrical {
            let side_width = col_widths[ContainerArea::Begin.index()]
                .max(col_widths[ContainerArea::End.index()]);
            col_widths[ContainerArea::Begin.index()] = side_width;
            col_widths[ContainerArea::End.index()] = side_width;
        }

        col_widths
    }

    fn min_row_heights(&self) -> [f64; 3] {
        let mut row_heights = [0.0, 0.0, 0.0];

        for row in ContainerArea::values() {
            let row_index = row.index();
            let max_height = ContainerArea::values()
                .iter()
                .map(|col| self.cells[row_index][col.index()].height)
                .fold(0.0, f64::max);
            row_heights[row_index] = max_height;
        }

        if let Some(center_min_size) = self.center_cell_min_size {
            row_heights[ContainerArea::Center.index()] =
                row_heights[ContainerArea::Center.index()].max(center_min_size.y);
        }

        if self.symmetrical {
            let side_height = row_heights[ContainerArea::Begin.index()]
                .max(row_heights[ContainerArea::End.index()]);
            row_heights[ContainerArea::Begin.index()] = side_height;
            row_heights[ContainerArea::End.index()] = side_height;
        }

        row_heights
    }
}

#[derive(Clone, Copy)]
struct AxisLayout {
    starts: [f64; 3],
    spans: [f64; 3],
    center_area_start: f64,
    center_area_span: f64,
}

fn layout_axis_three(
    start: f64,
    span: f64,
    padding_start: f64,
    padding_end: f64,
    min_spans: [f64; 3],
    gap: f64,
) -> AxisLayout {
    let mut free_content_span = span - padding_start - padding_end;
    let mut start_span_with_gap = min_spans[ContainerArea::Begin.index()];

    if min_spans[ContainerArea::Begin.index()] > 0.0 {
        start_span_with_gap += gap;
        free_content_span -= start_span_with_gap;
    }

    if min_spans[ContainerArea::End.index()] > 0.0 {
        free_content_span -= min_spans[ContainerArea::End.index()] + gap;
    }

    let center_span = min_spans[ContainerArea::Center.index()].max(free_content_span);

    let start_pos = start + padding_start;
    let center_pos =
        start + padding_start + start_span_with_gap - (center_span - free_content_span) / 2.0;
    let end_pos = start + span - padding_end - min_spans[ContainerArea::End.index()];

    let clamped_center_span = free_content_span.max(0.0);
    let center_area_start = start + padding_start + (clamped_center_span - free_content_span) / 2.0;

    AxisLayout {
        starts: [start_pos, center_pos, end_pos],
        spans: [
            min_spans[ContainerArea::Begin.index()],
            center_span,
            min_spans[ContainerArea::End.index()],
        ],
        center_area_start,
        center_area_span: clamped_center_span,
    }
}

struct StripContainerLayout {
    cells: [LabelCellLayout; 3],
    vertical_strip: bool,
    symmetrical: bool,
    gap: f64,
    padding: ElkPadding,
}

impl StripContainerLayout {
    fn new_horizontal(
        symmetrical: bool,
        gap: f64,
        padding: ElkPadding,
        label_gap: f64,
        horizontal_layout_mode: bool,
        vertical_alignment: VerticalLabelAlignment,
    ) -> Self {
        StripContainerLayout {
            cells: [
                LabelCellLayout::new(
                    HorizontalLabelAlignment::Left,
                    vertical_alignment,
                    horizontal_layout_mode,
                    label_gap,
                ),
                LabelCellLayout::new(
                    HorizontalLabelAlignment::Center,
                    vertical_alignment,
                    horizontal_layout_mode,
                    label_gap,
                ),
                LabelCellLayout::new(
                    HorizontalLabelAlignment::Right,
                    vertical_alignment,
                    horizontal_layout_mode,
                    label_gap,
                ),
            ],
            vertical_strip: false,
            symmetrical,
            gap,
            padding,
        }
    }

    fn new_vertical(
        symmetrical: bool,
        gap: f64,
        padding: ElkPadding,
        label_gap: f64,
        horizontal_layout_mode: bool,
        horizontal_alignment: HorizontalLabelAlignment,
    ) -> Self {
        StripContainerLayout {
            cells: [
                LabelCellLayout::new(
                    horizontal_alignment,
                    VerticalLabelAlignment::Top,
                    horizontal_layout_mode,
                    label_gap,
                ),
                LabelCellLayout::new(
                    horizontal_alignment,
                    VerticalLabelAlignment::Center,
                    horizontal_layout_mode,
                    label_gap,
                ),
                LabelCellLayout::new(
                    horizontal_alignment,
                    VerticalLabelAlignment::Bottom,
                    horizontal_layout_mode,
                    label_gap,
                ),
            ],
            vertical_strip: true,
            symmetrical,
            gap,
            padding,
        }
    }

    fn add_label(&mut self, area_index: usize, label: DynLabel) {
        self.cells[area_index].add_label(label);
    }

    fn has_labels(&self) -> bool {
        self.cells.iter().any(|cell| !cell.labels.is_empty())
    }

    fn min_cell_widths(&self) -> [f64; 3] {
        let mut widths = [
            self.cells[ContainerArea::Begin.index()].min_width(),
            self.cells[ContainerArea::Center.index()].min_width(),
            self.cells[ContainerArea::End.index()].min_width(),
        ];

        if self.symmetrical {
            let side_width =
                widths[ContainerArea::Begin.index()].max(widths[ContainerArea::End.index()]);
            widths[ContainerArea::Begin.index()] = side_width;
            widths[ContainerArea::End.index()] = side_width;
        }

        widths
    }

    fn min_cell_heights(&self) -> [f64; 3] {
        let mut heights = [
            self.cells[ContainerArea::Begin.index()].min_height(),
            self.cells[ContainerArea::Center.index()].min_height(),
            self.cells[ContainerArea::End.index()].min_height(),
        ];

        if self.symmetrical {
            let side_height =
                heights[ContainerArea::Begin.index()].max(heights[ContainerArea::End.index()]);
            heights[ContainerArea::Begin.index()] = side_height;
            heights[ContainerArea::End.index()] = side_height;
        }

        heights
    }

    fn min_width(&self) -> f64 {
        let width = if self.vertical_strip {
            self.min_cell_widths().into_iter().fold(0.0, f64::max)
        } else {
            sum_with_gaps_with_gap(self.min_cell_widths(), self.gap)
        };

        if self.has_labels() {
            width + self.padding.left + self.padding.right
        } else {
            0.0
        }
    }

    fn min_height(&self) -> f64 {
        let height = if self.vertical_strip {
            sum_with_gaps_with_gap(self.min_cell_heights(), self.gap)
        } else {
            self.min_cell_heights().into_iter().fold(0.0, f64::max)
        };

        if self.has_labels() {
            height + self.padding.top + self.padding.bottom
        } else {
            0.0
        }
    }

    fn apply_layout(&self, rect: Rect) {
        if self.vertical_strip {
            self.apply_vertical_strip_layout(rect);
        } else {
            self.apply_horizontal_strip_layout(rect);
        }
    }

    fn apply_horizontal_strip_layout(&self, rect: Rect) {
        let horizontal_axis = layout_axis_three(
            rect.x,
            rect.width,
            self.padding.left,
            self.padding.right,
            self.min_cell_widths(),
            self.gap,
        );

        let y = rect.y + self.padding.top;
        let height = (rect.height - self.padding.top - self.padding.bottom).max(0.0);

        for area in ContainerArea::values() {
            let index = area.index();
            self.cells[index].apply_layout(Rect {
                x: horizontal_axis.starts[index],
                y,
                width: horizontal_axis.spans[index],
                height,
            });
        }
    }

    fn apply_vertical_strip_layout(&self, rect: Rect) {
        let x = rect.x + self.padding.left;
        let width = (rect.width - self.padding.left - self.padding.right).max(0.0);
        let vertical_axis = layout_axis_three(
            rect.y,
            rect.height,
            self.padding.top,
            self.padding.bottom,
            self.min_cell_heights(),
            self.gap,
        );

        for area in ContainerArea::values() {
            let index = area.index();
            self.cells[index].apply_layout(Rect {
                x,
                y: vertical_axis.starts[index],
                width,
                height: vertical_axis.spans[index],
            });
        }
    }
}

struct InsideLabelLayoutGrid {
    cells: [[LabelCellLayout; 3]; 3],
    container_gap: f64,
    node_label_padding: ElkPadding,
    node_padding: ElkPadding,
    tabular: bool,
    symmetrical: bool,
    center_cell_min_size: Option<KVector>,
}

impl InsideLabelLayoutGrid {
    fn new<N, T>(
        node: &N,
        horizontal_layout_mode: bool,
        size_constraints: &EnumSet<SizeConstraint>,
        size_options: &EnumSet<SizeOptions>,
        label_gap: f64,
        container_gap: f64,
    ) -> Self
    where
        T: 'static,
        N: NodeAdapter<T>,
        N::Graph: GraphElementAdapter<T>,
    {
        let padding = IndividualSpacings::get_individual_or_inherited_adapter(
            node,
            CoreOptions::NODE_LABELS_PADDING,
        )
        .unwrap_or_default();
        let node_padding = ElkPadding::new();
        let tabular = size_options.contains(&SizeOptions::ForceTabularNodeLabels);
        let symmetrical = !size_options.contains(&SizeOptions::Asymmetrical);
        let center_cell_min_size = if size_constraints.contains(&SizeConstraint::MinimumSize)
            && size_options.contains(&SizeOptions::MinimumSizeAccountsForPadding)
        {
            Some(configured_minimum_size(node, size_options))
        } else {
            None
        };

        let cells = std::array::from_fn(|row| {
            std::array::from_fn(|col| {
                LabelCellLayout::new(
                    horizontal_alignment_for_area_index(col),
                    vertical_alignment_for_area_index(row),
                    horizontal_layout_mode,
                    label_gap,
                )
            })
        });

        InsideLabelLayoutGrid {
            cells,
            container_gap,
            node_label_padding: padding,
            node_padding,
            tabular,
            symmetrical,
            center_cell_min_size,
        }
    }

    fn add_label(&mut self, row: ContainerArea, col: ContainerArea, label: DynLabel) {
        self.cells[row.index()][col.index()].add_label(label);
    }

    fn ensure_cell_min_width(&mut self, row: ContainerArea, col: ContainerArea, width: f64) {
        let cell = &mut self.cells[row.index()][col.index()];
        cell.minimum_content_area_size.width = cell.minimum_content_area_size.width.max(width);
    }

    fn min_column_widths(&self, row: Option<ContainerArea>) -> [f64; 3] {
        let mut col_widths = [0.0, 0.0, 0.0];

        for col in ContainerArea::values() {
            let col_index = col.index();
            let max_width = if let Some(row) = row {
                self.cells[row.index()][col_index].min_width()
            } else {
                ContainerArea::values()
                    .iter()
                    .map(|iter_row| self.cells[iter_row.index()][col_index].min_width())
                    .fold(0.0, f64::max)
            };

            col_widths[col_index] = max_width;
        }

        if let Some(center_min_size) = self.center_cell_min_size {
            col_widths[ContainerArea::Center.index()] =
                col_widths[ContainerArea::Center.index()].max(center_min_size.x);
        }

        if self.symmetrical {
            let side_width = col_widths[ContainerArea::Begin.index()]
                .max(col_widths[ContainerArea::End.index()]);
            col_widths[ContainerArea::Begin.index()] = side_width;
            col_widths[ContainerArea::End.index()] = side_width;
        }

        col_widths
    }

    fn min_row_heights(&self) -> [f64; 3] {
        let mut row_heights = [0.0, 0.0, 0.0];

        for row in ContainerArea::values() {
            let row_index = row.index();
            let max_height = ContainerArea::values()
                .iter()
                .map(|col| self.cells[row_index][col.index()].min_height())
                .fold(0.0, f64::max);
            row_heights[row_index] = max_height;
        }

        if let Some(center_min_size) = self.center_cell_min_size {
            row_heights[ContainerArea::Center.index()] =
                row_heights[ContainerArea::Center.index()].max(center_min_size.y);
        }

        if self.symmetrical {
            let side_height = row_heights[ContainerArea::Begin.index()]
                .max(row_heights[ContainerArea::End.index()]);
            row_heights[ContainerArea::Begin.index()] = side_height;
            row_heights[ContainerArea::End.index()] = side_height;
        }

        row_heights
    }

    fn compute_minimum_size(&self, only_center_cell_contributes: bool) -> KVector {
        let (mut min_width, mut min_height) = if only_center_cell_contributes {
            if let Some(center_min_size) = self.center_cell_min_size {
                (center_min_size.x, center_min_size.y)
            } else {
                let center =
                    &self.cells[ContainerArea::Center.index()][ContainerArea::Center.index()];
                (center.min_width(), center.min_height())
            }
        } else {
            let mut width = 0.0;
            if self.tabular {
                width = sum_with_gaps_with_gap(self.min_column_widths(None), self.container_gap);
            } else {
                for row in ContainerArea::values() {
                    let row_width = sum_with_gaps_with_gap(
                        self.min_column_widths(Some(row)),
                        self.container_gap,
                    );
                    width = width.max(row_width);
                }
            }

            (
                width,
                sum_with_gaps_with_gap(self.min_row_heights(), self.container_gap),
            )
        };

        if min_width > 0.0 {
            min_width += self.node_padding.left
                + self.node_padding.right
                + self.node_label_padding.left
                + self.node_label_padding.right;
        }
        if min_height > 0.0 {
            min_height += self.node_padding.top
                + self.node_padding.bottom
                + self.node_label_padding.top
                + self.node_label_padding.bottom;
        }

        KVector::with_values(min_width, min_height)
    }

    fn apply_layout(&self, node_size: KVector) -> Rect {
        let grid_rect = Rect {
            x: self.node_padding.left,
            y: self.node_padding.top,
            width: (node_size.x - self.node_padding.left - self.node_padding.right).max(0.0),
            height: (node_size.y - self.node_padding.top - self.node_padding.bottom).max(0.0),
        };
        let row_heights = self.min_row_heights();
        let vertical_axis = layout_axis_three(
            grid_rect.y,
            grid_rect.height,
            self.node_label_padding.top,
            self.node_label_padding.bottom,
            row_heights,
            self.container_gap,
        );

        let tabular_widths = self.tabular.then(|| self.min_column_widths(None));

        for row in ContainerArea::values() {
            let row_index = row.index();
            let col_widths = tabular_widths.unwrap_or_else(|| self.min_column_widths(Some(row)));
            let horizontal_axis = layout_axis_three(
                grid_rect.x,
                grid_rect.width,
                self.node_label_padding.left,
                self.node_label_padding.right,
                col_widths,
                self.container_gap,
            );

            for col in ContainerArea::values() {
                let col_index = col.index();
                self.cells[row_index][col_index].apply_layout(Rect {
                    x: horizontal_axis.starts[col_index],
                    y: vertical_axis.starts[row_index],
                    width: horizontal_axis.spans[col_index],
                    height: vertical_axis.spans[row_index],
                });
            }
        }

        let center_widths =
            tabular_widths.unwrap_or_else(|| self.min_column_widths(Some(ContainerArea::Center)));
        let center_horizontal_axis = layout_axis_three(
            grid_rect.x,
            grid_rect.width,
            self.node_label_padding.left,
            self.node_label_padding.right,
            center_widths,
            self.container_gap,
        );

        Rect {
            x: center_horizontal_axis.center_area_start,
            y: vertical_axis.center_area_start,
            width: center_horizontal_axis.center_area_span,
            height: vertical_axis.center_area_span,
        }
    }
}

fn sum_with_gaps_with_gap(values: [f64; 3], gap: f64) -> f64 {
    let mut sum = 0.0;
    let mut active_count = 0usize;

    for value in values {
        if value > 0.0 {
            sum += value;
            active_count += 1;
        }
    }

    if active_count > 1 {
        sum += gap * (active_count - 1) as f64;
    }

    sum
}

fn inside_cell_for_placement(
    placement: &EnumSet<NodeLabelPlacement>,
) -> Option<(ContainerArea, ContainerArea)> {
    if !placement.contains(&NodeLabelPlacement::Inside)
        || placement.contains(&NodeLabelPlacement::Outside)
    {
        return None;
    }

    let row = if placement.contains(&NodeLabelPlacement::VTop) {
        ContainerArea::Begin
    } else if placement.contains(&NodeLabelPlacement::VCenter) {
        ContainerArea::Center
    } else if placement.contains(&NodeLabelPlacement::VBottom) {
        ContainerArea::End
    } else {
        return None;
    };

    let col = if placement.contains(&NodeLabelPlacement::HLeft) {
        ContainerArea::Begin
    } else if placement.contains(&NodeLabelPlacement::HCenter) {
        ContainerArea::Center
    } else if placement.contains(&NodeLabelPlacement::HRight) {
        ContainerArea::End
    } else {
        return None;
    };

    Some((row, col))
}

fn horizontal_alignment_for_area_index(index: usize) -> HorizontalLabelAlignment {
    match index {
        0 => HorizontalLabelAlignment::Left,
        1 => HorizontalLabelAlignment::Center,
        _ => HorizontalLabelAlignment::Right,
    }
}

fn vertical_alignment_for_area_index(index: usize) -> VerticalLabelAlignment {
    match index {
        0 => VerticalLabelAlignment::Top,
        1 => VerticalLabelAlignment::Center,
        _ => VerticalLabelAlignment::Bottom,
    }
}

fn horizontal_alignment_for_placement(
    placement: &EnumSet<NodeLabelPlacement>,
) -> Option<HorizontalLabelAlignment> {
    if placement.contains(&NodeLabelPlacement::HLeft) {
        Some(HorizontalLabelAlignment::Left)
    } else if placement.contains(&NodeLabelPlacement::HCenter) {
        Some(HorizontalLabelAlignment::Center)
    } else if placement.contains(&NodeLabelPlacement::HRight) {
        Some(HorizontalLabelAlignment::Right)
    } else {
        None
    }
}

fn vertical_alignment_for_placement(
    placement: &EnumSet<NodeLabelPlacement>,
) -> Option<VerticalLabelAlignment> {
    if placement.contains(&NodeLabelPlacement::VTop) {
        Some(VerticalLabelAlignment::Top)
    } else if placement.contains(&NodeLabelPlacement::VCenter) {
        Some(VerticalLabelAlignment::Center)
    } else if placement.contains(&NodeLabelPlacement::VBottom) {
        Some(VerticalLabelAlignment::Bottom)
    } else {
        None
    }
}

fn area_for_horizontal_alignment(horizontal_alignment: HorizontalLabelAlignment) -> ContainerArea {
    match horizontal_alignment {
        HorizontalLabelAlignment::Left => ContainerArea::Begin,
        HorizontalLabelAlignment::Center => ContainerArea::Center,
        HorizontalLabelAlignment::Right => ContainerArea::End,
    }
}

fn area_for_vertical_alignment(vertical_alignment: VerticalLabelAlignment) -> ContainerArea {
    match vertical_alignment {
        VerticalLabelAlignment::Top => ContainerArea::Begin,
        VerticalLabelAlignment::Center => ContainerArea::Center,
        VerticalLabelAlignment::Bottom => ContainerArea::End,
    }
}

fn node_label_location_info_for_placement(
    placement: &EnumSet<NodeLabelPlacement>,
) -> Option<NodeLabelLocationInfo> {
    let inside = placement.contains(&NodeLabelPlacement::Inside);
    let outside = placement.contains(&NodeLabelPlacement::Outside);
    if inside == outside {
        return None;
    }

    let horizontal_alignment = horizontal_alignment_for_placement(placement)?;
    let vertical_alignment = vertical_alignment_for_placement(placement)?;
    let row = area_for_vertical_alignment(vertical_alignment);
    let col = area_for_horizontal_alignment(horizontal_alignment);

    if inside {
        return Some(NodeLabelLocationInfo {
            inside: true,
            row,
            col,
            outside_side: None,
        });
    }

    let h_priority = placement.contains(&NodeLabelPlacement::HPriority);
    if h_priority {
        return match horizontal_alignment {
            HorizontalLabelAlignment::Left => Some(NodeLabelLocationInfo {
                inside: false,
                row,
                col: ContainerArea::Begin,
                outside_side: Some(OutsideSide::West),
            }),
            HorizontalLabelAlignment::Right => Some(NodeLabelLocationInfo {
                inside: false,
                row,
                col: ContainerArea::End,
                outside_side: Some(OutsideSide::East),
            }),
            HorizontalLabelAlignment::Center => match vertical_alignment {
                VerticalLabelAlignment::Top => Some(NodeLabelLocationInfo {
                    inside: false,
                    row: ContainerArea::Begin,
                    col: ContainerArea::Center,
                    outside_side: Some(OutsideSide::North),
                }),
                VerticalLabelAlignment::Bottom => Some(NodeLabelLocationInfo {
                    inside: false,
                    row: ContainerArea::End,
                    col: ContainerArea::Center,
                    outside_side: Some(OutsideSide::South),
                }),
                VerticalLabelAlignment::Center => None,
            },
        };
    }

    match vertical_alignment {
        VerticalLabelAlignment::Top => Some(NodeLabelLocationInfo {
            inside: false,
            row: ContainerArea::Begin,
            col,
            outside_side: Some(OutsideSide::North),
        }),
        VerticalLabelAlignment::Bottom => Some(NodeLabelLocationInfo {
            inside: false,
            row: ContainerArea::End,
            col,
            outside_side: Some(OutsideSide::South),
        }),
        VerticalLabelAlignment::Center => match horizontal_alignment {
            HorizontalLabelAlignment::Left => Some(NodeLabelLocationInfo {
                inside: false,
                row: ContainerArea::Center,
                col: ContainerArea::Begin,
                outside_side: Some(OutsideSide::West),
            }),
            HorizontalLabelAlignment::Right => Some(NodeLabelLocationInfo {
                inside: false,
                row: ContainerArea::Center,
                col: ContainerArea::End,
                outside_side: Some(OutsideSide::East),
            }),
            HorizontalLabelAlignment::Center => None,
        },
    }
}

fn has_effectively_fixed_size_constraints(size_constraints: &EnumSet<SizeConstraint>) -> bool {
    size_constraints.is_empty()
        || (size_constraints.len() == 1 && size_constraints.contains(&SizeConstraint::PortLabels))
}

fn configured_minimum_size<N, T>(node: &N, size_options: &EnumSet<SizeOptions>) -> KVector
where
    T: 'static,
    N: GraphElementAdapter<T>,
{
    let mut minimum_size = node
        .get_property(CoreOptions::NODE_SIZE_MINIMUM)
        .unwrap_or_default();
    if size_options.contains(&SizeOptions::DefaultMinimumSize) {
        if minimum_size.x <= 0.0 {
            minimum_size.x = ElkUtil::DEFAULT_MIN_WIDTH;
        }
        if minimum_size.y <= 0.0 {
            minimum_size.y = ElkUtil::DEFAULT_MIN_HEIGHT;
        }
    }
    minimum_size
}

fn place_outside_container(
    container: &StripContainerLayout,
    side: OutsideSide,
    node_size: KVector,
    outside_overhang: bool,
) {
    if !container.has_labels() {
        return;
    }

    let mut container_rect = Rect {
        x: 0.0,
        y: 0.0,
        width: container.min_width(),
        height: container.min_height(),
    };

    match side {
        OutsideSide::North | OutsideSide::South => {
            container_rect.width = container_rect.width.max(node_size.x);
            if container_rect.width > node_size.x && !outside_overhang {
                container_rect.width = node_size.x;
            }
            container_rect.x = -(container_rect.width - node_size.x) / 2.0;
            container_rect.y = if matches!(side, OutsideSide::North) {
                -container_rect.height
            } else {
                node_size.y
            };
        }
        OutsideSide::West | OutsideSide::East => {
            container_rect.height = container_rect.height.max(node_size.y);
            if container_rect.height > node_size.y && !outside_overhang {
                container_rect.height = node_size.y;
            }
            container_rect.y = -(container_rect.height - node_size.y) / 2.0;
            container_rect.x = if matches!(side, OutsideSide::West) {
                -container_rect.width
            } else {
                node_size.x
            };
        }
    }

    container.apply_layout(container_rect);
}

impl NodeLabelAndSizeCalculator {
    pub fn process<N, T>(node: &N, layout_direction: Direction) -> KVector
    where
        T: 'static,
        N: NodeAdapter<T>,
        N::Graph: GraphElementAdapter<T>,
        N::Label: 'static,
        N::LabelAdapter: 'static,
        N::Port: 'static,
        N::PortAdapter: 'static,
    {
        Self::process_with_options(node, layout_direction, true, false)
    }

    /// Processes a single node using Java's full 7-phase cell system pipeline.
    ///
    /// - `apply_stuff=true, ignore_inside_port_labels=false` matches Java's default
    ///   `NodeLabelAndSizeCalculator.process(...)`.
    /// - `apply_stuff=false, ignore_inside_port_labels=true` matches Java's
    ///   importer path for hierarchical minimum size calculation.
    pub fn process_with_options<N, T>(
        node: &N,
        layout_direction: Direction,
        apply_stuff: bool,
        ignore_inside_port_labels: bool,
    ) -> KVector
    where
        T: 'static,
        N: NodeAdapter<T>,
        N::Graph: GraphElementAdapter<T>,
        N::Label: 'static,
        N::LabelAdapter: 'static,
        N::Port: 'static,
        N::PortAdapter: 'static,
    {
        use super::internal::algorithm::{
            cell_system_configurator::CellSystemConfigurator,
            horizontal_port_placement_size_calculator::HorizontalPortPlacementSizeCalculator,
            inside_port_label_cell_creator::InsidePortLabelCellCreator,
            label_placer::LabelPlacer,
            node_label_and_size_utilities::NodeLabelAndSizeUtilities,
            node_label_cell_creator::NodeLabelCellCreator,
            node_size_calculator::NodeSizeCalculator,
            port_context_creator::PortContextCreator,
            port_label_placement_calculator::PortLabelPlacementCalculator,
            port_placement_calculator::PortPlacementCalculator,
            vertical_port_placement_size_calculator::VerticalPortPlacementSizeCalculator,
        };
        use super::internal::node_context::NodeContext;
        use super::internal::port_context::PortContext;

        // PREPARATORY PREPARATIONS: Create context objects
        let mut node_context = NodeContext::new(node);
        PortContextCreator::create_port_contexts(&mut node_context, node, ignore_inside_port_labels);

        // PHASE 1 (WONDEROUS WATERFOWL): Setup All Cells
        let horizontal_layout_mode = !layout_direction.is_vertical();
        NodeLabelCellCreator::create_node_label_cells(&mut node_context, node, false, horizontal_layout_mode);
        InsidePortLabelCellCreator::create_inside_port_label_cells(&mut node_context);

        // PHASE 2 (DEFECTIVE DUCK): Setup Client Area Space and Node Cell Padding
        NodeLabelAndSizeUtilities::setup_minimum_client_area_size(&mut node_context);
        NodeLabelAndSizeUtilities::setup_node_padding_for_ports_with_offset(&mut node_context);

        // PHASE 3 (SALVAGEABLE SWAN): Minimum Space Required to Place Ports
        HorizontalPortPlacementSizeCalculator::calculate_horizontal_port_placement_size(&mut node_context);
        VerticalPortPlacementSizeCalculator::calculate_vertical_port_placement_size(&mut node_context);

        // PHASE 4 (DAMNABLE DUCKLING): Setup Cell System Size Contribution Flags
        CellSystemConfigurator::configure_cell_system_size_contributions(&mut node_context);

        // PHASE 5 (DUCK AND COVER): Set Node Width and Place Horizontal Ports
        NodeSizeCalculator::set_node_width(&mut node_context);
        PortPlacementCalculator::place_horizontal_ports(&mut node_context);
        PortLabelPlacementCalculator::place_horizontal_port_labels(&mut node_context);

        // PHASE 6 (GIGANTIC GOOSE): Set Node Height and Place Vertical Ports
        CellSystemConfigurator::update_vertical_inside_port_label_cell_padding(&mut node_context);
        NodeSizeCalculator::set_node_height(&mut node_context);
        if !apply_stuff {
            return node_context.node_size;
        }
        NodeLabelAndSizeUtilities::offset_southern_ports_by_node_size(&mut node_context);
        PortPlacementCalculator::place_vertical_ports(&mut node_context);
        PortLabelPlacementCalculator::place_vertical_port_labels(&mut node_context);

        // PHASE 7 (THANKSGIVING): Place Labels and Apply Stuff
        LabelPlacer::place_labels(&mut node_context);
        NodeLabelAndSizeUtilities::set_node_padding(&node_context);

        // applyStuff: write computed sizes/positions back to the adapter
        node.set_size(node_context.node_size);

        // Apply port positions: collect all port contexts sorted by volatile_id,
        // then zip with ports (which are iterated in the same creation order).
        let mut all_port_contexts: Vec<&PortContext> =
            node_context.port_contexts.values().flat_map(|v| v.iter()).collect();
        all_port_contexts.sort_by_key(|pc| pc.volatile_id);
        for (port, pc) in node.get_ports().into_iter().zip(all_port_contexts.iter()) {
            port.set_position(pc.port_position);
        }

        node_context.node_size
    }

    pub fn process_node<N, T>(node: &N, layout_direction: Direction)
    where
        T: 'static,
        N: NodeAdapter<T>,
        N::Graph: GraphElementAdapter<T>,
        N::Label: 'static,
        N::LabelAdapter: 'static,
    {
        let trace_labels = std::env::var("ELK_TRACE_SIZING_LABELS").is_ok();
        let size_constraints = node
            .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
            .unwrap_or_default();
        if std::env::var("ELK_TRACE_SIZING").is_ok() {
            let label_count = node.get_labels().len();
            let sz = node.get_size();
            eprintln!("TRACE process_node: labels={} size=({},{}) size_constraints={:?} compound={}",
                label_count, sz.x, sz.y, size_constraints, node.is_compound_node());
        }
        let size_options = node
            .get_property(CoreOptions::NODE_SIZE_OPTIONS)
            .unwrap_or_default();
        let default_label_placement = node
            .get_property(CoreOptions::NODE_LABELS_PLACEMENT)
            .unwrap_or_default();
        let label_label_spacing = IndividualSpacings::get_individual_or_inherited_adapter(
            node,
            CoreOptions::SPACING_LABEL_LABEL,
        )
        .unwrap_or(0.0);
        let label_cell_spacing = 2.0 * label_label_spacing;
        let node_label_spacing = IndividualSpacings::get_individual_or_inherited_adapter(
            node,
            CoreOptions::SPACING_LABEL_NODE,
        )
        .unwrap_or(0.0);

        let horizontal_layout_mode = !layout_direction.is_vertical();
        let mut inside_layout = InsideLabelLayoutGrid::new(
            node,
            horizontal_layout_mode,
            &size_constraints,
            &size_options,
            label_label_spacing,
            label_cell_spacing,
        );

        let mut north_padding = ElkPadding::new();
        north_padding.bottom = node_label_spacing;
        let mut south_padding = ElkPadding::new();
        south_padding.top = node_label_spacing;
        let mut west_padding = ElkPadding::new();
        west_padding.right = node_label_spacing;
        let mut east_padding = ElkPadding::new();
        east_padding.left = node_label_spacing;

        let symmetrical = !size_options.contains(&SizeOptions::Asymmetrical);
        let mut north_labels = StripContainerLayout::new_horizontal(
            symmetrical,
            label_cell_spacing,
            north_padding,
            label_label_spacing,
            horizontal_layout_mode,
            VerticalLabelAlignment::Bottom,
        );
        let mut south_labels = StripContainerLayout::new_horizontal(
            symmetrical,
            label_cell_spacing,
            south_padding,
            label_label_spacing,
            horizontal_layout_mode,
            VerticalLabelAlignment::Top,
        );
        let mut west_labels = StripContainerLayout::new_vertical(
            symmetrical,
            label_cell_spacing,
            west_padding,
            label_label_spacing,
            horizontal_layout_mode,
            HorizontalLabelAlignment::Right,
        );
        let mut east_labels = StripContainerLayout::new_vertical(
            symmetrical,
            label_cell_spacing,
            east_padding,
            label_label_spacing,
            horizontal_layout_mode,
            HorizontalLabelAlignment::Left,
        );

        for label in node.get_labels() {
            let effective_placement = if label.has_property(CoreOptions::NODE_LABELS_PLACEMENT) {
                label
                    .get_property(CoreOptions::NODE_LABELS_PLACEMENT)
                    .unwrap_or_else(|| default_label_placement.clone())
            } else {
                default_label_placement.clone()
            };
            if trace_labels {
                let pos = label.get_position();
                let size = label.get_size();
                eprintln!(
                    "TRACE label(before): text='{}' placement={:?} pos=({}, {}) size=({}, {})",
                    label.get_text(),
                    effective_placement,
                    pos.x,
                    pos.y,
                    size.x,
                    size.y
                );
            }
            let dyn_label = DynLabel::new::<N::Label, N::LabelAdapter>(label);

            if let Some(label_location) =
                node_label_location_info_for_placement(&effective_placement)
            {
                if label_location.inside {
                    inside_layout.add_label(label_location.row, label_location.col, dyn_label);
                } else if let Some(side) = label_location.outside_side {
                    let strip_index = match side {
                        OutsideSide::North | OutsideSide::South => label_location.col.index(),
                        OutsideSide::West | OutsideSide::East => label_location.row.index(),
                    };
                    match side {
                        OutsideSide::North => north_labels.add_label(strip_index, dyn_label),
                        OutsideSide::South => south_labels.add_label(strip_index, dyn_label),
                        OutsideSide::East => east_labels.add_label(strip_index, dyn_label),
                        OutsideSide::West => west_labels.add_label(strip_index, dyn_label),
                    }
                }
            }
        }

        let port_labels_placement = node
            .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
            .unwrap_or_default();
        Self::setup_node_padding_for_ports_with_offset(
            &mut inside_layout,
            node,
            &port_labels_placement,
            &size_options,
        );
        Self::setup_inside_port_label_cell_minimums(
            &mut inside_layout,
            node,
            &port_labels_placement,
        );
        let minimum_size_accounts_for_padding = size_constraints
            .contains(&SizeConstraint::MinimumSize)
            && size_options.contains(&SizeOptions::MinimumSizeAccountsForPadding);
        let include_node_labels = size_constraints.contains(&SizeConstraint::NodeLabels);
        let outside_overhang = size_options.contains(&SizeOptions::OutsideNodeLabelsOverhang);
        let topdown_layout = node
            .get_property(CoreOptions::TOPDOWN_LAYOUT)
            .unwrap_or(false);
        let initial_node_size = node.get_size();
        let mut target_node_size = initial_node_size;

        if !has_effectively_fixed_size_constraints(&size_constraints) {
            let mut required_width = 0.0_f64;
            let mut required_height = 0.0_f64;
            let mut width_requested = false;
            let mut height_requested = false;

            if include_node_labels || minimum_size_accounts_for_padding {
                let only_center = minimum_size_accounts_for_padding && !include_node_labels;
                let inside_min_size = inside_layout.compute_minimum_size(only_center);
                required_width = required_width.max(inside_min_size.x);
                required_height = required_height.max(inside_min_size.y);
                width_requested = true;
                height_requested = true;
            }

            if include_node_labels && !outside_overhang {
                required_width = required_width
                    .max(north_labels.min_width())
                    .max(south_labels.min_width());
                required_height = required_height
                    .max(east_labels.min_height())
                    .max(west_labels.min_height());
                width_requested = true;
                height_requested = true;
            }

            if size_constraints.contains(&SizeConstraint::MinimumSize)
                && !minimum_size_accounts_for_padding
            {
                let minimum_size = configured_minimum_size(node, &size_options);
                required_width = required_width.max(minimum_size.x);
                required_height = required_height.max(minimum_size.y);
                width_requested = true;
                height_requested = true;
            }

            // Apply port-driven minimum height for EAST/WEST ports. This mirrors Java's
            // VerticalPortPlacementSizeCalculator which sets cell heights through the cell
            // system. We only apply this when height_requested is already true (i.e., other
            // size constraints like NodeLabels or MinimumSize are active) to avoid overriding
            // the initial_node_size fallback for nodes that rely solely on PORTS constraint.
            if size_constraints.contains(&SizeConstraint::Ports) && height_requested {
                let port_constraints = node
                    .get_property(CoreOptions::PORT_CONSTRAINTS)
                    .unwrap_or(PortConstraints::Undefined);
                if port_constraints != PortConstraints::FixedPos
                    && port_constraints != PortConstraints::FixedRatio
                {
                    let east_height = Self::vertical_port_size_for_free_side(
                        node,
                        PortSide::East,
                        &size_constraints,
                        &port_labels_placement,
                    );
                    let west_height = Self::vertical_port_size_for_free_side(
                        node,
                        PortSide::West,
                        &size_constraints,
                        &port_labels_placement,
                    );
                    let port_height = east_height.max(west_height);
                    required_height = required_height.max(port_height);
                }
            }

            if !width_requested {
                required_width = initial_node_size.x;
            }
            if !height_requested {
                required_height = initial_node_size.y;
            }

            if topdown_layout {
                required_width = required_width.max(initial_node_size.x);
                required_height = required_height.max(initial_node_size.y);
            }

            let node_size_fixed_graph_size = node
                .get_graph()
                .and_then(|graph| graph.get_property(CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE))
                .unwrap_or(false);

            target_node_size.x = if node_size_fixed_graph_size {
                initial_node_size.x.max(required_width)
            } else {
                required_width
            };
            target_node_size.y = if node_size_fixed_graph_size {
                initial_node_size.y.max(required_height)
            } else {
                required_height
            };
            node.set_size(target_node_size);
        }

        let final_node_size = node.get_size();
        let center_cell_rect = inside_layout.apply_layout(final_node_size);
        place_outside_container(
            &north_labels,
            OutsideSide::North,
            final_node_size,
            outside_overhang,
        );
        place_outside_container(
            &south_labels,
            OutsideSide::South,
            final_node_size,
            outside_overhang,
        );
        place_outside_container(
            &west_labels,
            OutsideSide::West,
            final_node_size,
            outside_overhang,
        );
        place_outside_container(
            &east_labels,
            OutsideSide::East,
            final_node_size,
            outside_overhang,
        );

        if trace_labels {
            for label in node.get_labels() {
                let pos = label.get_position();
                let size = label.get_size();
                let placement = if label.has_property(CoreOptions::NODE_LABELS_PLACEMENT) {
                    label
                        .get_property(CoreOptions::NODE_LABELS_PLACEMENT)
                        .unwrap_or_else(|| default_label_placement.clone())
                } else {
                    default_label_placement.clone()
                };
                eprintln!(
                    "TRACE label(after): text='{}' placement={:?} pos=({}, {}) size=({}, {}) node_size=({}, {})",
                    label.get_text(),
                    placement,
                    pos.x,
                    pos.y,
                    size.x,
                    size.y,
                    final_node_size.x,
                    final_node_size.y
                );
            }
        }

        if size_options.contains(&SizeOptions::ComputePadding) {
            let mut computed_padding = ElkPadding::new();
            computed_padding.left = center_cell_rect.x.max(0.0);
            computed_padding.top = center_cell_rect.y.max(0.0);
            computed_padding.right =
                (final_node_size.x - (center_cell_rect.x + center_cell_rect.width)).max(0.0);
            computed_padding.bottom =
                (final_node_size.y - (center_cell_rect.y + center_cell_rect.height)).max(0.0);
            node.set_padding(computed_padding);
        }
    }

    fn setup_node_padding_for_ports_with_offset<N, T>(
        inside_layout: &mut InsideLabelLayoutGrid,
        node: &N,
        port_labels_placement: &EnumSet<PortLabelPlacement>,
        size_options: &EnumSet<SizeOptions>,
    ) where
        T: 'static,
        N: NodeAdapter<T>,
        N::Graph: GraphElementAdapter<T>,
        N::Label: 'static,
        N::LabelAdapter: 'static,
    {
        let node_cell_padding = &mut inside_layout.node_padding;
        let symmetrical = !size_options.contains(&SizeOptions::Asymmetrical);

        for port in node.get_ports() {
            let port_border_offset = port
                .get_property(CoreOptions::PORT_BORDER_OFFSET)
                .unwrap_or(0.0);
            if std::env::var("DEBUG_NODE_LABEL_PADDING").is_ok() {
                eprintln!(
                    "[NODE_PADDING_DEBUG] node={} port={} side={:?} border_offset={:.3}",
                    node.get_volatile_id(),
                    port.get_volatile_id(),
                    port.get_side(),
                    port_border_offset
                );
            }
            if port_border_offset < 0.0 {
                let extension = -port_border_offset;
                match port.get_side() {
                    PortSide::North => {
                        node_cell_padding.top = node_cell_padding.top.max(extension);
                    }
                    PortSide::South => {
                        node_cell_padding.bottom = node_cell_padding.bottom.max(extension);
                    }
                    PortSide::East => {
                        node_cell_padding.right = node_cell_padding.right.max(extension);
                    }
                    PortSide::West | PortSide::Undefined => {
                        node_cell_padding.left = node_cell_padding.left.max(extension);
                    }
                }
            }

            if !PortLabelPlacement::is_fixed(port_labels_placement) {
                continue;
            }

            let mut min_x = 0.0;
            let mut min_y = 0.0;
            let mut max_x = 0.0;
            let mut max_y = 0.0;
            let mut has_label = false;

            for label in port.get_labels() {
                let pos = label.get_position();
                let size = label.get_size();
                let x1 = pos.x;
                let y1 = pos.y;
                let x2 = pos.x + size.x;
                let y2 = pos.y + size.y;

                if !has_label {
                    min_x = x1;
                    min_y = y1;
                    max_x = x2;
                    max_y = y2;
                    has_label = true;
                } else {
                    min_x = min_x.min(x1);
                    min_y = min_y.min(y1);
                    max_x = max_x.max(x2);
                    max_y = max_y.max(y2);
                }
            }

            if !has_label {
                continue;
            }

            let port_size = port.get_size();
            let inside_part = ElkUtil::compute_inside_part(
                &KVector::with_values(min_x, min_y),
                &KVector::with_values(max_x - min_x, max_y - min_y),
                &port_size,
                port_border_offset,
                port.get_side(),
            );

            if std::env::var("DEBUG_NODE_LABEL_PADDING").is_ok() {
                eprintln!(
                    "[NODE_PADDING_DEBUG] node={} port={} inside_part={:.3} port_size=({}, {}) label_bbox_min=({}, {}) label_bbox_size=({}, {})",
                    node.get_volatile_id(),
                    port.get_volatile_id(),
                    inside_part,
                    port_size.x,
                    port_size.y,
                    min_x,
                    min_y,
                    max_x - min_x,
                    max_y - min_y
                );
            }

            match port.get_side() {
                PortSide::North => {
                    let inside_part_is_bigger = inside_part > node_cell_padding.top;
                    node_cell_padding.top = node_cell_padding.top.max(inside_part);
                    if symmetrical && inside_part_is_bigger {
                        node_cell_padding.top = node_cell_padding.top.max(node_cell_padding.bottom);
                        node_cell_padding.bottom = node_cell_padding.top + port_border_offset;
                    }
                }
                PortSide::South => {
                    let inside_part_is_bigger = inside_part > node_cell_padding.bottom;
                    node_cell_padding.bottom = node_cell_padding.bottom.max(inside_part);
                    if symmetrical && inside_part_is_bigger {
                        node_cell_padding.bottom = node_cell_padding.bottom.max(node_cell_padding.top);
                        node_cell_padding.top = node_cell_padding.bottom + port_border_offset;
                    }
                }
                PortSide::East => {
                    let inside_part_is_bigger = inside_part > node_cell_padding.right;
                    node_cell_padding.right = node_cell_padding.right.max(inside_part);
                    if symmetrical && inside_part_is_bigger {
                        node_cell_padding.right = node_cell_padding.right.max(node_cell_padding.left);
                        node_cell_padding.left = node_cell_padding.right + port_border_offset;
                    }
                }
                PortSide::West | PortSide::Undefined => {
                    let inside_part_is_bigger = inside_part > node_cell_padding.left;
                    node_cell_padding.left = node_cell_padding.left.max(inside_part);
                    if symmetrical && inside_part_is_bigger {
                        node_cell_padding.left = node_cell_padding.left.max(node_cell_padding.right);
                        node_cell_padding.right = node_cell_padding.left + port_border_offset;
                    }
                }
            }
        }

        if std::env::var("DEBUG_NODE_LABEL_PADDING").is_ok() {
            eprintln!(
                "[NODE_PADDING_DEBUG] node={} after_setup node_cell_padding={:?} node_label_cell_padding={:?}",
                node.get_volatile_id(),
                inside_layout.node_padding,
                inside_layout.node_label_padding
            );
        }
    }

    /// Computes the minimum height for ports on one side (EAST or WEST) assuming free port placement.
    /// This mirrors Java's `VerticalPortPlacementSizeCalculator.calculateVerticalNodeSizeRequiredByFreePorts()`.
    fn vertical_port_size_for_free_side<N, T>(
        node: &N,
        side: PortSide,
        size_constraints: &EnumSet<SizeConstraint>,
        port_labels_placement: &EnumSet<PortLabelPlacement>,
    ) -> f64
    where
        T: 'static,
        N: NodeAdapter<T>,
        N::Graph: GraphElementAdapter<T>,
        N::Label: 'static,
        N::LabelAdapter: 'static,
    {
        let ports: Vec<_> = node
            .get_ports()
            .into_iter()
            .filter(|p| p.get_side() == side)
            .collect();
        if ports.is_empty() {
            return 0.0;
        }

        let port_port_spacing =
            IndividualSpacings::get_individual_or_inherited_adapter(
                node,
                CoreOptions::SPACING_PORT_PORT,
            )
            .unwrap_or(10.0)
            .max(0.0);
        let surrounding: ElkMargin =
            IndividualSpacings::get_individual_or_inherited_adapter(
                node,
                CoreOptions::SPACING_PORTS_SURROUNDING,
            )
            .unwrap_or_default();
        let port_label_spacing_v =
            IndividualSpacings::get_individual_or_inherited_adapter(
                node,
                CoreOptions::SPACING_LABEL_PORT_VERTICAL,
            )
            .unwrap_or(1.0);
        let treat_as_group = node
            .get_property(CoreOptions::PORT_LABELS_TREAT_AS_GROUP)
            .unwrap_or(true);
        let port_labels_outside = port_labels_placement.contains(&PortLabelPlacement::Outside);
        let next_to_port_if_possible = port_labels_placement
            .contains(&PortLabelPlacement::NextToPortIfPossible);
        let always_same_side = port_labels_placement
            .contains(&PortLabelPlacement::AlwaysSameSide);
        let always_other_same_side = port_labels_placement
            .contains(&PortLabelPlacement::AlwaysOtherSameSide);
        let space_efficient = port_labels_placement.contains(&PortLabelPlacement::SpaceEfficient);
        let space_efficient_port_labels =
            !always_same_side && !always_other_same_side && (space_efficient || ports.len() == 2);

        let include_port_labels = size_constraints.contains(&SizeConstraint::PortLabels);

        // Determine if labels on this side are placed "next to port" (beside it vertically)
        // For EAST/WEST ports with outside labels and NEXT_TO_PORT_IF_POSSIBLE, labels sit beside.
        let labels_next_to_port = if port_labels_outside {
            next_to_port_if_possible && (side == PortSide::East || side == PortSide::West)
        } else {
            // Inside labels on EAST/WEST are placed next to port
            port_labels_placement.contains(&PortLabelPlacement::Inside)
                && (side == PortSide::East || side == PortSide::West)
        };

        // Compute per-port top/bottom margins from labels
        struct PortMargin {
            top: f64,
            bottom: f64,
        }
        let mut margins: Vec<PortMargin> = Vec::with_capacity(ports.len());

        for port in &ports {
            let mut margin = PortMargin {
                top: 0.0,
                bottom: 0.0,
            };

            if include_port_labels {
                let labels = port.get_labels();
                let label_height: f64 = if labels.is_empty() {
                    0.0
                } else {
                    labels.iter().map(|l| l.get_size().y).sum::<f64>()
                        + port_label_spacing_v * (labels.len() as f64 - 1.0).max(0.0)
                };

                if label_height > 0.0 {
                    if labels_next_to_port {
                        let port_height = port.get_size().y;
                        if label_height > port_height {
                            if treat_as_group || labels.len() == 1 {
                                let overhang = (label_height - port_height) / 2.0;
                                margin.top = overhang;
                                margin.bottom = overhang;
                            } else {
                                let first_label_height = labels[0].get_size().y;
                                let first_label_overhang =
                                    (first_label_height - port_height) / 2.0;
                                margin.top = first_label_overhang.max(0.0);
                                margin.bottom =
                                    label_height - first_label_overhang - port_height;
                            }
                        }
                    } else {
                        // Label placed below the port
                        margin.bottom = port_label_spacing_v + label_height;
                    }
                }
            }

            margins.push(margin);
        }

        // For outside placement: topmost port margin.top = 0, bottommost port margin.bottom = 0
        if port_labels_outside && !margins.is_empty() {
            margins[0].top = 0.0;
            let last = margins.len() - 1;
            margins[last].bottom = 0.0;

            // Space-efficient: if topmost port's labels are NOT next to port, clear its bottom margin
            if space_efficient_port_labels && !labels_next_to_port {
                margins[0].bottom = 0.0;
            }
        }

        // Sum port heights + margins + inter-port spacing
        let mut height = 0.0_f64;
        for (i, port) in ports.iter().enumerate() {
            height += margins[i].top + port.get_size().y + margins[i].bottom;
            if i + 1 < ports.len() {
                height += port_port_spacing;
            }
        }

        // Add surrounding port margins (top/bottom for EAST/WEST)
        let (surr_top, surr_bottom) = match side {
            PortSide::East | PortSide::West => (surrounding.top, surrounding.bottom),
            _ => (0.0, 0.0),
        };
        height += surr_top + surr_bottom;

        // For DISTRIBUTED alignment, add 2 * port_port_spacing
        let port_alignment = match side {
            PortSide::East => node.get_property(CoreOptions::PORT_ALIGNMENT_EAST),
            PortSide::West => node.get_property(CoreOptions::PORT_ALIGNMENT_WEST),
            _ => None,
        }
        .or_else(|| node.get_property(CoreOptions::PORT_ALIGNMENT_DEFAULT))
        .unwrap_or(PortAlignment::Justified);

        if port_alignment == PortAlignment::Distributed {
            height += 2.0 * port_port_spacing;
        }

        height
    }

    fn setup_inside_port_label_cell_minimums<N, T>(
        inside_layout: &mut InsideLabelLayoutGrid,
        node: &N,
        port_labels_placement: &EnumSet<PortLabelPlacement>,
    ) where
        T: 'static,
        N: NodeAdapter<T>,
        N::Graph: GraphElementAdapter<T>,
        N::Label: 'static,
        N::LabelAdapter: 'static,
    {
        if !port_labels_placement.contains(&PortLabelPlacement::Inside) {
            return;
        }

        let label_spacing = node
            .get_property(CoreOptions::SPACING_LABEL_PORT_HORIZONTAL)
            .unwrap_or(0.0);
        let size_constraints = node
            .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
            .unwrap_or_default();
        let allow_inside_spacing_compensation =
            !size_constraints.contains(&SizeConstraint::MinimumSize);
        let port_spacing = node
            .get_property(CoreOptions::SPACING_PORT_PORT)
            .unwrap_or(0.0)
            .max(0.0);
        let node_label_spacing = node
            .get_property(CoreOptions::SPACING_LABEL_NODE)
            .unwrap_or(0.0)
            .max(0.0);
        let mut east_width = 0.0_f64;
        let mut west_width = 0.0_f64;
        let mut north_width = 0.0_f64;
        let mut south_width = 0.0_f64;
        let mut east_border_extension = 0.0_f64;
        let mut west_border_extension = 0.0_f64;

        for port in node.get_ports() {
            let max_label_width = port
                .get_labels()
                .iter()
                .map(|label| label.get_size().x)
                .fold(0.0_f64, f64::max);

            if max_label_width <= 0.0 {
                continue;
            }

            let border_extension = (-port
                .get_property(CoreOptions::PORT_BORDER_OFFSET)
                .unwrap_or(0.0))
            .max(0.0);

            match port.get_side() {
                PortSide::East => {
                    east_width = east_width.max(max_label_width);
                    east_border_extension = east_border_extension.max(border_extension);
                }
                PortSide::North => {
                    north_width = north_width.max(max_label_width);
                }
                PortSide::South => {
                    south_width = south_width.max(max_label_width);
                }
                PortSide::West | PortSide::Undefined => {
                    west_width = west_width.max(max_label_width);
                    west_border_extension = west_border_extension.max(border_extension);
                }
            }
        }

        if allow_inside_spacing_compensation {
            if north_width > 0.0 {
                north_width += port_spacing + 2.0 * label_spacing + 3.0;
            }
            if south_width > 0.0 {
                south_width += port_spacing + 2.0 * label_spacing + 3.0;
            }
            if east_width > 0.0 {
                east_width += port_spacing + node_label_spacing + east_border_extension + 4.0;
            }
            if west_width > 0.0 {
                west_width += port_spacing + node_label_spacing + west_border_extension + 4.0;
            }
        }

        if east_width > 0.0 {
            inside_layout.ensure_cell_min_width(
                ContainerArea::Center,
                ContainerArea::End,
                east_width + label_spacing,
            );
        }

        if west_width > 0.0 {
            inside_layout.ensure_cell_min_width(
                ContainerArea::Center,
                ContainerArea::Begin,
                west_width + label_spacing,
            );
        }

        if north_width > 0.0 {
            inside_layout.ensure_cell_min_width(
                ContainerArea::Begin,
                ContainerArea::Center,
                north_width + label_spacing,
            );
        }

        if south_width > 0.0 {
            inside_layout.ensure_cell_min_width(
                ContainerArea::End,
                ContainerArea::Center,
                south_width + label_spacing,
            );
        }
    }

    pub fn compute_inside_node_label_padding<N, T>(
        node: &N,
        layout_direction: Direction,
    ) -> ElkPadding
    where
        T: 'static,
        N: NodeAdapter<T>,
        N::Graph: GraphElementAdapter<T>,
        N::LabelAdapter: 'static,
    {
        let grid = InsideNodeLabelGrid::new(node, layout_direction);
        grid.compute_inside_padding()
    }

    pub fn compute_inside_node_label_container_minimum_size<N, T>(
        node: &N,
        layout_direction: Direction,
    ) -> KVector
    where
        T: 'static,
        N: NodeAdapter<T>,
        N::Graph: GraphElementAdapter<T>,
        N::LabelAdapter: 'static,
    {
        let grid = InsideNodeLabelGrid::new(node, layout_direction);
        let size_constraints = node
            .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
            .unwrap_or_default();
        let size_options = node
            .get_property(CoreOptions::NODE_SIZE_OPTIONS)
            .unwrap_or_default();

        let only_center_cell_contributes = size_constraints.contains(&SizeConstraint::MinimumSize)
            && size_options.contains(&SizeOptions::MinimumSizeAccountsForPadding)
            && !size_constraints.contains(&SizeConstraint::NodeLabels);

        let mut min_size = grid.compute_minimum_size(only_center_cell_contributes);
        if grid.tabular {
            // Tabular mode keeps per-column widths consistent across rows.
            let widths = grid.min_column_widths(None);
            let tabular_width = sum_with_gaps_with_gap(widths, grid.container_gap);
            if tabular_width > 0.0 {
                min_size.x = tabular_width + grid.padding.left + grid.padding.right;
            }
        } else if !only_center_cell_contributes {
            let mut non_tabular_width = 0.0_f64;
            for row in ContainerArea::values() {
                non_tabular_width = non_tabular_width.max(sum_with_gaps_with_gap(
                    grid.min_column_widths(Some(row)),
                    grid.container_gap,
                ));
            }
            if non_tabular_width > 0.0 {
                min_size.x = non_tabular_width + grid.padding.left + grid.padding.right;
            }
        }

        if !only_center_cell_contributes {
            let row_heights = grid.min_row_heights();
            let height = sum_with_gaps_with_gap(row_heights, grid.container_gap);
            if height > 0.0 {
                min_size.y = height + grid.padding.top + grid.padding.bottom;
            }
        }

        min_size
    }
}
