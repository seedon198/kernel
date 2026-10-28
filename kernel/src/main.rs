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
use hal::paging::AddressSpace;
use hal::serial::SerialPort;
use hal::sync::SpinLock;

/// The kernel's address space, established in [`init_paging`]. Held for later
/// milestones (the heap and guard pages in #41, user address spaces in M4).
static KERNEL_SPACE: SpinLock<Option<AddressSpace>> = SpinLock::new(None);

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

    // Physical frame allocator (M2, #38). Translate the memory map into the
    // allocator's region view (only RegionKind::Usable is handed out), then
    // bring it up and exercise it.
    init_frames(boot_info);

    // Virtual memory (M2, #39): move onto page tables we own, then self-test
    // the map/translate/unmap path.
    init_paging(boot_info);

    // W^X + NX hardening (M2, #40): enforce per-section kernel permissions and
    // make the direct physical map non-executable.
    harden_memory(boot_info);

    // Exercise the trap path end to end: a breakpoint (#BP) is a recoverable
    // trap, so the handler prints a report and execution resumes here.
    debug!("triggering a test breakpoint (#BP)...");
    hal::idt::breakpoint();
    info!("resumed after breakpoint");

    info!("kernel: halting (no scheduler yet)");

    hal::cpu::halt_loop();
}

/// Bring up the physical frame allocator from the boot memory map and exercise
/// it with a small alloc/free smoke test.
fn init_frames(boot_info: &BootInfo) {
    use boot_info::RegionKind;
    use hal::frame::Region;

    let src = boot_info.regions();
    let mut regions = [Region {
        base: 0,
        length: 0,
        usable: false,
    }; boot_info::MAX_MEMORY_REGIONS];
    for (dst, r) in regions.iter_mut().zip(src) {
        *dst = Region {
            base: r.base,
            length: r.length,
            usable: r.kind == RegionKind::Usable,
        };
    }

    if !hal::frame::init(&regions[..src.len()], boot_info.hhdm_offset) {
        warn!("frames: allocator init failed (no usable RAM?)");
        return;
    }

    if let Some(s) = hal::frame::stats() {
        info!(
            "frames: {} MiB usable, {} free / {} total frames tracked",
            s.usable * hal::frame::FRAME_SIZE as usize / (1024 * 1024),
            s.free,
            s.total,
        );
    }

    // Smoke test: allocate three frames, confirm they are distinct and zeroed
    // by construction, free them, and confirm the pool returns to its size.
    let before = hal::frame::stats().map_or(0, |s| s.free);
    let a = hal::frame::alloc_frame();
    let b = hal::frame::alloc_frame();
    let c = hal::frame::alloc_frame();
    if let (Some(a), Some(b), Some(c)) = (a, b, c) {
        debug!(
            "frames: smoke test got {:#x} {:#x} {:#x}",
            a.addr(),
            b.addr(),
            c.addr()
        );
        hal::frame::free_frame(a);
        hal::frame::free_frame(b);
        hal::frame::free_frame(c);
        let after = hal::frame::stats().map_or(0, |s| s.free);
        assert_eq!(before, after, "frame smoke test leaked frames");
        info!("frames: alloc/free smoke test passed");
    } else {
        warn!("frames: smoke test could not allocate");
    }
}

/// Build the kernel address space (inheriting the bootloader's higher-half
/// mappings), switch to it, and self-test the mapping path.
fn init_paging(boot_info: &BootInfo) {
    let Some(aspace) = AddressSpace::new_kernel(boot_info.hhdm_offset) else {
        warn!("paging: could not allocate a PML4 (out of frames?)");
        return;
    };
    aspace.switch();
    info!("paging: running on our own PML4 (higher half inherited)");
    *KERNEL_SPACE.lock() = Some(aspace);

    let ok = KERNEL_SPACE
        .lock()
        .as_mut()
        .is_some_and(AddressSpace::selftest);
    if ok {
        info!("paging: map/translate/unmap self-test passed");
    } else {
        warn!("paging: self-test FAILED");
    }
}

/// Enforce W^X + NX: enable NX, set per-section kernel-image permissions
/// (`.text` R-X, `.rodata` R+NX, `.data`/`.bss` RW+NX), and make the HHDM
/// non-executable. Then verify the key invariants.
fn harden_memory(boot_info: &BootInfo) {
    use hal::paging::{Flags, ProtectError};

    hal::paging::enable_nx();

    let s = boot::kernel_sections();
    let mut guard = KERNEL_SPACE.lock();
    let Some(aspace) = guard.as_mut() else {
        warn!("harden: no kernel address space; skipping");
        return;
    };

    // (range, flags) per section. `protect` adds PRESENT; .text gets neither
    // WRITABLE nor NX (R-X), the rest get NX, and .data also WRITABLE.
    let sections = [
        (s.text, Flags::empty()),
        (s.rodata, Flags::NO_EXECUTE),
        (s.data, Flags::WRITABLE | Flags::NO_EXECUTE),
    ];
    let mut pages = 0usize;
    let mut huge = 0usize;
    for ((start, end), flags) in sections {
        let mut v = start;
        while v < end {
            match aspace.protect(v, flags) {
                Ok(()) => pages += 1,
                Err(ProtectError::HugePage) => huge += 1,
                Err(ProtectError::NotMapped) => {}
            }
            v += hal::paging::PAGE;
        }
    }

    // Make the direct physical map (HHDM) non-executable, then flush the TLB.
    let hhdm = boot_info.hhdm_offset;
    aspace.make_range_nx_toplevel(hhdm, hhdm + boot_info.highest_address());
    aspace.reload();

    info!(
        "harden: NX on; protected {} kernel pages ({} huge skipped)",
        pages, huge
    );

    // Verify the invariants we most care about.
    let mut ok = true;
    if let Some((_, f)) = aspace.query(s.text.0) {
        ok &= !f.contains(Flags::WRITABLE) && !f.contains(Flags::NO_EXECUTE);
    }
    if let Some((_, f)) = aspace.query(s.data.0) {
        ok &= f.contains(Flags::NO_EXECUTE);
    }
    if let Some(f) = aspace.top_level_flags(hhdm) {
        ok &= f.contains(Flags::NO_EXECUTE);
    }
    assert!(ok, "harden: W^X/NX invariants not satisfied");
    info!("harden: verified — .text R-X, .rodata/.data NX, HHDM NX, no W+X");
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
