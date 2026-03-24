use std::fmt;
use std::ops::{Deref, DerefMut};

/// A Mutex wrapper around `parking_lot::Mutex` with a simplified API.
///
/// Unlike `std::sync::Mutex`, `lock()` returns `MutexGuard` directly (not
/// `Result`) because parking_lot never poisons.
///
/// parking_lot::Mutex advantages:
/// - No poisoning (lock() never fails)
/// - Faster uncontended lock/unlock (~2-5x on macOS)
/// - Smaller size (1 byte vs 40+ bytes for pthread-based)
pub struct Mutex<T: ?Sized>(parking_lot::Mutex<T>);

pub struct MutexGuard<'a, T: ?Sized>(parking_lot::MutexGuard<'a, T>);

impl<T> Mutex<T> {
    #[inline]
    pub fn new(val: T) -> Self {
        Mutex(parking_lot::Mutex::new(val))
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Acquires the mutex, returning a guard. Always succeeds (no poisoning).
    #[inline]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        MutexGuard(self.0.lock())
    }

    /// Compatibility helper: returns `Some(guard)`.
    /// Use `lock()` directly for new code. This exists only to ease migration
    /// from the old `lock().ok()` pattern.
    #[inline]
    pub fn lock_ok(&self) -> Option<MutexGuard<'_, T>> {
        Some(self.lock())
    }

    /// Attempts to acquire the mutex without blocking.
    /// Returns `Some(MutexGuard)` if successful, `None` otherwise.
    #[inline]
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        self.0.try_lock().map(MutexGuard)
    }
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.try_lock() {            Some(guard) => f.debug_tuple("Mutex").field(&&*guard).finish(),
            None => f.debug_tuple("Mutex").field(&"<locked>").finish(),
        }
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Mutex::new(T::default())
    }
}
