// Pet state machine for clawd-pet.
//
// 11 moods — one per mascot animation. Each maps to a frame dir the engine loads
// from assets/frames/<state>/ (the shipped Fox pack), with a synthetic fallback.
// Claude Code hook events map onto these moods (see src/events.rs).

/// What the pet is currently feeling/doing. Each maps to one sprite strip / animation.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PetState {
    Idle,      // resting, waiting for the user
    Working,   // tools running / generating
    Active,    // busy, energetic
    Thinking,  // pondering
    Happy,     // pleased / success
    Surprised, // big/unexpected result
    Sleepy,    // long inactivity
    Oops,      // recoverable slip / retry
    Error,     // tool/command error
    Angry,     // repeated/escalated failure
    Scared,    // destructive/dangerous op
}

impl PetState {
    pub const ALL: [PetState; 11] = [
        PetState::Idle,
        PetState::Working,
        PetState::Active,
        PetState::Thinking,
        PetState::Happy,
        PetState::Surprised,
        PetState::Sleepy,
        PetState::Oops,
        PetState::Error,
        PetState::Angry,
        PetState::Scared,
    ];

    /// Asset subdirectory name (assets/frames/<dir_name>/) — also the per-state
    /// frame-pack folder name in a theme.
    pub fn dir_name(self) -> &'static str {
        match self {
            PetState::Idle => "idle",
            PetState::Working => "working",
            PetState::Active => "active",
            PetState::Thinking => "thinking",
            PetState::Happy => "happy",
            PetState::Surprised => "surprised",
            PetState::Sleepy => "sleepy",
            PetState::Oops => "oops",
            PetState::Error => "error",
            PetState::Angry => "angry",
            PetState::Scared => "scared",
        }
    }

    pub fn label(self) -> &'static str {
        self.dir_name()
    }

    /// Parse a mood / dir_name string back into a PetState.
    pub fn from_dir_name(s: &str) -> Option<PetState> {
        PetState::ALL.into_iter().find(|st| st.dir_name() == s)
    }

    /// One-shot reactions play once, then the machine returns to Idle. Sustained
    /// states loop until something changes them.
    pub fn is_oneshot(self) -> bool {
        matches!(self, PetState::Surprised | PetState::Oops)
    }
}

/// Inactivity (ms) in Idle before drifting to Sleepy.
const SLEEP_AFTER_MS: u64 = 8_000;

/// Drives PetState over time. Holds the "intended" state; the Player renders it.
pub struct Machine {
    state: PetState,
    in_state_ms: u64,
}

impl Default for Machine {
    fn default() -> Self {
        Self { state: PetState::Idle, in_state_ms: 0 }
    }
}

impl Machine {
    pub fn state(&self) -> PetState {
        self.state
    }

    fn transition(&mut self, to: PetState) {
        if self.state != to {
            self.state = to;
            self.in_state_ms = 0;
        }
    }

    /// External activity (Claude working / a tool firing).
    pub fn nudge_active(&mut self) {
        self.transition(PetState::Working);
    }

    /// A one-shot pleased reaction.
    pub fn celebrate(&mut self) {
        self.transition(PetState::Happy);
    }

    /// Go back to resting.
    pub fn to_idle(&mut self) {
        self.transition(PetState::Idle);
    }

    /// Force a specific state (manual key control during Phase 1 testing; also the
    /// generic entry point Phase-3 hooks use after mapping an event→state).
    pub fn force(&mut self, to: PetState) {
        self.transition(to);
    }

    /// Reset the time-in-state clock without changing state. Called on every
    /// incoming event so repeated same-mood emits (e.g. back-to-back PreToolUse)
    /// keep the pet "awake" in that mood instead of decaying.
    pub fn refresh(&mut self) {
        self.in_state_ms = 0;
    }

    /// Apply an event-driven mood: switch to it (if different) and refresh the
    /// activity clock either way.
    pub fn apply(&mut self, to: PetState) {
        self.transition(to);
        self.in_state_ms = 0;
    }

    /// Advance time. `current_finished` = the Player's `finished()` flag, so a
    /// one-shot reaction returns to Idle when its animation ends.
    pub fn tick(&mut self, dt_ms: u64, current_finished: bool) {
        self.in_state_ms += dt_ms;
        if self.state.is_oneshot() && current_finished {
            self.transition(PetState::Idle);
            return;
        }
        if self.state == PetState::Idle && self.in_state_ms >= SLEEP_AFTER_MS {
            self.transition(PetState::Sleepy);
        }
    }
}
