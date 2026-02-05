use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::util::RectRowRef;

thread_local! {
    static ROWS_STORAGE: RefCell<HashMap<usize, Vec<RectRowRef>>> = RefCell::new(HashMap::new());
}

fn graph_key(graph: &ElkNodeRef) -> usize {
    Rc::as_ptr(graph) as usize
}

pub fn store_rows(graph: &ElkNodeRef, rows: Vec<RectRowRef>) -> usize {
    let key = graph_key(graph);
    ROWS_STORAGE.with(|storage| {
        storage.borrow_mut().insert(key, rows);
    });
    key
}

pub fn take_rows(key: usize) -> Option<Vec<RectRowRef>> {
    ROWS_STORAGE.with(|storage| storage.borrow_mut().remove(&key))
}

pub fn get_rows(key: usize) -> Option<Vec<RectRowRef>> {
    ROWS_STORAGE.with(|storage| storage.borrow().get(&key).cloned())
}
