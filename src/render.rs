// Unicode block image renderer for cc-petline (chafa-style), with TRANSPARENCY.
//
// Renders a sprite frame into terminal block glyphs. We render halfblocks
// ourselves (instead of ratatui-image) specifically so transparent pixels become
// blank cells / unset backgrounds — the pet sits on the terminal background
// instead of a black box. Three densities:
//
//   Halfblocks (▀/▄)         — 1x2 subpixels/cell. Samples the sprite SQUARE, so
//                              it looks best; this is the DEFAULT. fg=top, bg=bottom,
//                              bg left unset when a pixel is transparent.
//   Sextant   (U+1FB00 block) — 2x3 = 6 subpixels/cell. Denser but samples 4:3.
//   Quadrant  (U+25xx block)  — 2x2 = 4 subpixels/cell. Universal font support.
//
// Sextant/quadrant cells average their opaque subpixels into one fg with a blank
// (transparent) cell where no subpixel is opaque.

use image::{imageops::FilterType, DynamicImage};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// Alpha at/above which a subpixel counts as "ink".
const ALPHA_ON: u8 = 110;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenderMode {
    /// 2x3 Unicode sextants — finest, needs Symbols-for-Legacy-Computing glyphs.
    Sextant,
    /// 2x2 Unicode quadrants — universal font support, coarser.
    Quadrant,
    /// ratatui-image halfblocks (1x2) — the fallback.
    Halfblocks,
}

impl RenderMode {
    pub fn label(self) -> &'static str {
        match self {
            RenderMode::Sextant => "sextant",
            RenderMode::Quadrant => "quadrant",
            RenderMode::Halfblocks => "halfblocks",
        }
    }

    /// Cycle order: halfblocks → sextant → quadrant → halfblocks.
    pub fn next(self) -> RenderMode {
        match self {
            RenderMode::Halfblocks => RenderMode::Sextant,
            RenderMode::Sextant => RenderMode::Quadrant,
            RenderMode::Quadrant => RenderMode::Halfblocks,
        }
    }

    /// Subpixels per cell (width, height).
    fn cell_dims(self) -> (u32, u32) {
        match self {
            RenderMode::Sextant => (2, 3),
            RenderMode::Quadrant => (2, 2),
            RenderMode::Halfblocks => (1, 2),
        }
    }

    /// Glyph for an ink mask. Only sextant/quadrant use this; halfblocks is drawn
    /// directly in `render` (it needs per-side fg/bg, not a single mask glyph).
    fn glyph(self, mask: u32) -> char {
        match self {
            RenderMode::Sextant => sextant_char(mask),
            RenderMode::Quadrant => quadrant_char(mask),
            RenderMode::Halfblocks => match mask & 0b11 {
                0 => ' ',
                0b01 => '\u{2580}', // ▀
                0b10 => '\u{2584}', // ▄
                _ => '\u{2588}',    // █
            },
        }
    }
}

/// Map a 6-bit sextant ink mask to its Unicode glyph.
/// Bit = 1 << (y*2 + x): TL=1 TR=2 ML=4 MR=8 BL=16 BR=32.
/// The U+1FB00 block omits blank(0), left-half(21,▌), right-half(42,▐), full(63,█).
fn sextant_char(mask: u32) -> char {
    match mask {
        0 => ' ',
        21 => '\u{258C}', // ▌ left column
        42 => '\u{2590}', // ▐ right column
        63 => '\u{2588}', // █ full
        m => {
            let mut off = m - 1;
            if m > 21 {
                off -= 1;
            }
            if m > 42 {
                off -= 1;
            }
            char::from_u32(0x1FB00 + off).unwrap_or('\u{2588}')
        }
    }
}

