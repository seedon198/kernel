# Development Environment

You do **not** need an x86 machine to work on this kernel. Rust cross-compiles
to bare-metal x86-64 from any host, and QEMU runs the result. Real hardware is
only needed for occasional validation and for USB-boot testing.

## Contents

- [How it works](#how-it-works)
- [macOS (Apple Silicon and Intel)](#macos-apple-silicon-and-intel)
- [Linux](#linux)
- [Windows (WSL2)](#windows-wsl2)
- [Debugging](#debugging)
- [Testing on real hardware](#testing-on-real-hardware)
- [Troubleshooting](#troubleshooting)

## How it works

```
 Rust crates ──cargo build --target x86_64-unknown-none──▶ kernel ELF (higher half)
                                                            │
                               xtask + Limine + xorriso ────▼
                                                       kernel.iso
                                                            │
                                 qemu-system-x86_64 ◀───────┘
                                       │
                                       ├── serial output → your terminal
                                       └── GDB stub :1234 → gdb / lldb
```

- **Rust is a cross-compiler by default.** `rustup target add x86_64-unknown-none`
  installs a bare-metal (`no_std`, no OS) target. One `cargo build
  --target x86_64-unknown-none` on an Apple Silicon Mac produces an x86-64 ELF.
  Verified: `rustc` 1.94 builds and links a `no_std` x86-64 kernel on arm64 macOS.
- **No C cross-toolchain is required** for the kernel itself. If we later need a
  custom target spec (e.g. to disable the red zone or SSE), that uses
  `-Zbuild-std` on a **nightly** toolchain, pinned in `rust-toolchain.toml`.
- **QEMU** emulates a full x86-64 PC. On Apple Silicon it uses software
  emulation (TCG) — slower than native but deterministic and easy to debug.

## macOS (Apple Silicon and Intel)

```sh
# Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add x86_64-unknown-none
rustup component add rust-src llvm-tools clippy rustfmt

# Emulator + ISO tooling + a debugger
brew install qemu xorriso x86_64-elf-gdb
```

| Tool | Purpose |
|---|---|
| `rustup` / `cargo` / `rustc` | the Rust toolchain and cross-compilation |
| `rust-src` | required for `-Zbuild-std` if we use a custom target |
| `llvm-tools` | `llvm-objdump`, `llvm-nm`, `llvm-size` via `cargo` |
| `qemu` | `qemu-system-x86_64` |
| `xorriso` | builds the bootable ISO |
| `x86_64-elf-gdb` | source-level kernel debugging (`lldb` also works) |

The exact toolchain (stable vs. a pinned nightly) is set by `rust-toolchain.toml`
in the repo, so `cargo` selects it automatically.

## Linux

Debian / Ubuntu:

```sh
sudo apt install qemu-system-x86 xorriso gdb
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add x86_64-unknown-none
rustup component add rust-src llvm-tools clippy rustfmt
```

On an x86-64 Linux host QEMU can use KVM for near-native speed
(`cargo xtask run --kvm`, once available).

## Windows (WSL2)

Install Ubuntu under WSL2 and follow the Linux steps. QEMU's display works via
WSLg; serial output works everywhere.

## Debugging

QEMU exposes a GDB remote stub (wired up in M0):

```sh
cargo xtask debug          # boots QEMU paused, listening on :1234
# another terminal:
x86_64-elf-gdb -ex "target remote :1234" target/.../kernel
# or
lldb -o "gdb-remote 1234" target/.../kernel
```

Useful QEMU options (passthrough via `xtask`): `-serial stdio`,
`-d int,cpu_reset -no-reboot` (trace interrupts, stop on triple fault),
`-monitor stdio`, `-M q35`.

## Testing on real hardware

Write the ISO to a USB stick and boot a generic x86-64 PC from it. Watch the
serial port (USB-serial adapter) or the framebuffer.

```sh
# macOS — verify the disk number with `diskutil list` first!
sudo dd if=build/kernel.iso of=/dev/rdiskN bs=4m
```

> [!CAUTION]
> `dd` to the wrong disk destroys data. Double-check the device name.

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| `error[E0463]: can't find crate for core` | missing `rustup target add x86_64-unknown-none` |
| `-Zbuild-std` errors | not on the pinned nightly, or `rust-src` not installed |
| QEMU reboots in a loop | triple fault — rerun with `-d int,cpu_reset -no-reboot` |
| Black screen, no serial | check `limine.conf` path and that the kernel is on the ISO |
| `unsafe` won't compile in `kernel` | intended — `unsafe` belongs in `hal` (see ADR 0007) |
