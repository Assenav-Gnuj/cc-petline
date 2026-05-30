// Event bridge for clawd-pet (Phase 3).
//
// Claude Code hooks call `clawd-pet emit <event>` (fast, no TUI). That maps the
// hook event to a mood and appends it to a small state file. The long-running
// `clawd-pet watch` TUI polls that file each tick and drives the pet's mood.
//
// File format: a single line `<mood> <nanos>` (nanos makes every emit distinct so
// repeated same-mood events still register as activity). The state file lives at
// %USERPROFILE%\.clawd-pet\state  (or $HOME/.clawd-pet/state).

use std::fs;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

/// The shared state file both `emit` and `watch` use.
pub fn state_file() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".clawd-pet").join("state")
}

/// Map a hook event name (or a direct mood name) to a mood = PetState dir_name.
/// Returns None for events we intentionally ignore.
///
/// Accepts hook events in any case, with `-`/`_` separators (e.g. "PreToolUse",
/// "pre-tool-use", "pre_tool_use" all match). A bare mood name passes through so
/// `emit happy` works for testing.
pub fn event_to_mood(raw: &str) -> Option<&'static str> {
    let key: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();

    // Direct mood names (PetState::dir_name) pass through.
    const MOODS: [&str; 11] = [
        "idle", "working", "active", "thinking", "happy", "surprised", "sleepy",
        "oops", "error", "angry", "scared",
    ];
    if let Some(m) = MOODS.iter().find(|m| **m == key) {
        return Some(m);
    }

    // Hook event → mood.
    match key.as_str() {
        "sessionstart" => Some("happy"),
        "userpromptsubmit" => Some("thinking"),
        "pretooluse" => Some("working"),
        "posttooluse" => Some("active"),
        "subagentstart" => Some("active"),
        "subagentstop" => Some("active"),
        "notification" => Some("surprised"),
        "stop" => Some("happy"),
        // Failure-flavored events, if wired:
        "posttoolusefailure" | "stopfailure" | "permissiondenied" => Some("error"),
        "precompact" | "sessionend" => Some("sleepy"),
        _ => None,
    }
}

/// Write a mood to the state file (called by `emit`). Creates the dir if needed.
pub fn write_mood(mood: &str) -> Result<()> {
    let path = state_file();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    fs::write(&path, format!("{mood} {nanos}\n"))
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Read the raw state line (mood + nanos), or None if the file is missing/empty.
/// The TUI compares this raw string to detect new events (nanos differ per emit).
pub fn read_raw() -> Option<String> {
    let s = fs::read_to_string(state_file()).ok()?;
    let s = s.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// Extract just the mood token from a raw state line.
pub fn mood_of(raw: &str) -> &str {
    raw.split_whitespace().next().unwrap_or("")
}

/// True if `raw` names the PostToolUse hook event (case/separator-insensitive).
pub fn is_post_tool_use(raw: &str) -> bool {
    let key: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    key == "posttooluse"
}

/// PreToolUse "token-saver" guard (`clawd-pet guard`).
///
/// Reads the PreToolUse hook JSON from stdin. When `CLAWD_PET_GUARD=1` is set it
/// blocks a small, conservative denylist of wasteful/dangerous shell commands by
/// exiting 2 (Claude Code feeds the stderr message back so Claude self-corrects)
/// — saving the round trip the bad call would otherwise burn. When unset (the
/// default) it is a pure no-op that always exits 0, so it never interferes.
///
/// It only inspects `Bash`/shell-type tools; everything else passes untouched.
pub fn run_guard() -> Result<()> {
    if std::env::var("CLAWD_PET_GUARD").ok().as_deref() != Some("1") {
        return Ok(()); // disabled → never block
    }
    if std::io::stdin().is_terminal() {
        return Ok(()); // not a hook context
    }
    let mut payload = String::new();
    let _ = std::io::Read::read_to_string(&mut std::io::stdin(), &mut payload);
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&payload) else {
        return Ok(());
    };

    let tool = v.get("tool_name").and_then(|s| s.as_str()).unwrap_or("");
    if !tool.eq_ignore_ascii_case("Bash") {
        return Ok(()); // only guard shell commands
    }
    let cmd = v
        .pointer("/tool_input/command")
        .and_then(|s| s.as_str())
        .unwrap_or("");

    if let Some(reason) = guard_reason(cmd) {
        // Exit 2: Claude Code shows stderr to Claude and blocks the tool call.
        eprintln!("clawd-pet guard blocked this command: {reason}");
        std::process::exit(2);
    }
    Ok(())
}

/// Return Some(reason) if `cmd` matches the conservative block denylist.
///
/// Two flavours: (a) catastrophic/irreversible, and (b) token-wasteful patterns
/// that have a cheaper dedicated tool. Kept deliberately small and specific to
/// minimise false positives.
pub fn guard_reason(cmd: &str) -> Option<&'static str> {
    let c = cmd.to_lowercase();
    // (a) catastrophic / irreversible
    if c.contains("rm -rf /") || c.contains("rm -rf ~") || c.contains("rm -rf /*") {
        return Some("recursive force-delete of a root/home path");
    }
    if c.contains(":(){:|:&};:") || c.replace(' ', "").contains(":(){:|:&};:") {
        return Some("fork bomb");
    }
    if c.contains("git push") && c.contains("--force") && (c.contains("main") || c.contains("master"))
    {
        return Some("force-push to main/master");
    }
    if (c.contains("curl") || c.contains("wget")) && (c.contains("| sh") || c.contains("| bash")) {
        return Some("piping a network download straight into a shell");
    }
    // (b) token-wasteful: huge-output reads that have a cheaper dedicated tool
    if c.starts_with("find /") && !c.contains("-maxdepth") {
        return Some("unbounded `find /` — use Glob or add -maxdepth (saves a large output round trip)");
    }
    None
}

/// Best-effort: does a PostToolUse hook payload indicate the tool FAILED?
///
/// Claude Code pipes the tool result as JSON. There's no single universal "failed"
/// field across tools, so we look for the common markers and only return true on a
/// positive signal (so a parse miss falls back to the normal `active` mood).
pub fn stdin_indicates_failure(payload: &str) -> bool {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) else {
        return false;
    };
    let is_err = |node: &serde_json::Value| -> bool {
        node.get("is_error").and_then(|b| b.as_bool()) == Some(true)
            || node.get("success").and_then(|b| b.as_bool()) == Some(false)
            || node
                .get("status")
                .and_then(|s| s.as_str())
                .is_some_and(|s| s.eq_ignore_ascii_case("error") || s.eq_ignore_ascii_case("failed"))
    };
    if let Some(tr) = v.get("tool_response") {
        if is_err(tr) {
            return true;
        }
    }
    is_err(&v)
}
