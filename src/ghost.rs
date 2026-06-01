// A built-in synthetic "ghost" mascot — the alternate theme that ships without
// any art files (selected via CC_PETLINE_THEME=ghost). Self-contained: its own
// canvas + draw helpers so it never depends on sprite.rs internals.
//
// A spectral pale body: rounded dome on top, straight sides, a wavy/scalloped
// fringe at the bottom, and two dark eyes that vary by mood — mirroring the default
// mascot's expression set so all 11 moods read clearly at tiny sizes.

use image::{Rgba, RgbaImage};

use crate::state::PetState;

const W: u32 = 32;
const H: u32 = 32;

const BODY: [u8; 4] = [0xCF, 0xD8, 0xF7, 0xFF]; // pale spectral blue-white
const EYE: [u8; 4] = [0x22, 0x22, 0x33, 0xFF];
const PINK: [u8; 4] = [0xFF, 0x5F, 0x87, 0xFF];

/// Two frames per mood (a 1px bob/expression change), matching the mascot set.
pub fn frames_for(state: PetState) -> Vec<RgbaImage> {
    use PetState::*;
    match state {
        Idle => vec![draw(Eyes::Open, 0), draw(Eyes::Open, 1)],
        Working => vec![draw(Eyes::Open, 0), draw(Eyes::Wat, 1)],
        Active => vec![draw(Eyes::Open, 0), draw(Eyes::Happy, 1)],
        Thinking => vec![draw(Eyes::Look, 0), draw(Eyes::Look, 1)],
        Happy => vec![draw(Eyes::Happy, 0), draw(Eyes::Happy, 1)],
        Surprised => vec![draw(Eyes::Wide, 0), draw(Eyes::Wide, 1)],
        Sleepy => vec![draw(Eyes::Closed, 0), draw(Eyes::Closed, 1)],
        Oops => vec![draw(Eyes::Wat, 0), draw(Eyes::Wat, 1)],
        Error => vec![draw(Eyes::X, 0), draw(Eyes::X, 1)],
        Angry => vec![draw(Eyes::Angry, 0), draw(Eyes::Angry, 1)],
        Scared => vec![draw(Eyes::Wide, 0), draw(Eyes::Wide, 1)],
    }
}

enum Eyes {
    Open,
    Closed,
    Happy,
    Wide,
    Look,
    Wat,
    Angry,
    X,
}

fn draw(eyes: Eyes, bob: i32) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(W, H, Rgba([0, 0, 0, 0]));
    let cx = W as i32 / 2;
    let top = 6 + bob;
    let body_bottom = 23 + bob;
    let r = 9i32; // dome radius == half-width
    let dome_cy = top + r;

    // Body: per-column fill from a rounded dome top to a scalloped fringe bottom.
    for x in -r..=r {
        let dy = ((r * r - x * x).max(0) as f32).sqrt() as i32;
        let col_top = dome_cy - dy;
        // 3 scallops across the width: a 0..3 triangle wave for the foot length.
        let m = (x + r).rem_euclid(6);
        let foot = (3 - (m - 3).abs()).max(0);
        let col_bottom = body_bottom + foot;
        for y in col_top..=col_bottom {
            put(&mut img, cx + x, y, BODY);
        }
    }

    // Eyes — same vocabulary as the default mascot, scaled to the ghost face.
    let eye_y = dome_cy - 1;
    let lx = cx - 4;
    let rx = cx + 4;
    match eyes {
        Eyes::Open | Eyes::Look | Eyes::Wide | Eyes::Wat => {
            let er = if matches!(eyes, Eyes::Wide) { 3 } else { 2 };
            let look = matches!(eyes, Eyes::Look) as i32;
            let c = if matches!(eyes, Eyes::Wat) { PINK } else { EYE };
            disc(&mut img, lx + look, eye_y, er, c);
            disc(&mut img, rx + look, eye_y, er, c);
        }
        Eyes::Closed => {
            for x in -2..=2 {
                put(&mut img, lx + x, eye_y, EYE);
                put(&mut img, rx + x, eye_y, EYE);
            }
        }
        Eyes::Happy => {
            for x in -2..=2i32 {
                let yy = eye_y - (2 - x.abs());
                put(&mut img, lx + x, yy, EYE);
                put(&mut img, rx + x, yy, EYE);
            }
        }
        Eyes::Angry => {
            for x in -2..=2i32 {
                let yy = eye_y + (2 - x.abs()) - 2;
                put(&mut img, lx + x, yy, EYE);
                put(&mut img, rx + x, yy, EYE);
            }
            disc(&mut img, lx, eye_y + 1, 1, EYE);
            disc(&mut img, rx, eye_y + 1, 1, EYE);
        }
        Eyes::X => {
            for d in -2..=2i32 {
                put(&mut img, lx + d, eye_y + d, EYE);
                put(&mut img, lx + d, eye_y - d, EYE);
                put(&mut img, rx + d, eye_y + d, EYE);
                put(&mut img, rx + d, eye_y - d, EYE);
            }
        }
    }

    // A little round mouth for surprised/oops.
    if matches!(eyes, Eyes::Wide | Eyes::Wat) {
        disc(&mut img, cx, eye_y + 6, 2, EYE);
    }

    img
}

fn put(img: &mut RgbaImage, x: i32, y: i32, c: [u8; 4]) {
    if x >= 0 && x < W as i32 && y >= 0 && y < H as i32 {
        img.put_pixel(x as u32, y as u32, Rgba(c));
    }
}

fn disc(img: &mut RgbaImage, cx: i32, cy: i32, r: i32, c: [u8; 4]) {
    for y in -r..=r {
        for x in -r..=r {
            if x * x + y * y <= r * r {
                put(img, cx + x, cy + y, c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_state_has_ghost_frames() {
        for s in PetState::ALL {
            assert!(!frames_for(s).is_empty());
        }
    }
}
