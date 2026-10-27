//! CPU identification and feature detection via the `cpuid` instruction.
//!
//! The kernel queries this once at boot to log what it is running on and to
//! verify required features are present before depending on them.

use core::arch::asm;

/// Execute `cpuid` for `leaf`/`subleaf`, returning `(eax, ebx, ecx, edx)`.
fn cpuid(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let (eax, ebx, ecx, edx);
    // SAFETY: `cpuid` has no memory effects. `rbx` is preserved explicitly via a
    // scratch register because LLVM reserves it, so we save it, run cpuid, then
    // swap the result out and the original back.
    unsafe {
        asm!(
            "mov {scratch:r}, rbx",
            "cpuid",
            "xchg {scratch:r}, rbx",
            scratch = out(reg) ebx,
            inout("eax") leaf => eax,
            inout("ecx") subleaf => ecx,
            out("edx") edx,
            options(nostack, preserves_flags),
        );
    }
    (eax, ebx, ecx, edx)
}

/// Detected CPU feature flags relevant to the kernel.
#[derive(Debug, Clone, Copy)]
pub struct Features {
    /// No-Execute page protection.
    pub nx: bool,
    /// SSE2 (baseline for x86-64).
    pub sse2: bool,
    /// Physical Address Extension.
    pub pae: bool,
    /// Page Global Enable.
    pub pge: bool,
    /// Supervisor Mode Execution Prevention.
    pub smep: bool,
    /// Supervisor Mode Access Prevention.
    pub smap: bool,
    /// x2APIC mode.
    pub x2apic: bool,
    /// XSAVE/XRSTOR extended state.
    pub xsave: bool,
    /// Process-Context Identifiers.
    pub pcid: bool,
    /// 1 GiB large pages.
    pub gib_pages: bool,
    /// Hardware random number generator.
    pub rdrand: bool,
    /// Invariant (constant-rate) TSC.
    pub invariant_tsc: bool,
    /// `syscall`/`sysret` instructions.
    pub syscall: bool,
}

/// Identifying information about the current CPU.
#[derive(Clone, Copy)]
pub struct CpuInfo {
    vendor: [u8; 12],
    brand: [u8; 48],
    /// Detected feature flags.
    pub features: Features,
}

impl CpuInfo {
    /// The 12-character vendor string (e.g. `GenuineIntel`, `AuthenticAMD`).
    #[must_use]
    pub fn vendor(&self) -> &str {
        core::str::from_utf8(&self.vendor).unwrap_or("?")
    }

    /// The processor brand string, trimmed.
    #[must_use]
    pub fn brand(&self) -> &str {
        let end = self
            .brand
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.brand.len());
        core::str::from_utf8(&self.brand[..end])
            .unwrap_or("?")
            .trim()
    }
}

/// Query the CPU and return its identity and feature flags.
pub fn detect() -> CpuInfo {
    let (max_leaf, vb, vc, vd) = cpuid(0, 0);

    // Vendor string is EBX, EDX, ECX (in that order).
    let mut vendor = [0u8; 12];
    vendor[0..4].copy_from_slice(&vb.to_le_bytes());
    vendor[4..8].copy_from_slice(&vd.to_le_bytes());
    vendor[8..12].copy_from_slice(&vc.to_le_bytes());

    let (_, _, ecx1, edx1) = if max_leaf >= 1 {
        cpuid(1, 0)
    } else {
        (0, 0, 0, 0)
    };
    let (_, ebx7, _, _) = if max_leaf >= 7 {
        cpuid(7, 0)
    } else {
        (0, 0, 0, 0)
    };

    let ext_max = cpuid(0x8000_0000, 0).0;
    let edx81 = if ext_max >= 0x8000_0001 {
        cpuid(0x8000_0001, 0).3
    } else {
        0
    };
    let invariant_tsc = ext_max >= 0x8000_0007 && (cpuid(0x8000_0007, 0).3 & (1 << 8)) != 0;

    // Brand string from leaves 0x80000002..=0x80000004.
    let mut brand = [0u8; 48];
    if ext_max >= 0x8000_0004 {
        for (i, leaf) in (0x8000_0002..=0x8000_0004u32).enumerate() {
            let (a, b, c, d) = cpuid(leaf, 0);
            let base = i * 16;
            brand[base..base + 4].copy_from_slice(&a.to_le_bytes());
            brand[base + 4..base + 8].copy_from_slice(&b.to_le_bytes());
            brand[base + 8..base + 12].copy_from_slice(&c.to_le_bytes());
            brand[base + 12..base + 16].copy_from_slice(&d.to_le_bytes());
        }
    }

    let features = Features {
        sse2: edx1 & (1 << 26) != 0,
        pae: edx1 & (1 << 6) != 0,
        pge: edx1 & (1 << 13) != 0,
        xsave: ecx1 & (1 << 26) != 0,
        pcid: ecx1 & (1 << 17) != 0,
        x2apic: ecx1 & (1 << 21) != 0,
        rdrand: ecx1 & (1 << 30) != 0,
        smep: ebx7 & (1 << 7) != 0,
        smap: ebx7 & (1 << 20) != 0,
        nx: edx81 & (1 << 20) != 0,
        gib_pages: edx81 & (1 << 26) != 0,
        syscall: edx81 & (1 << 11) != 0,
        invariant_tsc,
    };

    CpuInfo {
        vendor,
        brand,
        features,
    }
}
