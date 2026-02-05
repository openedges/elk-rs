use std::sync::{Arc, Mutex, Weak};

pub mod dc_component;
pub mod dc_direction;
pub mod dc_element;
pub mod dc_extension;
pub mod dc_graph;

pub use dc_component::DCComponent;
pub use dc_direction::DCDirection;
pub use dc_element::DCElement;
pub use dc_extension::DCExtension;
pub use dc_graph::DCGraph;

pub type DCElementRef = Arc<Mutex<DCElement>>;
pub type DCComponentRef = Arc<Mutex<DCComponent>>;
pub type DCGraphRef = Arc<Mutex<DCGraph>>;
pub type DCComponentWeak = Weak<Mutex<DCComponent>>;
