// Procedural sprite generation for clawd-pet.
//
// These synthetic frames are the BUILT-IN FALLBACK: real Monamoji PNG frames load
// from assets/frames/<state>/ when present (see anim::build_library). Since those
// frames are gitignored (trademark), a fresh checkout runs entirely on these.
//
// Each PetState has a generator returning a Vec<DynamicImage>. They share one
// parameterized `draw(Pose)` so adding a mood = a new pose recipe, not new pixel
// code. The synthetic critter conveys mood via eye shape, bob, cheeks, body tint,
// and small markers (Zzz, sweat, sparkle).

use image::{DynamicImage, Rgba, RgbaImage};

use crate::state::PetState;

pub const N: u32 = 64;

// Charm palette (RGBA)
const BODY: [u8; 4] = [0x7D, 0x56, 0xF4, 0xFF];
const MINT: [u8; 4] = [0x73, 0xF5, 0x9F, 0xFF];
const PINK: [u8; 4] = [0xFF, 0x5F, 0x87, 0xFF];
const DARK: [u8; 4] = [0x1A, 0x1A, 0x1A, 0xFF];
const CLEAR: [u8; 4] = [0, 0, 0, 0];
// Mood tints (replace the purple body)
const RED: [u8; 4] = [0xD8, 0x3A, 0x3A, 0xFF];
const DEEPRED: [u8; 4] = [0xA1, 0x1F, 0x1F, 0xFF];
const PALE: [u8; 4] = [0x9A, 0x86, 0xE0, 0xFF];

#[derive(Clone, Copy, PartialEq)]
pub enum Eyes {
    Open,
    Big,    // wide (surprise / scared)
    Closed, // dashes (blink / sleep)
    Happy,  // ^ ^ arcs
    Angry,  // \ / slanted brows
}

pub struct Pose {
    pub bob: f32,
    pub eyes: Eyes,
    pub cheeks: bool,
    pub tint: [u8; 4],    // body color
    pub zzz: u8,          // 0 = none; 1..=3 ascending z's
    pub sweat: bool,      // a blue drop top-left (oops/scared)
    pub sparkle: bool,    // mint sparkles (celebrate/mindblown)
}

impl Pose {
    fn base() -> Self {
        Pose {
            bob: 0.0,
            eyes: Eyes::Open,
            cheeks: false,
            tint: BODY,
            zzz: 0,
            sweat: false,
            sparkle: false,
        }
    }
}

fn draw(p: &Pose) -> DynamicImage {
    let cx = 32.0;
    let cy = 34.0 + p.bob;
    let mut img = RgbaImage::from_pixel(N, N, Rgba(CLEAR));

    for y in 0..N {
        for x in 0..N {
            let xf = x as f32;
            let yf = y as f32;
            let dx = xf - cx;
            let dy = yf - cy;
            let body = (dx * dx) / (26.0 * 26.0) + (dy * dy) / (24.0 * 24.0);
            let mut px = Rgba(CLEAR);

            if body <= 1.0 {
                px = Rgba(p.tint);
                if dy < -6.0 && body < 0.7 {
                    // lighten the top highlight relative to tint
                    px = Rgba(lighten(p.tint));
                }
            }

            let eye_y = 30.0 + p.bob;
            match p.eyes {
                Eyes::Open => {
                    let e = |ex: f32| ((xf - ex).powi(2) + (yf - eye_y).powi(2)).sqrt();
                    if e(24.0) < 4.5 || e(40.0) < 4.5 {
                        px = Rgba(MINT);
                    }
                    if e(24.0) < 2.0 || e(40.0) < 2.0 {
                        px = Rgba(DARK);
                    }
                }
                Eyes::Big => {
                    let e = |ex: f32| ((xf - ex).powi(2) + (yf - eye_y).powi(2)).sqrt();
                    if e(24.0) < 6.0 || e(40.0) < 6.0 {
                        px = Rgba([0xFF, 0xFF, 0xFF, 0xFF]);
                    }
                    if e(24.0) < 2.5 || e(40.0) < 2.5 {
                        px = Rgba(DARK);
                    }
                }
                Eyes::Closed => {
                    let dash = |ex: f32| (xf - ex).abs() < 4.0 && (yf - eye_y).abs() < 1.0;
                    if dash(24.0) || dash(40.0) {
                        px = Rgba(DARK);
                    }
                }
                Eyes::Happy => {
                    let arc = |ex: f32| {
                        let lx = (xf - ex).abs();
                        lx < 4.0 && ((yf - eye_y) - (lx - 3.0)).abs() < 1.0
                    };
                    if arc(24.0) || arc(40.0) {
                        px = Rgba(DARK);
                    }
                }
                Eyes::Angry => {
                    // slanted brows: left eye "\", right eye "/"
                    let left = (xf - 24.0).abs() < 4.0
                        && ((yf - eye_y) - (xf - 24.0) * 0.5).abs() < 1.0;
                    let right = (xf - 40.0).abs() < 4.0
                        && ((yf - eye_y) + (xf - 40.0) * 0.5).abs() < 1.0;
                    if left || right {
                        px = Rgba(DARK);
                    }
                }
            }

            if p.cheeks {
                let cheek_y = 40.0 + p.bob;
                let c = |ex: f32| ((xf - ex).powi(2) + (yf - cheek_y).powi(2)).sqrt();
                if (c(20.0) < 3.0 || c(44.0) < 3.0) && body <= 1.0 {
                    px = Rgba(PINK);
                }
            }

            img.put_pixel(x, y, px);
        }
    }

    if p.zzz > 0 {
        draw_zzz(&mut img, p.zzz);
    }
    if p.sweat {
        draw_sweat(&mut img);
    }
    if p.sparkle {
        draw_sparkle(&mut img);
    }

    DynamicImage::ImageRgba8(img)
}

