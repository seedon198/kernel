//! Raw x86 port-mapped I/O.
//!
//! These are the lowest-level access primitives; device drivers in `hal` build
//! safe abstractions on top of them (see [`crate::serial`]). They are `unsafe`
//! because reading or writing an arbitrary I/O port can have any hardware side
//! effect.

use core::arch::asm;

/// Read a byte from I/O port `port`.
///
/// # Safety
///
/// The caller must ensure that reading `port` is valid and has no unsound side
/// effects (e.g. it names a real, side-effect-free device register).
#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    // SAFETY: `in al, dx` reads one byte from the I/O port in `dx`. It touches
    // no memory and only reads a device register; the caller guarantees `port`
    // is sound to read per this function's contract.
    unsafe {
        asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write `value` to I/O port `port`.
///
/// # Safety
///
/// The caller must ensure that writing `value` to `port` is valid and that the
/// resulting hardware side effects are intended.
#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    // SAFETY: `out dx, al` writes one byte to the I/O port in `dx`. It touches
    // no memory; the caller guarantees the write is sound per the contract.
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
    }
}
