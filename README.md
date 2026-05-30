# clawd-pet 🐱

An animated **Morgana-cat companion** for [Claude Code](https://claude.com/claude-code) —
a sprite mascot that reacts to what Claude is doing, plus a Charm-styled statusline
wrapper that feeds it live context%/cost.

Built in **Rust** (ratatui + the `image` crate), shipped as a Claude Code **plugin**.

## Two ways to run

### 1. Statusline column (static-per-refresh, ~3fps)
`clawd-pet statusline` wraps [ccstatusline](https://www.npmjs.com/package/ccstatusline):
it renders the Charm statusline **and** appends the cat as an ANSI half-block column to
its right — with a **speech bubble** (rotating programming quotes) and the current mood.

- The cat **frame-cycles** the real sprite frames by wall-clock, so it animates while you
  work and gently steps when idle (statusline can't loop — this is the ~3fps ceiling).
- The speech bubble flows a **lolcat-style rainbow** gradient.
- Wire it with the plugin's `/charm-setup` command (patches `settings.json`, reversible).

### 2. Animated pane (smooth, 25fps)
`clawd-pet watch` is a full ratatui TUI — the cat animates smoothly with an animated
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

## Build

```sh
cargo build --release          # binary at target/release/clawd-pet[.exe]
cargo run --example slice      # slice sprite strips → assets/frames/<state>/*.png
```

The plugin bundles a prebuilt binary via `plugin/build.ps1`.

## Config (env vars)

| Var | Default | Effect |
|-----|---------|--------|
| `CLAWD_PET_ROWS` | 6 | cat height in terminal rows (width = rows×2) |
| `CLAWD_PET_GAP` | 2 | spacing between statusline text and the cat |
| `CLAWD_PET_WIDTH` | auto | pin the column inside this total width (set to terminal cols) |
| `CLAWD_PET_BUBBLE` | 40 | speech-bubble inner text width (0 = no bubble) |
| `CLAWD_PET_FPS_MS` | 120 | statusline frame period (sampled per refresh) |
| `CLAWD_PET_ASSETS` | auto | path to the `assets/` dir if not auto-resolved |
| `CLAWD_PET_GUARD` | unset | `1` enables the PreToolUse command guard |

## Notes

- **Renderer:** transparent Unicode half-blocks (sixel does not render in Tabby). The cat
  sits on the terminal background — no black box.
- **Assets:** sprite media (`assets/source/*`, `assets/frames/**`) is **gitignored** —
  regenerate frames from your own strips via `cargo run --example slice`.