/// Map a 4-bit quadrant ink mask to its Unicode glyph.
/// Bit = 1 << (y*2 + x): TL=1 TR=2 BL=4 BR=8. All universally available.
fn quadrant_char(mask: u32) -> char {
    match mask & 0b1111 {
        0 => ' ',
        1 => '\u{2598}',  // ▘ TL
        2 => '\u{259D}',  // ▝ TR
        3 => '\u{2580}',  // ▀ TL+TR
        4 => '\u{2596}',  // ▖ BL
        5 => '\u{258C}',  // ▌ TL+BL
        6 => '\u{259E}',  // ▞ TR+BL
        7 => '\u{259B}',  // ▛ TL+TR+BL
        8 => '\u{2597}',  // ▗ BR
        9 => '\u{259A}',  // ▚ TL+BR
        10 => '\u{2590}', // ▐ TR+BR
        11 => '\u{259C}', // ▜ TL+TR+BR
        12 => '\u{2584}', // ▄ BL+BR
        13 => '\u{2599}', // ▙ TL+BL+BR
        14 => '\u{259F}', // ▟ TR+BL+BR
        _ => '\u{2588}',  // █ full
    }
}

/// Size of a small companion pet, in terminal ROWS of sprite height. clawd/codachi
/// are compact (~a handful of lines), not pane-filling. `max_rows` caps it; the
/// sprite still shrinks to fit a tiny area.
pub const DEFAULT_ROWS: u16 = 8;
pub const MIN_ROWS: u16 = 4;
pub const MAX_ROWS: u16 = 28;

/// Fit a square sprite into `area`, capped at `max_rows` so it stays small (cells
/// are ~1:2 w:h, so cols = 2*rows for a square on-screen block). Returns (cols, rows).
pub fn fit_square(area: Rect, max_rows: u16) -> (u16, u16) {
    let rows = area
        .height
        .min(area.width / 2)
        .min(max_rows)
        .max(1);
    let cols = (rows * 2).min(area.width).max(1);
    (cols, rows)
}

/// Render `img` to styled lines for `mode`, sized to `max_rows`, plus the centered
/// target Rect within `area`. Transparent subpixels render as a blank cell (or an
/// unset bg), so the pet sits on the terminal background — never a black box.
pub fn render(
    mode: RenderMode,
    img: &DynamicImage,
    area: Rect,
    max_rows: u16,
) -> (Vec<Line<'static>>, Rect) {
    let (sub_w, sub_h) = mode.cell_dims();
    let (cols, rows) = fit_square(area, max_rows);

    let gw = cols as u32 * sub_w;
    let gh = rows as u32 * sub_h;
    // Nearest, not Lanczos: the sprites are pixel art authored on a small grid and
    // supersampled. A smoothing filter averages across design cells and turns hard
    // edges into mud at statusline sizes; nearest preserves the crisp blocks.
    let small = img.resize_exact(gw, gh, FilterType::Nearest).to_rgba8();

    let mut lines = Vec::with_capacity(rows as usize);
    for cy in 0..rows as u32 {
        let mut spans = Vec::with_capacity(cols as usize);
        for cx in 0..cols as u32 {
            let span = if mode == RenderMode::Halfblocks {
                // 1x2 cell: top pixel = fg of ▀, bottom = bg. Leave a side UNSET
                // when its pixel is transparent so the terminal shows through.
                let top = small.get_pixel(cx, cy * 2);
                let bot = small.get_pixel(cx, cy * 2 + 1);
                let t_on = top[3] >= ALPHA_ON;
                let b_on = bot[3] >= ALPHA_ON;
                match (t_on, b_on) {
                    (false, false) => Span::raw(" "),
                    (true, true) => Span::styled(
                        "\u{2580}", // ▀
                        Style::new().fg(rgb(top)).bg(rgb(bot)),
                    ),
                    (true, false) => Span::styled("\u{2580}", Style::new().fg(rgb(top))), // ▀
                    (false, true) => Span::styled("\u{2584}", Style::new().fg(rgb(bot))), // ▄
                }
            } else {
                // sextant/quadrant: ink mask + averaged fg, transparent bg.
                let mut mask = 0u32;
                let (mut r, mut g, mut b, mut n) = (0u32, 0u32, 0u32, 0u32);
                for sy in 0..sub_h {
                    for sx in 0..sub_w {
                        let px = small.get_pixel(cx * sub_w + sx, cy * sub_h + sy);
                        if px[3] >= ALPHA_ON {
                            mask |= 1 << (sy * sub_w + sx);
                            r += px[0] as u32;
                            g += px[1] as u32;
                            b += px[2] as u32;
                            n += 1;
                        }
                    }
                }
                if n == 0 {
                    Span::raw(" ")
                } else {
                    let ch = mode.glyph(mask);
                    let fg = Color::Rgb((r / n) as u8, (g / n) as u8, (b / n) as u8);
                    Span::styled(ch.to_string(), Style::new().fg(fg))
                }
            };
            spans.push(span);
        }
        lines.push(Line::from(spans));
    }

    let x = area.x + area.width.saturating_sub(cols) / 2;
    let y = area.y + area.height.saturating_sub(rows) / 2;
    let target = Rect { x, y, width: cols, height: rows };
    (lines, target)
}

