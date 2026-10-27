//! Minimal synchronization primitives.
//!
//! For now this is a basic spinlock, sufficient for the single-core kernel of
//! M0/M1 where interrupts are mostly disabled. Interrupt-safe locking (which
//! must disable interrupts to avoid deadlocking against a handler on the same
//! core) and the fuller set of primitives — mutexes, wait queues — arrive with
//! the scheduler in M4.

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// A mutual-exclusion spinlock protecting a value of type `T`.
pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

// SAFETY: all access to the inner value goes through `lock()`, which serializes
// callers via the `locked` flag, so a `SpinLock<T>` can be shared across cores
// when `T` is `Send`.
unsafe impl<T: Send> Sync for SpinLock<T> {}
// SAFETY: moving the lock moves its `T`; sound whenever `T` is `Send`.
unsafe impl<T: Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// Create a new, unlocked spinlock wrapping `value`.
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    /// Acquire the lock, spinning until it is free. The returned guard releases
    /// the lock when dropped.
    pub fn lock(&self) -> SpinGuard<'_, T> {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Spin read-only until the lock looks free, to avoid hammering the
            // cache line with atomic RMW operations.
            while self.locked.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }
        SpinGuard { lock: self }
    }
}

/// RAII guard that releases its [`SpinLock`] when dropped and derefs to the
/// protected value.
pub struct SpinGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> Deref for SpinGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: holding this guard means we hold the lock, so we have
        // exclusive access to the inner value for the guard's lifetime.
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for SpinGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: as in `deref`; exclusive access is guaranteed while the guard
        // is alive.
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T> Drop for SpinGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}
