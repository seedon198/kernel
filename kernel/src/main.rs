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
mod log;

use boot_info::BootInfo;
use core::fmt::Write;
use core::panic::PanicInfo;
use hal::serial::SerialPort;

/// Architecture-neutral kernel entry, called by the boot code once the
/// bootloader handoff is validated and a [`BootInfo`] has been assembled.
fn kmain(boot_info: &BootInfo) -> ! {
    log::init();
    log::set_max_level(log::Level::Debug); // verbose during early bring-up
    info!("hello from the kernel");
    debug!("boot info received at {:p}", boot_info);

    hal::gdt::init();
    info!("gdt: loaded GDT, TSS, and segment registers");

    hal::idt::init();
    info!("idt: installed interrupt descriptor table");

    // Exercise the trap path end to end: a breakpoint (#BP) is a recoverable
    // trap, so the handler prints a report and execution resumes here.
    debug!("triggering a test breakpoint (#BP)...");
    hal::idt::breakpoint();
    info!("resumed after breakpoint");

    info!("kernel: halting (no scheduler yet)");

    hal::cpu::halt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    hal::cpu::disable_interrupts();

    // Report over a fresh serial port (not the logger), so a panic while the log
    // lock is held still reports.
    let mut com1 = SerialPort::com1();
    com1.init();
    let _ = writeln!(com1, "\n*** KERNEL PANIC ***");
    let _ = writeln!(com1, "  {info}");
    hal::backtrace::print(&mut com1);

    hal::cpu::halt_loop();
}
