#![no_std]
#![no_main]
#![deny(unsafe_code)]
#![deny(missing_docs)]

//! The microkernel binary.
//!
//! This crate is `#![deny(unsafe_code)]` with a single, CI-checked
//! `#[allow(unsafe_code)]` on the `_start` entry shim below — the only place
//! `unsafe` is permitted here, and only because the ELF entry symbol requires
//! the (otherwise linker-only) `#[unsafe(no_mangle)]` attribute. All *actual*
//! hardware access goes through the `hal` crate. See
//! `docs/adr/0007-unsafe-policy.md` and `docs/architecture/overview.md`.

use boot_info::BootInfo;
use core::panic::PanicInfo;

/// Kernel entry point.
///
/// For now this is called directly as the ELF entry (`_start`). Once the
/// Limine boot code lands (#2/#4), `_start` will validate the boot protocol,
/// build a [`BootInfo`], and call [`kmain`] with it.
///
/// The `#[allow(unsafe_code)]` here covers only the `#[unsafe(no_mangle)]`
/// attribute, which is linker glue, not memory-unsafe code. This is the sole
/// `unsafe` allowance in the crate.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let boot_info = BootInfo::new();
    kmain(&boot_info)
}

/// Architecture-neutral kernel main.
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
