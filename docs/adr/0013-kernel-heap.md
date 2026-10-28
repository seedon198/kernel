# 0013: Hand-rolled growable free-list kernel heap

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

M2 (#41) adds a kernel heap so `alloc` types (`Box`, `Vec`, `String`, …) work
in the kernel, alongside two smaller hardening steps in the same issue
(SMEP/SMAP and stack guard pages). The heap is the design decision: what
allocator, and how its backing memory is sized.

## Decision

**A hand-rolled first-fit free-list allocator with coalescing, in `hal::heap`.**
Free memory is threaded as an address-sorted singly linked list of nodes stored
inside the free blocks; allocation is first-fit, freeing coalesces adjacent
blocks, and a small header stored just before each allocation records the exact
block to return on `dealloc` (so any alignment is handled without heuristics). A
minimum allocation alignment keeps that header and any folded free node
naturally aligned. This matches the project's from-scratch, zero-dependency,
auditable-core ethos (as with the frame allocator and page tables), rather than
pulling in a third-party allocator crate next to the TCB.

**Growable from the start.** The heap begins with one mapped chunk and, when no
block fits, calls a `grow` hook that maps fresh frames (RW+NX) contiguously at
the next heap virtual address and hands the region back; it coalesces onto the
pool, so no fixed ceiling is baked in. The hook is supplied by the kernel (it is
the only part that knows the kernel address space and frame allocator), keeping
`hal::heap` decoupled. The heap lives in a dedicated higher-half window clear of
the kernel image.

**Where the `unsafe` lives.** Because the free list is stored *in* the managed
memory, the bookkeeping is inherently `unsafe` and therefore lives in `hal`
(not a pure, host-tested crate like `frame-alloc`/`page-table`). It is verified
by an in-kernel self-test that forces a growth and checks `Vec`/`Box`
round-trip and full reclamation.

**Companion hardening in #41:**

- **SMEP/SMAP** enabled in `CR4` where CPUID reports them. Safe now because no
  user mappings exist; once they do (M4), deliberate kernel access to user
  memory must bracket with `stac`/`clac`. (The default QEMU CPU lacks both, so
  the test VM runs `-cpu qemu64,+smep,+smap`.)
- **Stack guard pages:** each HAL-owned stack (the #DF/NMI/#MC IST stacks and
  `rsp0`) is laid out with a page-aligned guard page below it, which the kernel
  unmaps, so an overflow faults instead of silently corrupting a neighbor.

## Consequences

- General-purpose kernel allocation with zero dependencies; the allocator is in
  the audited `hal`.
- First-fit + coalescing keeps fragmentation bounded for typical kernel use; it
  is not optimized for highly concurrent or pathological workloads — revisit
  (e.g. slab caches) if profiling demands.
- The heap can grow without a preset ceiling; a partial mapping failure at true
  OOM leaks a few frames (acceptable — the system is out of memory).
- The grow hook must never itself allocate on the heap (it runs under the heap
  lock); lock order is heap → {kernel page tables, frame allocator}, and nothing
  on that path allocates, so there is no cycle.
- A negative test (actually faulting on an SMAP/SMEP or guard-page violation)
  awaits the page-fault handler not halting (#42); verification here is by the
  self-test and the enabled-bit/flag inspection.
