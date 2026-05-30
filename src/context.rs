// Statusline-data bridge: the "Charm CC" integration (statusline ↔ pet).
//
// `clawd-pet statusline` is wired as Claude Code's `statusLine` command (via the
// plugin's /charm-setup). Claude Code pipes a JSON payload to it (debounced 300ms).
// This module:
//   1. reads context% + cost straight from the payload (verified field paths),
//   2. writes a tiny `~/.clawd-pet/context` file the pet's `watch` loop reads,
//   3. forwards the SAME payload to ccstatusline and relays its rendered line,
//      so the Charm statusline still displays normally.
//
// Both fields are ONLY in the statusLine payload (hooks don't carry cost/context),
// which is why this is a statusline wrapper, not a hook. Verified schema:
//   context_window.used_percentage   (number 0..100, or null early/after compact)
//   context_window.context_window_size, context_window.current_usage.{...}_tokens
//   cost.total_cost_usd
//   model.id, exceeds_200k_tokens

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::Result;

/// Live session metrics the pet displays / reacts to.
#[derive(Clone, Copy, Debug, Default)]
pub struct Context {
    /// Context-window usage, 0.0..=1.0 (best-effort; 0 if unknown).
    pub pct: f32,
    /// Session cost in USD.
    pub cost_usd: f32,
    /// Estimated context tokens used.
    pub tokens: u64,
}

fn context_file() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".clawd-pet").join("context")
}

/// The statusLine wrapper entry point (`clawd-pet statusline`):
/// read stdin payload → write context file → render ccstatusline + a pet column → stdout.
///
/// The pet here is a STATIC mood frame (the statusline only redraws per message /
/// refresh, so it can't animate). The mood is read from the same `~/.clawd-pet/state`
/// file the hooks write via `emit`, so the cat reflects what Claude is doing.
pub fn run_statusline() -> Result<()> {
    let mut payload = String::new();
    let _ = std::io::stdin().read_to_string(&mut payload);

    // Best-effort: never let metric extraction break the statusline.
    if let Ok(ctx) = extract(&payload) {
        let _ = write_context(&ctx);
    }

    let status = render_ccstatusline(&payload).unwrap_or_default();
    print!("{}", compose_with_pet(&status));
    Ok(())
}

/// Pet column defaults (overridable via env so it can't wrap a narrow terminal):
///   CLAWD_PET_ROWS   — cat height in terminal rows (width = rows*2). Default 6.
///   CLAWD_PET_GAP    — spaces between statusline text and the cat. Default 2.
///   CLAWD_PET_WIDTH  — total target width to right-align the column within. If
///                      set, the cat hugs this column (pad to WIDTH - catwidth)
///                      instead of (statusline width + gap). Set it to your
///                      terminal columns to pin the sidebar; 0/unset = auto.
const DEFAULT_PET_ROWS: u16 = 6;
const DEFAULT_PET_GAP: usize = 2;

fn env_usize(key: &str) -> Option<usize> {
    std::env::var(key).ok().and_then(|v| v.trim().parse().ok())
}

