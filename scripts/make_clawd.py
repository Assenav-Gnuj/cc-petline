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

SCARE = (150, 140, 170, 255)  # pale desaturated body for scared

# Per-mood expression. At 10px only a few things read, so every mood leans on a
# COMBINATION of axes rather than eye-shape alone:
#   eyes : open | wide | happy | closed | dizzy   (dizzy = X-ed out, error)
#   dy   : vertical gaze offset in grid cells (-up / +down)
#   mouth: none | o | smile | flat | frown
#   brow : "" | "angry" (slanting V)
#   body : BODY (default) | ERR (red) | SCARE (pale)
EXPR = {
    #          eyes      dy    mouth    brow      body
    "idle":     ("open",  0.0, "none",  "",       BODY),
    "working":  ("open",  0.7, "flat",  "",       BODY),   # eyes down, focused
    "active":   ("open", -0.2, "smile", "",       BODY),   # bright, engaged
    "thinking": ("open", -0.6, "none",  "",       BODY),   # eyes up, pondering
    "happy":    ("happy", 0.0, "smile", "",       BODY),
    "surprised":("wide",  0.0, "o",     "",       BODY),
    "sleepy":   ("closed",0.3, "none",  "",       BODY),
    "oops":     ("wide",  0.4, "flat",  "",       BODY),   # wide + look down
    "error":    ("dizzy", 0.0, "flat",  "",       ERR),    # red + X eyes
    "angry":    ("open",  0.1, "frown", "angry",  ERR),    # red + V brows
    "scared":   ("wide",  0.0, "o",     "",        SCARE), # pale + wide + o
}


def u(c):
    return c * CELL


def draw(state):
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    eyes, dy, mouth, brow, body = EXPR[state]

    # ears — big triangles in the top corners (carry the silhouette at 10px)
    d.polygon([(u(0.6), 0), (u(3.3), 0), (u(2.0), u(2.6))], fill=body)
    d.polygon([(u(6.7), 0), (u(9.4), 0), (u(8.0), u(2.6))], fill=body)
    # head/body — one rounded block filling almost the whole frame
    d.rounded_rectangle([u(0.4), u(1.1), u(9.6), u(9.7)], radius=u(2.4), fill=body)
    # cream face patch, chunky and centered
    d.rounded_rectangle([u(1.6), u(2.6), u(8.4), u(7.7)], radius=u(1.6), fill=FACE)

    ex, ey = (3.5, 6.5), 4.6 + dy
    lw = int(CELL * 0.9)
    if eyes in ("open", "wide"):
        # wide = bigger SOLID ovals (a highlight just turns to a checker at 10px)
        rx, ry = (0.7, 0.9) if eyes == "open" else (0.95, 1.0)
        for cx in ex:
            d.ellipse([u(cx - rx), u(ey - ry), u(cx + rx), u(ey + ry)], fill=INK)
    elif eyes == "happy":       # ^^ upward arcs
        for cx in ex:
            d.arc([u(cx - 1.0), u(ey - 0.6), u(cx + 1.0), u(ey + 1.2)],
                  start=200, end=340, fill=INK, width=lw)
    elif eyes == "closed":      # — sleepy lines
        for cx in ex:
            d.line([u(cx - 0.9), u(ey), u(cx + 0.9), u(ey)], fill=INK, width=lw)
    elif eyes == "dizzy":       # X-ed out (error)
        for cx in ex:
            d.line([u(cx - 0.7), u(ey - 0.7), u(cx + 0.7), u(ey + 0.7)], fill=INK, width=lw)
            d.line([u(cx - 0.7), u(ey + 0.7), u(cx + 0.7), u(ey - 0.7)], fill=INK, width=lw)

    # mouth — one chunky feature below the eyes
    mx, my = 5.0, ey + 1.9
    if mouth == "o":
        d.ellipse([u(mx - 0.6), u(my - 0.55), u(mx + 0.6), u(my + 0.65)], fill=INK)
    elif mouth == "smile":
        d.arc([u(mx - 1.1), u(my - 1.0), u(mx + 1.1), u(my + 0.7)],
              start=20, end=160, fill=INK, width=lw)
    elif mouth == "frown":
        d.arc([u(mx - 1.1), u(my + 0.1), u(mx + 1.1), u(my + 1.8)],
              start=200, end=340, fill=INK, width=lw)
    elif mouth == "flat":
        d.line([u(mx - 0.8), u(my), u(mx + 0.8), u(my)], fill=INK, width=lw)

    # angry brows — heavy V slanting inward, above the eyes (reads at 10px)
    if brow == "angry":
        d.line([u(2.4), u(ey - 1.4), u(4.2), u(ey - 0.7)], fill=INK, width=int(CELL * 0.8))
        d.line([u(7.6), u(ey - 1.4), u(5.8), u(ey - 0.7)], fill=INK, width=int(CELL * 0.8))

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
