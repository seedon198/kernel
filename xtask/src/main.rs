#![forbid(unsafe_code)]

//! Workspace task runner.
//!
//! Invoked as `cargo xtask <command>` (see `.cargo/config.toml`). Runs on the
//! host and orchestrates cross-compiling the kernel, fetching Limine, and
//! assembling a bootable ISO. QEMU run/debug targets land in #6.

use std::path::{Path, PathBuf};
use std::process::{exit, Command};

/// Bare-metal target the kernel is compiled for (ADR 0002).
const KERNEL_TARGET: &str = "x86_64-unknown-none";

/// Pinned Limine release. The binary distribution is published on per-major
/// branches; `v11.x-binary` is the latest prebuilt branch (v12 is source-only).
/// See docs/adr/0003-bootloader.md.
const LIMINE_REPO: &str = "https://github.com/limine-bootloader/limine.git";
const LIMINE_BRANCH: &str = "v11.x-binary";

fn main() {
    let task = std::env::args().nth(1).unwrap_or_default();
    match task.as_str() {
        "build" => {
            build();
        }
        "iso" => {
            iso();
        }
        "" => usage(2),
        other => {
            eprintln!("unknown xtask command: {other:?}");
            usage(2);
        }
    }
}

fn usage(code: i32) -> ! {
    eprintln!("usage: cargo xtask <build|iso>");
    exit(code);
}

/// Run a command, inheriting stdio, and abort the task if it fails.
fn run(program: &str, args: &[&str]) {
    let status = Command::new(program)
        .args(args)
        .status()
        .unwrap_or_else(|e| fail(&format!("failed to spawn `{program}`: {e}")));
    if !status.success() {
        fail(&format!(
            "`{program}` exited with {}",
            status.code().unwrap_or(-1)
        ));
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("xtask: {msg}");
    exit(1);
}

/// `cargo` honoring the toolchain that invoked us.
fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}

/// Cross-compile the kernel and return the path to the resulting ELF.
fn build() -> PathBuf {
    run(
        &cargo(),
        &["build", "--package", "kernel", "--target", KERNEL_TARGET],
    );
    let elf = PathBuf::from("target")
        .join(KERNEL_TARGET)
        .join("debug")
        .join("kernel");
    if !elf.exists() {
        fail(&format!("expected kernel ELF at {}", elf.display()));
    }
    println!("kernel built: {}", elf.display());
    elf
}

/// Clone the pinned Limine binary branch and build its host `limine` tool.
/// Idempotent: skips the clone if `limine/` already exists.
fn fetch_limine() -> PathBuf {
    let dir = PathBuf::from("limine");
    if !dir.exists() {
        run(
            "git",
            &[
                "clone",
                LIMINE_REPO,
                "--branch",
                LIMINE_BRANCH,
                "--depth",
                "1",
                dir.to_str().unwrap(),
            ],
        );
    }
    // Build the `limine` host utility used to install the BIOS stage.
    run("make", &["-C", dir.to_str().unwrap()]);
    dir
}

fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) {
    let (from, to) = (from.as_ref(), to.as_ref());
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|e| fail(&format!("mkdir {}: {e}", parent.display())));
    }
    std::fs::copy(from, to)
        .unwrap_or_else(|e| fail(&format!("copy {} -> {}: {e}", from.display(), to.display())));
}

/// Build a bootable ISO (BIOS + UEFI) at `build/kernel.iso`.
fn iso() {
    let kernel = build();
    let limine = fetch_limine();

    // Lay out the ISO root.
    let root = PathBuf::from("build/iso_root");
    let _ = std::fs::remove_dir_all(&root);
    copy(&kernel, root.join("boot/kernel"));
    copy("boot/limine.conf", root.join("boot/limine/limine.conf"));
    for f in [
        "limine-bios.sys",
        "limine-bios-cd.bin",
        "limine-uefi-cd.bin",
    ] {
        copy(limine.join(f), root.join("boot/limine").join(f));
    }
    for f in ["BOOTX64.EFI", "BOOTIA32.EFI"] {
        copy(limine.join(f), root.join("EFI/BOOT").join(f));
    }

    // Build the hybrid El Torito (BIOS) + EFI ISO.
    let out = "build/kernel.iso";
    run(
        "xorriso",
        &[
            "-as",
            "mkisofs",
            "-b",
            "boot/limine/limine-bios-cd.bin",
            "-no-emul-boot",
            "-boot-load-size",
            "4",
            "-boot-info-table",
            "--efi-boot",
            "boot/limine/limine-uefi-cd.bin",
            "-efi-boot-part",
            "--efi-boot-image",
            "--protective-msdos-label",
            root.to_str().unwrap(),
            "-o",
            out,
        ],
    );

    // Install the BIOS boot stage into the ISO.
    let limine_tool = limine.join("limine");
    run(limine_tool.to_str().unwrap(), &["bios-install", out]);

    println!("ISO built: {out}");
}
