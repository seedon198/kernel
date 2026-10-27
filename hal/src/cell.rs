//! A one-time-initialization cell for boot-time statics.

use core::cell::UnsafeCell;

/// A cell for a static that is written once during single-threaded boot and
/// thereafter read by the CPU (e.g. the GDT and IDT).
///
/// Access is sound because the relevant `init()` runs exactly once, on the boot
/// CPU, before interrupts or other cores exist. This is a deliberate, narrow
/// escape hatch living in `hal`; higher layers never touch it.
#[repr(transparent)]
pub(crate) struct Racy<T>(UnsafeCell<T>);

// SAFETY: the contained value is written only once in a boot-time `init()`
// (single-threaded, pre-interrupt) and thereafter only read. No concurrent
// access occurs, so sharing across the (currently single) execution context is
// sound.
unsafe impl<T> Sync for Racy<T> {}

impl<T> Racy<T> {
    /// Create a new cell wrapping `value`.
    pub(crate) const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    /// Get a raw pointer to the inner value, for one-time initialization.
    pub(crate) fn get(&self) -> *mut T {
        self.0.get()
    }
}
