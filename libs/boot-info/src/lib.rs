#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Architecture-neutral boot information.
//!
//! The boot code (which talks to the bootloader, e.g. Limine) translates the
//! boot protocol's responses into this type, so nothing outside the boot
//! module depends on the bootloader. The physical memory map is **copied** in
//! (not referenced), so the kernel does not depend on bootloader-reclaimable
//! memory staying live.

/// Maximum number of physical memory regions we record. Real firmware memory
/// maps are well under this.
pub const MAX_MEMORY_REGIONS: usize = 128;

/// The kind of a physical memory region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    /// Free RAM available for the allocator.
    Usable,
    /// Firmware/hardware reserved; never usable.
    Reserved,
    /// Reclaimable once the bootloader is no longer needed.
    BootloaderReclaimable,
    /// Occupied by the kernel image and boot modules.
    KernelAndModules,
    /// The framebuffer.
    Framebuffer,
    /// ACPI tables, reclaimable after they are parsed.
    AcpiReclaimable,
    /// ACPI non-volatile storage; must be preserved.
    AcpiNvs,
    /// Known-bad RAM.
    BadMemory,
    /// Anything else the bootloader reported.
    Other,
}

/// A single physical memory region.
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    /// Physical base address.
    pub base: u64,
    /// Length in bytes.
    pub length: u64,
    /// What the region is.
    pub kind: RegionKind,
}

impl MemoryRegion {
    const EMPTY: Self = Self {
        base: 0,
        length: 0,
        kind: RegionKind::Other,
    };
}

/// A linear framebuffer the bootloader set up for us.
#[derive(Debug, Clone, Copy)]
pub struct Framebuffer {
    /// Virtual address of the framebuffer (mapped by the bootloader).
    pub address: u64,
    /// Visible width in pixels.
    pub width: u64,
    /// Visible height in pixels.
    pub height: u64,
    /// Bytes per scanline.
    pub pitch: u64,
    /// Bits per pixel.
    pub bpp: u16,
    /// Bit position of the red channel within a pixel.
    pub red_shift: u8,
    /// Bit position of the green channel within a pixel.
    pub green_shift: u8,
    /// Bit position of the blue channel within a pixel.
    pub blue_shift: u8,
}

/// Information handed from the boot code to `kmain`.
#[derive(Clone, Copy)]
pub struct BootInfo {
    /// Offset of the Higher-Half Direct Map: `virt = phys + hhdm_offset`.
    pub hhdm_offset: u64,
    /// Physical base address of the loaded kernel image.
    pub kernel_phys_base: u64,
    /// Virtual base address of the loaded kernel image.
    pub kernel_virt_base: u64,
    /// The framebuffer, if the bootloader provided one.
    pub framebuffer: Option<Framebuffer>,
    regions: [MemoryRegion; MAX_MEMORY_REGIONS],
    region_count: usize,
}

impl Default for BootInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl BootInfo {
    /// Create an empty `BootInfo`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            hhdm_offset: 0,
            kernel_phys_base: 0,
            kernel_virt_base: 0,
            framebuffer: None,
            regions: [MemoryRegion::EMPTY; MAX_MEMORY_REGIONS],
            region_count: 0,
        }
    }

    /// Append a memory region. Returns `false` if the table is full (the region
    /// is dropped), which the caller should treat as a configuration error.
    pub fn push_region(&mut self, region: MemoryRegion) -> bool {
        if self.region_count >= MAX_MEMORY_REGIONS {
            return false;
        }
        self.regions[self.region_count] = region;
        self.region_count += 1;
        true
    }

    /// The recorded memory regions.
    #[must_use]
    pub fn regions(&self) -> &[MemoryRegion] {
        &self.regions[..self.region_count]
    }

    /// Total bytes of actual RAM (all regions except firmware-reserved,
    /// framebuffer, and known-bad memory — i.e. excluding MMIO holes).
    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.regions()
            .iter()
            .filter(|r| {
                !matches!(
                    r.kind,
                    RegionKind::Reserved | RegionKind::Framebuffer | RegionKind::BadMemory
                )
            })
            .map(|r| r.length)
            .sum()
    }

    /// The highest physical address reported by any region (including MMIO).
    #[must_use]
    pub fn highest_address(&self) -> u64 {
        self.regions()
            .iter()
            .map(|r| r.base + r.length)
            .max()
            .unwrap_or(0)
    }

    /// Total bytes of [`RegionKind::Usable`] RAM.
    #[must_use]
    pub fn usable_bytes(&self) -> u64 {
        self.regions()
            .iter()
            .filter(|r| r.kind == RegionKind::Usable)
            .map(|r| r.length)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(base: u64, length: u64, kind: RegionKind) -> MemoryRegion {
        MemoryRegion { base, length, kind }
    }

    #[test]
    fn totals_exclude_mmio_and_sum_usable() {
        let mut bi = BootInfo::new();
        assert!(bi.push_region(region(0, 2 * MIB, RegionKind::Usable)));
        assert!(bi.push_region(region(2 * MIB, MIB, RegionKind::BootloaderReclaimable)));
        assert!(bi.push_region(region(0xfee0_0000, 4 * MIB, RegionKind::Reserved))); // MMIO hole
        assert!(bi.push_region(region(3 * MIB, MIB, RegionKind::Usable)));

        assert_eq!(bi.usable_bytes(), 3 * MIB);
        assert_eq!(bi.total_bytes(), 4 * MIB); // excludes the reserved MMIO region
        assert_eq!(bi.highest_address(), 0xfee0_0000 + 4 * MIB);
        assert_eq!(bi.regions().len(), 4);
    }

    #[test]
    fn push_region_reports_when_full() {
        let mut bi = BootInfo::new();
        for _ in 0..MAX_MEMORY_REGIONS {
            assert!(bi.push_region(region(0, 1, RegionKind::Usable)));
        }
        assert!(!bi.push_region(region(0, 1, RegionKind::Usable)));
        assert_eq!(bi.regions().len(), MAX_MEMORY_REGIONS);
    }

    #[test]
    fn empty_boot_info_has_zero_totals() {
        let bi = BootInfo::new();
        assert_eq!(bi.usable_bytes(), 0);
        assert_eq!(bi.total_bytes(), 0);
        assert_eq!(bi.highest_address(), 0);
    }

    const MIB: u64 = 1024 * 1024;
}