fn rgb(px: &image::Rgba<u8>) -> Color {
    Color::Rgb(px[0], px[1], px[2])
}

// ---- ANSI half-block renderer (for the statusline column, not the TUI) ---------
//
// The statusline is plain stdout text, so we emit raw ANSI truecolor half-blocks
// instead of ratatui Spans. Transparent pixels leave fg/bg unset so the terminal
// background shows through (no black box), matching the TUI renderer's behavior.

/// Render `img` to `rows` lines of ANSI half-block text (each line ~`rows*2` cols
/// wide). Halfblocks only — finest sampling, and animation is moot in a statusline.
pub fn render_ansi(img: &DynamicImage, rows: u16) -> Vec<String> {
    let rows = rows.max(1) as u32;
    let cols = rows * 2; // square sprite at 1:2 cell ratio
    // Nearest keeps pixel-art blocks crisp at tiny sizes (see render() above).
    let small = img
        .resize_exact(cols, rows * 2, FilterType::Nearest)
        .to_rgba8();

    let mut out = Vec::with_capacity(rows as usize);
    for cy in 0..rows {
        let mut line = String::new();
        for cx in 0..cols {
            let top = small.get_pixel(cx, cy * 2);
            let bot = small.get_pixel(cx, cy * 2 + 1);
            let t = top[3] >= ALPHA_ON;
            let b = bot[3] >= ALPHA_ON;
            match (t, b) {
                (false, false) => line.push(' '),
                (true, true) => {
                    line.push_str(&format!(
                        "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m\u{2580}\x1b[0m",
                        top[0], top[1], top[2], bot[0], bot[1], bot[2]
                    ));
                }
                (true, false) => {
                    line.push_str(&format!(
                        "\x1b[38;2;{};{};{}m\u{2580}\x1b[0m",
                        top[0], top[1], top[2]
                    ));
                }
                (false, true) => {
                    line.push_str(&format!(
                        "\x1b[38;2;{};{};{}m\u{2584}\x1b[0m",
                        bot[0], bot[1], bot[2]
                    ));
                }
            }
        }
        out.push(line);
    }
    out
}

/// Visible width of a string, ignoring ANSI escape sequences (`ESC [ ... m`).
/// Used to pad statusline rows so the pet column aligns on the right.
pub fn visible_width(s: &str) -> usize {
    let mut w = 0usize;
    let mut in_escape = false;
    for c in s.chars() {
        if in_escape {
            // A CSI sequence ends at its final byte: an ASCII letter (0x40-0x7E),
            // e.g. 'm' (SGR color), 'l'/'h' (mode set/reset like ?25l, ?7l).
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if c == '\x1b' {
            in_escape = true;
        } else {
            w += 1;
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn sextant_table_known_values() {
        assert_eq!(sextant_char(0), ' ');
        assert_eq!(sextant_char(63), '\u{2588}');
        assert_eq!(sextant_char(21), '\u{258C}');
        assert_eq!(sextant_char(42), '\u{2590}');
        assert_eq!(sextant_char(1), '\u{1FB00}'); // first
        assert_eq!(sextant_char(2), '\u{1FB01}');
        assert_eq!(sextant_char(3), '\u{1FB02}');
        assert_eq!(sextant_char(62), '\u{1FB3B}'); // last
    }

    #[test]
    fn sextant_all_distinct() {
        let set: HashSet<char> = (0u32..64).map(sextant_char).collect();
        assert_eq!(set.len(), 64, "all 64 sextant masks must map to distinct chars");
    }

    #[test]
    fn quadrant_all_distinct() {
        let set: HashSet<char> = (0u32..16).map(quadrant_char).collect();
        assert_eq!(set.len(), 16, "all 16 quadrant masks must map to distinct chars");
    }
}
