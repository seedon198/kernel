#![no_std]
#![deny(missing_docs)]
#![deny(clippy::undocumented_unsafe_blocks)]

//! Hardware abstraction layer for x86-64.
//!
//! This is the **only** crate in the workspace permitted to use `unsafe`
//! (see `docs/adr/0007-unsafe-policy.md`). Every `unsafe` block here is kept
//! minimal and is preceded by a `// SAFETY:` comment stating the invariant
//! that makes it sound. Higher layers (`kernel`, servers, tools) are
//! `#![forbid(unsafe_code)]` and reach the hardware only through the safe
//! abstractions exposed here.

mod cell;
pub mod cpu;
pub mod gdt;
pub mod idt;
pub mod io;
pub mod serial;
pub mod sync;
