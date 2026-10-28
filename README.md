<div align="center">

# kernel

**A from-scratch, security-focused x86-64 microkernel in Rust — hardened and
capability-based, built to grow into a native security platform.**

[![CI](https://github.com/seedon198/kernel/actions/workflows/ci.yml/badge.svg)](https://github.com/seedon198/kernel/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Status: pre-alpha](https://img.shields.io/badge/status-pre--alpha-orange)
![Arch: x86--64](https://img.shields.io/badge/arch-x86--64-informational)
![Language: Rust](https://img.shields.io/badge/language-Rust-b7410e)

</div>

---

> [!WARNING]
> **Early development — the kernel, not yet a full OS.** The kernel boots on
> x86-64 (UEFI + BIOS) in QEMU and installs its own CPU tables, fault handling,
> and physical memory management (as of
> [v0.1.0](https://github.com/seedon198/kernel/releases/tag/v0.1.0); milestones
> M0–M1). User space, the capability system, and the security tooling are still
> ahead. See the [Roadmap](docs/roadmap.md) and
> [milestones](https://github.com/seedon198/kernel/milestones). For authorized
> security research and testing.

## What this is

A clean-slate, security-focused **x86-64 microkernel**, written from scratch in
Rust. The kernel is the foundation; the longer-term aim is to grow a full
security/pentesting OS around it. Two goals drive the design:

1. **Hardened by construction.** A **microkernel** written in **Rust**: the
   privileged core is tiny, memory-safe, and holds no `unsafe` code outside a
   small audited hardware layer. Drivers, the network stack, and filesystems
   are designed to run as isolated, least-privilege user-space servers, so a
   bug in a parser cannot compromise the kernel.
2. **A native security platform (the goal).** The system is being built to
   *do* security work — raw packet access, sniffing, scanning, fuzzing —
   through **native tools** driven by an **embedded scripting runtime**, so you
   can write and run tooling without rebuilding the OS. This layer is future
   work (milestone M7); see the roadmap.

It targets **generic modern x86-64 PCs**; booting from USB on real hardware is
a design goal, and today it runs in QEMU (UEFI + BIOS).

> This project is for **authorized** security testing, research and education.
> See [SECURITY.md](SECURITY.md) and the [threat model](docs/architecture/threat-model.md).

## Design at a glance

| Area | Choice | Why | ADR |
|---|---|---|---|
| Language | **Rust** (`no_std`) | Memory safety removes whole exploit classes from the kernel | [0001](docs/adr/0001-implementation-language.md) |
| `unsafe` policy | **`forbid(unsafe)`** except an audited `hal` crate | Shrinks the trusted, unsafe surface to an auditable core | [0007](docs/adr/0007-unsafe-policy.md) |
| Kernel model | **Microkernel** | Minimal ring-0 TCB; isolate drivers/net/fs in user space | [0004](docs/adr/0004-kernel-architecture.md) |
| Security model | **Capabilities** (least privilege) | No ambient authority; every resource is an unforgeable handle | [0006](docs/adr/0006-security-model.md) |
| Offensive layer | **Native tools + scripting runtime** | Write/run security tooling without recompiling the OS | [0008](docs/adr/0008-native-tooling.md) |
| Toolchain | **Rust + LLVM**, QEMU | Cross-compiles to bare metal from macOS/Linux out of the box | [0002](docs/adr/0002-toolchain.md) |
| Bootloader | **Limine** | Long mode + higher-half + UEFI/BIOS + memory map on entry | [0003](docs/adr/0003-bootloader.md) |
| Build | **Cargo + `xtask`** | Native Rust build; a Rust task runner wraps ISO/run/debug | [0005](docs/adr/0005-build-system.md) |

Every decision above is recorded as an [ADR](docs/adr/) with the alternatives
considered. Decisions are made **with the maintainer**, not unilaterally.

## Getting started

Development works on **macOS (Apple Silicon and Intel)**, **Linux**, and
**Windows via WSL2** — you do not need an x86 machine. See
[docs/development-environment.md](docs/development-environment.md).

```sh
# macOS
rustup target add x86_64-unknown-none
brew install qemu xorriso
git clone https://github.com/seedon198/kernel.git && cd kernel
cargo xtask run        # build the ISO and boot it in QEMU (serial on your terminal)
```

## Project layout (planned)

```
.
├── kernel/            # the microkernel (no_std, forbid(unsafe))
│   ├── src/
│   │   ├── cap/       # capability system (the heart of the security model)
│   │   ├── mem/       # address spaces, page allocation
│   │   ├── sched/     # threads, scheduling
│   │   ├── ipc/       # message passing between user servers
│   │   └── syscall/   # the thin system-call boundary
├── hal/               # the ONLY crate allowed `unsafe`: CPU, MMIO, paging, asm
├── servers/           # user-space servers (drivers, net stack, VFS) — isolated
│   ├── devmgr/        # device manager
│   ├── net/           # network stack (high-risk parsing, sandboxed)
│   └── vfs/           # filesystem service
├── runtime/           # embedded scripting runtime for native security tooling
├── tools/             # native security tools (scanner, sniffer, fuzzer, ...)
├── libs/              # shared no_std libraries (capability API, IPC bindings)
├── xtask/             # build/run/debug/ISO automation (cargo xtask ...)
├── boot/              # Limine configuration
└── docs/              # design docs, ADRs, threat model
```

## Roadmap

| Milestone | Goal |
|---|---|
| **M0** Foundation | Rust bare-metal build, Limine boot, serial "hello", `xtask run`, CI |
| **M1** Kernel core | CPU bring-up (GDT/IDT/exceptions) in `hal`, panic, logging, memory map |
| **M2** Memory & isolation | Physical frames, per-address-space paging, W^X, KASLR |
| **M3** Capabilities & IPC | Capability tables, synchronous IPC, the microkernel boundary |
| **M4** Scheduling | Threads, preemptive scheduler, user/kernel split (ring 3) |
| **M5** User servers | First user-space server + device manager over IPC |
| **M6** Devices & network | virtio + real NIC drivers (as servers), the network stack |
| **M7** Security runtime | Embedded scripting runtime and the first native tools |

Details and hardening goals: [docs/roadmap.md](docs/roadmap.md).

## Documentation

- [Development environment](docs/development-environment.md)
- [Architecture overview](docs/architecture/overview.md)
- [Security model](docs/architecture/security-model.md) · [Threat model](docs/architecture/threat-model.md)
- [Boot process](docs/architecture/boot.md) · [Memory layout](docs/architecture/memory-layout.md)
- [Coding style](docs/coding-style.md)
- [Architecture Decision Records](docs/adr/)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and the [Code of Conduct](CODE_OF_CONDUCT.md).
Issues labelled [`good first issue`](https://github.com/seedon198/kernel/labels/good%20first%20issue)
are a good place to start.

## Learning resources

- [OSDev Wiki](https://wiki.osdev.org/) · [Writing an OS in Rust](https://os.phil-opp.com/) (Philipp Oppermann)
- [Intel® 64 and IA-32 SDM](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html) · [AMD64 APM](https://www.amd.com/en/search/documentation/hub.html)
- [seL4](https://sel4.systems/) — capability microkernel design and verification
- [Limine boot protocol](https://github.com/limine-bootloader/limine-protocol)
- *Operating Systems: Three Easy Pieces* ([free](https://pages.cs.wisc.edu/~remzi/OSTEP/))

## License

[MIT](LICENSE) © 2026 Adlin Seedon D'Souza
