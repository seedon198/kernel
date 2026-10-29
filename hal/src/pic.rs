//! The legacy 8259A PIC pair — remapped out of the way and fully masked.
//!
//! Interrupts come from the Local APIC (see [`crate::lapic`]); the PICs are not
//! used. They still power up mapped onto vectors 0x08–0x0F, which collide with
//! CPU exceptions (e.g. IRQ0 would arrive as a "#DF"), so we reprogram them onto
//! [`PIC1_VECTOR_BASE`]/[`PIC2_VECTOR_BASE`] and mask every line. A masked PIC
//! can still raise a *spurious* IRQ7/IRQ15; those land on vectors 0x27/0x2F and
//! are absorbed by the handlers registered here.
//!
//! Reference: Intel 8259A datasheet (ICW1–ICW4 initialization sequence).

use crate::idt::{self, TrapFrame};
use crate::io::{inb, outb};

const PIC1_CMD: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

/// ICW1: begin initialization, an ICW4 follows.
const ICW1_INIT_ICW4: u8 = 0x11;
/// ICW4: 8086/88 mode.
const ICW4_8086: u8 = 0x01;
/// Non-specific end-of-interrupt command.
const EOI: u8 = 0x20;

/// First vector of the master PIC after remapping (IRQ0–7 → 0x20–0x27).
pub const PIC1_VECTOR_BASE: u8 = 0x20;
/// First vector of the slave PIC after remapping (IRQ8–15 → 0x28–0x2F).
pub const PIC2_VECTOR_BASE: u8 = 0x28;

/// Remap both PICs onto vectors 0x20–0x2F, mask all sixteen lines, and install
/// handlers for their spurious IRQs. Call once, after [`crate::idt::init`] and
/// before enabling interrupts.
pub fn disable() {
    // SAFETY: ports 0x20/0x21/0xA0/0xA1 are the 8259A command/data registers on
    // every PC-compatible machine, and 0x80 is the conventional POST-code port
    // used as a short I/O delay. The ICW1–ICW4 sequence below is the documented
    // initialization; it ends with every line masked, so the PICs can no longer
    // deliver a real IRQ.
    unsafe {
        outb(PIC1_CMD, ICW1_INIT_ICW4);
        io_wait();
        outb(PIC2_CMD, ICW1_INIT_ICW4);
        io_wait();
        outb(PIC1_DATA, PIC1_VECTOR_BASE); // ICW2: vector offset
        io_wait();
        outb(PIC2_DATA, PIC2_VECTOR_BASE);
        io_wait();
        outb(PIC1_DATA, 1 << 2); // ICW3: slave on IRQ2
        io_wait();
        outb(PIC2_DATA, 2); // ICW3: cascade identity
        io_wait();
        outb(PIC1_DATA, ICW4_8086);
        io_wait();
        outb(PIC2_DATA, ICW4_8086);
        io_wait();
        outb(PIC1_DATA, 0xff); // mask all
        outb(PIC2_DATA, 0xff);
        // Read back so the masks are known to have landed before we return.
        let _ = inb(PIC1_DATA);
    }
    idt::register_handler(PIC1_VECTOR_BASE + 7, spurious_master);
    idt::register_handler(PIC2_VECTOR_BASE + 7, spurious_slave);
}

/// A tiny delay between PIC writes (old PICs need time to settle).
///
/// # Safety
/// Writes port 0x80, the POST-code port, which has no effect on running state.
unsafe fn io_wait() {
    // SAFETY: guaranteed by the contract above.
    unsafe { outb(0x80, 0) }
}

/// Spurious IRQ7 from the master: not a real interrupt, so no EOI.
fn spurious_master(_frame: &mut TrapFrame) {}

/// Spurious IRQ15 from the slave: the master *did* see a real cascade (IRQ2)
/// request, so it alone needs an EOI.
fn spurious_slave(_frame: &mut TrapFrame) {
    // SAFETY: a non-specific EOI to the master PIC's command port only clears
    // its in-service cascade bit; every line remains masked.
    unsafe { outb(PIC1_CMD, EOI) }
}
