//! The Rubber Duck and Quips (SPEC §6.6). Each Quip fires on a real event; the Duck says
//! at most one every 2 minutes, and `memes = false` silences everything. Pure.

/// Real events that earn a joke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quip {
    /// A boss survived 2+ attempts in a row.
    StubbornBoss,
    /// A hero has been blocked for 5+ minutes.
    LongBlock,
    /// A quest has run 30+ minutes.
    LongQuest,
    /// A boss finally fell after 3+ failures.
    Persistence,
    /// `git push --force` / `-f`.
    ForcePush,
    /// Compaction with 95%+ of the context used.
    FullRest,
}

pub const RATE_MS: u64 = 120_000;
pub const SAY_MS: u64 = 5_000;
pub const LONG_BLOCK_MS: u64 = 5 * 60_000;
pub const LONG_QUEST_MS: u64 = 30 * 60_000;
/// The Sahur Knocker's laugh (shown on the boss, not the Duck).
pub const SAHUR_LAUGH: &str = "TUNG TUNG TUNG!";

/// Original, text-only lines (no quotes from copyrighted works).
pub fn lines(q: Quip) -> &'static [&'static str] {
    match q {
        Quip::StubbornBoss => &[
            "Quack. Explain it to me, line by line.",
            "Have you tried reading the error?",
            "Quack. It's always the semicolon.",
        ],
        Quip::LongBlock => &[
            "Quack? Someone is waiting on you.",
            "The agent has a question, human.",
        ],
        Quip::LongQuest => &["This quest is a saga now.", "Quack. Maybe smaller quests?"],
        Quip::Persistence => &["It works on my machine."],
        Quip::ForcePush => &["No fear."],
        Quip::FullRest => &["I have no memory of this place."],
    }
}

/// Pick a line deterministically (so tests and demos are stable) but varied.
pub fn line(q: Quip, salt: u64) -> &'static str {
    let l = lines(q);
    l[(salt % l.len() as u64) as usize]
}

pub fn is_force_push(command: &str) -> bool {
    super::moves::commands(command).iter().any(|w| {
        w.len() >= 2
            && w[0] == "git"
            && w[1] == "push"
            && w.iter()
                .any(|a| a == "--force" || a == "-f" || a == "--force-with-lease")
    })
}

#[derive(Debug, Clone, Default)]
pub struct Duck {
    /// What the Duck is saying (text, ms left).
    pub says: Option<(String, u64)>,
    last_quip_at: Option<u64>,
    /// Quips already made for this occurrence (e.g. "block:Claude").
    fired: std::collections::HashSet<String>,
}

impl Duck {
    /// Try to quip about `q` for occurrence `key` at game time `now`. Each occurrence
    /// quips once; at most one quip per `RATE_MS`; nothing when memes are off.
    pub fn quip(&mut self, q: Quip, key: String, now: u64, memes: bool) -> bool {
        if !memes || self.fired.contains(&key) {
            return false;
        }
        if self.last_quip_at.is_some_and(|t| now < t + RATE_MS) {
            return false;
        }
        let said = line(q, now / 1000);
        tracing::info!("duck quips ({q:?}): {said}");
        self.says = Some((said.to_string(), SAY_MS));
        self.last_quip_at = Some(now);
        self.fired.insert(key);
        true
    }

    /// Forget a finished occurrence so it can quip again next time (e.g. unblocked).
    pub fn reset(&mut self, key: &str) {
        self.fired.remove(key);
    }

    pub fn update(&mut self, dt: u64) {
        if let Some((_, left)) = &mut self.says {
            *left = left.saturating_sub(dt);
            if *left == 0 {
                self.says = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limited_once_per_occurrence_and_silenced_by_memes() {
        let mut d = Duck::default();
        assert!(d.quip(Quip::StubbornBoss, "boss:1".into(), 0, true));
        assert!(d.says.is_some());
        assert!(
            !d.quip(Quip::LongBlock, "block:A".into(), 60_000, true),
            "2 min limit"
        );
        assert!(d.quip(Quip::LongBlock, "block:A".into(), RATE_MS, true));
        assert!(
            !d.quip(Quip::LongBlock, "block:A".into(), RATE_MS * 5, true),
            "once"
        );
        d.reset("block:A");
        assert!(d.quip(Quip::LongBlock, "block:A".into(), RATE_MS * 6, true));
        let mut quiet = Duck::default();
        assert!(!quiet.quip(Quip::StubbornBoss, "x".into(), 0, false));
        assert!(quiet.says.is_none());
    }

    #[test]
    fn says_fades() {
        let mut d = Duck::default();
        d.quip(Quip::ForcePush, "p".into(), 0, true);
        assert_eq!(d.says.as_ref().unwrap().0, "No fear.");
        d.update(SAY_MS);
        assert!(d.says.is_none());
    }

    #[test]
    fn force_push_detection() {
        assert!(is_force_push("git push --force origin main"));
        assert!(is_force_push("cd app && git push -f"));
        assert!(is_force_push("git push --force-with-lease"));
        assert!(!is_force_push("git push origin main"));
        assert!(!is_force_push("echo git push --force"));
    }
}
