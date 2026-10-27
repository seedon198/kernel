#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Architecture-neutral boot information.
//!
//! The boot code (which talks to the bootloader, e.g. Limine) translates the
//! boot protocol's responses into this type, so nothing outside the boot
//! module depends on the bootloader. Fields are added as milestones need them
//! (memory map and HHDM offset in M2, framebuffer in M1, ...).

/// Information handed from the boot code to `kmain`.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct BootInfo {}

impl BootInfo {
    /// Create an empty `BootInfo`. Placeholder until real fields land.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}
