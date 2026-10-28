//! Retyping Untyped memory into object storage.
//!
//! The pure accounting lives in [`capability::object::Untyped`]; this is the
//! HAL half that actually touches the carved physical memory: it zeroes the
//! storage through the HHDM so a freshly retyped object never exposes whatever
//! previously occupied that memory (important when an Untyped region is revoked
//! and reused). See [ADR 0016](../../docs/adr/0016-capability-implementation.md).

use capability::object::{ObjectRef, Untyped};
use capability::space::{CapSpace, Slot};

/// Carve `count` objects of `size` bytes each (aligned to `align`) from
/// `untyped`, **zero** their storage, and return the physical base of the
/// first. Returns `None` if the request is malformed or does not fit; on
/// failure nothing is carved or zeroed.
///
/// `hhdm` is the Higher-Half Direct Map offset, through which the carved
/// physical range is reached.
pub fn retype(untyped: &mut Untyped, size: u64, align: u64, count: u64, hhdm: u64) -> Option<u64> {
    let base = untyped.allocate(size, align, count)?;
    let len = (size * count) as usize;
    let ptr = (hhdm + base) as *mut u8;
    // SAFETY: `base .. base + size*count` was just carved from `untyped` by
    // `allocate` (which checked it lies within the region), so it is backing
    // RAM mapped in the HHDM at `hhdm` and now owned exclusively by the objects
    // being created. Zeroing that range stays within it.
    unsafe {
        core::ptr::write_bytes(ptr, 0, len);
    }
    Some(base)
}

/// Form a capability space over a CNode's retyped storage: a `Slot` array at
/// physical `phys` with `slots` entries, reached through the HHDM.
///
/// Each slot is initialized to empty (overwriting the retyped zeros with a
/// valid `Slot::empty()`, so no possibly-invalid bytes are ever read), then the
/// slice is handed back as a [`CapSpace`].
///
/// The caller must pass a `phys` that is a CNode just retyped for `slots` slots
/// — so `phys .. phys + slots * size_of::<Slot<ObjectRef>>()` is our RAM, mapped
/// in the HHDM at `hhdm` and owned exclusively by this CNode — and must not form
/// a second `CapSpace` aliasing the same storage while this one lives. (A safe
/// wrapper over internal `unsafe`, like the other HHDM-backed HAL constructors.)
#[must_use]
pub fn cnode_cap_space(phys: u64, slots: usize, hhdm: u64) -> CapSpace<'static, ObjectRef> {
    let ptr = (hhdm + phys) as *mut Slot<ObjectRef>;
    // SAFETY: per the caller contract above, `ptr .. ptr + slots` is our
    // exclusively-owned, HHDM-mapped CNode storage. We `write` (not read)
    // `Slot::empty()` into each entry first, so the slice then references only
    // initialized, valid `Slot`s and aliases nothing.
    unsafe {
        for i in 0..slots {
            ptr.add(i).write(Slot::empty());
        }
        CapSpace::new(core::slice::from_raw_parts_mut(ptr, slots))
    }
}
