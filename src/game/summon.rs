//! Summons (SPEC §6.5): a subagent becomes a small Joke Class character that rises from
//! shadow beside the hero who launched it and sinks back when it's done. Pure.

/// The 12 Joke Classes: (sprite id `joke_<id>`, name, entrance quip).
pub const JOKES: [(&str, &str, &str); 12] = [
    ("tenx", "10x Engineer", "I'll rewrite it in a weekend."),
    ("jester", "Jester", "Chaos? I prefer 'emergent behavior'."),
    ("night_owl", "Night Owl", "I only debug after midnight."),
    ("intern", "Intern", "Coffee's here! Also I pushed to main?"),
    (
        "pilgrim",
        "Stack Overflow Pilgrim",
        "Closed as duplicate. Again.",
    ),
    ("vibe", "Vibe Coder", "Trust me, it feels right."),
    ("senior", "Senior Dev", "It depends."),
    ("yaml", "YAML Wizard", "Mind the indentation."),
    ("pm", "Product Manager", "Quick sync about scope?"),
    ("paladin", "On-Call Paladin", "*pager noises*"),
    ("cultist", "Rubber Duck Cultist", "Praise the Duck."),
    (
        "twins",
        "Merge Conflict Twins",
        "<<<<<<< mine >>>>>>> yours",
    ),
];

pub const RISE_MS: u64 = 600;
pub const SINK_MS: u64 = 600;
/// Background subagents return at once; let them stand around a while instead.
pub const BACKGROUND_MS: u64 = 60_000;
/// An async subagent we can track leaves when it hands back; this is only a safety net.
pub const TRACKED_MS: u64 = 30 * 60_000;
/// Summon XP for the hero: 1 per 10 s it worked, capped per summon.
const XP_CAP: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Rising,
    Active,
    Sinking,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Summon {
    /// The `Agent` tool call id that summoned it.
    pub tool_id: String,
    /// Index into `JOKES`.
    pub joke: usize,
    pub phase: Phase,
    /// Time in the current phase.
    pub phase_ms: u64,
    /// Total time alive (XP, background expiry).
    pub alive_ms: u64,
    pub background: bool,
    /// Async subagent id, once its launch result named it; it leaves on hand-back.
    pub agent: Option<String>,
}

/// Stable across runs and platforms (same id → same costume).
fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

impl Summon {
    pub fn new(tool_id: &str, background: bool) -> Self {
        Summon {
            tool_id: tool_id.to_string(),
            joke: (fnv1a(tool_id) % JOKES.len() as u64) as usize,
            phase: Phase::Rising,
            phase_ms: 0,
            alive_ms: 0,
            background,
            agent: None,
        }
    }

    pub fn id(&self) -> &'static str {
        JOKES[self.joke].0
    }

    pub fn quip(&self) -> &'static str {
        JOKES[self.joke].2
    }

    /// The subagent finished: start sinking.
    pub fn dismiss(&mut self) {
        if self.phase != Phase::Sinking {
            self.phase = Phase::Sinking;
            self.phase_ms = 0;
        }
    }

    fn max_ms(&self) -> u64 {
        if self.agent.is_some() {
            TRACKED_MS
        } else {
            BACKGROUND_MS
        }
    }

    /// XP it earned for its hero so far.
    pub fn xp(&self) -> u64 {
        (self.alive_ms / 10_000).min(XP_CAP)
    }

    /// 0.0 → 1.0 how much of the body is out of the shadow.
    pub fn emerged(&self) -> f32 {
        match self.phase {
            Phase::Rising => (self.phase_ms as f32 / RISE_MS as f32).min(1.0),
            Phase::Active => 1.0,
            Phase::Sinking => 1.0 - (self.phase_ms as f32 / SINK_MS as f32).min(1.0),
        }
    }

    /// Advance; returns false once it has fully sunk.
    pub fn update(&mut self, dt: u64) -> bool {
        self.phase_ms += dt;
        self.alive_ms += dt;
        match self.phase {
            Phase::Rising if self.phase_ms >= RISE_MS => {
                self.phase = Phase::Active;
                self.phase_ms = 0;
            }
            Phase::Active if self.background && self.alive_ms >= self.max_ms() => self.dismiss(),
            Phase::Sinking if self.phase_ms >= SINK_MS => return false,
            _ => {}
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_tool_id_same_joke_class() {
        assert_eq!(
            Summon::new("toolu_abc", false).joke,
            Summon::new("toolu_abc", false).joke
        );
        let all: std::collections::HashSet<_> = (0..300)
            .map(|i| Summon::new(&format!("toolu_{i}"), false).joke)
            .collect();
        assert_eq!(all.len(), JOKES.len(), "every Joke Class shows up");
    }

    #[test]
    fn rises_works_and_sinks() {
        let mut s = Summon::new("t", false);
        assert_eq!(s.emerged(), 0.0);
        s.update(RISE_MS / 2);
        assert!((s.emerged() - 0.5).abs() < 0.01);
        s.update(RISE_MS);
        assert_eq!(s.phase, Phase::Active);
        s.update(25_000);
        assert_eq!(s.xp(), 2);
        s.dismiss();
        assert!(s.update(SINK_MS - 1));
        assert!(!s.update(1), "gone after sinking");
    }

    #[test]
    fn background_summons_leave_on_their_own_and_xp_caps() {
        let mut s = Summon::new("bg", true);
        s.update(RISE_MS);
        s.update(BACKGROUND_MS);
        assert_eq!(s.phase, Phase::Sinking);
        let mut long = Summon::new("long", false);
        long.update(3_600_000);
        assert_eq!(long.xp(), XP_CAP);
    }
}
