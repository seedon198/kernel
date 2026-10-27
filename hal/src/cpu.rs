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
