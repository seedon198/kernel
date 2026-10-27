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
use core::fmt::Write;
use core::panic::PanicInfo;
use hal::serial::SerialPort;

/// Architecture-neutral kernel entry, called by the boot code once the
/// bootloader handoff is validated and a [`BootInfo`] has been assembled.
fn kmain(_boot_info: &BootInfo) -> ! {
    let mut com1 = SerialPort::com1();
    com1.init();
    // Structured logging (levels, sinks) arrives in M1 (#10/#12); for now a
    // direct greeting proves the boot path end to end.
    let _ = writeln!(com1, "hello from the kernel");
    let _ = writeln!(com1, "kernel: halting (no scheduler yet)");

    hal::cpu::halt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    hal::cpu::disable_interrupts();

    // Best-effort panic report over serial. A full register dump and backtrace
    // arrive in M1 (#11).
    let mut com1 = SerialPort::com1();
    com1.init();
    let _ = writeln!(com1, "KERNEL PANIC: {info}");

    hal::cpu::halt_loop();
}
