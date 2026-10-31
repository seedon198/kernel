# 0006: Security model: capabilities

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

Given a microkernel ([ADR 0004](0004-kernel-architecture.md)) whose point is a
minimal TCB, we need a model for *authority*: how a task is granted the right
to use a resource (memory, a device, another task). The model shapes every
interface in the system. It must enforce least privilege and make privilege
flows auditable - both for hardening and so that "offensive" capabilities are
contained rather than ambient.

## Options considered

1. **Unix-style (users, groups, rwx, root)**: familiar, but built on *ambient
   authority* - a process acts with all its user's privileges implicitly, and
   "root" is all-powerful. Poor fit for least privilege and a small TCB.
2. **ACLs per object**: finer-grained than Unix, but still ambient (the subject
   is identified, then checked) and centralizes policy.
3. **Capabilities (object-capability model, seL4-style)**: authority *is*
   possession of an unforgeable handle. No ambient authority, no root; a task
   can do exactly what its capabilities permit. Delegation is explicit (send a
   capability over IPC) and attenuable (derive weaker caps). Proven in seL4.
   Cost: unfamiliar; everything must be expressed as capabilities and IPC.

## Decision

A **capability system**. Every resource (physical memory, frames, address
spaces, threads, IPC endpoints, IRQs, I/O regions) is a kernel object named by
an unforgeable capability in a per-task table. Tasks hold no ambient authority;
rights are granted by explicit delegation over IPC and can only be attenuated
when derived. Details in [security-model.md](../architecture/security-model.md).

## Consequences

- Least privilege is the default, not an add-on: a driver gets only its
  device's region and IRQ.
- Powerful/offensive primitives (raw sockets, promiscuous mode) are just
  capabilities, so they are confined and their delegation is auditable.
- Requires designing the full object/rights set and the IPC that carries
  capabilities (M3). Adds conceptual overhead for contributors used to Unix.
- Enables a possible future verification effort on the core.
