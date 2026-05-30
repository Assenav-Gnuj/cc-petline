// clawd-pet — animated sprite companion for Claude Code, rendered in a Tabby pane.
//
// Two modes (see `main` dispatch):
//   clawd-pet              → long-running TUI: renders the pet, polls the state file
//   clawd-pet watch        → same as above (explicit)
//   clawd-pet emit <event> → fast, no TUI: maps a Claude Code hook event to a mood
//                            and writes the state file the TUI watches. Called by
//                            the plugin's hooks. (See src/events.rs.)
//
// Phase 0 proved sixel does NOT work in Tabby. We render via our own transparent
// halfblocks (render.rs); `r` cycles halfblocks → sextant → quadrant.
//
// Modules:
//   sprite.rs — procedural per-state frames (synthetic fallback for all 11 moods)
//   anim.rs   — Animation + Player + on-disk PNG loader (build_library)
//   state.rs  — PetState (11 moods) + Machine (event-driven; one-shot→idle, idle→sleepy)
//   render.rs — transparent halfblocks / sextant / quadrant renderer
//   events.rs — emit/watch state-file bridge + hook-event→mood mapping
//
// Manual controls in the TUI (hooks drive state automatically when wired):
//   1 idle  2 working  3 active   4 thinking 5 happy   6 surprised
//   7 sleepy 8 oops    9 error    0 angry    - scared
//   n next  p prev   r render-mode   + bigger   _ smaller   q quit

mod anim;
mod context;
mod events;
mod quips;
mod render;
mod sprite;
mod state;

use std::io::{IsTerminal, Read as _};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use anim::{Library, Player};
use render::RenderMode;
use state::{Machine, PetState};

const PURPLE: Color = Color::Rgb(0x7D, 0x56, 0xF4);
const MINT: Color = Color::Rgb(0x73, 0xF5, 0x9F);
const PINK: Color = Color::Rgb(0xFF, 0x5F, 0x87);
const CREAM: Color = Color::Rgb(0xFF, 0xFD, 0xF5);
const GREY: Color = Color::Rgb(0x88, 0x88, 0x88);

const TICK_MS: u64 = 40;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        // Hook bridge: map event → mood, write state file, exit immediately.
        Some("emit") => {
            let raw = args.get(1).map(String::as_str).unwrap_or("");
            // For PostToolUse, peek at the piped hook JSON (only when stdin is a
            // real pipe, i.e. the hook context) to show `error` when the tool
            // failed instead of the normal `active`. Zero-token: a local read of
            // the payload Claude Code already provides.
            let mood = if events::is_post_tool_use(raw) && !std::io::stdin().is_terminal() {
                let mut buf = String::new();
                let _ = std::io::stdin().read_to_string(&mut buf);
                if events::stdin_indicates_failure(&buf) {
                    Some("error")
                } else {
                    events::event_to_mood(raw)
                }
            } else {
                events::event_to_mood(raw)
            };
            if let Some(mood) = mood {
                events::write_mood(mood)?;
            }
            // Unknown/ignored events are a silent no-op so hooks never error.
            Ok(())
        }
        // Token-saver PreToolUse guard. Reads the piped hook JSON and, when
        // enabled (CLAWD_PET_GUARD=1), blocks a small denylist of wasteful/
        // dangerous commands via exit code 2 (Claude Code feeds stderr back so
        // Claude self-corrects). Default OFF → always exit 0, never interferes.
        Some("guard") => events::run_guard(),
        // Statusline wrapper: render via ccstatusline + extract context%/cost for the pet.
        Some("statusline") => context::run_statusline(),
        // Long-running TUI (default, or explicit `watch`).
        Some("watch") | None => watch(),
        Some(other) => {
            eprintln!(
                "clawd-pet: unknown command {other:?} (use: watch | emit <event> | statusline)"
            );
            std::process::exit(2);
        }
    }
}

/// Build per-character rainbow-colored spans for the pane's speech bubble. Hue
/// runs across the text and shifts by `phase` (advanced each frame) so the
/// gradient flows. Italic, matching the original styling.
fn rainbow_spans(text: &str, phase: f32) -> Vec<Span<'static>> {
    text.chars()
        .enumerate()
        .map(|(i, ch)| {
            let hue = (phase + i as f32 * 14.0) % 360.0;
            let (r, g, b) = hsv_to_rgb(hue, 0.7, 1.0);
            Span::styled(
                ch.to_string(),
                Style::new().fg(Color::Rgb(r, g, b)).italic(),
            )
        })
        .collect()
}

/// HSV (h 0..360, s/v 0..1) → 8-bit RGB. (Mirrors context.rs's converter.)
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

