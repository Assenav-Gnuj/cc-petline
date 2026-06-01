# clawd-pet frame assets

The animation engine (`src/anim.rs::build_library`) loads frames from here when
present, and falls back to the synthetic `src/sprite.rs` generators otherwise.
The shipped **Fox** pack lives here (one folder per mood); the media is
**gitignored**, so a fresh checkout runs on the synthetic fallback until you add
frames.

## Layout

```
assets/frames/
  idle/      0000.png 0001.png ...   (looping)
  working/   0000.png 0001.png ...   (looping)
  happy/     0000.png 0001.png ...   (one-shot, then → idle)
  sleepy/    0000.png 0001.png ...   (looping)
  ...                                (one dir per PetState::dir_name)
```

- Frames play in **lexical filename order** — zero-pad (`0000.png`) so they sort right.
- Any `*.png` (case-insensitive) is picked up; other files are ignored.
- An empty state dir → that state uses the synthetic fallback, so you can fill in
  one state at a time.
- Per-state timing/loop behaviour lives in `anim.rs`, not here.

## Making a pack

Slice horizontal sprite strips (`<state>_strip.png`) into per-frame PNGs:

```sh
cargo run --example slice -- <strip-dir>                 # → assets/frames/<state>/
cargo run --example slice -- --theme <name> <strip-dir>  # → assets/themes/<name>/frames/
```

Keep frames small (≈64–120px square) — halfblocks render 2 vertical px per cell,
so large frames cost CPU without visible gain in a small pane.
