//! The game state the renderer draws. Changes enter through `GameEvent` (demo, and later
//! transcripts) or `party::sync` (live herdr agents), so every source drives the same code.

use std::collections::{HashMap, HashSet};

use super::boss::{self, BossKind, Tier};
use super::hero::{Action, Class, Hero, QuestDone, Seat, Status};
use super::town::Town;

#[derive(Debug, Clone, PartialEq)]
pub enum GameEvent {
    Status(usize, Status),
    Act(usize, Action, Option<String>),
    /// The hero's current work title (names its quest).
    Title(usize, String),
    /// Hero `i` ran a boss command (kind from the classifier, the command text).
    BossFight(usize, BossKind, String),
    /// Hero `i`'s boss command finished; `true` = it failed.
    BossResult(usize, bool),
    Xp(usize, u64),
    ContextUsed(usize, Option<u8>),
    /// Compaction: Rest scene; context percent afterwards if known.
    Rest(usize, Option<u8>),
    /// A new session / `/clear`: new Chapter.
    NewRun(usize),
}

/// How long the "Chapter N · New Run" banner stays up.
pub const BANNER_MS: u64 = 3_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossState {
    Idle,
    Laugh,
    Dying,
}

#[derive(Debug, Clone)]
pub struct Boss {
    pub kind: BossKind,
    /// Name of the hero fighting it (gets the XP).
    pub owner: String,
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
/// How long a departing hero takes to walk off-screen.
pub const LEAVE_MS: u64 = 1500;

impl Boss {
    pub fn new(kind: BossKind, command: &str, owner: &str) -> Self {
        let s = boss::species(kind, command);
        Boss {
            kind,
            owner: owner.to_string(),
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

/// A quest that just ended, waiting to be written to the quest log.
#[derive(Debug, Clone)]
pub struct Finished {
    pub hero: String,
    pub seat: Option<Seat>,
    pub quest: QuestDone,
}

/// One row of the quest log screen (SPEC §5.6).
#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    /// Repo key; `None` for demo heroes.
    pub repo: Option<String>,
    pub hero: String,
    pub title: String,
    pub duration_ms: u64,
    pub xp: u64,
    pub result: String,
    pub bosses: String,
    pub chapter: u32,
}

impl From<&Finished> for LogEntry {
    fn from(f: &Finished) -> Self {
        LogEntry {
            repo: f.seat.as_ref().map(|s| s.repo.key.clone()),
            hero: f.hero.clone(),
            title: f.quest.title.clone(),
            duration_ms: f.quest.duration_ms,
            xp: f.quest.xp,
            result: f.quest.result.to_string(),
            bosses: f.quest.bosses.clone(),
            chapter: f.quest.chapter,
        }
    }
}

/// Repo key for demo heroes (no seat).
pub const DEMO_REPO: &str = "demo";

#[derive(Debug, Clone)]
pub struct World {
    pub repo: String,
    pub theme: String,
    pub heroes: Vec<Hero>,
    pub boss: Option<Boss>,
    /// Total game time; drives ambient animation (slimes, ground, stars).
    pub clock_ms: u64,
    /// Shown on the quest line, e.g. "reconnecting to herdr…".
    pub banner: Option<String>,
    /// Quests finished since the caller last drained this.
    pub finished: Vec<Finished>,
    /// Progress changed in a way worth saving right away (level-up, quest end).
    pub save_now: bool,
    /// herdr toasts to send (drained by the live loop).
    pub toasts: Vec<String>,
    /// Big centered banner (text, ms left), e.g. "Chapter 3 · New Run".
    pub chapter_banner: Option<(String, u64)>,
    /// Config `reduced_motion`: no dashes, hops, sway, or shake.
    pub reduced_motion: bool,
    /// Key of the repo shown in the top bar (most heroes).
    pub repo_key: String,
    /// Town per repo key, from `git` (off the render thread).
    pub towns: HashMap<String, Town>,
    /// Finished quests, newest last (loaded from disk per repo + this session's).
    pub log: Vec<LogEntry>,
    /// Repos whose saved quest log is already in `log`.
    pub logs_loaded: HashSet<String>,
}

impl World {
    pub fn new(repo: &str, theme: &str) -> Self {
        World {
            repo: repo.to_string(),
            theme: theme.to_string(),
            heroes: Vec::new(),
            boss: None,
            clock_ms: 0,
            banner: None,
            finished: Vec::new(),
            save_now: false,
            toasts: Vec::new(),
            chapter_banner: None,
            reduced_motion: false,
            repo_key: DEMO_REPO.into(),
            towns: HashMap::new(),
            log: Vec::new(),
            logs_loaded: HashSet::new(),
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

    /// Move newly finished quests into the log; returns them for the caller to persist.
    pub fn drain_finished(&mut self) -> Vec<Finished> {
        let done = std::mem::take(&mut self.finished);
        self.log.extend(done.iter().map(LogEntry::from));
        done
    }

    /// The repo key a hero belongs to.
    pub fn repo_of(h: &Hero) -> String {
        h.seat
            .as_ref()
            .map_or(DEMO_REPO.into(), |s| s.repo.key.clone())
    }

    /// The hero whose quest the HUD shows: focused first, else most recently active.
    pub fn quest_hero(&self) -> Option<&Hero> {
        let with_quest = || self.heroes.iter().filter(|h| h.quest.is_some());
        with_quest()
            .find(|h| h.focused)
            .or_else(|| with_quest().max_by_key(|h| h.last_active_ms))
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
            GameEvent::Title(i, t) => {
                if let Some(h) = self.heroes.get_mut(i) {
                    h.title = Some(t);
                }
            }
            GameEvent::BossFight(i, kind, cmd) => {
                let Some(h) = self.heroes.get(i) else { return };
                // The same kind of boss stays up between attempts, so failures add up
                // and its HP drops; a different kind replaces it.
                match &mut self.boss {
                    Some(b) if b.kind == kind && b.state != BossState::Dying => {
                        b.owner.clone_from(&h.name);
                    }
                    _ => self.boss = Some(Boss::new(kind, &cmd, &h.name)),
                }
            }
            GameEvent::BossResult(i, failed) => {
                let Some(b) = self.boss.as_mut().filter(|b| b.state != BossState::Dying) else {
                    return;
                };
                let Some(h) = self.heroes.get_mut(i) else {
                    return;
                };
                if let Some(q) = &mut h.quest {
                    q.bosses.push(if failed { '✗' } else { '✓' });
                }
                if failed {
                    b.fail();
                    h.life.bosses_lost += 1;
                    h.life.streak = 0;
                    h.say(format!("{} laughs!", b.name), 'r');
                } else {
                    let xp = boss::victory_xp(b.fails);
                    let bonus = match b.fails {
                        0 => " first try!",
                        1 => "",
                        _ => " persistence!",
                    };
                    b.defeat();
                    h.life.bosses_won += 1;
                    h.life.streak += 1;
                    h.life.best_streak = h.life.best_streak.max(h.life.streak);
                    if b.fails == 0 {
                        h.life.first_tries += 1;
                    }
                    h.say(format!("+{xp} XP{bonus}"), 'y');
                    if h.gain_xp(xp) > 0 {
                        self.save_now = true;
                    }
                }
            }
            GameEvent::Xp(i, n) => {
                if let Some(h) = self.heroes.get_mut(i)
                    && h.gain_xp(n) > 0
                {
                    self.save_now = true;
                }
            }
            GameEvent::ContextUsed(i, pct) => {
                if let Some(h) = self.heroes.get_mut(i) {
                    h.context_used = pct.map(|p| p.min(100));
                }
            }
            GameEvent::Rest(i, pct) => {
                if let Some(h) = self.heroes.get_mut(i) {
                    h.rest(pct);
                    h.say(format!("+{} XP · VIT up", super::hero::REST_XP), 'C');
                    self.save_now = true;
                }
            }
            GameEvent::NewRun(i) => {
                if let Some(h) = self.heroes.get_mut(i) {
                    if let Some(quest) = h.finish_quest() {
                        self.finished.push(Finished {
                            hero: h.name.clone(),
                            seat: h.seat.clone(),
                            quest,
                        });
                    }
                    h.new_run();
                    let text = format!("Chapter {} · New Run", h.life.chapters + 1);
                    self.chapter_banner = Some((format!("{} — {text}", h.name), BANNER_MS));
                    self.save_now = true;
                }
            }
        }
    }

    pub fn update(&mut self, dt: u64) {
        self.clock_ms += dt;
        if let Some((_, left)) = &mut self.chapter_banner {
            *left = left.saturating_sub(dt);
            if *left == 0 {
                self.chapter_banner = None;
            }
        }
        for h in &mut self.heroes {
            // One "running out of stamina" toast per quest (SPEC §5.4).
            if h.fatigue() == super::hero::Fatigue::Exhausted
                && let Some(q) = &mut h.quest
                && !q.exhausted_toast
            {
                q.exhausted_toast = true;
                self.toasts
                    .push(format!("{} is running out of stamina", h.name));
            }
            let level = h.level;
            if let Some(quest) = h.update(dt) {
                self.finished.push(Finished {
                    hero: h.name.clone(),
                    seat: h.seat.clone(),
                    quest,
                });
                self.save_now = true;
            }
            if h.level > level {
                self.save_now = true;
            }
            if h.status == Status::Working {
                h.last_active_ms = self.clock_ms;
            }
        }
        self.heroes
            .retain(|h| h.leaving_ms.is_none_or(|ms| ms < LEAVE_MS));
        if let Some(b) = &mut self.boss
            && !b.update(dt)
        {
            self.boss = None;
        }
    }

    /// Anything animating beyond idle breathing? Drives adaptive FPS (SPEC §8.5).
    pub fn is_busy(&self) -> bool {
        self.boss.is_some()
            || self.chapter_banner.is_some()
            || self.heroes.iter().any(|h| {
                h.status != Status::Idle
                    || h.busy()
                    || h.level_up_ms > 0
                    || h.leaving_ms.is_some()
                    || h.rest_ms > 0
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fight(w: &mut World, i: usize, failed: bool) {
        w.apply(GameEvent::BossFight(i, BossKind::Test, "cargo test".into()));
        w.apply(GameEvent::BossResult(i, failed));
    }

    #[test]
    fn boss_fails_never_kill_and_defeat_finishes() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        for _ in 0..5 {
            fight(&mut w, a, true);
        }
        let b = w.boss.as_ref().unwrap();
        assert_eq!((b.hp, b.fails, b.state), (25, 5, BossState::Laugh));
        w.update(LAUGH_MS);
        assert_eq!(w.boss.as_ref().unwrap().state, BossState::Idle);
        fight(&mut w, a, false);
        assert_eq!(w.boss.as_ref().unwrap().hp, 0);
        w.update(DYING_MS);
        assert!(w.boss.is_none());
    }

    #[test]
    fn m5_fail_then_pass_pays_the_right_xp() {
        // "cargo test failing then passing": laugh, -25% HP, then dies; 40 XP, no bonus.
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        fight(&mut w, a, true);
        assert_eq!(w.boss.as_ref().unwrap().hp, 75);
        assert_eq!(w.boss.as_ref().unwrap().state, BossState::Laugh);
        fight(&mut w, a, false);
        assert_eq!(w.heroes[a].xp, 40);
        assert_eq!(w.heroes[a].float.as_ref().unwrap().0, "+40 XP");
    }

    #[test]
    fn first_try_and_persistence_bonuses() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        fight(&mut w, a, false);
        assert_eq!(w.heroes[a].xp, 70, "first try");
        w.update(DYING_MS);
        let before = (w.heroes[a].level, w.heroes[a].xp);
        fight(&mut w, a, true);
        fight(&mut w, a, true);
        fight(&mut w, a, false);
        let gained = crate::game::xp::xp_to_next(before.0) * (w.heroes[a].level - before.0) as u64
            + w.heroes[a].xp
            - before.1;
        assert_eq!(gained, 60, "40 + persistence 20");
    }

    #[test]
    fn a_different_kind_replaces_the_boss() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        fight(&mut w, a, true);
        w.apply(GameEvent::BossFight(
            a,
            BossKind::Lint,
            "cargo clippy".into(),
        ));
        let b = w.boss.as_ref().unwrap();
        assert_eq!((b.kind, b.hp, b.fails), (BossKind::Lint, 100, 0));
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

    #[test]
    fn hud_quest_prefers_focused_then_recent() {
        let mut w = World::new("r", "t");
        let (a, b) = (w.add_hero("A", Class::Knight), w.add_hero("B", Class::Mage));
        w.apply(GameEvent::Status(b, Status::Working));
        w.update(10);
        w.apply(GameEvent::Status(a, Status::Working));
        w.apply(GameEvent::Status(b, Status::Idle));
        w.update(10);
        assert_eq!(w.quest_hero().unwrap().name, "A", "A worked most recently");
        w.heroes[b].focused = true;
        assert_eq!(w.quest_hero().unwrap().name, "B");
    }

    #[test]
    fn finished_quests_are_collected_and_flag_a_save() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        w.apply(GameEvent::Status(a, Status::Working));
        w.update(1);
        w.apply(GameEvent::Status(a, Status::Idle));
        w.update(crate::game::hero::QUEST_END_IDLE_MS);
        assert_eq!(w.finished.len(), 1);
        assert!(w.save_now);
    }

    #[test]
    fn exhausted_toast_once_per_quest() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        w.apply(GameEvent::Status(a, Status::Working));
        w.update(1);
        w.apply(GameEvent::ContextUsed(a, Some(95)));
        w.update(1);
        w.update(1);
        assert_eq!(w.toasts, ["A is running out of stamina"]);
    }

    #[test]
    fn new_run_banner_and_rest() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        w.apply(GameEvent::NewRun(a));
        assert_eq!(
            w.chapter_banner.as_ref().unwrap().0,
            "A — Chapter 2 · New Run"
        );
        w.update(BANNER_MS);
        assert!(w.chapter_banner.is_none());
        w.apply(GameEvent::Rest(a, Some(10)));
        assert!(w.is_busy());
        assert_eq!(w.heroes[a].life.rests, 1);
    }

    #[test]
    fn finished_quests_land_in_the_log_with_bosses_and_chapter() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        w.apply(GameEvent::Status(a, Status::Working));
        w.update(1);
        fight(&mut w, a, true);
        fight(&mut w, a, false);
        w.apply(GameEvent::Status(a, Status::Idle));
        w.update(crate::game::hero::QUEST_END_IDLE_MS);
        let done = w.drain_finished();
        assert_eq!(done.len(), 1);
        assert_eq!(w.log.len(), 1);
        assert_eq!((w.log[0].bosses.as_str(), w.log[0].chapter), ("✗✓", 1));
        assert_eq!(w.heroes[a].life.best_streak, 1);
    }

    #[test]
    fn boss_record_counts() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        fight(&mut w, a, true);
        fight(&mut w, a, false);
        let l = &w.heroes[a].life;
        assert_eq!((l.bosses_won, l.bosses_lost, l.first_tries), (1, 1, 0));
    }

    #[test]
    fn leaving_heroes_are_removed_after_walking_off() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        w.heroes[a].leaving_ms = Some(0);
        w.update(LEAVE_MS - 1);
        assert_eq!(w.heroes.len(), 1);
        w.update(1);
        assert!(w.heroes.is_empty());
    }
}
