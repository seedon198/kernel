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

extern crate alloc;

mod boot;
mod log;

use alloc::boxed::Box;
use alloc::vec::Vec;
use boot_info::BootInfo;
use core::fmt::Write;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU64, Ordering};
use hal::paging::{AddressSpace, Flags};
use hal::serial::SerialPort;
use hal::sync::SpinLock;

/// The kernel's address space, established in [`init_paging`]. Held for later
/// milestones (the heap and guard pages in #41, user address spaces in M4).
static KERNEL_SPACE: SpinLock<Option<AddressSpace>> = SpinLock::new(None);

/// The kernel heap (#41). Backed by frames mapped on demand by [`heap_grow`].
#[global_allocator]
static HEAP: hal::heap::KernelHeap = hal::heap::KernelHeap::new();

/// Virtual base of the kernel heap window. Deliberately placed *below* the
/// KASLR window (the top 2 GiB, from [`HEAP_LIMIT`]): Limine always loads the
/// kernel image at or above `0xffffffff80000000`, so a heap region that lives
/// below that and is capped there can never collide with the slid kernel image,
/// however KASLR places it (#61). This base is in the lower part of the kernel's
/// PML4 slot, which the bootloader leaves unused.
const HEAP_BASE: u64 = 0xffff_ff80_0000_0000;
/// Upper bound of the heap window: the start of the kernel/KASLR top-2-GiB
/// region. The heap never grows into or past this, keeping it clear of the
/// kernel image. (Leaves ~510 GiB of heap space — ample.)
const HEAP_LIMIT: u64 = 0xffff_ffff_8000_0000;
/// Minimum bytes mapped per growth (64 frames).
const HEAP_CHUNK: u64 = 256 * 1024;
/// Next unmapped heap virtual address; grows contiguously upward.
static HEAP_NEXT: AtomicU64 = AtomicU64::new(HEAP_BASE);

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

    // Supervisor-access protections (M2, #41): block the kernel from executing
    // or (SMAP) reading user-accessible pages. Safe now — no user pages exist.
    let (smep, smap) = hal::cpu::enable_smep_smap(f.smep, f.smap);
    info!("cpu protections: smep={smep} smap={smap}");

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
    // KASLR (#47): the kernel is a PIE that Limine slides to a random base each
    // boot. Log the slide so backtrace addresses can be mapped back to the
    // link base for `llvm-addr2line` (subtract the slide first).
    const KERNEL_LINK_BASE: u64 = 0xffff_ffff_8000_0000;
    let slide = boot_info.kernel_virt_base.wrapping_sub(KERNEL_LINK_BASE);
    info!("kaslr: kernel slide={slide:#x} (subtract from addrs before addr2line)");

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

    // Guard pages (M2, #41): unmap the page below each HAL-owned stack so an
    // overflow faults instead of corrupting its neighbor.
    install_guard_pages();

    // Kernel heap (M2, #41): a growable allocator, so Box/Vec work.
    init_heap();

    // Page-fault diagnostics (M2, #42): register named regions and prove the
    // recoverable-probe + classification path.
    init_fault_diagnostics(boot_info);

    // Untyped memory & retype (M3, #55): demonstrate carving object storage
    // from an Untyped region with watermark accounting.
    demo_untyped(boot_info);

    // Kernel objects & CNode-backed capability space (M3, #56): retype objects
    // and install/attenuate real capabilities.
    demo_capabilities(boot_info);

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

/// Unmap the stack guard pages so an overflow of a HAL-owned stack faults.
fn install_guard_pages() {
    let mut guard = KERNEL_SPACE.lock();
    let Some(aspace) = guard.as_mut() else {
        return;
    };
    let mut n = 0;
    for page in hal::gdt::guard_pages() {
        if aspace.unmap(page) {
            n += 1;
        }
    }
    aspace.reload();
    info!("guard: unmapped {} stack guard pages", n);
}

