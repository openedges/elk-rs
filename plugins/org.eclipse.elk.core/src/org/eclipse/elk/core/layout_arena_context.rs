//! Thread-local ElkGraphArenaSync for layout-scoped arena access.
//!
//! Set by RecursiveGraphLayoutEngine before layout, cleared after.
//! Consumers call `with_layout_arena(|sync| ...)` to access the arena
//! without requiring signature changes throughout the call chain.

use std::cell::RefCell;

use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkGraphArenaSync, ElkNodeRef};

thread_local! {
    static LAYOUT_ARENA: RefCell<Option<ElkGraphArenaSync>> = const { RefCell::new(None) };
}

/// Initialize the layout arena from an ElkGraph root.
/// Call at the start of layout; the arena is valid until `clear_layout_arena()`.
pub fn init_layout_arena(root: &ElkNodeRef) {
    let sync = ElkGraphArenaSync::from_root(root);
    LAYOUT_ARENA.with(|cell| {
        *cell.borrow_mut() = Some(sync);
    });
}

/// Clear the layout arena. Call at the end of layout.
pub fn clear_layout_arena() {
    LAYOUT_ARENA.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

/// Access the layout arena if available. Returns `None` if not in a layout context.
pub fn with_layout_arena<R>(f: impl FnOnce(&ElkGraphArenaSync) -> R) -> Option<R> {
    LAYOUT_ARENA.with(|cell| {
        let borrow = cell.borrow();
        borrow.as_ref().map(f)
    })
}
