//! Kernel object references and **Untyped-memory** accounting.
//!
//! A capability names a kernel object by an [`ObjectRef`] — its [`ObjectKind`]
//! and the physical address of its storage. Object storage comes from
//! retyping [`Untyped`] memory (ADR 0016): `Untyped` is a physical region with
//! a bump **watermark**; carving advances it and is never individually
//! reclaimed (you revoke the whole region). This module is the pure accounting
//! (no `unsafe`, host-tested); the HAL zeroes the carved physical memory
//! through the HHDM when it performs a retype.

/// The kind of a kernel object a capability can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    /// Unused physical memory that can be retyped into other objects.
    Untyped,
    /// A node of a capability space (holds capability slots).
    CNode,
    /// A physical page frame (mappable).
    Frame,
    /// A synchronous IPC rendezvous point.
    Endpoint,
    /// An asynchronous signal.
    Notification,
    /// A page-table root (address space).
    AddressSpace,
    /// A schedulable execution context.
    Thread,
}

/// A reference to a kernel object: its kind and the physical address of its
/// storage (in retyped Untyped memory, reached by the kernel via the HHDM).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectRef {
    /// What kind of object this is — tells the kernel how to interpret it.
    pub kind: ObjectKind,
    /// Physical address of the object's storage.
    pub phys: u64,
}

impl ObjectRef {
    /// A reference to a `kind` object stored at physical `phys`.
    #[must_use]
    pub const fn new(kind: ObjectKind, phys: u64) -> Self {
        Self { kind, phys }
    }
}

/// Round `addr` up to a power-of-two `align`, or `None` on overflow.
const fn align_up(addr: u64, align: u64) -> Option<u64> {
    match addr.checked_add(align - 1) {
        Some(x) => Some(x & !(align - 1)),
        None => None,
    }
}

/// Bump-watermark accounting for a region of Untyped physical memory.
///
/// Objects are carved by advancing [`watermark`](Self::watermark) from
/// [`base`](Self::base); nothing is freed individually (reclaim the whole
/// region by revoking it). This type does the arithmetic only — the kernel
/// zeroes the carved memory when it retypes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Untyped {
    base: u64,
    size: u64,
    watermark: u64,
}

impl Untyped {
    /// An untyped region `[base, base + size)` with nothing yet carved.
    #[must_use]
    pub const fn new(base: u64, size: u64) -> Self {
        Self {
            base,
            size,
            watermark: base,
        }
    }

    /// Physical base of the region.
    #[must_use]
    pub const fn base(&self) -> u64 {
        self.base
    }

    /// Total size of the region in bytes.
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// Current watermark (first unused physical address).
    #[must_use]
    pub const fn watermark(&self) -> u64 {
        self.watermark
    }

    /// Bytes still available for carving (after the watermark).
    #[must_use]
    pub const fn free(&self) -> u64 {
        (self.base + self.size) - self.watermark
    }

    /// Carve `count` objects of `size` bytes each, aligned to `align` (a power
    /// of two), advancing the watermark past all of them. Returns the physical
    /// base of the first object, or `None` if the request is malformed or does
    /// not fit (the watermark is unchanged on failure).
    pub fn allocate(&mut self, size: u64, align: u64, count: u64) -> Option<u64> {
        if size == 0 || count == 0 || align == 0 || !align.is_power_of_two() {
            return None;
        }
        let start = align_up(self.watermark, align)?;
        let total = size.checked_mul(count)?;
        let end = start.checked_add(total)?;
        if end > self.base.checked_add(self.size)? {
            return None;
        }
        self.watermark = end;
        Some(start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_advances_watermark_and_aligns() {
        let mut u = Untyped::new(0x1_0000, 0x4000); // 16 KiB at 64 KiB
        assert_eq!(u.watermark(), 0x1_0000);
        // 2 objects of 100 bytes, 64-aligned.
        let a = u.allocate(100, 64, 2).unwrap();
        assert_eq!(a, 0x1_0000); // already aligned
        assert_eq!(u.watermark(), 0x1_0000 + 200);
        // Next allocation is realigned up to 64.
        let b = u.allocate(8, 64, 1).unwrap();
        assert_eq!(b % 64, 0);
        assert!(b >= 0x1_0000 + 200);
    }

    #[test]
    fn allocate_rejects_overflow_of_region() {
        let mut u = Untyped::new(0, 0x1000); // 4 KiB
                                             // One 4 KiB object fits exactly.
        assert_eq!(u.allocate(0x1000, 0x1000, 1), Some(0));
        assert_eq!(u.free(), 0);
        // Nothing more fits.
        assert_eq!(u.allocate(1, 1, 1), None);
    }

    #[test]
    fn allocate_count_must_all_fit() {
        let mut u = Untyped::new(0, 0x1000);
        // 5 x 1 KiB = 5 KiB does not fit in 4 KiB; reject entirely.
        assert_eq!(u.allocate(1024, 1024, 5), None);
        // Watermark untouched on failure.
        assert_eq!(u.watermark(), 0);
        // 4 do fit.
        assert_eq!(u.allocate(1024, 1024, 4), Some(0));
    }

    #[test]
    fn allocate_validates_arguments() {
        let mut u = Untyped::new(0x2000, 0x2000);
        assert_eq!(u.allocate(0, 8, 1), None); // zero size
        assert_eq!(u.allocate(8, 0, 1), None); // zero align
        assert_eq!(u.allocate(8, 24, 1), None); // non-power-of-two align
        assert_eq!(u.allocate(8, 8, 0), None); // zero count
        assert_eq!(u.watermark(), 0x2000); // unchanged
    }

    #[test]
    fn allocate_detects_arithmetic_overflow() {
        let mut u = Untyped::new(u64::MAX - 0xfff, 0x1000);
        // A count that overflows u64 is rejected, not wrapped.
        assert_eq!(u.allocate(u64::MAX, 1, 2), None);
    }

    #[test]
    fn object_ref_carries_kind_and_addr() {
        let r = ObjectRef::new(ObjectKind::Frame, 0x3000);
        assert_eq!(r.kind, ObjectKind::Frame);
        assert_eq!(r.phys, 0x3000);
    }
}
