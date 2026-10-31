# 0016: Capability system implementation (flat cap space, Untyped retype)

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

[ADR 0006](0006-security-model.md) fixed *what* the security model is - seL4-style
unforgeable capabilities, no ambient authority, attenuation-only derivation.
M3 implements it. This ADR records the *how*, and the sequencing, since IPC has
a prerequisite the roadmap places later.

## Decisions

**1. Scope: build the capability model in M3; defer blocking IPC and the
syscall entry to M4.** Synchronous IPC needs threads that can block, and threads,
userspace, and the ring-3 `syscall` entry are M4. The capability *model* (cap
space, object types, rights, attenuation, the invocation API) is largely pure
logic that stands alone and is host-testable now. So M3 delivers the model;
M4 adds threads and, on top of them, blocking `Endpoint` IPC and the user→kernel
`syscall` boundary that invokes capabilities. No throwaway harness.

**2. Capability space: a flat per-task table.** A `CapSpace` is a (two-level,
for growth) table of slots; a `CapId` is an index. Lookup is O(1); the structure
is small and auditable. We deliberately do **not** adopt seL4's CNode radix tree;
its sparse flexibility is not worth the complexity at our stage. The capability
*algebra* (rights, attenuation, copy/mint/move/delete) lives in a pure
`capability` crate, `#![forbid(unsafe_code)]` and host-tested, generic over the
object-handle type so the kernel binds it to real objects.

**3. Object memory: Untyped retype (seL4-style), no kernel heap for objects.**
Kernel objects (`Frame`, `Endpoint`, `Thread`, `AddressSpace`, `Notification`,
`CNode`, …) are created by **retyping** `Untyped` memory a task already holds;
the kernel never allocates object storage from its own heap on a task's behalf.
A task therefore spends its own memory budget, and cannot exhaust kernel memory -
a real security property, and what [security-model.md](../architecture/security-model.md)
already commits to. The root server receives `Untyped` covering free RAM at boot
and hands out attenuated capabilities from there.

**4. Rights and derivation.** Rights are bits on a capability. A capability may
be copied only into an equal-or-weaker one (attenuation); a derived capability
can never gain rights or widen a range. `Endpoint` capabilities carry a **badge**
(an identifier the receiver sees) that can be set once when minting.

## Consequences

- The pure capability core (rights, attenuation, cap space) is unit-tested on
  the host with no hardware; the kernel instantiates it over real object handles.
- Untyped/retype adds machinery (region accounting, object initialization in
  retyped memory via the HHDM - the `unsafe` for that stays in `hal`), but gives
  a bounded-kernel-memory guarantee from day one.
- The capability-invocation API is defined in M3 as kernel functions; M4 wires
  the ring-3 `syscall` entry to them, so there is a clean seam.
- `Endpoint` objects can be created in M3, but their blocking send/receive/call
  lands in M4 with threads.
- The roadmap's M3 "synchronous IPC" and "syscall surface" items move to M4;
  this is recorded here and in the epics.
