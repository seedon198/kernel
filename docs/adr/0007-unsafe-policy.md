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

**Option 2.** Every crate declares `#![forbid(unsafe_code)]` except `hal` and
the kernel *binary* crate (see the exception below). `hal` is the small,
audited home of all genuine `unsafe`; each block is minimal and preceded by a
`// SAFETY:` comment justifying soundness. We additionally run **Miri** on
host-testable crates and **clippy** (`undocumented_unsafe_blocks`) in CI.
Formal verification (Option 3) remains a "beyond" goal, layered on later
without changing this policy.

### Exception: the binary entry shim

As of the Rust 2024 edition, `#[no_mangle]` is classified as an *unsafe
attribute* (`#[unsafe(no_mangle)]`), and `#![forbid(unsafe_code)]` rejects it
with no possible local override — so a crate that defines the ELF entry symbol
`_start` cannot be `forbid`. This attribute is linker glue, not memory-unsafe
code.

Therefore the **kernel binary crate** uses `#![deny(unsafe_code)]` (not
`forbid`) with exactly **one** `#[allow(unsafe_code)]`, placed on the `_start`
shim, whose body only builds `BootInfo` and calls the safe `kmain`. `deny`
keeps every other line in the crate unable to use `unsafe`; the single
allowance is greppable and **CI-enforced**: CI asserts that outside `hal/` the
only `#[allow(unsafe_code)]` attribute is the one on `_start`, and that no
crate re-enables `unsafe` otherwise. Pure-logic crates (`libs/*`, and later the
user-space servers and tools) remain `#![forbid(unsafe_code)]`.

(We chose this over introducing a separate `boot` bin that wraps a `forbid`
kernel *library*: the one audited, CI-checked line is simpler than an extra
crate, and the guarantee — no `unsafe` anywhere in kernel logic — is the same.)

## Consequences

- The dangerous surface is one small, reviewable crate (`hal`) plus a single
  CI-checked attribute line, instead of the whole tree — a concrete,
  enforceable security property.
- Forces clean, safe abstractions over hardware, which improves design.
- Some ergonomics cost: occasionally a safe wrapper must be added to `hal`
  before higher layers can use a feature. Accepted.
- CI must enforce it (`forbid`/`deny` are per-crate; a grep guards against a
  stray `#[allow(unsafe_code)]` or `unsafe` token outside `hal/`, allowing only
  the one `_start` allowance). The grep must match the *attribute*, not
  mentions of it in comments or docs.
