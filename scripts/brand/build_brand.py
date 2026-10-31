"""Generate Magnetite brand SVGs with all text converted to outlines.

Usage: build_brand.py <fonts_dir> <out_dir>
"""
import sys
from pathlib import Path

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

FONTS, OUT = Path(sys.argv[1]), Path(sys.argv[2])
OUT.mkdir(parents=True, exist_ok=True)

archivo_vf = TTFont(FONTS / "Archivo[wdth,wght].ttf")
ARCHIVO_X = instancer.instantiateVariableFont(TTFont(FONTS / "Archivo[wdth,wght].ttf"), {"wdth": 125, "wght": 800})
ARCHIVO_B = instancer.instantiateVariableFont(archivo_vf, {"wdth": 125, "wght": 700})
MONO = TTFont(FONTS / "IBMPlexMono-Regular.ttf")
MONO_M = TTFont(FONTS / "IBMPlexMono-Medium.ttf")


def run(font, text, size, tracking=0.0):
    """Outline `text` at origin (baseline y=0). Returns (path d, advance)."""
    gs = font.getGlyphSet()
    cmap = font.getBestCmap()
    upm = font["head"].unitsPerEm
    s = size / upm
    pen = SVGPathPen(gs)
    x = 0.0
    for ch in text:
        name = cmap[ord(ch)]
        tp = TransformPen(pen, (s, 0, 0, -s, x, 0))
        gs[name].draw(tp)
        x += gs[name].width * s + tracking * size
    return pen.getCommands(), x - tracking * size


def place(d, x, y):
    return f'<path transform="translate({x:.2f} {y:.2f})" d="{d}"/>'


def octahedron(x, y, w, edge):
    """Magnetite's crystal habit; one facet in temper colour."""
    k = w / 100
    p = lambda pts: " ".join(f"{x + a * k:.2f},{y + b * k:.2f}" for a, b in pts)
    T, B, L, R, F = (50, 4), (50, 96), (12, 50), (88, 50), (56, 54)
    sw = max(1.0, 1.4 * k)
    return (
        f'<g stroke="{edge}" stroke-width="{sw:.2f}" stroke-linejoin="round">'
        f'<polygon points="{p([T, L, F])}" fill="#9aa5b8"/>'
        f'<polygon points="{p([T, F, R])}" fill="url(#temper)"/>'
        f'<polygon points="{p([L, B, F])}" fill="#3a4250"/>'
        f'<polygon points="{p([F, B, R])}" fill="#22272f"/>'
        "</g>"
    )


TEMPER = (
    '<linearGradient id="temper" x1="0" y1="0" x2="1" y2="1">'
    '<stop offset="0" stop-color="#6b4fa3"/><stop offset=".55" stop-color="#2f55c8"/>'
    '<stop offset="1" stop-color="#7fa6e8"/></linearGradient>'
)

THEMES = {
    "light": {"fg": "#1f2328", "muted": "#59636e"},
    "dark": {"fg": "#f0f6fc", "muted": "#9198a1"},
}


def tile(ox, oy, size, fg, muted):
    """The element tile, `size` px square, top-left at (ox, oy)."""
    k = size / 240
    parts = [
        f'<rect x="{ox + 8 * k:.2f}" y="{oy + 8 * k:.2f}" width="{224 * k:.2f}" height="{224 * k:.2f}" '
        f'rx="{10 * k:.2f}" fill="none" stroke="{fg}" stroke-width="{5 * k:.2f}"/>'
    ]
    d, _ = run(MONO, "231.53", 20 * k)
    parts.append(f'<g fill="{muted}">{place(d, ox + 28 * k, oy + 48 * k)}</g>')
    d, _ = run(MONO, "magnetite", 20 * k)
    parts.append(f'<g fill="{muted}">{place(d, ox + 28 * k, oy + 208 * k)}</g>')
    parts.append(octahedron(ox + 170 * k, oy + 22 * k, 44 * k, fg))

    # Fe3O4, scaled so the formula spans the tile's inner width.
    big, sub = 1.0, 0.52
    segs = [("Fe", big, 0), ("3", sub, 0.16), ("O", big, 0), ("4", sub, 0.16)]
    gap = 0.02
    width = sum(run(ARCHIVO_X, t, s)[1] for t, s, _ in segs) + gap * (len(segs) - 1)
    em = (184 * k) / width
    x, base = ox + 28 * k, oy + 156 * k
    g = []
    for t, s, dy in segs:
        d, adv = run(ARCHIVO_X, t, s * em)
        g.append(place(d, x, base + dy * em))
        x += adv + gap * em
    parts.append(f'<g fill="{fg}">{"".join(g)}</g>')
    return "".join(parts)


def svg(w, h, body, title):
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" '
        f'role="img" aria-label="{title}"><title>{title}</title><defs>{TEMPER}</defs>{body}</svg>\n'
    )


for theme, c in THEMES.items():
    # The tile on its own.
    (OUT / f"logo-{theme}.svg").write_text(svg(240, 240, tile(0, 0, 240, c["fg"], c["muted"]), "Magnetite"))

    # README banner: large tile centred, the name as a subtitle beneath.
    W, H, T = 640, 330, 220
    d, adv = run(ARCHIVO_B, "magnetite", 46, tracking=0.02)
    body = tile((W - T) / 2, 8, T, c["fg"], c["muted"])
    body += f'<g fill="{c["fg"]}">{place(d, (W - adv) / 2, 8 + T + 76)}</g>'
    (OUT / f"banner-{theme}.svg").write_text(svg(W, H, body, "Magnetite"))

# Small mark: just "Fe" in the frame, for favicons and avatars.
d, adv = run(ARCHIVO_X, "Fe", 34)
small = (
    '<rect x="3" y="3" width="58" height="58" rx="5" fill="#0f1218" stroke="#c9d1dc" stroke-width="2.5"/>'
    f'<g fill="#f0f6fc">{place(d, (64 - adv) / 2, 44)}</g>'
)
(OUT / "icon.svg").write_text(svg(64, 64, small, "Magnetite"))

# Social preview, 1280x640, always dark.
W, H = 1280, 640
bg, fg, muted = "#0f1218", "#f0f6fc", "#9198a1"
body = f'<rect width="{W}" height="{H}" fill="{bg}"/>'
body += tile(100, 140, 360, fg, muted)
d, _ = run(ARCHIVO_B, "magnetite", 100, tracking=0.01)
body += f'<g fill="{fg}">{place(d, 540, 300)}</g>'
for i, line in enumerate(["Capability-based x86-64 microkernel", "in Rust, hardened by construction."]):
    d, _ = run(MONO, line, 30)
    body += f'<g fill="{muted}">{place(d, 546, 372 + i * 44)}</g>'
d, _ = run(MONO_M, "github.com/seedon198/magnetite", 24)
body += f'<g fill="#7fa6e8">{place(d, 546, 500)}</g>'
(OUT / "social-preview.svg").write_text(svg(W, H, body, "Magnetite"))

print("\n".join(sorted(p.name for p in OUT.iterdir())))
