# clawd-pet plugin

Packages the **hook-wiring half** of clawd-pet so Claude Code drives the pet's moods.
The pet itself (the animated pane) runs separately — plugins can't host a persistent
visual pane, only commands/agents/skills/hooks/MCP.

## What's in here

```
plugin/
├── .claude-plugin/
│   ├── plugin.json              # plugin manifest
│   └── marketplace.json         # marketplace manifest (lists this plugin, source "./")
├── hooks/hooks.json             # 7 events → `clawd-pet emit <event>`
├── commands/clawd-pet.md        # /clawd-pet — launch help + state explainer
├── commands/charm-setup.md      # /charm-setup — wire statusLine → pet wrapper
├── scripts/setup-statusline.js  # patches settings.json (run by /charm-setup)
└── bin/clawd-pet.exe            # the binary (copied by build.ps1; gitignored)
```

The watcher (`clawd-pet watch`) is **not** owned by the plugin — run it in a Tabby pane.

## "Charm CC" — statusline + pet integration

The binary has three modes:
- `clawd-pet watch` — the pet pane (default)
- `clawd-pet emit <event>` — hook bridge → mood (writes `~/.clawd-pet/state`)
- `clawd-pet statusline` — **statusline wrapper**: forwards the payload to
  ccstatusline (renders the Charm statusline) AND extracts context% + cost into
  `~/.clawd-pet/context`, which the pet reads to show a colored `ctx% $cost` line.

Run **`/charm-setup`** once to point `statusLine` at the wrapper. Revert with
`node scripts/setup-statusline.js --revert`. A plugin can't set `statusLine` itself
(settings.json field), so this is a command, not an automatic hook.

## Build + install

1. Build the binary and copy it into `bin/` (run from the crate root):

   ```powershell
   powershell ./plugin/build.ps1
   ```

   This does `cargo build --release` and copies `target/release/clawd-pet.exe` to
   `plugin/bin/`. The pet's `assets/` must be reachable from wherever you run `watch`
   (the binary loads `assets/frames/<state>/` relative to its working dir), so launch
   the pane from the crate root: `cd C:\Users\Oreo\charm\clawd-pet; ./plugin/bin/clawd-pet.exe watch`.

2. Register + install (the dir is a single-plugin marketplace via
   `.claude-plugin/marketplace.json`):

   ```
   /plugin marketplace add C:\Users\Oreo\charm\clawd-pet\plugin
   /plugin            → install clawd-pet → enable
   ```

   Hooks register automatically and toggle as a unit when you disable/enable the plugin.

## How the two halves talk

```
Claude Code event ─▶ plugin hook ─▶ clawd-pet emit <event>
                                        └─ writes ~/.clawd-pet/state  (mood + nanos)
clawd-pet watch (Tabby pane) ──poll 5x/s──▶ reads state ─▶ animates mood + quip
```

## Boundary (why it's split)

A Claude Code plugin is declarative — it can bundle hooks, a prebuilt binary in `bin/`
(on PATH while enabled), and commands, all resolved via `${CLAUDE_PLUGIN_ROOT}`. It
**cannot** run a long-lived TUI or render a pane. So the plugin owns `emit` + wiring;
the pane stays a separately-launched process.
