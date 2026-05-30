#!/usr/bin/env python
"""Generate a 'Clawd' (Claude) mascot sprite pack for clawd-pet.

IMPORTANT: this is tuned for a TINY render. The half-block statusline collapses
the sprite to ~10x10 px (a 5-row statusline) — so the art is authored on a 10-cell
grid: fill the whole frame, big silhouette-defining ears, two chunky eye-pixels,
and NOTHING sub-pixel (no spark/blush/mouth — they just turn to mud at 10px).
Saved at 120x120 (an integer 12x of the 10-cell grid) so it downsamples cleanly
to the statusline AND still has enough to upscale in the watch pane.

One frame per mood; the only thing that varies at this size is the eyes.
Output: assets/themes/claude/frames/<state>/0000.png
"""
import os
from PIL import Image, ImageDraw

CELL = 12                    # supersampled px per 10-grid cell
G = 10                       # 10x10 design grid
S = G * CELL                 # 120px working/output canvas

# Claude terracotta palette
BODY = (217, 119, 87, 255)   # #D97757 terracotta
EAR_IN = (193, 95, 60, 255)  # inner-ear / shade
FACE = (245, 226, 214, 255)  # cream face patch
INK = (43, 33, 24, 255)      # near-black brown (eyes)
ERR = (200, 70, 55, 255)     # error tint

STATES = ["idle", "working", "active", "thinking", "happy", "surprised",
          "sleepy", "oops", "error", "angry", "scared"]

# Eye treatment per mood (what actually survives at 10px):
#   open   - two filled ovals (default)
#   wide   - bigger ovals (surprise/scare/oops)
#   happy  - upward arcs ^^
#   closed - horizontal lines (sleepy)
EYES = {
    "idle": "open", "working": "open", "active": "wide", "thinking": "open",
    "happy": "happy", "surprised": "wide", "sleepy": "closed", "oops": "wide",
    "error": "open", "angry": "open", "scared": "wide",
}


def u(c):
    return c * CELL


def draw(state):
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    body = ERR if state == "error" else BODY

    # ears — big triangles in the top corners (carry the silhouette at 10px)
    d.polygon([(u(0.6), 0), (u(3.3), 0), (u(2.0), u(2.6))], fill=body)
    d.polygon([(u(6.7), 0), (u(9.4), 0), (u(8.0), u(2.6))], fill=body)
    # head/body — one rounded block filling almost the whole frame
    d.rounded_rectangle([u(0.4), u(1.1), u(9.6), u(9.7)], radius=u(2.4), fill=body)
    # cream face patch, chunky and centered
    d.rounded_rectangle([u(1.6), u(2.6), u(8.4), u(7.7)], radius=u(1.6), fill=FACE)

    mode = EYES[state]
    ex, ey = (3.5, 6.5), 4.6
    if mode in ("open", "wide"):
        rx, ry = (0.8, 1.0) if mode == "open" else (1.0, 1.15)
        for cx in ex:
            d.ellipse([u(cx - rx), u(ey - ry), u(cx + rx), u(ey + ry)], fill=INK)
    elif mode == "happy":
        for cx in ex:
            d.arc([u(cx - 1.0), u(ey - 0.6), u(cx + 1.0), u(ey + 1.2)],
                  start=200, end=340, fill=INK, width=int(CELL * 0.9))
    elif mode == "closed":
        for cx in ex:
            d.line([u(cx - 0.9), u(ey), u(cx + 0.9), u(ey)],
                   fill=INK, width=int(CELL * 0.9))

    # angry: heavy brows slanting inward (reads even at 10px as a V)
    if state == "angry":
        d.line([u(2.4), u(3.4), u(4.2), u(4.1)], fill=INK, width=int(CELL * 0.8))
        d.line([u(7.6), u(3.4), u(5.8), u(4.1)], fill=INK, width=int(CELL * 0.8))

    return img


def main():
    root = os.path.join(os.path.dirname(__file__), "..", "assets", "themes", "claude", "frames")
    for state in STATES:
        out_dir = os.path.join(root, state)
        os.makedirs(out_dir, exist_ok=True)
        draw(state).save(os.path.join(out_dir, "0000.png"))
    print(f"wrote {len(STATES)} states ({S}x{S}, 10px-tuned) to {os.path.normpath(root)}")


if __name__ == "__main__":
    main()