/// Join the ccstatusline output (left) with a pet sprite column (right), aligned
/// so the cat forms a clean right column like a sidebar. Sizing is env-driven so
/// it never wraps: set CLAWD_PET_WIDTH to your terminal columns to pin it.
fn compose_with_pet(status: &str) -> String {
    let rows_cfg = env_usize("CLAWD_PET_ROWS").unwrap_or(DEFAULT_PET_ROWS as usize) as u16;
    let pet_rows = rows_cfg.clamp(1, 24);
    let gap = env_usize("CLAWD_PET_GAP").unwrap_or(DEFAULT_PET_GAP);
    let total_width = env_usize("CLAWD_PET_WIDTH").filter(|w| *w > 0);

    // Current mood from the shared state file (hooks/emit write it); default Idle.
    let raw = crate::events::read_raw();
    let mood = raw
        .as_deref()
        .and_then(|r| crate::state::PetState::from_dir_name(crate::events::mood_of(r)))
        .unwrap_or(crate::state::PetState::Idle);
    // The state file's trailing nanos token seeds quip rotation, so the bubble
    // changes whenever the mood/state changes.
    let seed = raw
        .as_deref()
        .and_then(|r| r.split_whitespace().nth(1))
        .and_then(|n| n.parse::<u64>().ok())
        .unwrap_or(0);

    // Frame-cycle for a ~3fps animation: the statusline can't loop, but it re-runs
    // every refresh, so we pick the frame for the CURRENT wall-clock time. Modulo
    // advances frames in order regardless of how often refreshes land — smooth
    // during activity (frequent refreshes), gently jumping when idle. Frame period
    // via CLAWD_PET_FPS_MS (default 120ms ≈ 8fps source rate, sampled at refresh).
    let frame_ms = env_usize("CLAWD_PET_FPS_MS").unwrap_or(120).max(1) as u128;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let frame_idx = (now_ms / frame_ms) as usize;
    let cat = match crate::anim::frame_at(mood, frame_idx) {
        Some(img) => crate::render::render_ansi(&img, pet_rows),
        None => return status.to_string(), // no assets → just the statusline
    };
    let cat_w = cat.iter().map(|l| crate::render::visible_width(l)).max().unwrap_or(0);

    let status_lines: Vec<&str> = status.trim_end_matches('\n').split('\n').collect();
    let rows = status_lines.len().max(cat.len());
    let status_w = status_lines
        .iter()
        .map(|l| crate::render::visible_width(l))
        .max()
        .unwrap_or(0);

    // Render the quote as a real rounded speech bubble placed to the RIGHT of the
    // cat and vertically centered — the mascot "speaking" beside its image. The
    // mood word goes on its own line BELOW the bubble. Inner text width via
    // CLAWD_PET_BUBBLE (default 40, clamp 12..100); 0 disables the whole column.
    let bubble_w = env_usize("CLAWD_PET_BUBBLE").unwrap_or(40);
    let bubble: Vec<String> = if bubble_w == 0 {
        Vec::new()
    } else {
        speech_bubble(&quote_line(seed), bubble_w.clamp(12, 100))
    };
    let bubble_h = bubble.len();
    // Rainbow phase flows the lolcat gradient over the bubble. Tie it to wall-clock
    // so it drifts each refresh (the bubble text itself only changes on mood change).
    let phase = ((now_ms / 80) % 360) as f32;
    // Right-of-cat column, PRE-STYLED: rainbow-gradient bubble box, then a plain
    // bold mood word line below it.
    let mut right: Vec<String> = bubble
        .iter()
        .enumerate()
        .map(|(r, l)| rainbow_line(l, r, phase))
        .collect();
    if bubble_h > 0 {
        // bold mint mood word, matching the Charm palette
        right.push(format!(
            "\x1b[1;38;2;115;245;159m\u{bb} {}\x1b[0m",
            mood_word(mood)
        ));
    }
    let b_off = cat.len().saturating_sub(right.len()) / 2;
    let tail_row = b_off + bubble_h / 2; // tail sprouts from the bubble's middle
    // Grow the block if the right column is taller than both cat and statusline.
    let rows = rows.max(b_off + right.len());

    // Cat sits just past the statusline text when a bubble is shown (so the
    // bubble has room to its right); otherwise honour CLAWD_PET_WIDTH, never
    // overlapping the text.
    let cat_col = if !right.is_empty() {
        status_w + gap
    } else {
        match total_width {
            Some(w) => w.saturating_sub(cat_w).max(status_w + 1),
            None => status_w + gap,
        }
    };

    let mut out = String::new();
    for i in 0..rows {
        let left = status_lines.get(i).copied().unwrap_or("");
        let pad = cat_col.saturating_sub(crate::render::visible_width(left));
        out.push_str(left);
        out.push_str(&" ".repeat(pad));
        // Cat slice (may be blank on rows past the cat's height).
        if let Some(cat_line) = cat.get(i) {
            out.push_str(cat_line);
        } else {
            out.push_str(&" ".repeat(cat_w));
        }
        // Right column (bubble box + mood line) beside the cat, with a tail
        // pointing back at the mascot on the bubble's middle row.
        if let Some(rel) = i.checked_sub(b_off) {
            if let Some(line) = right.get(rel) {
                let tail = if i == tail_row && gap >= 1 {
                    format!("{}\u{25c2}", " ".repeat(gap - 1)) // ◂ points left at the cat
                } else {
                    " ".repeat(gap)
                };
                out.push_str(&tail);
                out.push_str(line); // already styled (rainbow bubble / mint banner)
            }
        }
        out.push('\n');
    }

    out
}

