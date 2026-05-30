# clawd-pet frame assets

The animation engine (`src/anim.rs::build_library`) loads frames from here when
present, and falls back to the synthetic `src/sprite.rs` generators otherwise.

## Layout

```
assets/frames/
  idle/    0001.png 0002.png ...   (looping)
  active/  0001.png 0002.png ...   (looping)
  happy/   0001.png 0002.png ...   (one-shot, then → idle)
  sleep/   0001.png 0002.png ...   (looping)
```

- Frames are played in **lexical filename order** — zero-pad (`0001.png`) so they sort right.
- Any `*.png` (case-insensitive) is picked up; other files are ignored.
- An empty state dir → that state uses the synthetic fallback. So you can convert
  one state at a time.
- Per-state timing/loop behaviour lives in `anim.rs::timing`, not here.

## Monamoji pipeline (Phase 2 / when real art is chosen)

Monamoji ships animated Octocat GIFs. To turn one GIF into a state's frames:

```bash
# 64x64 RGBA PNG frames, transparent background preserved:
ffmpeg -i octocat-dance.gif \
  -vf "scale=64:64:flags=lanczos" \
  assets/frames/happy/%04d.png
```

Notes:
- Keep the sprite small (≈64×64) — halfblocks renders 2 vertical px per cell, so
  large frames just cost CPU without visible gain in a small pane.
- If a GIF has a matte/solid background, add a colorkey/transparency filter, e.g.
  `-vf "scale=64:64,colorkey=0xFFFFFF:0.1:0.0"`.
- Trim frame count to what reads well at the state's `frame_ms`; more frames ≠
  smoother if the tick rate is the bottleneck.
```
