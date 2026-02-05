use std::fmt::Debug;

pub trait Grid<T: Clone>: Debug {
    fn put(&mut self, col: usize, row: usize, item: T);
    fn get(&self, col: usize, row: usize) -> Option<T>;
    fn get_row(&self, row: usize) -> Vec<Option<T>>;
    fn get_column(&self, col: usize) -> Vec<Option<T>>;
    fn columns(&self) -> usize;
    fn rows(&self) -> usize;
    fn set_grid_size(&mut self, cols: usize, rows: usize);
}
