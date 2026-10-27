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

use boot_info::{BootInfo, MemoryRegion, RegionKind};
use limine::memory_map::EntryType;
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

    let mut boot_info = BootInfo::new();

    if let Some(hhdm) = HHDM.get_response() {
        boot_info.hhdm_offset = hhdm.offset();
    }
    if let Some(addr) = KERNEL_ADDRESS.get_response() {
        boot_info.kernel_phys_base = addr.physical_base();
        boot_info.kernel_virt_base = addr.virtual_base();
    }
    if let Some(mmap) = MEMORY_MAP.get_response() {
        for entry in mmap.entries() {
            boot_info.push_region(MemoryRegion {
                base: entry.base,
                length: entry.length,
                kind: region_kind(entry.entry_type),
            });
        }
    }

    crate::kmain(&boot_info)
}

/// Translate a Limine memory-map entry type into our architecture-neutral kind.
fn region_kind(entry_type: EntryType) -> RegionKind {
    match entry_type {
        EntryType::USABLE => RegionKind::Usable,
        EntryType::RESERVED => RegionKind::Reserved,
        EntryType::BOOTLOADER_RECLAIMABLE => RegionKind::BootloaderReclaimable,
        EntryType::EXECUTABLE_AND_MODULES => RegionKind::KernelAndModules,
        EntryType::FRAMEBUFFER => RegionKind::Framebuffer,
        EntryType::ACPI_RECLAIMABLE => RegionKind::AcpiReclaimable,
        EntryType::ACPI_NVS => RegionKind::AcpiNvs,
        EntryType::BAD_MEMORY => RegionKind::BadMemory,
        _ => RegionKind::Other,
    }
}
