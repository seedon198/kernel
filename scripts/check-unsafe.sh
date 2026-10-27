#!/usr/bin/env bash
# Enforce the unsafe-code policy (ADR 0007): `unsafe` lives only in the `hal`
# crate, with exactly one audited exception — the `#[allow(unsafe_code)]` on the
# kernel's `_start` entry shim (the ELF entry needs `#[unsafe(no_mangle)]`,
# which `forbid` cannot permit).
#
# The compiler already enforces most of this: non-`hal` crates declare
# `#![forbid(unsafe_code)]` or `#![deny(unsafe_code)]`, so real `unsafe` code
# will not compile there. This script is defense in depth against someone
# sneaking in an extra `#[allow(unsafe_code)]` or weakening a crate's lint.
#
# Exit non-zero on any violation. Portable bash (macOS and Linux).
set -euo pipefail

cd "$(dirname "$0")/.."

# The single sanctioned allowance outside `hal`.
readonly ALLOWED_FILE="kernel/src/main.rs"

fail=0

# Crates permitted to contain `unsafe` at all.
is_hal() { [[ "$1" == hal/* ]]; }

# 1) Find every `allow(unsafe_code)` ATTRIBUTE (inner or outer), ignoring
#    mentions inside comments/docs by requiring the line to start (after
#    optional whitespace) with the attribute form `#[...]` / `#![...]`.
mapfile -t allow_hits < <(
  grep -REn --include='*.rs' '^[[:space:]]*#!?\[[[:space:]]*allow\([[:space:]]*unsafe_code[[:space:]]*\)' \
    kernel libs xtask servers runtime tools 2>/dev/null || true
)

for hit in "${allow_hits[@]:-}"; do
  [[ -z "$hit" ]] && continue
  file="${hit%%:*}"
  if is_hal "$file"; then continue; fi
  if [[ "$file" != "$ALLOWED_FILE" ]]; then
    echo "DENIED: allow(unsafe_code) outside hal and not the sanctioned entry shim:"
    echo "  $hit"
    fail=1
  fi
done

# There must be exactly one such allowance, and it must be the entry shim.
count_allowed=0
for hit in "${allow_hits[@]:-}"; do
  [[ -z "$hit" ]] && continue
  [[ "${hit%%:*}" == "$ALLOWED_FILE" ]] && count_allowed=$((count_allowed + 1))
done
if [[ "$count_allowed" -ne 1 ]]; then
  echo "DENIED: expected exactly 1 allow(unsafe_code) in $ALLOWED_FILE, found $count_allowed"
  fail=1
fi

# 2) Every non-`hal` crate root must declare forbid/deny(unsafe_code), so the
#    compiler keeps enforcing the rule even in new crates.
while IFS= read -r root; do
  is_hal "$root" && continue
  if ! grep -Eq '^[[:space:]]*#!\[[[:space:]]*(forbid|deny)\([[:space:]]*unsafe_code[[:space:]]*\)' "$root"; then
    echo "DENIED: crate root does not #![forbid]/#![deny](unsafe_code): $root"
    fail=1
  fi
done < <(find kernel libs xtask servers runtime tools -name lib.rs -o -name main.rs 2>/dev/null)

if [[ "$fail" -ne 0 ]]; then
  echo
  echo "unsafe-code policy violation(s) found. See docs/adr/0007-unsafe-policy.md"
  exit 1
fi

echo "unsafe-code policy OK: unsafe confined to hal/, one audited allowance in $ALLOWED_FILE"
