"""Render a QEMU serial log as a terminal-window SVG for the README.

Shows the boot banner and system summary in full, then a curated set of real
log lines (those matching KEEP); each run of omitted lines becomes a dim ⋮.

Usage: term_svg.py <serial.log> <out.svg>
"""
import re
import sys
from xml.sax.saxutils import escape

KEEP = [
    "hello from the kernel",
    "harden: verified",
    "guard: unmapped",
    "pf: probe verified",
    "caps: CNode",
    "invoke:",
    "sched: both threads ran",
    "syscall: ring-3 call",
    "lapic: timer calibrated",
    "preempt: both spinners finished",
    "ready:",
    "kernel: idling",
]

log, out = sys.argv[1], sys.argv[2]
ansi = re.compile(r"\x1b\[[0-9;=?]*[A-Za-z]")
lines = [ansi.sub("", l).rstrip("\n").rstrip() for l in open(log, encoding="utf-8", errors="replace")]
start = next(i for i, l in enumerate(lines) if l.strip().startswith(".---"))
hello = next(i for i, l in enumerate(lines) if "hello from the kernel" in l)

BG, FG, DIM, ACCENT, BRIGHT = "#0f1218", "#c9d1dc", "#6b7484", "#7fa6e8", "#f0f6fc"
TAG = {"[INFO]": "#7fb88a", "[WARN]": "#e0b062", "[ERROR]": "#e5737a"}
TILE_W = 30  # two-space indent + the 28-column tile

rows = []  # each row: list of (text, colour)
for l in lines[start:hello]:
    if l.startswith("  ") and l.strip()[:1] in ".|'":
        right = l[TILE_W:]
        colour = BRIGHT if right.strip()[:1] in "_|" else FG
        rows.append([(l[:TILE_W], ACCENT), (right, colour)])
    elif re.match(r"  \w+ {2,}", l):
        key, value = l[:13], l[13:]
        rows.append([(key, DIM), (value, FG)])
    else:
        rows.append([(l, FG)])

skipped = False
for l in lines[hello:]:
    if any(k in l for k in KEEP):
        if skipped:
            rows.append([("       ⋮", DIM)])
            skipped = False
        m = re.match(r"(\[\w+\]) (.*)", l)
        tag, msg = (m.group(1), m.group(2)) if m else ("", l)
        rows.append([(f"{tag:<7} ", TAG.get(tag, DIM)), (msg, FG)])
        if "kernel: idling" in l:
            break
    elif l.strip():
        skipped = True

COLS = max(sum(len(t) for t, _ in r) for r in rows)
CW, LH, PADX, TOP = 7.8, 19, 22, 58
W = int(PADX * 2 + CW * COLS)
H = TOP + LH * (len(rows) + 1) + 16
FONT = "ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', monospace"

body = [f'<text x="{PADX}" y="{TOP + 14}" fill="{DIM}">$ cargo xtask run</text>']
y = TOP + 14
for r in rows:
    y += LH
    spans = "".join(f'<tspan fill="{c}">{escape(t)}</tspan>' for t, c in r)
    body.append(f'<text x="{PADX}" y="{y}" xml:space="preserve">{spans}</text>')

svg = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" role="img" aria-label="Magnetite booting in QEMU: boot banner, system summary and serial log">
<title>Magnetite booting in QEMU (serial console)</title>
<rect width="{W}" height="{H}" rx="10" fill="{BG}"/>
<rect width="{W}" height="36" rx="10" fill="#1a1f28"/><rect y="26" width="{W}" height="10" fill="#1a1f28"/>
<circle cx="22" cy="18" r="6" fill="#e5737a"/><circle cx="42" cy="18" r="6" fill="#e0b062"/><circle cx="62" cy="18" r="6" fill="#7fb88a"/>
<text x="{W / 2}" y="22" text-anchor="middle" font-family="{FONT}" font-size="12" fill="{DIM}">qemu-system-x86_64 · q35 · UEFI (OVMF) · serial</text>
<g font-family="{FONT}" font-size="13" fill="{FG}">
{chr(10).join(body)}
</g>
</svg>
"""
open(out, "w").write(svg)
print(f"{len(rows)} rows, {W}x{H}")
