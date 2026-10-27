//! Bootloader handoff: the Limine protocol boundary.
//!
//! This is the kernel's only bootloader-specific code. It declares the Limine
//! requests, provides the ELF entry point `_start`, validates the protocol
//! revision, and (as later milestones need) translates Limine's responses into
//! the architecture-neutral [`BootInfo`] consumed by the rest of the kernel.
//!
//! ## `unsafe` in this module
//!
//! This module carries `#![allow(unsafe_code)]` — the only place outside `hal`
//! permitted to, and CI-enforced to stay that way (see
//! `docs/adr/0007-unsafe-policy.md`). The `unsafe` here is limited to linker
//! glue: the `#[unsafe(link_section = ...)]` placement of the Limine request
//! statics (so the bootloader can find them) and the `#[unsafe(no_mangle)]`
//! entry symbol. It is not memory-unsafe code; real hardware access still goes
//! through `hal`.
#![allow(unsafe_code)]

use boot_info::BootInfo;
use limine::request::{
    ExecutableAddressRequest, HhdmRequest, MemoryMapRequest, RequestsEndMarker, RequestsStartMarker,
};
use limine::BaseRevision;

/// The base revision of the Limine protocol this kernel is built against. The
/// bootloader refuses to boot (or reports unsupported) if it cannot satisfy it.
#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

/// Map of physical memory: which regions are usable, reserved, or reclaimable.
/// Consumed by the physical frame allocator (M2).
#[used]
#[unsafe(link_section = ".requests")]
static MEMORY_MAP: MemoryMapRequest = MemoryMapRequest::new();

/// Offset of the Higher-Half Direct Map, through which the kernel reaches
/// physical memory (M2).
#[used]
#[unsafe(link_section = ".requests")]
static HHDM: HhdmRequest = HhdmRequest::new();

/// Physical and virtual base of the loaded kernel image, for phys<->virt
/// translation and the KASLR base (M2).
#[used]
#[unsafe(link_section = ".requests")]
static KERNEL_ADDRESS: ExecutableAddressRequest = ExecutableAddressRequest::new();

/// Markers bounding the request section so the bootloader's scan is fast.
#[used]
#[unsafe(link_section = ".requests_start_marker")]
static REQUESTS_START: RequestsStartMarker = RequestsStartMarker::new();
#[used]
#[unsafe(link_section = ".requests_end_marker")]
static REQUESTS_END: RequestsEndMarker = RequestsEndMarker::new();

/// The ELF entry point. Limine hands control here in 64-bit long mode with the
/// kernel mapped at its higher-half link address.
///
/// It validates the protocol revision, assembles a [`BootInfo`], and calls the
/// architecture-neutral [`crate::kmain`]. If the bootloader could not satisfy
/// our base revision, there is nothing safe to do but halt.
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    if !BASE_REVISION.is_supported() {
        hal::cpu::halt_loop();
    }

    // Touch each request so the linker cannot drop it, and so a missing
    // response surfaces here rather than as a mysterious fault later. Fuller
    // translation into `BootInfo` (memory map, HHDM, kernel base) lands in M2.
    let _ = MEMORY_MAP.get_response();
    let _ = HHDM.get_response();
    let _ = KERNEL_ADDRESS.get_response();

    let boot_info = BootInfo::new();
    crate::kmain(&boot_info)
}
