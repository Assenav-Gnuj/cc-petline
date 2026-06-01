// slice.rs — cut horizontal sprite strips into per-frame PNGs.
//
// Source art lives as `<dir>/<state>_strip.png`, each a row of square frames laid
// left→right. This slices every strip into frames the engine loads:
//     assets/frames/<state>/0001.png, 0002.png, ...
//
// Built as an example so it doesn't fight the main pet binary's .exe lock, and so
// it reuses the `image` crate already in deps (no external tool, cross-platform).
//
// Usage (run from crate root):
//   cargo run --example slice                                # default src = assets/strips
//   cargo run --example slice -- <src_dir>                   # custom source dir
//   cargo run --example slice -- <src_dir> <px>              # override frame size (default = strip height)
//   cargo run --example slice -- --theme <name> <src_dir>    # slice into a theme pack
//
// State name = the part before `_strip` in the filename. The engine's
// PetState::dir_name() values are what actually get loaded; unmapped names are
// still sliced (so you can stage extras) but the engine ignores dirs it has no
// state for.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use image::GenericImageView;

fn main() -> Result<()> {
    // Optional `--theme <name>` flag; remaining args are positional (<src> <px>).
    let mut theme: Option<String> = None;
    let mut pos: Vec<String> = Vec::new();
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        if a == "--theme" {
            theme = it.next();
        } else {
            pos.push(a);
        }
    }

    let src = PathBuf::from(
        pos.first()
            .cloned()
            .unwrap_or_else(|| "assets/strips".to_string()),
    );
    let forced_size: Option<u32> = pos.get(1).and_then(|s| s.parse().ok());

    // Default → assets/frames (the shipped Fox default art); --theme <name> → a
    // pack at assets/themes/<name>/frames that CLAWD_PET_THEME=<name> picks up.
    let out_root = match &theme {
        Some(t) => PathBuf::from("assets/themes").join(t).join("frames"),
        None => PathBuf::from("assets/frames"),
    };
    eprintln!("slicing into {}", out_root.display());

    if !src.is_dir() {
        bail!("source dir not found: {}", src.display());
    }

    let mut strips: Vec<PathBuf> = fs::read_dir(&src)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with("_strip.png"))
        })
        .collect();
    strips.sort();

    if strips.is_empty() {
        bail!("no *_strip.png files in {}", src.display());
    }

    let mut total_states = 0;
    let mut total_frames = 0;

    for strip in &strips {
        let stem = strip.file_name().unwrap().to_str().unwrap();
        // <state>_strip.png  ->  <state>
        let state = stem
            .strip_suffix("_strip.png")
            .context("unexpected filename")?
            .to_string();

        let n = slice_one(strip, &out_root.join(&state), forced_size)?;
        println!("{state:<10} {n:>4} frames");
        total_states += 1;
        total_frames += n;
    }

    println!("\ndone: {total_states} states, {total_frames} frames -> {}", out_root.display());
    Ok(())
}

/// Slice one horizontal strip into square frames of `size` (default = image height).
fn slice_one(strip: &Path, out_dir: &Path, forced_size: Option<u32>) -> Result<usize> {
    let img = image::open(strip).with_context(|| format!("opening {}", strip.display()))?;
    let (w, h) = img.dimensions();
    let size = forced_size.unwrap_or(h);

    if size == 0 || w < size {
        bail!("{}: bad geometry {w}x{h} (frame size {size})", strip.display());
    }
    let count = w / size;
    if w % size != 0 {
        eprintln!(
            "  warn {}: width {w} not a multiple of {size}; using {count} frames, {} px remainder",
            strip.display(),
            w % size
        );
    }

    // Fresh output dir so stale frames never linger.
    if out_dir.exists() {
        fs::remove_dir_all(out_dir).ok();
    }
    fs::create_dir_all(out_dir)?;

    for i in 0..count {
        let frame = img.crop_imm(i * size, 0, size, size);
        let path = out_dir.join(format!("{:04}.png", i + 1));
        frame
            .save(&path)
            .with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(count as usize)
}
