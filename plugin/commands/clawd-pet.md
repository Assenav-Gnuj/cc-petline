---
name: clawd-pet
description: Show how to launch the clawd-pet companion pane and explain its current state.
---

The **clawd-pet** companion renders in its own terminal pane — Claude Code plugins
can't host a persistent visual pane themselves, so the watcher runs separately while
this plugin's hooks feed it moods.

To launch the pet, tell the user to run this in a dedicated Tabby split pane:

```
"${CLAUDE_PLUGIN_ROOT}/bin/clawd-pet.exe" watch
```

(or just `clawd-pet.exe` — `watch` is the default). On this machine the binary also
lives at `C:\Users\Oreo\charm\clawd-pet\target\release\clawd-pet.exe`.

While it runs:
- The plugin's hooks fire `clawd-pet emit <event>` on SessionStart, UserPromptSubmit,
  PreToolUse, PostToolUse, Notification, SubagentStop, and Stop.
- `emit` maps each event to a mood and writes `~/.clawd-pet/state`; the watcher polls
  that file (~5x/sec) and animates the matching mood with a speech-bubble quip.
- Mood map: SessionStart/Stop → happy, UserPromptSubmit → thinking, PreToolUse →
  working, PostToolUse/SubagentStop → active, Notification → surprised. Idle drifts to
  sleepy after 8s.

In the pane, keys 1-0/- force moods, `r` cycles render mode, `+`/`_` resize, `q` quits.

If asked to test without Claude driving it, run e.g.
`"${CLAUDE_PLUGIN_ROOT}/bin/clawd-pet.exe" emit happy` in another shell and watch the
pet react.
