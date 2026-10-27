//! Workspace task runner.
//!
//! Invoked as `cargo xtask <command>` (see `.cargo/config.toml`). This runs on
//! the host and orchestrates cross-compiling the kernel, and later building
//! the ISO and launching QEMU (#3, #6).

use std::process::{exit, Command};

/// The bare-metal target the kernel is compiled for (ADR 0002).
const KERNEL_TARGET: &str = "x86_64-unknown-none";

fn main() {
    let task = std::env::args().nth(1).unwrap_or_default();
    match task.as_str() {
        "build" => build(),
        "" => {
            eprintln!("usage: cargo xtask <build>");
            exit(2);
        }
        other => {
            eprintln!("unknown xtask command: {other:?}\nusage: cargo xtask <build>");
            exit(2);
        }
    }
}

/// Cross-compile the kernel for the bare-metal target.
fn build() {
    // Reuse the cargo that invoked us so the pinned toolchain is honored.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(cargo)
        .args(["build", "--package", "kernel", "--target", KERNEL_TARGET])
        .status()
        .expect("failed to spawn cargo");
    if !status.success() {
        exit(status.code().unwrap_or(1));
    }
    println!("kernel built for {KERNEL_TARGET}");
}
