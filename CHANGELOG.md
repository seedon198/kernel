# Changelog

All notable changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/) (pre-1.0: anything may change).

## [Unreleased]

### M2: Memory & isolation

- Physical frame allocator (#38): a bitmap allocator (one bit per 4 KiB frame)
  built from the memory map, per [ADR 0010](docs/adr/0010-bitmap-frame-allocator.md).
  The allocation *policy* lives in the new `frame-alloc` crate —
  `#![forbid(unsafe_code)]`, host-unit-tested — which decides which frame to
  hand out over an abstract bitmap. `hal::frame` backs that bitmap with real
  RAM through the HHDM and zeroes every frame on allocation (no stale data
  leaks to a new owner). Frame 0 and the bitmap's own storage are reserved;
  double-free and free-of-untracked are detected as bugs. `kmain` brings the
  allocator up from `BootInfo` and runs an alloc/free smoke test. Single-frame
  only; large-page / DMA-contiguous allocation is deferred (ADR 0010).

## [0.1.0] - 2026-10-28

M1: the kernel takes ownership of the CPU. It installs its own GDT/TSS and IDT,
handles exceptions with diagnostics and backtraces, understands its physical
memory and CPU features, and logs to both serial and an on-screen framebuffer
console.

### Added

- Structured kernel logging (#12): `hal::sync::SpinLock<T>` (minimal spinlock)
  and a `kernel::log` module with severity levels, a runtime level filter, a
  spinlock-guarded serial sink, and `error!`/`warn!`/`info!`/`debug!`/`trace!`
  macros. `kmain` now logs `[INFO]`/`[DEBUG]` records; the panic handler
  deliberately bypasses the logger (writes to its own serial port) to stay
  deadlock-free.
- GDT and TSS (#9): `hal::gdt` installs our own GDT (kernel + user code/data
  descriptors ordered for a future `syscall`/`sysret`) and a 64-bit TSS with
  dedicated IST stacks for #DF, NMI, and #MC plus an `rsp0` stack, then reloads
  the segment registers and the task register. Hand-rolled per
  [ADR 0009](docs/adr/0009-hand-rolled-cpu-tables.md).
- IDT and exception handling (#10): `hal::idt` installs a 256-entry IDT whose
  gates point at generated asm stubs that normalize the error code, build a
  `TrapFrame`, and call a table-driven dispatcher. `register_handler(vector, fn)`
  lets IRQ drivers hook in later (M3); CPU exceptions without a handler print a
  diagnostic report (vector, error code, CR2 for #PF, register dump) and halt,
  except #BP which resumes. Critical faults use the IST stacks from #9. A
  shared `Racy` one-time-init cell was factored out of `gdt`. `kmain` triggers a
  test `#BP` and resumes.
- Panic reporting with backtrace (#11): `hal::backtrace` walks the frame-pointer
  chain (the kernel is now built with `force-frame-pointers=yes`). The panic
  handler prints the message, location, and a backtrace; fatal exceptions add a
  backtrace to their register dump. Addresses resolve with
  `llvm-addr2line -e <kernel> <addr>` (verified against a real call chain).
- Memory map & CPU features (#13): the boot code parses the Limine memory map
  into `BootInfo` (regions copied into a fixed array; usable/total RAM, HHDM
  offset, kernel phys/virt base), and `hal::cpuid` detects the CPU vendor, brand,
  and features (NX, SSE2, SMEP, SMAP, x2APIC, XSAVE, PCID, 1 GiB pages, RDRAND,
  invariant TSC, syscall). `kmain` logs a RAM and CPU summary and asserts NX +
  SSE2 are present. Host unit tests cover the `BootInfo` accounting.
- Framebuffer console (#14): `hal::framebuffer` renders the bundled public-domain
  8x8 bitmap font (`hal::font`) onto the Limine framebuffer — glyphs, newlines,
  tabs, and scrolling — and registers as a second log sink, so kernel output
  appears on screen as well as serial (the path that matters on real hardware
  without a serial port). `BootInfo` carries the framebuffer geometry.

## [0.0.1] - 2026-10-26

M0: the project boots. Repository foundation, toolchain, and a bootable image
that reaches the kernel and prints over serial in QEMU.

### Added

- Project foundation for a security-focused Rust microkernel: README,
  contributing guide, code of conduct, security policy, changelog.
- Development environment guide (Rust bare-metal on macOS/Linux/WSL2).
- Architecture docs: overview (microkernel), security model (capabilities),
  threat model, boot process, memory layout.
- ADRs 0001–0008: Rust, Rust+LLVM toolchain, Limine, microkernel, Cargo+xtask,
  capability security model, `forbid(unsafe)` policy, native tooling runtime.
- Roadmap with milestones M0–M7 and a continuous hardening checklist.
- GitHub issue forms (bug, feature, security/tooling) and PR template.
- Repo config: `rust-toolchain.toml`, `rustfmt.toml`, `.editorconfig`,
  `.gitignore`, `.gitattributes`.
- Cargo workspace skeleton (#1): `kernel`, `hal`, `libs/boot-info`, `xtask`
  crates; `cargo xtask build` cross-compiles the kernel to
  `x86_64-unknown-none`. `hal` owns the only `unsafe` (a `hlt` halt loop); the
  `kernel` binary links with `_start` → `kmain` and a halting panic handler.
- CI pipeline (#8, part of #7): GitHub Actions running rustfmt, clippy
  (`-D warnings`) for the bare-metal and host crates, `cargo xtask build`, the
  `scripts/check-unsafe.sh` policy check, and `cargo deny` / `cargo audit` for
  supply chain. `deny.toml` added. CI badge in the README. The QEMU boot smoke
  test is deferred to when the boot path (#2–#5) lands.
- Higher-half linker script and Limine requests (#2): `kernel/arch/x86_64/linker.ld`
  links the kernel at `0xffffffff80000000` with separate R-X / R / RW segments.
  `kernel/src/boot/` declares the Limine requests (base revision, memory map,
  HHDM, executable address) and the ELF `_start`, which validates the protocol
  and calls `kmain`. `limine` 0.5 dependency (stable-compatible). All
  bootloader/linker `unsafe` is confined to the `boot` module, per the updated
  ADR 0007.
- Bootable ISO (#3): `cargo xtask iso` fetches the pinned Limine binary
  (`v11.x-binary`), assembles an `iso_root` with the kernel and `boot/limine.conf`,
  and builds a hybrid BIOS + UEFI image at `build/kernel.iso` via `xorriso` +
  `limine bios-install`. UEFI boot verified in QEMU (reaches long mode, clean
  handoff to the kernel).
- 16550 UART serial driver (#5): `hal::io` (raw port I/O) and `hal::serial`
  (polled COM1 driver with a loopback self-test, `\n`→`\r\n`, and a
  `core::fmt::Write` impl). `kmain` prints "hello from the kernel" and the panic
  handler reports over serial. First visible end-to-end boot, verified in QEMU
  (UEFI): `hello from the kernel`.
- `cargo xtask run` / `debug` (#6): build the ISO and boot it in QEMU in one
  command. Defaults to UEFI with automatic OVMF discovery (`OVMF` env override);
  `--bios` and `--kvm` flags; extra args pass through after `--`. `debug` boots
  paused with a GDB stub on `:1234`.
- CI boot smoke test (#7, completes #7): `scripts/boot-test.sh` boots the ISO
  headless in QEMU and asserts the kernel prints its serial marker; a CI matrix
  runs it for both BIOS and UEFI (BIOS boot is now covered on Linux runners).
  Serial logs are uploaded as artifacts. **M0 complete.**

[0.1.0]: https://github.com/seedon198/kernel/releases/tag/v0.1.0
[0.0.1]: https://github.com/seedon198/kernel/releases/tag/v0.0.1
