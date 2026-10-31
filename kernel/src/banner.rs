//! The boot banner: the Magnetite logo, wordmark and a one-screen system summary,
//! printed to every log sink before bring-up starts.
//!
//! Everything shown is either stamped in at build time (`build.rs`) or already
//! known on entry to `kmain` (boot handoff, CPUID). Protections that are only
//! switched on later in bring-up are reported by `kmain` once verified.

use boot_info::BootInfo;
use hal::cpuid::CpuInfo;

use crate::log;

/// The logo: the Fe3O4 element tile (molar mass, formula, name, and an
/// octahedron, magnetite's crystal habit), as in `docs/assets/brand/`. Pure
/// ASCII so it renders on the serial line and the framebuffer font alike.
const TILE: [&str; 11] = [
    r".--------------------------.",
    r"| 231.53               /\  |",
    r"|                     /__\ |",
    r"|                     \  / |",
    r"|  ___         ___     \/  |",
    r"| | __| ___   / _ \        |",
    r"| | _| / -_) | (_) |       |",
    r"| |_|  \___|3 \___/ 4      |",
    r"|                          |",
    r"| magnetite                |",
    r"'--------------------------'",
];

/// The wordmark, in the FIGlet "small" style, set beside tile rows 1-4.
const WORDMARK: [&str; 4] = [
    r" __  __   _   ___ _  _ ___ _____ ___ _____ ___",
    r"|  \/  | /_\ / __| \| | __|_   _|_ _|_   _| __|",
    r"| |\/| |/ _ \ (_ | .` | _|  | |  | |  | | | _|",
    r"|_|  |_/_/ \_\___|_|\_|___| |_| |___| |_| |___|",
];

/// Link-time base of the kernel image; the KASLR slide is measured from here.
pub const KERNEL_LINK_BASE: u64 = 0xffff_ffff_8000_0000;

/// Print the banner and summary.
pub fn print(boot_info: &BootInfo, cpu: &CpuInfo) {
    log::raw(format_args!(""));
    for (i, tile) in TILE.iter().enumerate() {
        match i {
            1..=4 => log::raw(format_args!("  {tile}   {}", WORDMARK[i - 1])),
            6 => log::raw(format_args!(
                "  {tile}   Fe3O4 - a capability microkernel for x86_64"
            )),
            7 => log::raw(format_args!(
                "  {tile}   {} - {} build",
                env!("MAGNETITE_VERSION"),
                env!("MAGNETITE_PROFILE")
            )),
            8 => log::raw(format_args!(
                "  {tile}   pre-alpha - for authorized security research"
            )),
            _ => log::raw(format_args!("  {tile}")),
        }
    }
    log::raw(format_args!(""));
    log::raw(format_args!("  toolchain  {}", env!("MAGNETITE_RUSTC")));

    let loader = boot_info.bootloader_name.as_str();
    let loader = if loader.is_empty() {
        "unknown bootloader"
    } else {
        loader
    };
    log::raw(format_args!(
        "  boot       {} {} via {}",
        loader,
        boot_info.bootloader_version.as_str(),
        boot_info.firmware.as_str()
    ));

    log::raw(format_args!(
        "  cpu        {} \"{}\"",
        cpu.vendor(),
        cpu.brand()
    ));
    let f = &cpu.features;
    let flags = [
        ("nx", f.nx),
        ("sse2", f.sse2),
        ("syscall", f.syscall),
        ("smep", f.smep),
        ("smap", f.smap),
        ("x2apic", f.x2apic),
        ("xsave", f.xsave),
        ("pcid", f.pcid),
        ("1gib", f.gib_pages),
        ("rdrand", f.rdrand),
        ("invtsc", f.invariant_tsc),
    ];
    let mut have = Line::new();
    let mut lack = Line::new();
    for (name, present) in flags {
        if present {
            have.push(name)
        } else {
            lack.push(name)
        }
    }
    log::raw(format_args!("  features   {}", have.as_str()));
    if !lack.as_str().is_empty() {
        log::raw(format_args!("  missing    {}", lack.as_str()));
    }

    log::raw(format_args!(
        "  memory     {} MiB usable / {} MiB total, {} regions",
        boot_info.usable_bytes() / (1024 * 1024),
        boot_info.total_bytes() / (1024 * 1024),
        boot_info.regions().len()
    ));
    log::raw(format_args!(
        "  layout     hhdm={:#x} kernel phys={:#x} virt={:#x}",
        boot_info.hhdm_offset, boot_info.kernel_phys_base, boot_info.kernel_virt_base
    ));
    // Subtract the slide from backtrace addresses before `llvm-addr2line`.
    log::raw(format_args!(
        "  kaslr      slide={:#x}",
        boot_info.kernel_virt_base.wrapping_sub(KERNEL_LINK_BASE)
    ));
    log::raw(format_args!(""));
}

/// A space-separated list of short names in a fixed buffer (no heap yet).
struct Line {
    buf: [u8; 96],
    len: usize,
}

impl Line {
    fn new() -> Self {
        Self {
            buf: [0; 96],
            len: 0,
        }
    }

    /// Append `word`, preceded by a space if not first. Words that would not
    /// fit are dropped; the flag names above fit with room to spare.
    fn push(&mut self, word: &str) {
        let sep = usize::from(self.len > 0);
        if self.len + sep + word.len() > self.buf.len() {
            return;
        }
        if sep == 1 {
            self.buf[self.len] = b' ';
        }
        let start = self.len + sep;
        self.buf[start..start + word.len()].copy_from_slice(word.as_bytes());
        self.len = start + word.len();
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}
