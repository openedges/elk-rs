/// Shared random-call tracing infrastructure for crossing minimization.
///
/// Enable with the environment variable `CROSSMIN_RANDOM_TRACE=1`.
/// Each random call prints a line to stderr:
///
///   [random #N] method() = value @ location_description
///
/// The counter is a global atomic so it accumulates across all call sites in
/// the same process, giving a total ordering that can be compared with Java.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::LazyLock;

static COUNTER: AtomicUsize = AtomicUsize::new(0);
static CROSSMIN_RANDOM_TRACE: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("CROSSMIN_RANDOM_TRACE").is_some());

/// Returns `true` when `CROSSMIN_RANDOM_TRACE` is set in the environment.
#[inline]
pub fn is_enabled() -> bool {
    *CROSSMIN_RANDOM_TRACE
}

/// Reset the counter to zero (call once at the start of a new graph layout).
pub fn reset_counter() {
    COUNTER.store(0, Ordering::Relaxed);
}

/// Emit one trace line and return the value unchanged.
///
/// `method`   – e.g. `"next_long"`
/// `value`    – formatted representation of the produced value
/// `location` – description of the call site / purpose
#[inline]
pub fn trace(method: &str, value: &str, location: &str) {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    eprintln!("[random #{n}] {method}() = {value} @ {location}");
}

/// Convenience wrappers that return the value after tracing it.
pub fn trace_next_long(value: i64, location: &str) -> i64 {
    if is_enabled() {
        trace("next_long", &value.to_string(), location);
    }
    value
}

pub fn trace_next_boolean(value: bool, location: &str) -> bool {
    if is_enabled() {
        trace("next_boolean", &value.to_string(), location);
    }
    value
}

pub fn trace_next_float(value: f64, location: &str) -> f64 {
    if is_enabled() {
        trace("next_float", &format!("{value:.7}"), location);
    }
    value
}

pub fn trace_next_double(value: f64, location: &str) -> f64 {
    if is_enabled() {
        trace("next_double", &format!("{value:.7}"), location);
    }
    value
}

pub fn trace_next_int(value: i32, location: &str) -> i32 {
    if is_enabled() {
        trace("next_int", &value.to_string(), location);
    }
    value
}