fn watch() -> Result<()> {
    let assets_root = PathBuf::from("assets");
    let lib = anim::build_library(Some(&assets_root));
    let (on_disk, total) = anim::asset_summary(&assets_root);

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &lib, on_disk, total);
    ratatui::restore();
    result
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    lib: &Library,
    on_disk: usize,
    total: usize,
) -> Result<()> {
    let mut machine = Machine::default();
    let mut player = Player::new(machine.state());
    let mut mode = RenderMode::Halfblocks; // best look + transparent bg; r cycles
    let mut max_rows: u16 = render::DEFAULT_ROWS; // small companion, clawd/codachi-sized
    let mut quips = quips::Quips::new(); // rotating speech / quotes / citations

    // Last raw state line we acted on, so we only react to NEW emits (the nanos
    // suffix changes every emit even for a repeated mood).
    let mut last_seen = events::read_raw();
    let mut poll_acc: u64 = 0;
    const POLL_MS: u64 = 200; // how often to check the state/context files

    // Live session metrics from the statusline wrapper (context% + cost).
    let mut ctx = context::read_context().unwrap_or_default();

    // Frame counter drives the flowing rainbow on the speech bubble (pane runs at
    // ~25fps so the gradient animates smoothly, unlike the static statusline).
    let mut frame: u64 = 0;

    loop {
        frame = frame.wrapping_add(1);
        // React to hook-driven mood changes + refresh context metrics.
        poll_acc += TICK_MS;
        if poll_acc >= POLL_MS {
            poll_acc = 0;
            let cur = events::read_raw();
            if cur != last_seen {
                if let Some(raw) = &cur {
                    if let Some(st) = PetState::from_dir_name(events::mood_of(raw)) {
                        machine.apply(st);
                        quips.refresh(st); // fresh comment on every mood change
                    }
                }
                last_seen = cur;
            }
            if let Some(c) = context::read_context() {
                ctx = c;
            }
        }

        player.set_state(machine.state());
        let img = player.current(lib).cloned();

        terminal.draw(|f| {
            let area = f.area();
            let block = Block::bordered()
                .border_style(Style::new().fg(PURPLE))
                .title(Line::from(" clawd-pet · phase 1 ".bold().fg(CREAM)).centered());
            let inner = block.inner(area);
            f.render_widget(block, area);

            // pet art · speech bubble · status
            let rows = Layout::vertical([
                Constraint::Min(3),
                Constraint::Length(2),
                Constraint::Length(5),
            ])
            .split(inner);
            let art_area = rows[0];
            let speech_area = rows[1];

            if let Some(img) = &img {
                let (lines, target) = render::render(mode, img, art_area, max_rows);
                f.render_widget(Paragraph::new(lines), target);
            }

            // Speech bubble: the pet's current commentary / quote / citation,
            // with a flowing lolcat rainbow animated by the frame counter.
            let mut spans = vec!["“".fg(GREY)];
            spans.extend(rainbow_spans(quips.current(), frame as f32 * 6.0));
            spans.push("”".fg(GREY));
            let speech = Paragraph::new(Line::from(spans)).centered();
            f.render_widget(speech, speech_area);

            // Context% colored by pressure: mint < 70% < pink < 90% < red.
            let pct = (ctx.pct * 100.0).round() as u16;
            let pct_color = if ctx.pct >= 0.90 {
                Color::Rgb(0xD8, 0x3A, 0x3A)
            } else if ctx.pct >= 0.70 {
                PINK
            } else {
                MINT
            };
            let status = Paragraph::new(vec![
                Line::from(vec![
                    "mood: ".fg(MINT),
                    machine.state().label().bold().fg(PINK),
                    "   ctx: ".fg(MINT),
                    format!("{pct}%").bold().fg(pct_color),
                    "   $".fg(MINT),
                    format!("{:.3}", ctx.cost_usd).fg(GREY),
                ]),
                Line::from("1 idle 2 working 3 active 4 thinking 5 happy 6 surprised".fg(GREY)),
                Line::from("7 sleepy 8 oops 9 error 0 angry - scared  (hooks drive auto)".fg(GREY)),
                Line::from("n next  p prev  r mode  +/_ size  q quit".fg(GREY)),
            ])
            .centered();
            f.render_widget(status, rows[2]);
        })?;

        if event::poll(Duration::from_millis(TICK_MS))? {
            if let Event::Key(k) = event::read()? {
                let cur = machine.state();
                let idx = PetState::ALL.iter().position(|s| *s == cur).unwrap_or(0);
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('r') => mode = mode.next(),
                    KeyCode::Char('+') => {
                        max_rows = (max_rows + 1).min(render::MAX_ROWS)
                    }
                    KeyCode::Char('_') => {
                        max_rows = max_rows.saturating_sub(1).max(render::MIN_ROWS)
                    }
                    KeyCode::Char('1') => machine.force(PetState::Idle),
                    KeyCode::Char('2') => machine.force(PetState::Working),
                    KeyCode::Char('3') => machine.force(PetState::Active),
                    KeyCode::Char('4') => machine.force(PetState::Thinking),
                    KeyCode::Char('5') => machine.force(PetState::Happy),
                    KeyCode::Char('6') => machine.force(PetState::Surprised),
                    KeyCode::Char('7') => machine.force(PetState::Sleepy),
                    KeyCode::Char('8') => machine.force(PetState::Oops),
                    KeyCode::Char('9') => machine.force(PetState::Error),
                    KeyCode::Char('0') => machine.force(PetState::Angry),
                    KeyCode::Char('-') => machine.force(PetState::Scared),
                    KeyCode::Char('n') => {
                        machine.force(PetState::ALL[(idx + 1) % PetState::ALL.len()])
                    }
                    KeyCode::Char('p') => {
                        let len = PetState::ALL.len();
                        machine.force(PetState::ALL[(idx + len - 1) % len])
                    }
                    _ => {}
                }
            }
        }

        player.tick(TICK_MS, lib);
        let before = machine.state();
        machine.tick(TICK_MS, player.finished());
        // Fresh quip whenever the state changed this tick (manual key earlier in the
        // loop, auto idle→sleepy, or one-shot→idle); otherwise rotate on its timer.
        if machine.state() != before {
            quips.refresh(machine.state());
        } else {
            quips.tick(TICK_MS, machine.state());
        }
    }
    Ok(())
}
