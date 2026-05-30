// Theme dispatch for the SYNTHETIC mascot (the fallback used when a theme has no
// on-disk frame PNGs). On-disk art always wins — see anim::frame_at /
// anim::build_library, which only call this when their frame dir is empty.
//
// A theme is chosen via CLAWD_PET_THEME:
//   morgana | default | clawd  → the cat-blob in sprite.rs (the shipped default)
//   ghost                       → the spectral ghost in ghost.rs
//   <anything else>             → falls back to the default character
//
// To add a real custom mascot, drop a sliced sprite pack at
// `<assets>/themes/<name>/frames/<state>/*.png` (see examples/slice.rs --theme);
// then this synthetic path is never reached for that theme.

use image::RgbaImage;

use crate::state::PetState;

/// Synthetic frames for `state` under the named `theme`. Normalized to `RgbaImage`
/// (the cat's `sprite::frames_for` yields `DynamicImage`, so convert it).
pub fn frames_for(state: PetState, theme: &str) -> Vec<RgbaImage> {
    match theme {
        "ghost" => crate::ghost::frames_for(state),
        _ => crate::sprite::frames_for(state)
            .into_iter()
            .map(|d| d.into_rgba8())
            .collect(),
    }
}
