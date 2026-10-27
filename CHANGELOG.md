# Changelog

All notable changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/) (pre-1.0: anything may change).

## [Unreleased]

### Added

- Project foundation for a security-focused Rust microkernel: README,
  contributing guide, code of conduct, security policy, changelog.
- Development environment guide (Rust bare-metal on macOS/Linux/WSL2).
- Architecture docs: overview (microkernel), security model (capabilities),
  threat model, boot process, memory layout.
- ADRs 0001–0008: Rust, Rust+LLVM toolchain, Limine, microkernel, Cargo+xtask,
  capability security model, `forbid(unsafe)` policy, native tooling runtime.
- Roadmap with milestones M0–M7 and a continuous hardening checklist.
- GitHub issue forms (bug, feature, security/tooling) and PR template.
- Repo config: `rust-toolchain.toml`, `rustfmt.toml`, `.editorconfig`,
  `.gitignore`, `.gitattributes`.
