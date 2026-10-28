//! Retyping Untyped memory into object storage.
//!
//! The pure accounting lives in [`capability::object::Untyped`]; this is the
//! HAL half that actually touches the carved physical memory: it zeroes the
//! storage through the HHDM so a freshly retyped object never exposes whatever
//! previously occupied that memory (important when an Untyped region is revoked
//! and reused). See [ADR 0016](../../docs/adr/0016-capability-implementation.md).

use capability::object::Untyped;

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
