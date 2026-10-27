# Architecture Decision Records

An ADR captures one significant decision: its context, the options considered,
and the consequences. ADRs are immutable once accepted; to change a decision,
write a new ADR that *supersedes* the old one.

Significant decisions are **discussed with the maintainer before being
accepted** — ADRs record the outcome of that discussion, they don't replace it.

| # | Title | Status |
|---|---|---|
| [0001](0001-implementation-language.md) | Implementation language: Rust | Accepted |
| [0002](0002-toolchain.md) | Toolchain: Rust + LLVM, QEMU | Accepted |
| [0003](0003-bootloader.md) | Bootloader: Limine | Accepted |
| [0004](0004-kernel-architecture.md) | Kernel architecture: microkernel | Accepted |
| [0005](0005-build-system.md) | Build system: Cargo + xtask | Accepted |
| [0006](0006-security-model.md) | Security model: capabilities | Accepted |
| [0007](0007-unsafe-policy.md) | `unsafe` policy: forbid outside `hal` | Accepted |
| [0008](0008-native-tooling.md) | Native tooling via an embedded scripting runtime | Accepted |
| [0009](0009-hand-rolled-cpu-tables.md) | Hand-roll the x86 CPU tables (no `x86_64` crate) | Accepted |

New ADR: copy [0000-template.md](0000-template.md), take the next number, and
open a PR labelled `adr`.
