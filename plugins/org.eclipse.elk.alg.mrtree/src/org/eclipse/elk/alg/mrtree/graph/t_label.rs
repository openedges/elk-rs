use std::fmt;
use std::sync::{Arc, Weak};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use super::{TEdgeRef, TShape};

pub type TLabelRef = Arc<Mutex<TLabel>>;

pub struct TLabel {
    shape: TShape,
    edge: Weak<Mutex<super::TEdge>>,
    text: Option<String>,
}

impl TLabel {
    pub fn new(edge: &TEdgeRef, text: impl Into<String>) -> TLabelRef {
        let label = Arc::new(Mutex::new(TLabel {
            shape: TShape::default(),
            edge: Arc::downgrade(edge),
            text: Some(text.into()),
        }));
        if let Some(mut edge_guard) = edge.lock_ok() {
            edge_guard.labels_mut().push(label.clone());
        }
        label
    }

    pub fn shape(&mut self) -> &mut TShape {
        &mut self.shape
    }

    pub fn shape_ref(&self) -> &TShape {
        &self.shape
    }

    pub fn edge(&self) -> Option<TEdgeRef> {
        self.edge.upgrade()
    }

    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }
}

impl fmt::Display for TLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(text) = self.text.as_ref() {
            if text.is_empty() {
                if let Some(edge) = self.edge() {
                    if let Some(edge_guard) = edge.lock_ok() {
                        return write!(f, "l[{}]", *edge_guard);
                    }
                }
                return write!(f, "l[]");
            }
            write!(f, "l_{text}")
        } else if let Some(edge) = self.edge() {
            if let Some(edge_guard) = edge.lock_ok() {
                write!(f, "l[{}]", *edge_guard)
            } else {
                write!(f, "l[]")
            }
        } else {
            write!(f, "l[]")
        }
    }
}
