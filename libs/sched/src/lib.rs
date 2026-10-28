#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! A pure **round-robin** scheduler policy.
//!
//! The scheduler owns a ready queue of [`ThreadId`]s; the front is the running
//! thread and [`rotate`](Scheduler::rotate) advances to the next. It is pure
//! logic (no `unsafe`, no hardware) and host-tested; the kernel pairs it with
//! the `hal` context switch to actually run threads. Round-robin is the first
//! policy (ADR 0017); priorities can replace it later by swapping this module.

/// Maximum number of threads the ready queue holds.
pub const MAX_THREADS: usize = 64;

/// Identifies a thread by its control-block index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ThreadId(pub usize);

/// A round-robin ready queue. The front ([`current`](Self::current)) is the
/// running thread; admitting adds to the back; rotating moves the front to the
/// back and returns the new front.
pub struct Scheduler {
    ready: [ThreadId; MAX_THREADS],
    len: usize,
}

impl Scheduler {
    /// An empty scheduler.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ready: [ThreadId(0); MAX_THREADS],
            len: 0,
        }
    }

    /// Number of threads in the ready queue.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether the ready queue is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Add `id` to the back of the ready queue. Returns `false` if full or if
    /// `id` is already queued.
    pub fn admit(&mut self, id: ThreadId) -> bool {
        if self.len == MAX_THREADS || self.contains(id) {
            return false;
        }
        self.ready[self.len] = id;
        self.len += 1;
        true
    }

    /// Whether `id` is in the ready queue.
    #[must_use]
    pub fn contains(&self, id: ThreadId) -> bool {
        self.ready[..self.len].contains(&id)
    }

    /// The currently running thread (the front), or `None` if the queue is
    /// empty.
    #[must_use]
    pub fn current(&self) -> Option<ThreadId> {
        (self.len > 0).then(|| self.ready[0])
    }

    /// Move the current thread to the back and return the new current. With 0
    /// or 1 threads this returns the unchanged current (or `None`).
    pub fn rotate(&mut self) -> Option<ThreadId> {
        if self.len == 0 {
            return None;
        }
        if self.len > 1 {
            let front = self.ready[0];
            self.ready.copy_within(1..self.len, 0);
            self.ready[self.len - 1] = front;
        }
        Some(self.ready[0])
    }

    /// Remove `id` from the ready queue (e.g. on thread exit or block).
    /// Returns whether it was present.
    pub fn remove(&mut self, id: ThreadId) -> bool {
        if let Some(pos) = self.ready[..self.len].iter().position(|&t| t == id) {
            self.ready.copy_within(pos + 1..self.len, pos);
            self.len -= 1;
            true
        } else {
            false
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admit_and_current() {
        let mut s = Scheduler::new();
        assert!(s.is_empty());
        assert!(s.admit(ThreadId(0)));
        assert!(s.admit(ThreadId(1)));
        assert_eq!(s.len(), 2);
        assert_eq!(s.current(), Some(ThreadId(0)));
        // No duplicates.
        assert!(!s.admit(ThreadId(0)));
    }

    #[test]
    fn rotate_cycles_round_robin() {
        let mut s = Scheduler::new();
        for i in 0..3 {
            s.admit(ThreadId(i));
        }
        // Front is 0; rotating cycles 0 -> 1 -> 2 -> 0.
        assert_eq!(s.rotate(), Some(ThreadId(1)));
        assert_eq!(s.rotate(), Some(ThreadId(2)));
        assert_eq!(s.rotate(), Some(ThreadId(0)));
        assert_eq!(s.current(), Some(ThreadId(0)));
    }

    #[test]
    fn rotate_single_thread_returns_itself() {
        let mut s = Scheduler::new();
        s.admit(ThreadId(7));
        assert_eq!(s.rotate(), Some(ThreadId(7)));
        assert_eq!(s.rotate(), Some(ThreadId(7)));
    }

    #[test]
    fn rotate_empty_is_none() {
        let mut s = Scheduler::new();
        assert_eq!(s.rotate(), None);
        assert_eq!(s.current(), None);
    }

    #[test]
    fn remove_takes_a_thread_out_of_rotation() {
        let mut s = Scheduler::new();
        for i in 0..3 {
            s.admit(ThreadId(i));
        }
        // Remove the middle one; order of the rest is preserved.
        assert!(s.remove(ThreadId(1)));
        assert_eq!(s.len(), 2);
        assert_eq!(s.current(), Some(ThreadId(0)));
        assert_eq!(s.rotate(), Some(ThreadId(2)));
        assert_eq!(s.rotate(), Some(ThreadId(0)));
        assert!(!s.remove(ThreadId(1))); // already gone
    }
}
