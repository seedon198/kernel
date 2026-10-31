//! Stamp build identity into the kernel for the boot banner.
//!
//! Sets `MAGNETITE_VERSION` (from `git describe`, falling back to the crate
//! version outside a git checkout), `MAGNETITE_RUSTC` and `MAGNETITE_PROFILE`.
//! No timestamps: the same commit and toolchain produce the same banner.

use std::process::Command;

fn output(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_owned())
}

fn main() {
    let version = output("git", &["describe", "--tags", "--always", "--dirty"])
        .unwrap_or_else(|| format!("v{}", env!("CARGO_PKG_VERSION")));
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let rustc = output(&rustc, &["--version"]).unwrap_or_else(|| "rustc (unknown)".into());
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".into());

    println!("cargo:rustc-env=MAGNETITE_VERSION={version}");
    println!("cargo:rustc-env=MAGNETITE_RUSTC={rustc}");
    println!("cargo:rustc-env=MAGNETITE_PROFILE={profile}");

    // Re-run when HEAD moves or the index changes (new commit, checkout,
    // staged edits that flip `-dirty`).
    for path in ["../.git/HEAD", "../.git/index", "../.git/refs/tags"] {
        println!("cargo:rerun-if-changed={path}");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
