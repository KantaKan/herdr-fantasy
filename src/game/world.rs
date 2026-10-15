//! The game state the renderer draws. Everything enters through `GameEvent`, so demo,
//! herdr (M2), and transcripts (M4) all drive the same code.

use super::boss::{self, BossKind, Tier};
use super::hero::{Action, Class, Hero, Status};

#[derive(Debug, Clone, PartialEq)]
pub enum GameEvent {
    Status(usize, Status),
    Act(usize, Action, Option<String>),
    QuestStart(String),
    QuestEnd,
    /// A boss command started (kind from the classifier, the command text).
    BossEnter(BossKind, String),
    /// The boss command failed: boss laughs and loses 25% HP.
    BossFail,
    /// The boss command succeeded.
    BossDefeat,
    Xp(usize, u64),
    ContextUsed(usize, Option<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossState {
    Idle,
    Laugh,
    Dying,
}

#[derive(Debug, Clone)]
pub struct Boss {
    pub name: &'static str,
    /// Sprite id; sheets are `boss_<id>_<tier>`.
    pub id: &'static str,
    pub tier: Tier,
    /// Cosmetic HP percent (SPEC §6.3).
    pub hp: u8,
    pub state: BossState,
    pub state_ms: u64,
    pub fails: u32,
}

const LAUGH_MS: u64 = 1200;
const DYING_MS: u64 = 1200;

impl Boss {
    pub fn new(kind: BossKind, command: &str) -> Self {
        let s = boss::species(kind, command);
        Boss {
            name: s.name,
            id: s.id,
            tier: boss::tier(command),
            hp: 100,
            state: BossState::Idle,
            state_ms: 0,
            fails: 0,
        }
    }

    fn set(&mut self, s: BossState) {
        self.state = s;
        self.state_ms = 0;
    }

    /// Failing knocks off 25% but never kills; only victory finishes the boss.
    pub fn fail(&mut self) {
        self.fails += 1;
        self.hp = self.hp.saturating_sub(25).max(25);
        self.set(BossState::Laugh);
    }

    pub fn defeat(&mut self) {
        self.hp = 0;
        self.set(BossState::Dying);
    }

    /// Advance timers; returns false once the death animation is over.
    fn update(&mut self, dt: u64) -> bool {
        self.state_ms += dt;
        match self.state {
            BossState::Laugh if self.state_ms >= LAUGH_MS => self.set(BossState::Idle),
            BossState::Dying if self.state_ms >= DYING_MS => return false,
            _ => {}
        }
        true
    }

    pub fn tag(&self) -> &'static str {
        match self.state {
            BossState::Idle => "idle",
            BossState::Laugh => "laugh",
            BossState::Dying => "die",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Quest {
    pub title: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone)]
pub struct World {
    pub repo: String,
    pub theme: String,
    pub heroes: Vec<Hero>,
    pub quest: Option<Quest>,
    pub boss: Option<Boss>,
    /// Total game time; drives ambient animation (slimes, ground).
    pub clock_ms: u64,
}

impl World {
    pub fn new(repo: &str, theme: &str) -> Self {
        World {
            repo: repo.to_string(),
            theme: theme.to_string(),
            heroes: Vec::new(),
            quest: None,
            boss: None,
            clock_ms: 0,
        }
    }

    pub fn add_hero(&mut self, name: &str, class: Class) -> usize {
        self.heroes.push(Hero::new(name, class));
        self.heroes.len() - 1
    }

    /// Mean hero level, rounded (SPEC §5.2). 0 with no heroes.
    pub fn party_level(&self) -> u32 {
        let n = self.heroes.len() as u32;
        if n == 0 {
            return 0;
        }
        let sum: u32 = self.heroes.iter().map(|h| h.level).sum();
        (sum + n / 2) / n
    }

    pub fn apply(&mut self, ev: GameEvent) {
        match ev {
            GameEvent::Status(i, s) => {
                if let Some(h) = self.heroes.get_mut(i) {
                    h.set_status(s);
                }
            }
            GameEvent::Act(i, a, detail) => {
                if let Some(h) = self.heroes.get_mut(i) {
                    h.act(a, detail);
                }
            }
            GameEvent::QuestStart(title) => {
                self.quest = Some(Quest {
                    title,
                    elapsed_ms: 0,
                })
            }
            GameEvent::QuestEnd => self.quest = None,
            GameEvent::BossEnter(kind, cmd) => self.boss = Some(Boss::new(kind, &cmd)),
            GameEvent::BossFail => {
                if let Some(b) = &mut self.boss {
                    b.fail();
                }
            }
            GameEvent::BossDefeat => {
                if let Some(b) = &mut self.boss {
                    b.defeat();
                }
            }
            GameEvent::Xp(i, n) => {
                if let Some(h) = self.heroes.get_mut(i) {
                    h.gain_xp(n);
                }
            }
            GameEvent::ContextUsed(i, pct) => {
                if let Some(h) = self.heroes.get_mut(i) {
                    h.context_used = pct.map(|p| p.min(100));
                }
            }
        }
    }

    pub fn update(&mut self, dt: u64) {
        self.clock_ms += dt;
        for h in &mut self.heroes {
            h.update(dt);
        }
        if let Some(q) = &mut self.quest {
            q.elapsed_ms += dt;
        }
        if let Some(b) = &mut self.boss
            && !b.update(dt)
        {
            self.boss = None;
        }
    }

    /// Anything animating beyond idle breathing? Drives adaptive FPS (SPEC §8.5).
    pub fn is_busy(&self) -> bool {
        self.boss.is_some()
            || self
                .heroes
                .iter()
                .any(|h| h.status != Status::Idle || h.busy())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_fails_never_kill_and_defeat_finishes() {
        let mut w = World::new("r", "t");
        w.apply(GameEvent::BossEnter(BossKind::Test, "cargo test".into()));
        for _ in 0..5 {
            w.apply(GameEvent::BossFail);
        }
        let b = w.boss.as_ref().unwrap();
        assert_eq!((b.hp, b.fails, b.state), (25, 5, BossState::Laugh));
        w.update(LAUGH_MS);
        assert_eq!(w.boss.as_ref().unwrap().state, BossState::Idle);
        w.apply(GameEvent::BossDefeat);
        assert_eq!(w.boss.as_ref().unwrap().hp, 0);
        w.update(DYING_MS);
        assert!(w.boss.is_none());
    }

    #[test]
    fn party_level_rounds_mean() {
        let mut w = World::new("r", "t");
        assert_eq!(w.party_level(), 0);
        let a = w.add_hero("A", Class::Knight);
        w.add_hero("B", Class::Mage);
        w.heroes[a].level = 2; // (2 + 1) / 2 = 1.5 → 2
        assert_eq!(w.party_level(), 2);
    }

    #[test]
    fn idle_party_is_not_busy() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        assert!(!w.is_busy());
        w.apply(GameEvent::Status(a, Status::Working));
        assert!(w.is_busy());
    }
}
