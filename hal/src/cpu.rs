//! Core CPU control primitives.

use core::arch::asm;

/// Halt the logical CPU until the next interrupt, forever.
///
/// Used as the kernel's final resting state (and by the panic handler) until
/// a scheduler exists.
pub fn halt_loop() -> ! {
    loop {
        // SAFETY: `hlt` halts the logical CPU until the next interrupt. It has
        // no memory operands and no effect on program state besides stopping
        // until an interrupt, so it is always sound to execute in ring 0.
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

/// Disable maskable interrupts on the current CPU.
pub fn disable_interrupts() {
    // SAFETY: `cli` only clears the interrupt flag; it has no memory effects.
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
}

/// `CR4.SMEP` — Supervisor Mode Execution Prevention (bit 20).
const CR4_SMEP: u64 = 1 << 20;
/// `CR4.SMAP` — Supervisor Mode Access Prevention (bit 21).
const CR4_SMAP: u64 = 1 << 21;

/// Enable SMEP and/or SMAP in `CR4`, where the CPU supports them.
///
/// SMEP faults on supervisor *execution* of user-accessible pages; SMAP faults
/// on supervisor *data access* to them (unless `EFLAGS.AC` is set via
/// `stac`/`clac`). With no user mappings yet, enabling both is safe; once user
/// memory exists (M4), deliberate accesses to it must bracket with `stac`/
/// `clac`. Returns the `(smep, smap)` bits actually enabled.
pub fn enable_smep_smap(smep: bool, smap: bool) -> (bool, bool) {
    if !smep && !smap {
        return (false, false);
    }
    let mut cr4 = read_cr4();
    if smep {
        cr4 |= CR4_SMEP;
    }
    if smap {
        cr4 |= CR4_SMAP;
    }
    // SAFETY: we only set the SMEP/SMAP enable bits (read-modify-write preserves
    // all others). Both only *add* supervisor-access faults for user pages, of
    // which the kernel currently maps none, so enabling them cannot fault any
    // existing access.
    unsafe { write_cr4(cr4) }
    (smep, smap)
}

/// Read `CR4`.
fn read_cr4() -> u64 {
    let value: u64;
    // SAFETY: reading CR4 is a side-effect-free privileged register read.
    unsafe { asm!("mov {}, cr4", out(reg) value, options(nomem, nostack, preserves_flags)) }
    value
}

/// Write `CR4`.
///
/// # Safety
/// `value` must be a valid `CR4` configuration for the current mode (e.g. it
/// must not clear bits required to stay in long mode).
unsafe fn write_cr4(value: u64) {
    // SAFETY: guaranteed by the caller's contract above.
    unsafe { asm!("mov cr4, {}", in(reg) value, options(nomem, nostack, preserves_flags)) }
}
