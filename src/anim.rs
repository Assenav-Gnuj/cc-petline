// Animation engine + frame library for clawd-pet.
//
// An `Animation` is a list of frames plus playback timing. The `Player` advances
// the current animation per tick. The `Library` maps each `PetState` to an
// animation.
//
// ASSET SEAM: `build_library` prefers on-disk PNG frames (the ffmpeg → Monamoji
// output from scripts/convert-gifs.ps1) and falls back to the synthetic `sprite`
// generators when no assets are present. Drop frames into:
//     assets/frames/<state>/*.png   (sorted lexically = playback order)
// where <state> is PetState::dir_name(). No engine changes needed.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use image::DynamicImage;

use crate::sprite;
use crate::state::PetState;

pub struct Animation {
    pub frames: Vec<DynamicImage>,
    pub frame_ms: u64,
    pub looping: bool,
}

impl Animation {
    pub fn new(frames: Vec<DynamicImage>, frame_ms: u64, looping: bool) -> Self {
        Self { frames, frame_ms, looping }
    }
}

pub type Library = HashMap<PetState, Animation>;

/// Advances frames for the active state and reports when a one-shot finishes.
pub struct Player {
    state: PetState,
    idx: usize,
    elapsed: u64,
    finished: bool,
}

impl Player {
    pub fn new(state: PetState) -> Self {
        Self { state, idx: 0, elapsed: 0, finished: false }
    }

    pub fn frame_index(&self) -> usize {
        self.idx
    }

    /// True if the current (non-looping) animation has completed.
    pub fn finished(&self) -> bool {
        self.finished
    }

    /// Switch to a new state, resetting playback. No-op if already on it.
    pub fn set_state(&mut self, state: PetState) {
        if self.state != state {
            self.state = state;
            self.idx = 0;
            self.elapsed = 0;
            self.finished = false;
        }
    }

    /// Advance playback by `dt_ms`. Loops or clamps at the end per the animation.
    pub fn tick(&mut self, dt_ms: u64, lib: &Library) {
        let anim = match lib.get(&self.state) {
            Some(a) if !a.frames.is_empty() => a,
            _ => return,
        };
        self.elapsed += dt_ms;
        while self.elapsed >= anim.frame_ms {
            self.elapsed -= anim.frame_ms;
            if self.idx + 1 < anim.frames.len() {
                self.idx += 1;
            } else if anim.looping {
                self.idx = 0;
            } else {
                self.finished = true;
                break; // hold on the last frame
            }
        }
    }

    /// The frame to render right now.
    pub fn current<'a>(&self, lib: &'a Library) -> Option<&'a DynamicImage> {
        lib.get(&self.state).and_then(|a| a.frames.get(self.idx))
    }
}

/// Resolve the assets root (the dir containing `frames/`), trying several
/// locations so it works whether launched from the crate (pane), from a project
/// cwd (statusline command), or via an explicit override.
pub fn resolve_assets_root() -> Option<std::path::PathBuf> {
    let has_frames = |p: &Path| p.join("frames").is_dir();

    // 1. Explicit override.
    if let Ok(p) = std::env::var("CLAWD_PET_ASSETS") {
        let p = std::path::PathBuf::from(p);
        if has_frames(&p) {
            return Some(p);
        }
    }
    // 2. Current working directory (pane launched from crate root).
    let cwd = std::path::PathBuf::from("assets");
    if has_frames(&cwd) {
        return Some(cwd);
    }
    // 3. Relative to the executable: <exe_dir>/../../assets covers both
    //    target/<profile>/clawd-pet.exe and plugin/bin/clawd-pet.exe.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for up in [dir.join("../../assets"), dir.join("../assets"), dir.join("assets")] {
                if has_frames(&up) {
                    return Some(up);
                }
            }
        }
    }
    None
}

/// Load the `index`-th on-disk frame for a state (wrapping by frame count), so the
/// statusline can CYCLE frames across refreshes for a ~3fps animation instead of a
/// fixed mid-frame. Falls back to a synthetic frame when no PNGs exist.
pub fn frame_at(state: PetState, index: usize) -> Option<DynamicImage> {
    let root = resolve_assets_root()?;
    let dir = root.join("frames").join(state.dir_name());
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("png")))
        .collect();
    paths.sort();
    if paths.is_empty() {
        let frames = sprite::frames_for(state);
        let n = frames.len();
        return frames.into_iter().nth(if n == 0 { 0 } else { index % n });
    }
    let i = index % paths.len();
    image::open(&paths[i]).ok()
}

/// Load PNG frames from a directory, sorted by filename.
pub fn load_dir_frames(dir: &Path, frame_ms: u64, looping: bool) -> Result<Animation> {
    let mut paths: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("reading frame dir {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("png")))
        .collect();
    paths.sort();

    let mut frames = Vec::with_capacity(paths.len());
    for p in &paths {
        let img = image::open(p).with_context(|| format!("decoding {}", p.display()))?;
        frames.push(img);
    }
    Ok(Animation::new(frames, frame_ms, looping))
}

/// Natural playback rate for real GIF-extracted frame sequences (≈14fps). Disk
/// frames are dense (60–100), so they want a short per-frame time, unlike the
/// 2–4-frame synthetic poses.
const ASSET_FRAME_MS: u64 = 70;

/// Per-state timing for the SYNTHETIC fallback (few hand-made frames → slow each)
/// plus loop behaviour (shared with disk frames). One-shot states don't loop.
fn synthetic_timing(state: PetState) -> (u64, bool) {
    let looping = !state.is_oneshot();
    let frame_ms = match state {
        PetState::Idle => 220,
        PetState::Working => 110,
        PetState::Active => 100,
        PetState::Thinking => 170,
        PetState::Happy => 150,
        PetState::Surprised => 120,
        PetState::Sleepy => 400,
        PetState::Oops => 140,
        PetState::Error => 150,
        PetState::Angry => 80,
        PetState::Scared => 120,
    };
    (frame_ms, looping)
}

/// Build the animation library. Uses on-disk frames when present under
/// `assets/frames/<state>/` (played at ASSET_FRAME_MS), otherwise the synthetic
/// generators at their hand-tuned per-state timing.
pub fn build_library(assets_root: Option<&Path>) -> Library {
    let mut lib = Library::new();
    for state in PetState::ALL {
        let (syn_ms, looping) = synthetic_timing(state);
        let anim = assets_root
            .map(|root| root.join("frames").join(state.dir_name()))
            .filter(|d| d.is_dir())
            .and_then(|d| load_dir_frames(&d, ASSET_FRAME_MS, looping).ok())
            .filter(|a| !a.frames.is_empty())
            .unwrap_or_else(|| Animation::new(sprite::frames_for(state), syn_ms, looping));
        lib.insert(state, anim);
    }
    lib
}

/// How many states are rendering from real on-disk assets vs synthetic (for the
/// status line / diagnostics).
pub fn asset_summary(assets_root: &Path) -> (usize, usize) {
    let mut on_disk = 0;
    for state in PetState::ALL {
        let d = assets_root.join("frames").join(state.dir_name());
        let has = d
            .is_dir()
            .then(|| fs::read_dir(&d).ok())
            .flatten()
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .any(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("png")))
            })
            .unwrap_or(false);
        if has {
            on_disk += 1;
        }
    }
    (on_disk, PetState::ALL.len())
}
