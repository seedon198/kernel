#![no_std]
#![no_main]
#![deny(unsafe_code)]
#![deny(missing_docs)]

//! The microkernel binary.
//!
//! This crate is `#![deny(unsafe_code)]`. All `unsafe` is confined to two
//! places, both CI-enforced (see `docs/adr/0007-unsafe-policy.md`): the `hal`
//! crate (hardware access) and the [`boot`] module (bootloader/linker glue).
//! Kernel logic — [`kmain`] and below — contains no `unsafe`.

mod boot;

use boot_info::BootInfo;
use core::panic::PanicInfo;

/// Architecture-neutral kernel entry, called by the boot code once the
/// bootloader handoff is validated and a [`BootInfo`] has been assembled.
fn kmain(_boot_info: &BootInfo) -> ! {
    // M0 #5 wires serial output here ("hello"). For now, idle.
    hal::cpu::halt_loop();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // M1 #11 replaces this with a register dump + backtrace over serial.
    hal::cpu::disable_interrupts();
    hal::cpu::halt_loop();
}
