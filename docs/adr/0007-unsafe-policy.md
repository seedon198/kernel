# 0007: `unsafe` policy: forbid outside `hal`

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

Rust ([ADR 0001](0001-implementation-language.md)) guarantees memory safety
only for *safe* code; `unsafe` blocks opt out and are where memory-safety bugs
can still occur. A kernel unavoidably needs some `unsafe` (MMIO, MSRs, page
tables, inline asm). The security value of Rust depends on how tightly that
`unsafe` is contained and reviewed. We want the strongest discipline the
project can sustain.

## Options considered

1. **Pragmatic unsafe**: use `unsafe` wherever convenient, wrapped in safe
   APIs. Fastest, normal practice — but `unsafe` ends up scattered across many
   crates, enlarging the audit surface.
2. **`forbid(unsafe_code)` outside a single audited `hal` crate**: all raw
   hardware access is concentrated in `hal`; the kernel, servers and tools
   cannot write `unsafe` at all (the compiler rejects it). Every `unsafe` block
   in `hal` carries a `// SAFETY:` invariant and is reviewed. Strong guarantee;
   requires designing clean safe abstractions over hardware.
3. **Formal/verified unsafe** (Kani, Miri-proven, Prusti): highest assurance,
   but research-grade effort that would dominate early development.

## Decision

**Option 2.** Every crate declares `#![forbid(unsafe_code)]` except `hal`.
`hal` is the small, audited home of all `unsafe`; each block is minimal and
preceded by a `// SAFETY:` comment justifying soundness. We additionally run
**Miri** on host-testable crates and **clippy** (`undocumented_unsafe_blocks`)
in CI. Formal verification (Option 3) remains a "beyond" goal, layered on top
later without changing this policy.

## Consequences

- The dangerous surface is one small, reviewable crate instead of the whole
  tree — a concrete, enforceable security property.
- Forces clean, safe abstractions over hardware, which improves design.
- Some ergonomics cost: occasionally a safe wrapper must be added to `hal`
  before higher layers can use a feature. Accepted.
- CI must enforce it (`forbid` is per-crate; clippy/grep guard against a stray
  `#![allow(unsafe_code)]`).
