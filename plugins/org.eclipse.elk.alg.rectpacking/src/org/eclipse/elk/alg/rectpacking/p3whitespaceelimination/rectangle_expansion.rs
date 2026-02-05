use crate::org::eclipse::elk::alg::rectpacking::util::RectRowRef;

pub struct RectangleExpansion;

impl RectangleExpansion {
    pub fn expand(
        rows: &[RectRowRef],
        drawing_width: f64,
        additional_height: f64,
        _node_node_spacing: f64,
    ) {
        if rows.is_empty() {
            return;
        }
        let height_per_row = additional_height / rows.len() as f64;
        for (index, row) in rows.iter().enumerate() {
            let mut row_mut = row.borrow_mut();
            let new_y = row_mut.y() + height_per_row * index as f64;
            row_mut.set_y(new_y);
            row_mut.expand(drawing_width, height_per_row);
        }
    }
}
