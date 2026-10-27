# 0009: Hand-roll the x86 CPU tables (no `x86_64` crate)

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

The kernel needs the architectural CPU structures: the GDT and TSS (M1),
the IDT (M1), page tables (M2), and APIC/MSR access (M3). The Rust ecosystem
has the mature, widely used `x86_64` crate that provides safe wrappers for all
of these. We must decide whether to depend on it or write our own in `hal`.

This decision sets a precedent for every CPU-specific structure in `hal`, so
it is recorded here once.

## Options considered

1. **Hand-roll in `hal`**: write the descriptor encodings, TSS, IDT entries,
   page-table types, and the `lgdt`/`lidt`/`ltr`/MSR asm ourselves, each
   `unsafe` block carrying a `// SAFETY:` note. Zero dependencies in or near the
   Trusted Computing Base; fully auditable; matches the project's "from
   scratch" goal and is the most educational. Cost: more code, and subtle
   descriptor-bit mistakes are possible (they manifest as triple faults, caught
   by boot-testing).
2. **Depend on the `x86_64` crate**: far less code, battle-tested correctness,
   faster progress. Cost: a third-party dependency with its own `unsafe` sits
   adjacent to the TCB; less "from scratch"; `cargo deny`/`audit` must cover it.
3. **Hybrid**: hand-roll the small tables (GDT/TSS/IDT), adopt the crate later
   only for the fiddly parts (paging/APIC) if hand-rolling proves slow.

## Decision

**Hand-roll in `hal`** (option 1). For a security-focused kernel whose central
claim is a small, auditable TCB, writing the privileged CPU structures
ourselves keeps everything in the trust boundary under our review, and the
structures (GDT/TSS/IDT) are small and well-specified. We accept the extra code
and rely on boot-testing in CI to catch encoding mistakes.

If a later component (e.g. full paging or ACPI/APIC) proves disproportionately
error-prone or slow to hand-roll, we may revisit *that component* with a new
ADR — this decision does not forbid the crate forever, it sets the default.

## Consequences

- No third-party code in the TCB; the entire privileged surface is ours and is
  reviewed.
- More code to write and get right; correctness is guarded by `_Static_assert`-
  style `const` size checks and the QEMU boot test (bad descriptors triple-fault).
- Each hand-rolled structure cites the Intel SDM section it implements.
- Reversible per-component via a future ADR if the cost outweighs the benefit.
