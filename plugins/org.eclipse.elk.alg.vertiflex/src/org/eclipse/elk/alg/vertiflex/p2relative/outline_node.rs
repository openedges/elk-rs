#[derive(Clone, Debug)]
pub struct OutlineNode {
    relative_x: f64,
    absolute_y: f64,
    next: Option<Box<OutlineNode>>,
}

impl OutlineNode {
    pub fn new(relative_x: f64, absolute_y: f64, next: Option<OutlineNode>) -> Self {
        OutlineNode {
            relative_x,
            absolute_y,
            next: next.map(Box::new),
        }
    }

    pub fn absolute_y(&self) -> f64 {
        self.absolute_y
    }

    pub fn set_absolute_y(&mut self, absolute_y: f64) {
        self.absolute_y = absolute_y;
    }

    pub fn relative_x(&self) -> f64 {
        self.relative_x
    }

    pub fn set_relative_x(&mut self, relative_x: f64) {
        self.relative_x = relative_x;
    }

    pub fn next(&self) -> Option<&OutlineNode> {
        self.next.as_deref()
    }

    pub fn next_mut(&mut self) -> Option<&mut OutlineNode> {
        self.next.as_deref_mut()
    }

    pub fn set_next(&mut self, next: Option<OutlineNode>) {
        self.next = next.map(Box::new);
    }

    pub fn next_cloned(&self) -> Option<OutlineNode> {
        self.next.as_deref().cloned()
    }

    pub fn is_last(&self) -> bool {
        self.next.is_none()
    }
}
