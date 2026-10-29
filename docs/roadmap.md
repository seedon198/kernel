# Roadmap

Development is organized into **milestones**. Each ends in a state that boots
in QEMU (and, where relevant, on real hardware) and passes CI. Tracked on
GitHub: <https://github.com/seedon198/kernel/milestones>.

Two themes run through every milestone:

- **Hardening** — the privileged core stays tiny and memory-safe; every
  addition is justified against the [threat model](architecture/threat-model.md).
- **Offense enablement** — the system is steadily shaped toward running native
  security tooling.

---

## M0: Foundation

*Goal: a Rust `no_std` kernel boots via Limine and prints "hello" over serial;
`cargo xtask run` does it in one command; CI proves it.*

- [ ] Workspace skeleton (`kernel`, `hal`, `xtask`, `libs`) and `rust-toolchain.toml`
- [ ] `x86_64-unknown-none` build; `panic=abort`, `build-std` if a custom target is needed
- [ ] Higher-half linker script; Limine protocol requests
- [ ] `_start` → `kmain`, base-revision check, halt loop
- [ ] 16550 UART serial driver (in `hal`) for early output
- [ ] `cargo xtask {build,run,debug,iso}` wrapping Limine + xorriso + QEMU
- [ ] CI: build, `cargo clippy -D warnings`, `rustfmt --check`, QEMU boot smoke test
- [ ] `forbid(unsafe)` enforced in `kernel` from commit one

## M1: Kernel core

- [ ] `hal`: GDT/TSS, IDT, all CPU exception handlers (the audited unsafe core)
- [ ] `panic` handler with register dump + backtrace
- [ ] Structured logging (levels, serial sink; framebuffer sink later)
- [ ] Parse the Limine memory map and CPU features (NX, SMEP, SMAP, x2APIC)
- [ ] Host-testable core logic where possible

## M2: Memory & isolation

See [architecture/memory-management.md](architecture/memory-management.md).

- [x] Physical frame allocator (bitmap)
- [x] Virtual memory: own page tables, per-address-space isolation
- [x] **W^X** enforced, **NX** everywhere (incl. HHDM), **KASLR** for the kernel image
- [x] SMEP/SMAP enabled; guard pages; kernel heap
- [x] Page-fault diagnostics
- [x] `architecture/memory-management.md`; threat model updated

## M3: Capabilities & IPC

*The microkernel's heart. See [security model](architecture/security-model.md)
and [capabilities.md](architecture/capabilities.md).*

- [x] Capability object model (unforgeable handles, rights, derivation)
- [x] Per-task capability space; no ambient authority (flat table / CNode)
- [x] Untyped memory & retype; core object kinds
- [x] Capability invocation API (rights-checked `invoke`)
- [x] ADR finalizing the capability design ([ADR 0016](adr/0016-capability-implementation.md))

Synchronous IPC (blocking `Endpoint` send/receive/call) and the ring-3
`syscall` entry need threads/userspace and are part of **M4**.

## M4: Scheduling & userspace

- [x] Threads, context switch, preemptive scheduler (LAPIC timer)
- [ ] Ring 3 entry/return (`syscall`/`sysret`) with SMEP/SMAP
- [ ] First user task running under a restricted capability set

## M5: User servers

- [ ] Server runtime library (`libs/`): receive-loop, capability plumbing
- [ ] Device manager server; bus enumeration (PCI/PCIe via ECAM)
- [ ] Driver-as-server model proven end to end (e.g. serial or a timer server)

## M6: Devices & network

- [ ] virtio drivers (net, block) as servers — QEMU bring-up
- [ ] Real NIC drivers (Intel e1000e / Realtek) for generic laptops
- [ ] NVMe and xHCI (USB) drivers as servers
- [ ] **Sandboxed** network stack (Ethernet/ARP/IPv4/IPv6/TCP/UDP) in a server
- [ ] **Raw packet / promiscuous-mode** capability for offensive tooling

## M7: Security runtime & native tools

- [ ] Embedded scripting runtime in a user-space service
- [ ] Capability-mediated API surface exposed to scripts (raw sockets, etc.)
- [ ] First native tools: port scanner, packet sniffer, a basic fuzzer harness
- [ ] Tooling that is auditable and runs without recompiling the OS

## Beyond

Wireless (802.11 + firmware — hard), storage filesystems, a shell, a package
format for native tools, reproducible builds, optional formal verification of
the capability core, self-hosting.

## Hardening checklist (continuous)

- [ ] `forbid(unsafe)` outside `hal`; every `unsafe` block carries a `// SAFETY:` invariant
- [ ] `cargo clippy -D warnings`, `cargo deny`, `cargo audit` in CI
- [ ] Miri on host-testable crates; fuzzing on all parsers
- [ ] W^X, NX, KASLR, SMEP/SMAP, stack probes, guard pages
- [ ] Documented, minimal TCB; each privileged addition reviewed against the threat model
