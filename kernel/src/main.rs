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

    // Bring up the on-screen console as a second log sink, if we have a
    // framebuffer. Serial remains the primary sink.
    if let Some(fb) = boot_info.framebuffer {
        hal::framebuffer::init(hal::framebuffer::FramebufferInfo {
            address: fb.address,
            width: fb.width as usize,
            height: fb.height as usize,
            pitch: fb.pitch as usize,
            bpp: fb.bpp,
            red_shift: fb.red_shift,
            green_shift: fb.green_shift,
            blue_shift: fb.blue_shift,
        });
    }

    info!("hello from the kernel");
    debug!("boot info received at {:p}", boot_info);

    hal::gdt::init();
    info!("gdt: loaded GDT, TSS, and segment registers");

    hal::idt::init();
    info!("idt: installed interrupt descriptor table");

    // CPU identity and required features.
    let cpu = hal::cpuid::detect();
    info!("cpu: {} \"{}\"", cpu.vendor(), cpu.brand());
    let f = &cpu.features;
    info!(
        "cpu features: nx={} sse2={} smep={} smap={} x2apic={} xsave={} pcid={} 1gib={} rdrand={} invtsc={} syscall={}",
        f.nx, f.sse2, f.smep, f.smap, f.x2apic, f.xsave, f.pcid, f.gib_pages, f.rdrand, f.invariant_tsc, f.syscall
    );
    assert!(f.nx, "CPU lacks NX (No-Execute); required for W^X");
    assert!(f.sse2, "CPU lacks SSE2; required baseline for x86-64");

    // Physical memory summary (full map consumed by the allocator in M2).
    info!(
        "memory: {} MiB usable / {} MiB total across {} regions",
        boot_info.usable_bytes() / (1024 * 1024),
        boot_info.total_bytes() / (1024 * 1024),
        boot_info.regions().len()
    );
    info!(
        "layout: hhdm={:#x}  kernel phys={:#x} virt={:#x}",
        boot_info.hhdm_offset, boot_info.kernel_phys_base, boot_info.kernel_virt_base
    );

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
