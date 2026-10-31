"""Render a QEMU serial log as a terminal-window SVG.

Usage: term_svg.py <serial.log> <out.svg>
"""
import re
import sys
import textwrap
from xml.sax.saxutils import escape

log, out = sys.argv[1], sys.argv[2]
ansi = re.compile(r"\x1b\[[0-9;=?]*[A-Za-z]")
lines = [ansi.sub("", l).rstrip() for l in open(log, encoding="utf-8", errors="replace")]
start = next(i for i, l in enumerate(lines) if "hello from the kernel" in l)
end = next(i for i, l in enumerate(lines) if "back in the boot thread" in l)
keep = [l for l in lines[start : end + 1] if not l.startswith("[DEBUG]")]

WRAP = 104
rows = []
for l in keep:
    m = re.match(r"(\[\w+\]) (.*)", l)
    tag, msg = (m.group(1), m.group(2)) if m else ("", l)
    parts = textwrap.wrap(msg, WRAP - 7) or [""]
    rows.append((tag, parts[0]))
    rows += [("", "  " + p) for p in parts[1:]]

CW, LH, PADX, TOP = 7.8, 19, 22, 58
W = int(PADX * 2 + CW * WRAP)
H = TOP + LH * (len(rows) + 2) + 10
FONT = "ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', monospace"
color = {"[INFO]": "#7fb88a", "[WARN]": "#e0b062", "[ERROR]": "#e5737a"}

out_rows = []
y = TOP + 14
out_rows.append(f'<text x="{PADX}" y="{y}" fill="#6b7484">$ cargo xtask run</text>')
for tag, msg in rows:
    y += LH
    t = f'<tspan fill="{color.get(tag, "#6b7484")}">{escape(tag):<7}</tspan>' if tag else '<tspan>       </tspan>'
    out_rows.append(f'<text x="{PADX}" y="{y}" xml:space="preserve">{t} {escape(msg)}</text>')

svg = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" role="img" aria-label="Magnetite booting in QEMU: serial console output">
<title>Magnetite booting in QEMU (serial console)</title>
<rect width="{W}" height="{H}" rx="10" fill="#0f1218"/>
<rect width="{W}" height="36" rx="10" fill="#1a1f28"/><rect y="26" width="{W}" height="10" fill="#1a1f28"/>
<circle cx="22" cy="18" r="6" fill="#e5737a"/><circle cx="42" cy="18" r="6" fill="#e0b062"/><circle cx="62" cy="18" r="6" fill="#7fb88a"/>
<text x="{W / 2}" y="22" text-anchor="middle" font-family="{FONT}" font-size="12" fill="#6b7484">qemu-system-x86_64 · q35 · UEFI (OVMF) · serial</text>
<g font-family="{FONT}" font-size="13" fill="#c9d1dc">
{chr(10).join(out_rows)}
</g>
</svg>
"""
open(out, "w").write(svg)
print(f"{len(rows)} rows, {W}x{H}")
