// Speech / commentary for clawd-pet.
//
// The pet shows a rotating speech bubble: random commentary, plus quotations with
// attribution (the "citation"). Lines are mood-aware — each PetState has its own
// pool, mixed with a shared set of programming quotes — so the pet feels reactive.
//
// No external RNG dependency: a tiny xorshift seeded from SystemTime nanos picks
// lines. (The workflow-runtime ban on Date/random is JS-only; this is native Rust.)

use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::PetState;

/// Minimal xorshift64* PRNG — enough for picking quips, zero deps.
pub struct Rng(u64);

impl Rng {
    pub fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15)
            | 1;
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn pick<'a>(&mut self, items: &'a [&'a str]) -> &'a str {
        if items.is_empty() {
            return "";
        }
        items[(self.next_u64() % items.len() as u64) as usize]
    }
}

// Programming quotations — the "citations". Attribution is part of the line so the
// pet is quoting a source, not claiming the words.
const QUOTES: &[&str] = &[
    "\"Talk is cheap. Show me the code.\" — Linus Torvalds",
    "\"Premature optimization is the root of all evil.\" — Knuth",
    "\"There are only two hard things in CS: cache invalidation and naming.\" — Karlton",
    "\"Programs must be written for people to read.\" — Abelson & Sussman",
    "\"Simplicity is prerequisite for reliability.\" — Dijkstra",
    "\"Make it work, make it right, make it fast.\" — Kent Beck",
    "\"Any fool can write code a computer understands.\" — Martin Fowler",
    "\"Weeks of coding can save you hours of planning.\" — folklore",
    "\"The best error message is the one that never shows up.\" — Thomas Fuchs",
    "\"Code is like humor. When you have to explain it, it's bad.\" — Cory House",
    "\"First, solve the problem. Then, write the code.\" — John Johnson",
    "\"It works on my machine.\" — every developer, eventually",
];

// Shared idle musings — generic ambient chatter.
const MUSINGS: &[&str] = &[
    "just vibing...",
    "what shall we build?",
    "ready when you are.",
    "the cursor blinks expectantly.",
    "another day, another diff.",
    "got snacks?",
    "ship it? maybe.",
];

/// Mood-specific one-liners.
fn mood_lines(state: PetState) -> &'static [&'static str] {
    match state {
        PetState::Idle => &[
            "just vibing...",
            "waiting for orders.",
            "stretch break.",
            "the repo is quiet. too quiet.",
        ],
        PetState::Working => &[
            "on it!",
            "tools spinning up...",
            "crunch crunch crunch.",
            "this is the fun part.",
        ],
        PetState::Active => &[
            "making moves.",
            "tool ran clean.",
            "next!",
            "momentum building.",
        ],
        PetState::Thinking => &[
            "hmm...",
            "let me think about this.",
            "considering the options.",
            "rubber-ducking it.",
        ],
        PetState::Happy => &[
            "yesss!",
            "that worked!",
            "we did it!",
            "green across the board.",
        ],
        PetState::Surprised => &[
            "whoa!",
            "didn't see that coming.",
            "!!!",
            "plot twist.",
        ],
        PetState::Sleepy => &[
            "zzz...",
            "so quiet... *yawn*",
            "wake me for the build.",
            "dreaming of clean diffs.",
        ],
        PetState::Oops => &[
            "oops.",
            "my bad, retrying.",
            "let's try that again.",
            "nobody saw that.",
        ],
        PetState::Error => &[
            "uh oh.",
            "that errored.",
            "check the logs.",
            "red. not great.",
        ],
        PetState::Angry => &[
            "AGAIN?!",
            "grr. flaky much?",
            "fix it fix it fix it.",
            "I am NOT amused.",
        ],
        PetState::Scared => &[
            "are you sure about that?",
            "that looks destructive...",
            "double-check before you commit!",
            "eep. careful.",
        ],
    }
}

/// Holds RNG + the current line, and rotates it over time / on demand.
pub struct Quips {
    rng: Rng,
    current: String,
    elapsed_ms: u64,
    rotate_ms: u64,
}

impl Quips {
    pub fn new() -> Self {
        let mut q = Quips {
            rng: Rng::new(),
            current: String::new(),
            elapsed_ms: 0,
            rotate_ms: 6_000, // new line every ~6s
        };
        q.current = q.pick_for(PetState::Idle);
        q
    }

    /// Pick a fresh line for `state`: ~1 in 4 is a programming quote (citation),
    /// otherwise a mood line or a shared musing.
    fn pick_for(&mut self, state: PetState) -> String {
        let roll = self.rng.next_u64() % 4;
        let line = if roll == 0 {
            self.rng.pick(QUOTES)
        } else {
            let lines = mood_lines(state);
            // blend in shared musings for idle-ish moods
            if matches!(state, PetState::Idle | PetState::Sleepy) && roll == 1 {
                self.rng.pick(MUSINGS)
            } else {
                self.rng.pick(lines)
            }
        };
        line.to_string()
    }

    /// Force a fresh line immediately (call on mood change).
    pub fn refresh(&mut self, state: PetState) {
        self.current = self.pick_for(state);
        self.elapsed_ms = 0;
    }

    /// Advance the rotation timer; swap to a new line when it elapses.
    pub fn tick(&mut self, dt_ms: u64, state: PetState) {
        self.elapsed_ms += dt_ms;
        if self.elapsed_ms >= self.rotate_ms {
            self.current = self.pick_for(state);
            self.elapsed_ms = 0;
        }
    }

    pub fn current(&self) -> &str {
        &self.current
    }
}
