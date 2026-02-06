use org_eclipse_elk_core::org::eclipse::elk::core::math::elk_margin::ElkMargin;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::LShape;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    top: f64,
    bottom: f64,
    left: f64,
    right: f64,
}

impl Rectangle {
    pub fn new(left: f64, top: f64, right: f64, bottom: f64) -> Self {
        if top > bottom {
            panic!("Top must be smaller or equal to bottom.");
        }
        if left > right {
            panic!("Left must be smaller or equal to right.");
        }
        Rectangle {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn from_other(rectangle: &Rectangle) -> Self {
        *rectangle
    }

    pub fn from_vector(position: &KVector) -> Self {
        Rectangle {
            top: position.y,
            bottom: position.y,
            right: position.x,
            left: position.x,
        }
    }

    pub fn from_shape(shape: &LShape) -> Self {
        let position = *shape.position_ref();
        let mut extend = position;
        extend.add(shape.size_ref());

        Rectangle {
            top: position.y.min(extend.y),
            bottom: position.y.max(extend.y),
            left: position.x.min(extend.x),
            right: position.x.max(extend.x),
        }
    }

    pub fn from_vectors(vectors: &[KVector]) -> Self {
        if vectors.is_empty() {
            panic!("The list of vectors may not be empty.");
        }
        let mut rect = Rectangle {
            top: f64::MAX,
            bottom: -f64::MAX,
            left: f64::MAX,
            right: -f64::MAX,
        };
        for vector in vectors {
            rect.top = rect.top.min(vector.y);
            rect.right = rect.right.max(vector.x);
            rect.bottom = rect.bottom.max(vector.y);
            rect.left = rect.left.min(vector.x);
        }
        rect
    }

    pub fn from_vectors_iter<'a, I>(vectors: I) -> Self
    where
        I: IntoIterator<Item = &'a KVector>,
    {
        let mut iter = vectors.into_iter();
        let first = iter.next().unwrap_or_else(|| panic!("The list of vectors may not be empty."));
        let mut rect = Rectangle::from_vector(first);
        for vector in iter {
            rect.union_vector(vector);
        }
        rect
    }

    pub fn union_vector(&mut self, vector: &KVector) {
        self.top = self.top.min(vector.y);
        self.right = self.right.max(vector.x);
        self.bottom = self.bottom.max(vector.y);
        self.left = self.left.min(vector.x);
    }

    pub fn union_shape(&mut self, shape: &LShape) {
        self.union_vector(shape.position_ref());
        let mut bottom_right = *shape.position_ref();
        bottom_right.add(shape.size_ref());
        self.union_vector(&bottom_right);
    }

    pub fn union_rectangle(&mut self, rectangle: &Rectangle) {
        self.top = self.top.min(rectangle.top);
        self.right = self.right.max(rectangle.right);
        self.bottom = self.bottom.max(rectangle.bottom);
        self.left = self.left.min(rectangle.left);
    }

    pub fn union(rect1: &Rectangle, rect2: &Rectangle) -> Rectangle {
        let mut ret = Rectangle::from_other(rect1);
        ret.union_rectangle(rect2);
        ret
    }

    pub fn union_all(rectangles: &[Rectangle]) -> Rectangle {
        let first = rectangles
            .first()
            .unwrap_or_else(|| panic!("The list of vectors may not be null."));
        let mut ret = Rectangle::from_other(first);
        for rect in rectangles.iter().skip(1) {
            ret.union_rectangle(rect);
        }
        ret
    }

    pub fn enlarge(&mut self, enlargement: f64) {
        self.top -= enlargement;
        self.left -= enlargement;
        self.right += enlargement;
        self.bottom += enlargement;
    }

    pub fn get_height(&self) -> f64 {
        self.bottom - self.top
    }

    pub fn get_width(&self) -> f64 {
        self.right - self.left
    }

    pub fn get_top(&self) -> f64 {
        self.top
    }

    pub fn get_right(&self) -> f64 {
        self.right
    }

    pub fn get_bottom(&self) -> f64 {
        self.bottom
    }

    pub fn get_left(&self) -> f64 {
        self.left
    }

    pub fn get_from_port_side(&self, side: PortSide) -> f64 {
        match side {
            PortSide::North => self.top,
            PortSide::East => self.right,
            PortSide::South => self.bottom,
            PortSide::West => self.left,
            _ => 0.0,
        }
    }

    pub fn to_node_margins(&self, shape: &LShape) -> ElkMargin {
        let mut ret_val = ElkMargin::new();
        let size = shape.size_ref();
        let shape_rectangle = Rectangle::new(0.0, 0.0, size.x, size.y);
        ret_val.top = (shape_rectangle.top - self.top).max(0.0);
        ret_val.left = (shape_rectangle.left - self.left).max(0.0);
        ret_val.bottom = (self.bottom - shape_rectangle.bottom).max(0.0);
        ret_val.right = (self.right - shape_rectangle.right).max(0.0);
        ret_val
    }
}

impl std::fmt::Display for Rectangle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[top= {:.1},left= {:.1},bottom= {:.1},right= {:.1}]",
            self.top, self.left, self.bottom, self.right
        )
    }
}