/// The short mood word shown on the line below the bubble.
fn mood_word(mood: crate::state::PetState) -> &'static str {
    use crate::state::PetState::*;
    match mood {
        Idle => "idle",
        Working => "working",
        Active => "on it",
        Thinking => "thinking",
        Happy => "yay",
        Surprised => "whoa",
        Sleepy => "zzz",
        Oops => "oops",
        Error => "uh oh",
        Angry => "grr",
        Scared => "eep",
    }
}

/// The line that goes inside the speech bubble: a rotating programming quote with
/// attribution, varied by `seed` so it refreshes when the mood/state changes.
fn quote_line(seed: u64) -> String {
    const QUOTES: &[(&str, &str)] = &[
        ("Premature optimization is the root of all evil.", "Knuth"),
        ("Talk is cheap. Show me the code.", "Torvalds"),
        ("Simplicity is prerequisite for reliability.", "Dijkstra"),
        ("Programs must be written for people to read.", "Abelson"),
        ("Make it work, make it right, make it fast.", "Beck"),
        ("Two hard things: cache invalidation and naming things.", "Karlton"),
        ("When you have to explain a joke, it's bad. Same with code.", "Fowler"),
        ("First, solve the problem. Then, write the code.", "Johnson"),
        ("Good programmers write code humans understand.", "Fowler"),
        ("Weeks of coding can save you hours of planning.", "anon"),
    ];
    let (q, who) = QUOTES[(seed as usize) % QUOTES.len()];
    format!("\"{q}\" \u{2014} {who}")
}

/// Colorize one (plain) line with a flowing lolcat-style truecolor rainbow.
/// Hue runs across columns and shifts by `row` (diagonal flow) and `phase`
/// (advances per refresh). Spaces are emitted uncolored to keep escapes lean.
fn rainbow_line(line: &str, row: usize, phase: f32) -> String {
    let mut out = String::new();
    for (col, ch) in line.chars().enumerate() {
        if ch == ' ' {
            out.push(' ');
            continue;
        }
        let hue = (phase + col as f32 * 12.0 + row as f32 * 18.0) % 360.0;
        let (r, g, b) = hsv_to_rgb(hue, 0.75, 1.0);
        out.push_str(&format!("\x1b[38;2;{r};{g};{b}m{ch}"));
    }
    out.push_str("\x1b[0m");
    out
}

/// HSV (h in 0..360, s/v in 0..1) → 8-bit RGB.
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h as u32 / 60) % 6 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8,
    )
}


/// Render `text` as a rounded speech-bubble box (a Vec of rows), wrapping the
/// text to `inner_w` columns. Pure box-drawing chars; the caller adds the tail
/// and dimming. Box is sized to the widest wrapped line, not the full inner_w.
fn speech_bubble(text: &str, inner_w: usize) -> Vec<String> {
    let lines = wrap_text(text, inner_w);
    let w = lines
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0)
        .max(1);
    let bar = "\u{2500}".repeat(w + 2); // ─, +2 for the inner single-space padding
    let mut out = Vec::with_capacity(lines.len() + 2);
    out.push(format!("\u{256d}{bar}\u{256e}")); // ╭ ─ ╮
    for l in &lines {
        let padn = w - l.chars().count();
        out.push(format!("\u{2502} {}{} \u{2502}", l, " ".repeat(padn))); // │ … │
    }
    out.push(format!("\u{2570}{bar}\u{256f}")); // ╰ ─ ╯
    out
}

