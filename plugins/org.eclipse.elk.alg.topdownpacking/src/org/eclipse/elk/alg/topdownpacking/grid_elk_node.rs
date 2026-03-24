use std::fmt;

use org_eclipse_elk_core::org::eclipse::elk::core::layout_arena_context::with_layout_arena;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use super::grid::Grid;

#[derive(Clone)]
pub struct GridElkNode {
    node: ElkNodeRef,
    grid: Vec<Vec<Option<ElkNodeRef>>>,
    rows: usize,
    cols: usize,
}

impl GridElkNode {
    pub fn new(node: ElkNodeRef) -> Self {
        GridElkNode {
            node,
            grid: Vec::new(),
            rows: 0,
            cols: 0,
        }
    }

    pub fn node(&self) -> &ElkNodeRef {
        &self.node
    }

    pub fn node_ref(&self) -> ElkNodeRef {
        self.node.clone()
    }

    pub fn identifier(&self) -> String {
        let mut node = self.node.borrow_mut();
        node.connectable()
            .shape()
            .graph_element()
            .identifier()
            .unwrap_or("")
            .to_string()
    }

    pub fn width(&self) -> f64 {
        let mut node = self.node.borrow_mut();
        node.connectable().shape().width()
    }

    pub fn height(&self) -> f64 {
        let mut node = self.node.borrow_mut();
        node.connectable().shape().height()
    }

    pub fn x(&self) -> f64 {
        let mut node = self.node.borrow_mut();
        node.connectable().shape().x()
    }

    pub fn y(&self) -> f64 {
        let mut node = self.node.borrow_mut();
        node.connectable().shape().y()
    }

    pub fn set_dimensions(&self, width: f64, height: f64) {
        let mut node = self.node.borrow_mut();
        node.connectable().shape().set_dimensions(width, height);
    }

    pub fn set_location(&self, x: f64, y: f64) {
        let mut node = self.node.borrow_mut();
        node.connectable().shape().set_location(x, y);
    }

    pub fn children(&self) -> Vec<ElkNodeRef> {
        if let Some(children) = with_layout_arena(|sync| {
            sync.node_id(&self.node).map(|nid|
                sync.arena().node_children[nid.idx()]
                    .iter().map(|&cid| sync.node_ref(cid).clone()).collect::<Vec<_>>())
        }).flatten() {
            return children;
        }
        let mut node = self.node.borrow_mut();
        node.children().iter().cloned().collect()
    }

    pub fn get_property<T: Clone + Send + Sync + 'static>(
        &self,
        property: &Property<T>,
    ) -> Option<T> {
        if let Some(result) = with_layout_arena(|sync| {
            sync.node_id(&self.node)
                .and_then(|nid| sync.arena().node_properties[nid.idx()].get_property(property))
        }).flatten() {
            return Some(result);
        }
        let mut node = self.node.borrow_mut();
        node.connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .get_property(property)
    }

    pub fn set_property<T: Clone + Send + Sync + 'static>(
        &self,
        property: &Property<T>,
        value: Option<T>,
    ) {
        let mut node = self.node.borrow_mut();
        node.connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(property, value);
    }

    pub fn columns(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    fn ensure_bounds(&self, col: usize, row: usize) {
        if col >= self.cols || row >= self.rows {
            panic!(
                "The grid has a size of ({} x {}). The requested index of ({} x {}) is out of bounds.",
                self.cols, self.rows, col, row
            );
        }
    }
}

impl Grid<ElkNodeRef> for GridElkNode {
    fn put(&mut self, col: usize, row: usize, item: ElkNodeRef) {
        self.ensure_bounds(col, row);
        let row_ref = self.grid.get_mut(row).expect("row exists");
        row_ref.insert(col, Some(item));
    }

    fn get(&self, col: usize, row: usize) -> Option<ElkNodeRef> {
        self.ensure_bounds(col, row);
        self.grid
            .get(row)
            .and_then(|row_ref| row_ref.get(col).cloned())
            .unwrap_or(None)
    }

    fn get_row(&self, row: usize) -> Vec<Option<ElkNodeRef>> {
        if row >= self.rows {
            panic!(
                "The grid has a size of ({} x {}). The requested row {} is out of bounds.",
                self.cols, self.rows, row
            );
        }
        self.grid.get(row).cloned().unwrap_or_default()
    }

    fn get_column(&self, col: usize) -> Vec<Option<ElkNodeRef>> {
        if col >= self.cols {
            panic!(
                "The grid has a size of ({} x {}). The requested column {} is out of bounds.",
                self.cols, self.rows, col
            );
        }
        let mut result = Vec::with_capacity(self.rows);
        for row in &self.grid {
            result.push(row.get(col).cloned().unwrap_or(None));
        }
        result
    }

    fn columns(&self) -> usize {
        self.cols
    }

    fn rows(&self) -> usize {
        self.rows
    }

    fn set_grid_size(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;
        self.grid = vec![vec![None; cols]; rows];
    }
}

impl fmt::Debug for GridElkNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GridElkNode")
            .field("identifier", &self.identifier())
            .field("rows", &self.rows)
            .field("cols", &self.cols)
            .finish()
    }
}
