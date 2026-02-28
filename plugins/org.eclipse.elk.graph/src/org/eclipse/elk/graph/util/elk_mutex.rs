use std::fmt;
use std::ops::{Deref, DerefMut};

/// A Mutex wrapper around `parking_lot::Mutex` that provides the same API as
/// `std::sync::Mutex` (returning `Result` from `lock()`), but uses parking_lot
/// for better performance on uncontended locks.
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
    /// Returns `Result` for API compatibility with `std::sync::Mutex::lock()`.
    #[inline]
    pub fn lock(&self) -> Result<MutexGuard<'_, T>, std::sync::PoisonError<MutexGuard<'_, T>>> {
        Ok(MutexGuard(self.0.lock()))
    }

    /// Attempts to acquire the mutex without blocking.
    /// Returns `Ok(MutexGuard)` if successful, `Err(TryLockError::WouldBlock)` otherwise.
    #[inline]
    pub fn try_lock(&self) -> Result<MutexGuard<'_, T>, std::sync::TryLockError<MutexGuard<'_, T>>> {
        match self.0.try_lock() {
            Some(guard) => Ok(MutexGuard(guard)),
            None => Err(std::sync::TryLockError::WouldBlock),
        }
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
        match self.0.try_lock() {
            Some(guard) => f.debug_tuple("Mutex").field(&&*guard).finish(),
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