fn lighten(c: [u8; 4]) -> [u8; 4] {
    let f = |v: u8| ((v as u16 * 5 / 4).min(255)) as u8;
    [f(c[0]), f(c[1]), f(c[2]), c[3]]
}

fn draw_zzz(img: &mut RgbaImage, count: u8) {
    let count = count.min(3);
    for i in 0..count {
        let bx = 46 + (i as i32) * 4;
        let by = 14 - (i as i32) * 4;
        for yy in 0..3i32 {
            for xx in 0..3i32 {
                let on = yy == 0 || yy == 2 || (xx + yy == 2);
                if on {
                    put(img, bx + xx, by + yy, MINT);
                }
            }
        }
    }
}

fn draw_sweat(img: &mut RgbaImage) {
    // a small blue drop near the top-left of the head
    let blue = [0x6C, 0xC0, 0xF0, 0xFF];
    for (dx, dy) in [(0, 0), (0, 1), (-1, 1), (1, 1), (0, 2)] {
        put(img, 18 + dx, 16 + dy, blue);
    }
}

fn draw_sparkle(img: &mut RgbaImage) {
    // little mint plus-signs in opposite corners
    for (cxp, cyp) in [(12, 12), (52, 14)] {
        for d in -2..=2i32 {
            put(img, cxp + d, cyp, MINT);
            put(img, cxp, cyp + d, MINT);
        }
    }
}

fn put(img: &mut RgbaImage, x: i32, y: i32, c: [u8; 4]) {
    if x >= 0 && y >= 0 && (x as u32) < N && (y as u32) < N {
        img.put_pixel(x as u32, y as u32, Rgba(c));
    }
}

// ---- Per-state frame sequences -------------------------------------------------

/// idle (eye-rolls): gentle bob with an occasional blink.
fn idle() -> Vec<DynamicImage> {
    vec![
        draw(&Pose { bob: -2.0, ..Pose::base() }),
        draw(&Pose { bob: 0.0, ..Pose::base() }),
        draw(&Pose { bob: 1.0, eyes: Eyes::Closed, ..Pose::base() }),
        draw(&Pose { bob: 0.0, ..Pose::base() }),
    ]
}

/// working (typing): quick tight bob, focused.
fn working() -> Vec<DynamicImage> {
    vec![
        draw(&Pose { bob: -1.0, ..Pose::base() }),
        draw(&Pose { bob: 1.0, ..Pose::base() }),
    ]
}

