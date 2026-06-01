# clawd-pet 🦊

An animated **Fox companion** for [Claude Code](https://claude.com/claude-code) —
a sprite mascot that reacts to what Claude is doing, plus a Charm-styled statusline
wrapper that feeds it live context%/cost.

Built in **Rust** (ratatui + the `image` crate), shipped as a Claude Code **plugin**.

## Two ways to run

### 1. Statusline column (static-per-refresh, ~3fps)
`clawd-pet statusline` wraps [ccstatusline](https://www.npmjs.com/package/ccstatusline):
it renders the Charm statusline **and** appends the fox as an ANSI half-block column to
its right — with a **speech bubble** (rotating programming quotes) and the current mood.

- The fox **frame-cycles** the real sprite frames by wall-clock, so it animates while you
  work and gently steps when idle (statusline can't loop — this is the ~3fps ceiling).
- The speech bubble flows a **lolcat-style rainbow** gradient.
- Wire it with the plugin's `/charm-setup` command (patches `settings.json`, reversible).

### 2. Animated pane (smooth, 25fps)
`clawd-pet watch` is a full ratatui TUI — the fox animates smoothly with an animated
rainbow speech bubble. Run it in a dedicated terminal pane (e.g. a Tabby split).

## How moods work

The plugin's hooks call `clawd-pet emit <Event>`, which maps a Claude Code hook event to a
mood and writes `~/.clawd-pet/state`. Both the statusline and the pane read that file.

| Hook | Mood |
|------|------|
| SessionStart / Stop | happy |
| UserPromptSubmit | thinking |
| PreToolUse | working |
| PostToolUse / SubagentStop | active (→ **error** if the tool failed) |
| Notification | surprised |
| PreCompact / SessionEnd | sleepy |

There's also an **opt-in** token-saver `guard` hook (PreToolUse): off by default, set
`CLAWD_PET_GUARD=1` to block a small denylist of catastrophic/wasteful Bash commands via
exit-2 (so Claude self-corrects).

## Themes (mascots)

The mascot is themeable via `CLAWD_PET_THEME`. **Fox** is the default and the headline
mascot — its sprite frames ship under `assets/frames/`.

| Theme | What you get |
|-------|--------------|
| `fox` *(default)* | the **Fox** — the shipped sprite pack under `assets/frames/` |
| `ghost` | a built-in **synthetic** spectral ghost — needs no art files |
| *custom* | your own sprite pack under `assets/themes/<name>/frames/` |

```sh
CLAWD_PET_THEME=ghost   # instant, no art needed (synthetic character)
```

Add your **own** mascot by slicing horizontal strips into a theme pack, then
selecting it:

```sh
cargo run --example slice -- --theme robot C:/path/to/robot/strips
CLAWD_PET_THEME=robot
```

A theme with no on-disk pack and no built-in character falls back to a synthetic
character, so it never breaks. Built-in synthetic characters live in `src/`
(the fallback blob in `sprite.rs`, the ghost in `ghost.rs`); the selector is
`src/theme.rs`.

## Build

```sh
cargo build --release          # binary at target/release/clawd-pet[.exe]
cargo run --example slice      # slice sprite strips → assets/frames/<state>/*.png
```

The plugin bundles a prebuilt binary via `plugin/build.ps1`.

## Config (env vars)

| Var | Default | Effect |
|-----|---------|--------|
| `CLAWD_PET_ROWS` | 6 | fox height in terminal rows (width = rows×2) |
| `CLAWD_PET_GAP` | 2 | spacing between statusline text and the fox |
| `CLAWD_PET_WIDTH` | auto | pin the column inside this total width (set to terminal cols) |
| `CLAWD_PET_BUBBLE` | 40 | speech-bubble inner text width (0 = no bubble) |
| `CLAWD_PET_FPS_MS` | 120 | statusline frame period (sampled per refresh) |
| `CLAWD_PET_ASSETS` | auto | path to the `assets/` dir if not auto-resolved |
| `CLAWD_PET_THEME` | fox | mascot theme: `fox`, `ghost`, or a custom pack name |
| `CLAWD_PET_GUARD` | unset | `1` enables the PreToolUse command guard |

## Notes

- **Renderer:** transparent Unicode half-blocks (sixel does not render in Tabby). The fox
  sits on the terminal background — no black box.
- **Assets:** the **Fox** frame pack ships in the repo (`assets/frames/**`). Custom theme
  packs under `assets/themes/<name>/frames/` are gitignored — bring your own via
  `cargo run --example slice`.

## License

- **Code:** [MIT](LICENSE).
- **Sprite art** (the Fox frames under `assets/frames/`): [CC BY 4.0](assets/LICENSE) —
  © 2026 Oreo. Reuse freely with attribution.