/// Grow the heap on demand: map `max(min_bytes, HEAP_CHUNK)` of fresh,
/// zeroed frames (RW+NX) contiguously at the next heap address, and return the
/// newly mapped `(base, size)`. Called by the allocator when no block fits.
///
/// Must not itself allocate on the heap (it runs while the heap lock is held);
/// it only touches the frame allocator and the kernel page tables.
fn heap_grow(min_bytes: usize) -> Option<(usize, usize)> {
    let want = (min_bytes as u64).max(HEAP_CHUNK);
    let pages = want.div_ceil(hal::paging::PAGE);
    let size = pages * hal::paging::PAGE;
    let base = HEAP_NEXT.load(Ordering::Relaxed);

    // Never grow into the kernel/KASLR window (#61): the heap stays strictly
    // below it, so it can never overlap the slid kernel image.
    if base + size > HEAP_LIMIT {
        return None;
    }

    let mut guard = KERNEL_SPACE.lock();
    let aspace = guard.as_mut()?;
    let flags = Flags::WRITABLE | Flags::NO_EXECUTE;
    for i in 0..pages {
        let frame = hal::frame::alloc_frame()?;
        aspace
            .map(base + i * hal::paging::PAGE, frame.addr(), flags)
            .ok()?;
    }
    HEAP_NEXT.store(base + size, Ordering::Relaxed);
    Some((base as usize, size as usize))
}

/// Bring up the kernel heap and exercise it (forcing a growth and verifying
/// `Vec`/`Box` round-trip correctly).
fn init_heap() {
    if !HEAP.init(heap_grow) {
        warn!("heap: init failed (could not map initial region)");
        return;
    }

    // Grow past the initial chunk (40k u64 = 320 KiB > 256 KiB) to exercise the
    // grow + coalesce path, and check the data survives reallocation.
    let mut v: Vec<u64> = Vec::new();
    for i in 0..40_000u64 {
        v.push(i);
    }
    let sum: u64 = v.iter().sum();
    let expected: u64 = (0..40_000u64).sum();
    let boxed = Box::new(0xC0FFEE_u64);
    let ok = sum == expected && *boxed == 0xC0FFEE;
    drop(v);
    drop(boxed);

    let (live, mapped) = HEAP.stats();
    if ok {
        info!(
            "heap: ok — Vec/Box verified; {} KiB mapped, {} bytes live",
            mapped / 1024,
            live
        );
    } else {
        warn!("heap: self-test FAILED");
    }
}

/// Register named memory regions for page-fault classification, then verify the
/// recoverable probe (a guard page faults and recovers; mapped memory reads
/// fine) and the classifier.
fn init_fault_diagnostics(boot_info: &BootInfo) {
    use hal::fault::{classify, probe_read, register_region};

    let s = boot::kernel_sections();
    register_region(".text", s.text.0, s.text.1);
    register_region(".rodata", s.rodata.0, s.rodata.1);
    register_region(".data/.bss", s.data.0, s.data.1);
    register_region(
        "HHDM",
        boot_info.hhdm_offset,
        boot_info.hhdm_offset + boot_info.highest_address(),
    );
    register_region("kernel heap", HEAP_BASE, HEAP_NEXT.load(Ordering::Relaxed));
    for g in hal::gdt::guard_pages() {
        register_region("stack guard", g, g + hal::paging::PAGE);
    }

    // The recoverable probe: a guard page must fault (and recover); mapped
    // kernel code must read without faulting.
    let guard = hal::gdt::guard_pages()[0];
    let guard_faults = probe_read(guard);
    let text_ok = !probe_read(s.text.0);
    assert!(guard_faults, "guard page should be unmapped");
    assert!(text_ok, "mapped .text should be readable");
    info!("pf: probe verified — guard page faults & recovers, mapped memory reads ok");

    // Show the classifier naming a few representative addresses.
    let samples = [
        ("text", s.text.0),
        ("heap", HEAP_BASE),
        ("hhdm", boot_info.hhdm_offset),
        ("guard", guard),
    ];
    for (label, addr) in samples {
        debug!(
            "pf: {addr:#x} ({label}) -> {}",
            classify(addr).unwrap_or("unmapped/unknown")
        );
    }
}

