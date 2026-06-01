---
name: charm-setup
description: Wire the Charm statusline + clawd-pet together — point Claude Code's statusLine at the clawd-pet wrapper so the statusline renders AND the pet gets live context%/cost.
---

This finishes the "Charm CC" integration: it repoints `statusLine` in
`~/.claude/settings.json` to **`clawd-pet statusline`**, a wrapper that:

1. forwards the payload to **ccstatusline** (so the Charm 4-line statusline renders
   exactly as before — this assumes ccstatusline is already installed/configured), and
2. extracts **context% + session cost** into `~/.clawd-pet/context`, which the pet's
   `watch` pane reads to show a colored ctx% / `$cost` line and react near the limit.

## Do this

1. Confirm the bundled binary exists: `${CLAUDE_PLUGIN_ROOT}/bin/clawd-pet.exe`.
   If not, tell the user to run `powershell ./plugin/build.ps1` from the crate root.

2. Run the setup script (it backs up settings.json and remembers the previous
   statusLine for revert):

   ```
   node "${CLAUDE_PLUGIN_ROOT}/scripts/setup-statusline.js" "${CLAUDE_PLUGIN_ROOT}/bin/clawd-pet.exe"
   ```

3. Tell the user to start the pet pane (Tabby profile **clawd-pet**, or
   `"${CLAUDE_PLUGIN_ROOT}/bin/clawd-pet.exe" watch` from the crate root) and that the
   ctx%/cost line updates as they work (statusLine fires ≤ every 300ms).

## Sizing the fox column (avoid wrap on narrow terminals)

The fox is a right column appended to each statusline row. Tune it via env vars on
the statusLine command (set in `~/.claude/settings.json` `statusLine.command`, or as
user env vars):

- `CLAWD_PET_ROWS` — fox height in terminal rows (width = rows×2). Default `6`.
- `CLAWD_PET_GAP` — spaces between the statusline text and the fox. Default `2`.
- `CLAWD_PET_WIDTH` — pin the column inside this total width (set to your terminal's
  column count, e.g. `120`, to right-align the fox to the edge and keep it from
  wrapping). `0`/unset = auto (fox sits just past the statusline text).
- `CLAWD_PET_ASSETS` — absolute path to the `assets/` dir if the fox doesn't appear
  (the statusLine command's cwd is the project, not the crate).

Example statusLine command with sizing:
`"<plugin>/bin/clawd-pet.exe" statusline` and set `CLAWD_PET_WIDTH=120` in env.

## Notes / caveats

- A Claude Code plugin **cannot set `statusLine` itself** (it's a settings.json field
  CC reads directly) — that's why this is a command that patches settings.json, not an
  automatic hook.
- **Revert:** `node "${CLAUDE_PLUGIN_ROOT}/scripts/setup-statusline.js" --revert`
  (restores the previous statusLine; backup also at `settings.json.bak-clawdpet`).
- ccstatusline must be on PATH (the wrapper calls it). If it isn't installed, the
  statusline renders blank but the pet still gets context data.
