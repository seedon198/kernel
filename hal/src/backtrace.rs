//! Stack backtraces via frame-pointer walking.
//!
//! The kernel is compiled with `-C force-frame-pointers=yes`, so each function
//! prologue sets up the standard chain: at `[rbp]` is the caller's saved `rbp`,
//! and at `[rbp + 8]` is the return address. Walking that chain yields the call
//! stack.
//!
//! Addresses are printed raw. Resolve them to source locations with, e.g.:
//! `llvm-addr2line -e target/x86_64-unknown-none/debug/kernel <addr>`.
//! (Embedding a symbol table for in-kernel symbolization is a later stretch.)

use core::arch::asm;
use core::fmt::Write;

/// Maximum number of frames to print, to bound a walk over a corrupt stack.
const MAX_FRAMES: usize = 32;

/// Print a backtrace starting from the current frame.
pub fn print(w: &mut impl Write) {
    let rbp: u64;
    // SAFETY: reads the frame-base register; no memory access, no side effects.
    unsafe {
        asm!("mov {}, rbp", out(reg) rbp, options(nomem, nostack, preserves_flags));
    }
    print_from(w, rbp);
}

/// Print a backtrace starting from a given frame-base pointer (e.g. a trap
/// frame's `rbp`).
pub fn print_from(w: &mut impl Write, mut rbp: u64) {
    let _ = writeln!(w, "backtrace (most recent call first):");
    for frame in 0..MAX_FRAMES {
        // A valid kernel frame pointer is 8-aligned and in the higher half.
        if rbp == 0 || rbp & 0x7 != 0 || !is_higher_half(rbp) {
            break;
        }
        // SAFETY: `rbp` passed the alignment/canonical checks above; the kernel
        // stack it points into is mapped by the bootloader. We read the saved
        // rbp and return address with volatile, bounds-stopping reads.
        let (saved_rbp, return_addr) = unsafe {
            (
                core::ptr::read_volatile(rbp as *const u64),
                core::ptr::read_volatile((rbp + 8) as *const u64),
            )
        };

        if return_addr == 0 {
            break;
        }
        let _ = writeln!(w, "  #{frame:<2} {return_addr:#018x}");

        // Frame pointers must increase as we move toward the stack base;
        // anything else means a corrupt or finished chain.
        if saved_rbp <= rbp {
            break;
        }
        rbp = saved_rbp;
    }
}

/// Whether `addr` is a canonical higher-half address (kernel space).
fn is_higher_half(addr: u64) -> bool {
    addr >= 0xffff_8000_0000_0000
}
