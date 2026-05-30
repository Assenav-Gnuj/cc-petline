# clawd-pet source GIFs (drop zone)

Put your Monamoji (or any) GIFs here, named after the pet state they map to, then
run the converter. Each GIF becomes a PNG frame sequence the engine auto-loads.

## 1. Name the GIFs by state

```
assets/source/
  idle.gif      → assets/frames/idle/
  active.gif    → assets/frames/active/
  happy.gif     → assets/frames/happy/
  sleep.gif     → assets/frames/sleep/
```

- Filename stem (before `.gif`) must be one of: `idle`, `active`, `happy`, `sleep`.
- Any other name is skipped (with a warning) so you can stage extras safely.
- A `.gif`, `.webp`, or `.apng` works (ffmpeg decodes all three).
- You only need the states you want to override; the rest stay synthetic.

## 2. Convert

From the crate root (`C:\Users\Oreo\charm\clawd-pet`):

```powershell
powershell ./scripts/convert-gifs.ps1            # all GIFs in assets/source/
powershell ./scripts/convert-gifs.ps1 happy      # just one state
powershell ./scripts/convert-gifs.ps1 -Size 80   # override the 64px default
```

The script clears the target `assets/frames/<state>/` first, then writes
`0001.png, 0002.png, …` (zero-padded so they sort = play in order).

## 3. Run

```powershell
cargo run        # quit a running pet (q) first to release the .exe lock
```

The engine loads on-disk frames when present, else the synthetic critter.

## Trademark note

The GitHub Octocat / Monamoji are GitHub trademarks — fine for a personal, local
pet, but **don't commit or redistribute the sprite frames**. `assets/source/` and
`assets/frames/*/` PNGs are gitignored for this reason; only the synthetic
fallback and the recipe ship.
