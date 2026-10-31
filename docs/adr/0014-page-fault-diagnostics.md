# 0014: Page-fault diagnostics and recoverable probes

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

M2 (#42) makes page faults legible: decode the #PF error code, say *where*
the faulting address lives (kernel section, HHDM, heap, a stack guard,
unmapped), and show a backtrace. It should also distinguish a guard-page hit
and a permission (W^X/NX) violation. A standing gap from #40/#41 was the lack
of a *negative* test - code that actually triggers a protection fault - because
the #PF handler halts.

## Decision

**Decode in the pure crate.** `page-table` gains `PageFaultError`, which
decodes the CPU's #PF error-code bits (present/protection, read/write,
user/supervisor, reserved-bit, instruction-fetch, protection-key,
shadow-stack). It is `#![forbid(unsafe_code)]` and host-unit-tested; the #PF
reporter in `hal::idt` uses it to print a plain-language cause.

**Classify via a region registry.** `hal::fault` keeps a small fixed table of
named virtual ranges that the kernel registers (its sections, the HHDM, the
heap window, each stack guard). `classify(addr)` returns the **narrowest**
containing region, so a specific range (a stack guard sitting inside `.bss`)
wins over the enclosing one. The #PF report names the region `CR2` fell in.

**A recoverable probe (one-entry exception table).** `hal::fault::probe_read`
reads one byte through a tiny assembly stub; if that read faults, the #PF
handler recognizes the stub's instruction pointer and resumes at a fixup that
returns "faulted" instead of halting. This proves a mapping is absent (e.g. a
stack guard) without crashing, and is the seed of the fault-tolerant accessors
that safe user-memory access will need (M4). The handler runs this recovery
check first, so probe faults resume silently; all other faults report and halt
as before.

## Consequences

- A real #PF now prints: the decoded cause, the faulting address and the region
  it belongs to, the register dump, and a backtrace.
- Guard-page hits and permission violations are distinguishable from the report
  (region name + "protection violation" vs "not present", and instruction-fetch
  for an NX/SMEP violation).
- `probe_read` gives us a crash-free way to test mappings and verifies, in
  `kmain`, that a guard page faults-and-recovers while mapped memory reads
  cleanly.
- The one-entry exception table is deliberately minimal; it generalizes later
  into a proper fixup table for `copy_from_user`-style accessors (M4).
- The registry is a small fixed array; it is ample for the kernel's own ranges.