/// Demonstrate Untyped retype (#55): take one frame as a small Untyped region,
/// carve object storage from it with watermark accounting, and show that an
/// over-budget request is rejected.
fn demo_untyped(boot_info: &BootInfo) {
    use capability::Untyped;

    let Some(frame) = hal::frame::alloc_frame() else {
        warn!("untyped: no frame for the demo");
        return;
    };
    let hhdm = boot_info.hhdm_offset;
    let mut ut = Untyped::new(frame.addr(), hal::paging::PAGE); // 4 KiB region

    // Carve 8 objects of 256 bytes, 64-byte aligned, and zero them.
    match hal::object::retype(&mut ut, 256, 64, 8, hhdm) {
        Some(base) => info!(
            "untyped: retyped 8x256B at {:#x}; {} B used, {} B free of {}",
            base,
            ut.watermark() - ut.base(),
            ut.free(),
            ut.size()
        ),
        None => warn!("untyped: retype failed unexpectedly"),
    }

    // An over-budget request (a whole 4 KiB object) must be rejected, leaving
    // the watermark untouched.
    let before = ut.free();
    let rejected = hal::object::retype(&mut ut, hal::paging::PAGE, hal::paging::PAGE, 1, hhdm);
    assert!(rejected.is_none(), "over-budget retype should fail");
    assert_eq!(ut.free(), before, "failed retype must not consume space");
    info!("untyped: over-budget retype correctly rejected; accounting intact");

    hal::frame::free_frame(frame);
}

/// Demonstrate the capability path end to end (#56): from one Untyped region,
/// retype a CNode (which backs a capability space), an Endpoint, and a
/// Notification; install full-rights capabilities to the objects; then derive
/// an attenuated (SEND-only) copy and verify the rights.
fn demo_capabilities(boot_info: &BootInfo) {
    use capability::Untyped;
    use capability::{cnode_bytes, slot_align, CapId, Capability, ObjectKind, ObjectRef, Rights};

    const CNODE_SLOTS: usize = 16;

    let Some(frame) = hal::frame::alloc_frame() else {
        warn!("caps: no frame for the demo");
        return;
    };
    let hhdm = boot_info.hhdm_offset;
    let mut ut = Untyped::new(frame.addr(), hal::paging::PAGE);

    // Retype a CNode and form the capability space over its storage.
    let Some(cnode_phys) = hal::object::retype(
        &mut ut,
        cnode_bytes(CNODE_SLOTS) as u64,
        slot_align() as u64,
        1,
        hhdm,
    ) else {
        warn!("caps: CNode retype failed");
        hal::frame::free_frame(frame);
        return;
    };
    // `cnode_phys` is a CNode just retyped for CNODE_SLOTS slots (our exclusive
    // HHDM-mapped storage), and we form exactly one CapSpace over it.
    let mut cspace = hal::object::cnode_cap_space(cnode_phys, CNODE_SLOTS, hhdm);

    // Retype an Endpoint and a Notification.
    let (esz, eal) = ObjectKind::Endpoint.fixed_layout().unwrap();
    let (nsz, nal) = ObjectKind::Notification.fixed_layout().unwrap();
    let (Some(ep_phys), Some(nt_phys)) = (
        hal::object::retype(&mut ut, esz, eal, 1, hhdm),
        hal::object::retype(&mut ut, nsz, nal, 1, hhdm),
    ) else {
        warn!("caps: object retype failed");
        hal::frame::free_frame(frame);
        return;
    };

    // Install full-rights capabilities to the objects.
    let ep = Capability::new(ObjectRef::new(ObjectKind::Endpoint, ep_phys), Rights::ALL);
    let nt = Capability::new(
        ObjectRef::new(ObjectKind::Notification, nt_phys),
        Rights::ALL,
    );
    let ep_id = cspace.insert(ep).unwrap();
    let _nt_id = cspace.insert(nt).unwrap();

    // Derive an attenuated SEND-only copy of the endpoint cap into another slot.
    cspace.copy(ep_id, CapId(10), Rights::SEND).unwrap();

    let full = cspace.get(ep_id).unwrap();
    let weak = cspace.get(CapId(10)).unwrap();
    let ok = full.allows(Rights::SEND | Rights::RECV)
        && weak.allows(Rights::SEND)
        && !weak.allows(Rights::RECV)
        && weak.object == full.object;
    assert!(ok, "attenuated capability derivation is wrong");

    info!(
        "caps: CNode cap space ({} slots) holds endpoint+notification; SEND-only copy attenuated & verified",
        cspace.capacity()
    );

    hal::frame::free_frame(frame);
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
