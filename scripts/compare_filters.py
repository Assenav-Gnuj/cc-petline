#!/usr/bin/env python
"""Show Lanczos (old) vs Nearest (new) downsampling of the Clawd frames.

Mirrors render_ansi: source 120x120 -> resize_exact to the statusline pixel grid.
For a 5-row pet that grid is 10 wide x 10 tall (cols = rows*2 ... wait: cols=rows*2,
height=rows*2 -> 10x10 at rows=5). We then upscale 24x with NEAREST purely so the
tiny result is inspectable on screen. The terminal shows the LEFT-of-upscale pixels.
"""
import os
from PIL import Image

ROOT = os.path.join(os.path.dirname(__file__), "..", "assets", "themes", "claude", "frames")
MOODS = ["idle", "happy", "sleepy", "surprised", "angry", "error"]
ROWS = 5
GW, GH = ROWS * 2, ROWS * 2  # 10 x 10 px target (matches render_ansi at 5 rows)
ZOOM = 26                    # inspection upscale only
PAD = 8
BG = (26, 27, 38)

def grid(img120, flt):
    small = img120.resize((GW, GH), flt)
    return small.resize((GW * ZOOM, GH * ZOOM), Image.NEAREST)

cell_w, cell_h = GW * ZOOM, GH * ZOOM
cols = len(MOODS)
W = PAD + cols * (cell_w + PAD)
H = PAD + 2 * (cell_h + PAD) + 24
canvas = Image.new("RGB", (W, H), BG)

for i, mood in enumerate(MOODS):
    src = Image.open(os.path.join(ROOT, mood, "0000.png")).convert("RGBA")
    bg = Image.new("RGBA", src.size, BG + (255,))
    src = Image.alpha_composite(bg, src).convert("RGB")
    x = PAD + i * (cell_w + PAD)
    canvas.paste(grid(src, Image.LANCZOS), (x, PAD))               # top row: OLD
    canvas.paste(grid(src, Image.NEAREST), (x, PAD + cell_h + PAD))  # bottom: NEW

out = os.path.join(os.path.dirname(__file__), "clawd_filter_compare.png")
canvas.save(out)
print("wrote", out, "| top=LANCZOS(old)  bottom=NEAREST(new) | moods:", " ".join(MOODS))
