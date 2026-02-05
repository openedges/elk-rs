pub trait IThreeValueGrid {
    fn get_width(&self) -> usize;
    fn get_height(&self) -> usize;
    fn is_empty(&self, x: usize, y: usize) -> bool;
    fn is_blocked(&self, x: usize, y: usize) -> bool;
    fn is_weakly_blocked(&self, x: usize, y: usize) -> bool;
    fn in_bounds(&self, x: usize, y: usize) -> bool;
    fn reinitialize(&mut self, width: usize, height: usize);
    fn set_empty(&mut self, x: usize, y: usize);
    fn set_blocked(&mut self, x: usize, y: usize);
    fn set_weakly_blocked(&mut self, x: usize, y: usize);
}
