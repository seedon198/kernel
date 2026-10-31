#!/usr/bin/env bash
# House style: no en-dashes (U+2013) or em-dashes (U+2014) anywhere in the
# repository. Use a plain hyphen (`-`), including for ranges like `0x20-0x2F`.
#
# The characters are built from their UTF-8 bytes so this script does not
# contain them itself. Exit non-zero on any hit. Portable bash (macOS and Linux).
set -euo pipefail

cd "$(dirname "$0")/.."

en=$(printf '\xe2\x80\x93')
em=$(printf '\xe2\x80\x94')

if git grep -n -I -e "$en" -e "$em"; then
  echo "check-dashes: found en/em dashes above; replace them with '-'" >&2
  exit 1
fi
echo "dash policy OK: no en or em dashes"