/// thinking (funny): slow look around (eyes via bob), no blink.
fn thinking() -> Vec<DynamicImage> {
    vec![
        draw(&Pose { bob: 0.0, ..Pose::base() }),
        draw(&Pose { bob: -1.0, ..Pose::base() }),
        draw(&Pose { bob: 0.0, eyes: Eyes::Big, ..Pose::base() }),
        draw(&Pose { bob: -1.0, ..Pose::base() }),
    ]
}

/// happy (love-hearts): happy eyes + cheeks, soft bob.
fn happy() -> Vec<DynamicImage> {
    vec![
        draw(&Pose { bob: -2.0, eyes: Eyes::Happy, cheeks: true, ..Pose::base() }),
        draw(&Pose { bob: 0.0, eyes: Eyes::Happy, cheeks: true, ..Pose::base() }),
    ]
}

/// surprised / mindblown: big eyes, sparkles, small shake. One-shot.
fn mindblown() -> Vec<DynamicImage> {
    vec![
        draw(&Pose { bob: -3.0, eyes: Eyes::Big, sparkle: true, ..Pose::base() }),
        draw(&Pose { bob: -1.0, eyes: Eyes::Big, sparkle: true, ..Pose::base() }),
        draw(&Pose { bob: -3.0, eyes: Eyes::Big, ..Pose::base() }),
    ]
}

/// sleepy (zzz): closed eyes, slow breathing, ascending Zzz.
fn sleep() -> Vec<DynamicImage> {
    vec![
        draw(&Pose { bob: 1.0, eyes: Eyes::Closed, zzz: 1, ..Pose::base() }),
        draw(&Pose { bob: 2.0, eyes: Eyes::Closed, zzz: 2, ..Pose::base() }),
        draw(&Pose { bob: 1.0, eyes: Eyes::Closed, zzz: 3, ..Pose::base() }),
        draw(&Pose { bob: 2.0, eyes: Eyes::Closed, zzz: 2, ..Pose::base() }),
    ]
}

/// oops (oops-mistake): closed eyes + sweat drop, sheepish. One-shot.
fn oops() -> Vec<DynamicImage> {
    vec![
        draw(&Pose { bob: 0.0, eyes: Eyes::Closed, sweat: true, ..Pose::base() }),
        draw(&Pose { bob: 1.0, eyes: Eyes::Closed, sweat: true, ..Pose::base() }),
    ]
}

/// error (angry): red tint, angry brows.
fn error() -> Vec<DynamicImage> {
    vec![
        draw(&Pose { bob: -1.0, eyes: Eyes::Angry, tint: RED, ..Pose::base() }),
        draw(&Pose { bob: 0.0, eyes: Eyes::Angry, tint: RED, ..Pose::base() }),
    ]
}

/// angry / rage: deep red, angry, fast shake.
fn rage() -> Vec<DynamicImage> {
    vec![
        draw(&Pose { bob: -1.0, eyes: Eyes::Angry, tint: DEEPRED, ..Pose::base() }),
        draw(&Pose { bob: 1.0, eyes: Eyes::Angry, tint: DEEPRED, ..Pose::base() }),
        draw(&Pose { bob: -1.0, eyes: Eyes::Angry, tint: DEEPRED, cheeks: false, ..Pose::base() }),
    ]
}

/// scared (oooh-scared): pale, big eyes, sweat, tremble.
fn scared() -> Vec<DynamicImage> {
    vec![
        draw(&Pose { bob: -1.0, eyes: Eyes::Big, tint: PALE, sweat: true, ..Pose::base() }),
        draw(&Pose { bob: 0.0, eyes: Eyes::Big, tint: PALE, sweat: true, ..Pose::base() }),
        draw(&Pose { bob: -1.0, eyes: Eyes::Big, tint: PALE, ..Pose::base() }),
    ]
}

/// The synthetic fallback frames for any state. (Used only when a state has no
/// on-disk sprite frames; the user's mona strips normally cover all 11.)
pub fn frames_for(state: PetState) -> Vec<DynamicImage> {
    match state {
        PetState::Idle => idle(),
        PetState::Working => working(),
        PetState::Active => working(),
        PetState::Thinking => thinking(),
        PetState::Happy => happy(),
        PetState::Surprised => mindblown(),
        PetState::Sleepy => sleep(),
        PetState::Oops => oops(),
        PetState::Error => error(),
        PetState::Angry => rage(),
        PetState::Scared => scared(),
    }
}