/// Word-wrap a plain (ANSI-free) string into lines of at most `width` columns.
/// A single word longer than `width` is hard-split.
fn wrap_text(s: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        let wlen = word.chars().count();
        if cur.is_empty() {
            if wlen > width {
                let mut chunk = String::new();
                for ch in word.chars() {
                    if chunk.chars().count() == width {
                        lines.push(std::mem::take(&mut chunk));
                    }
                    chunk.push(ch);
                }
                cur = chunk;
            } else {
                cur = word.to_string();
            }
        } else if cur.chars().count() + 1 + wlen <= width {
            cur.push(' ');
            cur.push_str(word);
        } else {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// Parse context% and cost straight from the statusLine payload.
fn extract(payload: &str) -> Result<Context> {
    let v: serde_json::Value = serde_json::from_str(payload)?;

    let cost_usd = v
        .pointer("/cost/total_cost_usd")
        .and_then(|n| n.as_f64())
        .unwrap_or(0.0) as f32;

    let cw = v.get("context_window");

    // Preferred: the pre-computed percentage (0..100). May be null early / post-compact.
    let mut pct = cw
        .and_then(|c| c.get("used_percentage"))
        .and_then(|n| n.as_f64())
        .map(|p| (p / 100.0) as f32);

    // Tokens used (input side counts toward the window).
    let tokens = cw
        .and_then(|c| c.get("current_usage"))
        .map(|u| {
            let g = |k: &str| u.get(k).and_then(|n| n.as_u64()).unwrap_or(0);
            g("input_tokens") + g("cache_read_input_tokens") + g("cache_creation_input_tokens")
        })
        .unwrap_or(0);

    // Fallback: compute pct from tokens / window if used_percentage was absent.
    if pct.is_none() {
        let window = cw
            .and_then(|c| c.get("context_window_size"))
            .and_then(|n| n.as_u64())
            .unwrap_or(200_000);
        if window > 0 && tokens > 0 {
            pct = Some((tokens as f32 / window as f32).min(1.0));
        }
    }

    // Last-ditch coarse flag.
    let pct = pct.unwrap_or_else(|| {
        if v.get("exceeds_200k_tokens").and_then(|b| b.as_bool()) == Some(true) {
            1.0
        } else {
            0.0
        }
    });

    Ok(Context { pct: pct.clamp(0.0, 1.0), cost_usd, tokens })
}

/// Pipe the payload to ccstatusline (npm global) and return its rendered output.
/// On Windows the global bin is a .cmd shim, so go through `cmd /c`.
fn render_ccstatusline(payload: &str) -> Option<String> {
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/c", "ccstatusline"]);
        c
    } else {
        Command::new("ccstatusline")
    };

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let mut stdin = child.stdin.take()?;
    let _ = stdin.write_all(payload.as_bytes());
    drop(stdin); // close stdin so ccstatusline finishes reading

    let out = child.wait_with_output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

fn write_context(ctx: &Context) -> Result<()> {
    let path = context_file();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    // "pct cost tokens" — one line, trivial for the pet to parse.
    fs::write(&path, format!("{:.4} {:.4} {}\n", ctx.pct, ctx.cost_usd, ctx.tokens))?;
    Ok(())
}

/// Read the context file (pet side). None if absent/unparseable.
pub fn read_context() -> Option<Context> {
    let s = fs::read_to_string(context_file()).ok()?;
    let mut it = s.split_whitespace();
    let pct = it.next()?.parse().ok()?;
    let cost_usd = it.next()?.parse().ok()?;
    let tokens = it.next().and_then(|t| t.parse().ok()).unwrap_or(0);
    Some(Context { pct, cost_usd, tokens })
}
