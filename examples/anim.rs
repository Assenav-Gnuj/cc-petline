// clawd-pet — halfblocks animation test.
// Cycles 3 hand-built frames (bounce + blink) to judge whether halfblock Monamoji
// playback looks good enough before committing Phase 1 to it.
//
// Builds as an example (target/debug/examples/anim.exe) so it doesn't fight the
// main pet binary for the file lock.
//
// Keys: q quit   space pause   [ slower   ] faster

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use image::{DynamicImage, Rgba, RgbaImage};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::{Resize, StatefulImage};

const PURPLE: Color = Color::Rgb(0x7D, 0x56, 0xF4);
const MINT: Color = Color::Rgb(0x73, 0xF5, 0x9F);
const PINK: Color = Color::Rgb(0xFF, 0x5F, 0x87);
const CREAM: Color = Color::Rgb(0xFF, 0xFD, 0xF5);
const GREY: Color = Color::Rgb(0x88, 0x88, 0x88);

/// Build one 64x64 frame. `bob` shifts the body vertically (bounce); `blink`
/// closes the eyes to a thin line.
fn make_frame(bob: f32, blink: bool) -> DynamicImage {
    let n = 64u32;
    let cx = 32.0;
    let cy = 34.0 + bob;
    let mut img = RgbaImage::from_pixel(n, n, Rgba([0, 0, 0, 0]));
    for y in 0..n {
        for x in 0..n {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let body = (dx * dx) / (26.0 * 26.0) + (dy * dy) / (24.0 * 24.0);
            let mut px = Rgba([0, 0, 0, 0]);
            if body <= 1.0 {
                px = Rgba([0x7D, 0x56, 0xF4, 0xFF]);
                if dy < -6.0 && body < 0.7 {
                    px = Rgba([0xA5, 0x50, 0xDF, 0xFF]);
                }
            }
            let eye_y = 30.0 + bob;
            let eye = |ex: f32| ((x as f32 - ex).powi(2) + (y as f32 - eye_y).powi(2)).sqrt();
            if !blink {
                if eye(24.0) < 4.5 || eye(40.0) < 4.5 {
                    px = Rgba([0x73, 0xF5, 0x9F, 0xFF]);
                }
                if eye(24.0) < 2.0 || eye(40.0) < 2.0 {
                    px = Rgba([0x1A, 0x1A, 0x1A, 0xFF]);
                }
            } else {
                // closed eyes: short horizontal dashes
                let near = |ex: f32| (x as f32 - ex).abs() < 4.0 && (y as f32 - eye_y).abs() < 1.0;
                if near(24.0) || near(40.0) {
                    px = Rgba([0x1A, 0x1A, 0x1A, 0xFF]);
                }
            }
            let cheek_y = 40.0 + bob;
            let cheek = |ex: f32| ((x as f32 - ex).powi(2) + (y as f32 - cheek_y).powi(2)).sqrt();
            if (cheek(20.0) < 3.0 || cheek(44.0) < 3.0) && body <= 1.0 {
                px = Rgba([0xFF, 0x5F, 0x87, 0xFF]);
            }
            img.put_pixel(x, y, px);
        }
    }
    DynamicImage::ImageRgba8(img)
}

fn main() -> Result<()> {
    // 3-frame bounce + blink cycle.
    let frames = [
        make_frame(-3.0, false), // up, eyes open
        make_frame(0.0, true),   // mid, blinking
        make_frame(3.0, false),  // down, eyes open
    ];

    let mut picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    picker.set_protocol_type(ProtocolType::Halfblocks);

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut picker, &frames);
    ratatui::restore();
    result
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    picker: &mut Picker,
    frames: &[DynamicImage],
) -> Result<()> {
    let mut idx = 0usize;
    let mut paused = false;
    let mut delay_ms: u64 = 180;
    let mut elapsed: u64 = 0;
    let tick = 30u64; // event-poll granularity

    loop {
        let frame_img = frames[idx].clone();
        let mut protocol = picker.new_resize_protocol(frame_img);

        terminal.draw(|f| {
            let area = f.area();
            let block = Block::bordered()
                .border_style(Style::new().fg(PURPLE))
                .title(Line::from(" clawd-pet · halfblocks anim test ".bold().fg(CREAM)).centered());
            let inner = block.inner(area);
            f.render_widget(block, area);

            let rows = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(inner);
            let image = StatefulImage::new().resize(Resize::Fit(None));
            f.render_stateful_widget(image, rows[0], &mut protocol);

            let status = Paragraph::new(vec![
                Line::from(vec![
                    "frame ".fg(MINT),
                    format!("{}/{}", idx + 1, frames.len()).bold().fg(PINK),
                    "   delay ".fg(MINT),
                    format!("{delay_ms}ms").bold().fg(PINK),
                    if paused { "   [PAUSED]".fg(PINK) } else { "".into() },
                ]),
                Line::from("q quit   space pause   [ slower   ] faster".fg(GREY)),
            ])
            .centered();
            f.render_widget(status, rows[1]);
        })?;

        if event::poll(Duration::from_millis(tick))? {
            if let Event::Key(k) = event::read()? {
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char(' ') => paused = !paused,
                    KeyCode::Char('[') => delay_ms = (delay_ms + 30).min(2000),
                    KeyCode::Char(']') => delay_ms = delay_ms.saturating_sub(30).max(30),
                    _ => {}
                }
            }
        }

        if !paused {
            elapsed += tick;
            if elapsed >= delay_ms {
                elapsed = 0;
                idx = (idx + 1) % frames.len();
            }
        }
    }
    Ok(())
}
