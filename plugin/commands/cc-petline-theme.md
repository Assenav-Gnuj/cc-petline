---
name: cc-petline-theme
description: Show or switch the cc-petline mascot theme (fox, ghost, or a custom sprite pack).
argument-hint: "[theme-name]"
---

The user wants to view or change the **cc-petline mascot theme**. The argument (a theme
name) is: `$ARGUMENTS`

Run the binary's `theme` subcommand — it persists the choice to `~/.cc-petline/theme`,
which the pet reads on its next statusline refresh / pane tick (this is how the change
sticks across the separate processes Claude Code spawns for the statusline). Use the
plugin's bundled binary:

- **No argument given** → show the current theme and the list of available ones:

  ```
  "${CLAUDE_PLUGIN_ROOT}/bin/cc-petline.exe" theme
  ```

- **A theme name given** → set it:

  ```
  "${CLAUDE_PLUGIN_ROOT}/bin/cc-petline.exe" theme $ARGUMENTS
  ```

If `${CLAUDE_PLUGIN_ROOT}/bin/cc-petline.exe` isn't found, fall back to
`C:\Users\Oreo\charm\clawd-pet\target\release\cc-petline.exe`.

Then relay the command's output to the user verbatim (it already explains what will
render). Notes to keep in mind:

- Built-in themes: **fox** (the default mascot, on-disk sprite frames under
  `assets/frames/`) and **ghost** (a fully synthetic spectral character, no art files).
  Any folder at `assets/themes/<name>/frames/` is also selectable by `<name>`.
- An unrecognized name with no on-disk pack falls back to the synthetic character — the
  command warns when that happens.
- If `CC_PETLINE_THEME` is set in the environment, it overrides the saved theme; the
  command warns about this too.
- The theme updates on the next refresh — no restart needed. If the pet pane is open
  it picks the new mascot up within a tick.

To add a real custom mascot: generate sprite strips (e.g. with the Sprites-project
tool), slice them with
`cargo run --example slice -- --theme <name> <strip-dir>`, then `/cc-petline-theme <name>`.
