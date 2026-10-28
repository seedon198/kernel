# 0010: Bitmap physical frame allocator

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

M2 needs a physical frame allocator: the authority that hands out 4 KiB
physical frames (from the Limine memory map parsed in #13) to the page-table
code, the kernel heap, and later device drivers. It is a foundational piece of
the Trusted Computing Base — every other memory consumer sits on top of it — so
its design is recorded here.

We must choose a data structure, and decide whether to support contiguous /
large (2 MiB, DMA) allocations from the start.

## Options considered

1. **Bitmap**: one bit per frame in a kernel-owned array (the array itself is
   carved out of a usable region at init). `alloc` scans from a next-free hint
   for a clear bit and sets it; `free` clears it. Contiguous runs fall out of a
   multi-bit scan. Smallest, most auditable code; deterministic; the metadata
   lives *outside* the frames, so a freed frame has no in-band links to corrupt,
   and a double-free or a free of a non-owned/reserved frame is detectable.
   Cost: `alloc` is O(n) worst case, mitigated by the hint and acceptable at our
   scale (a few hundred thousand frames per GiB).
2. **Free-list stack**: a LIFO list threaded through the free frames
   themselves. O(1) alloc/free, but the intrusive links are a classic
   use-after-free / corruption surface, validating that a freed frame is
   genuinely free-able is hard, and it offers no contiguous allocation.
3. **Buddy allocator**: power-of-two block splitting and coalescing. Natively
   serves contiguous, aligned, and large-page/DMA allocations in O(log n). But
   it is substantially more code — the largest TCB to audit — for capability we
   do not need until paging maps large pages (later in M2) or devices need DMA
   (M6).

## Decision

**Bitmap** (option 1), **single-frame now, contiguous deferred**. For a kernel
whose central claim is a small, auditable TCB, the bitmap is the least code and
the easiest to reason about, and keeping allocation metadata out of the frames
removes a whole class of corruption bugs. We expose `alloc_frame` / `free_frame`
now; the bitmap also makes a bounded `alloc_contiguous(n)` scan cheap to add as
a bonus, but a first-class large-page / DMA-aligned path is **deferred** until a
real consumer needs it, at which point we revisit with a new ADR (bitmap runs,
or a buddy layer above the bitmap).

## Consequences

- Minimal, auditable allocator in the TCB; metadata outside the frames.
- Double-free and free-of-reserved-frame are detectable and treated as bugs
  (panic in debug, at minimum a logged error) rather than silent corruption.
- `alloc` is O(n) worst case; the next-free hint keeps the common case fast, and
  the cost is acceptable at our frame counts. Revisit if profiling says
  otherwise.
- Large-page / DMA-contiguous allocation is **not** provided yet; the first
  consumer that needs it triggers a follow-up ADR.
- The bitmap is `forbid(unsafe)`-clean Rust in a kernel module; the only
  `unsafe` is reading/writing physical frames through the HHDM, which lives in
  `hal`.
