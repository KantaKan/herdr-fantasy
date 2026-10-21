//! The game state the renderer draws. Changes enter through `GameEvent` (demo, and later
//! transcripts) or `party::sync` (live herdr agents), so every source drives the same code.

use std::collections::{HashMap, HashSet, VecDeque};

use super::battle::{self, Messages, Mob};
use super::boss::{self, BossKind, Tier};
use super::duck::{self, Quip};
use super::hero::{Action, Class, Hero, QuestDone, Seat, Status};
use super::town::Town;
use super::{badges, daily, loot};

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
    /// Hero `i` launched a subagent (tool id, runs in background?).
    Summon(usize, String, bool),
    /// That subagent's result came back.
    Dismiss(usize, String),
    /// A quip-worthy moment (event, occurrence key).
    Quip(super::duck::Quip, String),
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
    /// Heroes in this fight, first one first. Two or more = a team fight (M13).
    pub fighters: Vec<String>,
    pub name: &'static str,
    /// Sprite id; sheets are `boss_<id>_<tier>`.
    pub id: &'static str,
    pub tier: Tier,
    /// Cosmetic HP percent (SPEC §6.3).
    pub hp: u8,
    pub state: BossState,
    pub state_ms: u64,
    pub fails: u32,
    /// Time left of the hit flash/shake after each result (SPEC §8.4).
    pub hit_ms: u64,
}

/// Boss hit flash + shake length.
pub const HIT_MS: u64 = 150;

const LAUGH_MS: u64 = 1200;
const DYING_MS: u64 = 1200;
/// The black wipe when a boss draws near.
pub const ENCOUNTER_MS: u64 = 600;
/// Party level-up fireworks length.
pub const FIREWORKS_MS: u64 = 3_000;

/// The party's title and color by level.
pub fn party_rank(level: u32) -> (&'static str, char) {
    match level {
        0..=2 => ("Rookies", 's'),
        3..=5 => ("Adventurers", 'G'),
        6..=9 => ("Guild Heroes", 'c'),
        10..=14 => ("Veterans", 'y'),
        15..=19 => ("Champions", 'o'),
        20..=29 => ("Legends", 'r'),
        _ => ("Mythic", 'p'),
    }
}

/// Auto-battle: one field fight (charge + about 3 swings), the gap before the next,
/// and the melee swing period (the renderer's attack rhythm).
const FIGHT_MS: u64 = 2_400;
const FIGHT_GAP_MS: u64 = 800;
pub const SWING_MS: u64 = 600;
/// A shot's flight time, the Cleric's heal sparkle, and the Bard's attack song.
pub const MISSILE_MS: u64 = 350;
pub const HEAL_MS: u64 = 900;
const BUFF_MS: u64 = 8_000;

/// A ranged attack flying from `hero` to the front enemy; `dmg` lands on arrival
/// (0 = for show, e.g. at a boss) and `line` is narrated with the hit.
#[derive(Debug, Clone)]
pub struct Shot {
    pub hero: String,
    pub kind: super::hero::Missile,
    pub t_ms: u64,
    pub dmg: u32,
    pub line: Option<String>,
}

/// Run to the boss, and back home.
const CHARGE_MS: f32 = 500.0;
const RETURN_MS: f32 = 800.0;
/// How long a departing hero takes to walk off-screen.
pub const LEAVE_MS: u64 = 1500;

impl Boss {
    pub fn new(kind: BossKind, command: &str, hero: &str) -> Self {
        let s = boss::species(kind, command);
        Boss {
            kind,
            fighters: vec![hero.to_string()],
            name: s.name,
            id: s.id,
            tier: boss::tier(command),
            hp: 100,
            state: BossState::Idle,
            state_ms: 0,
            fails: 0,
            hit_ms: 0,
        }
    }

    fn set(&mut self, s: BossState) {
        self.state = s;
        self.state_ms = 0;
        self.hit_ms = HIT_MS;
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
        self.hit_ms = self.hit_ms.saturating_sub(dt);
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

/// Show `text` in the headline banner now, or after the ones already waiting.
fn announce(now: &mut Option<(String, u64)>, queue: &mut VecDeque<String>, text: String) {
    if now.is_none() {
        *now = Some((text, BANNER_MS));
    } else if !queue.contains(&text) {
        queue.push_back(text);
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
    /// Day it ended (days since 1970, UTC; 0 = unknown), for the weekly recap.
    pub day: u32,
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
            day: 0,
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
    /// Big banner across the top (text, ms left): "Chapter 3 · New Run", a loot drop.
    pub headline: Option<(String, u64)>,
    /// Banners waiting for the current one to finish.
    pub headlines: VecDeque<String>,
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
    pub duck: super::duck::Duck,
    /// Config `memes`: Quips on/off.
    pub memes: bool,
    /// Today as days since 1970 (UTC), set by the frame loop; 0 = no daily quests.
    pub today: u32,
    /// Config `[pets]` (agent kind → pet) and `skins` (M14).
    pub pets: std::collections::BTreeMap<String, String>,
    /// Config `[classes]`, kept live so settings changes reach new panes too (M18).
    pub classes: std::collections::BTreeMap<String, String>,
    pub skins: bool,
    /// (hero count, party level) last tick: a level-up only counts with the same party.
    party_seen: (usize, u32),
    /// Party level-up fireworks time left.
    pub fireworks_ms: u64,
    /// The first-run welcome card is still up (closed by any key).
    pub welcome: bool,
    /// Dragon Quest message window and field enemies (M21).
    pub messages: Messages,
    pub mobs: Vec<Mob>,
    /// Boss-encounter wipe time left (M21).
    pub encounter_ms: u64,
    /// Shots in flight, the Bard's song (attack up) and the Cleric's heal sparkle (M22).
    pub missiles: Vec<Shot>,
    pub buff_ms: u64,
    pub heal_ms: u64,
    waves: u32,
    wave_gap_ms: u64,
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
            headline: None,
            headlines: VecDeque::new(),
            reduced_motion: false,
            repo_key: DEMO_REPO.into(),
            towns: HashMap::new(),
            log: Vec::new(),
            logs_loaded: HashSet::new(),
            duck: Default::default(),
            memes: true,
            today: 0,
            pets: Default::default(),
            classes: Default::default(),
            skins: true,
            party_seen: (0, 0),
            fireworks_ms: 0,
            welcome: true,
            messages: Messages::default(),
            mobs: Vec::new(),
            encounter_ms: 0,
            missiles: Vec::new(),
            buff_ms: 0,
            heal_ms: 0,
            waves: 0,
            // The first wave draws near on the first tick.
            wave_gap_ms: battle::WAVE_GAP_MS,
        }
    }

    pub fn add_hero(&mut self, name: &str, class: Class) -> usize {
        self.heroes.push(Hero::new(name, class));
        self.heroes.len() - 1
    }

    /// Mean hero level counting XP into the next level, e.g. 9.4 (SPEC §5.2).
    pub fn party_exact(&self) -> f32 {
        let n = self.heroes.len();
        if n == 0 {
            return 0.0;
        }
        let sum: f32 = self
            .heroes
            .iter()
            .map(|h| h.level as f32 + h.xp as f32 / super::xp::xp_to_next(h.level).max(1) as f32)
            .sum();
        sum / n as f32
    }

    /// Party level (whole part of `party_exact`); 0 with no heroes.
    pub fn party_level(&self) -> u32 {
        self.party_exact() as u32
    }

    /// How far the party is to its next level, 0.0–1.0.
    pub fn party_progress(&self) -> f32 {
        self.party_exact().fract()
    }

    /// Move newly finished quests into the log; returns them for the caller to persist.
    pub fn drain_finished(&mut self) -> Vec<Finished> {
        let done = std::mem::take(&mut self.finished);
        let today = self.today;
        self.log.extend(done.iter().map(|f| LogEntry {
            day: today,
            ..LogEntry::from(f)
        }));
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
                // Demo edits hit for 4-20.
                let dmg = 4 + ((self.clock_ms / 66 + i as u64 * 7) % 17) as u32;
                self.on_action(i, a, Some(dmg));
            }
            GameEvent::Title(i, t) => {
                if let Some(h) = self.heroes.get_mut(i) {
                    h.title = Some(t);
                }
            }
            GameEvent::BossFight(i, kind, cmd) => {
                let Some(h) = self.heroes.get_mut(i) else {
                    return;
                };
                // The same kind of boss stays up between attempts, so failures add up
                // and its HP drops; a different kind replaces it. Another hero running
                // the same kind joins the fight.
                match &mut self.boss {
                    Some(b) if b.kind == kind && b.state != BossState::Dying => {
                        h.engaged = true;
                        if !b.fighters.contains(&h.name) {
                            b.fighters.push(h.name.clone());
                            h.say("joins the fight!".into(), 'c');
                            self.messages.say(format!("{} joins the fight!", h.name));
                        } else {
                            self.messages
                                .say(format!("{} attacks again: {cmd}!", h.name));
                        }
                    }
                    _ => {
                        h.engaged = true;
                        let b = Boss::new(kind, &cmd, &h.name);
                        self.messages.say(format!("The {} draws near!", b.name));
                        self.encounter_ms = ENCOUNTER_MS;
                        self.messages.say(format!("{} attacks with {cmd}!", h.name));
                        self.boss = Some(b);
                    }
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
                    // Knocked back: it runs home and tries again next command.
                    h.engaged = false;
                    b.fail();
                    h.life.bosses_lost += 1;
                    h.life.streak = 0;
                    if b.fails >= 2 {
                        let key = format!("boss:{}:{}", b.name, h.life.bosses_won);
                        self.duck
                            .quip(Quip::StubbornBoss, key, self.clock_ms, self.memes);
                    }
                    h.say(format!("{} laughs!", b.name), 'r');
                    self.messages
                        .say(format!("The {} shrugs it off and laughs!", b.name));
                } else {
                    let team: Vec<String> = b.fighters.clone();
                    let combo = team.len() >= 2;
                    // A team win pays everyone in the fight, plus half again.
                    let base = boss::victory_xp(b.fails);
                    let xp = if combo { base * 3 / 2 } else { base };
                    let bonus = match b.fails {
                        _ if combo => " team attack!",
                        0 => " first try!",
                        1 => "",
                        _ => " persistence!",
                    };
                    if b.fails >= 3 {
                        let key = format!("persist:{}:{}", h.name, h.life.bosses_won);
                        self.duck
                            .quip(Quip::Persistence, key, self.clock_ms, self.memes);
                    }
                    b.defeat();
                    h.life.bosses_won += 1;
                    h.life.streak += 1;
                    h.life.best_streak = h.life.best_streak.max(h.life.streak);
                    if b.fails == 0 {
                        h.life.first_tries += 1;
                    }
                    if b.fails >= 3 {
                        h.life.comebacks += 1;
                    }
                    if let Some(item) = loot::roll(&h.name, h.life.bosses_won, b.id, b.tier) {
                        tracing::info!("{} found {}", h.name, item.name);
                        let text = format!("{} found {}!", h.name, item.name);
                        self.messages.say(text.clone());
                        announce(&mut self.headline, &mut self.headlines, text);
                        h.life.loot.push(item);
                        self.save_now = true;
                    }
                    h.say(format!("+{xp} XP{bonus}"), 'y');
                    self.messages.say(format!("The {} is defeated!", b.name));
                    if combo {
                        self.messages
                            .say(format!("Team attack! Everyone gains {xp} XP."));
                    } else {
                        self.messages.say(format!("{} gains {xp} XP.", h.name));
                    }
                    if h.gain_xp(xp) > 0 {
                        self.save_now = true;
                    }
                    if combo {
                        tracing::info!("team attack: {}", team.join(" + "));
                        let text = format!("⚔ Team attack! {}", team.join(" + "));
                        announce(&mut self.headline, &mut self.headlines, text);
                        for (k, o) in self.heroes.iter_mut().enumerate() {
                            if k == i || !team.contains(&o.name) {
                                continue;
                            }
                            o.life.bosses_won += 1;
                            o.life.streak += 1;
                            o.life.best_streak = o.life.best_streak.max(o.life.streak);
                            o.act(Action::Attack, None);
                            o.say(format!("+{xp} XP team attack!"), 'y');
                            if o.gain_xp(xp) > 0 {
                                self.save_now = true;
                            }
                        }
                        if let Some(h) = self.heroes.get_mut(i) {
                            h.act(Action::Attack, None);
                        }
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
                    if h.context_used.is_some_and(|u| u >= 95) {
                        let key = format!("rest:{}:{}", h.name, h.life.rests);
                        self.duck
                            .quip(Quip::FullRest, key, self.clock_ms, self.memes);
                    }
                    h.rest(pct);
                    h.say(format!("+{} XP · VIT up", super::hero::REST_XP), 'C');
                    self.messages
                        .say(format!("{} rests by the fire... Stamina restored!", h.name));
                    self.save_now = true;
                }
            }
            GameEvent::Summon(i, id, background) => {
                if let Some(h) = self.heroes.get_mut(i) {
                    let s = super::summon::Summon::new(&id, background);
                    self.messages.say(format!(
                        "{} summons a {} from the shadows!",
                        h.name,
                        s.name()
                    ));
                    h.summons.push(s);
                    h.life.shadows += 1;
                }
            }
            GameEvent::Dismiss(i, id) => {
                if let Some(s) = self
                    .heroes
                    .get_mut(i)
                    .and_then(|h| h.summons.iter_mut().find(|s| s.tool_id == id))
                {
                    s.dismiss();
                }
            }
            GameEvent::Quip(q, key) => {
                self.duck.quip(q, key, self.clock_ms, self.memes);
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
                    let text = format!("{} — Chapter {} · New Run", h.name, h.life.chapters + 1);
                    self.messages.say(format!(
                        "{} begins Chapter {}!",
                        h.name,
                        h.life.chapters + 1
                    ));
                    announce(&mut self.headline, &mut self.headlines, text);
                    self.save_now = true;
                }
            }
        }
    }

    pub fn update(&mut self, dt: u64) {
        self.clock_ms += dt;
        if let Some((_, left)) = &mut self.headline {
            *left = left.saturating_sub(dt);
            if *left == 0 {
                self.headline = self.headlines.pop_front().map(|t| (t, BANNER_MS));
            }
        }
        self.fireworks_ms = self.fireworks_ms.saturating_sub(dt);
        self.messages.update(dt);
        self.encounter_ms = self.encounter_ms.saturating_sub(dt);
        for m in &mut self.mobs {
            m.hit_ms = m.hit_ms.saturating_sub(dt);
            if let Some(d) = &mut m.dying_ms {
                *d += dt;
            }
        }
        self.mobs
            .retain(|m| m.dying_ms.is_none_or(|d| d < battle::DIE_MS));
        if self.mobs.is_empty() {
            self.wave_gap_ms += dt;
            if self.wave_gap_ms >= battle::WAVE_GAP_MS {
                self.mobs = battle::wave(self.waves, &self.theme);
                self.waves += 1;
                self.wave_gap_ms = 0;
                if self.boss.is_none() && !self.heroes.is_empty() {
                    self.messages.say(battle::appear_line(&self.mobs));
                }
            }
        }
        for h in &mut self.heroes {
            if h.announced_level == 0 {
                h.announced_level = h.level;
            } else if h.level > h.announced_level {
                h.announced_level = h.level;
                self.messages
                    .say(format!("{} is now level {}!", h.name, h.level));
            }
        }
        let party = (self.heroes.len(), self.party_level());
        if party.0 == self.party_seen.0 && party.1 > self.party_seen.1 && party.0 > 0 {
            let text = format!("🎉 Party Lv {} — {}!", party.1, party_rank(party.1).0);
            tracing::info!("{text}");
            announce(&mut self.headline, &mut self.headlines, text);
            self.fireworks_ms = FIREWORKS_MS;
        }
        self.party_seen = party;
        for h in &mut self.heroes {
            if self.today > 0 && daily::tick(&mut h.life, &h.name, self.today) {
                let (name, ..) = daily::progress(&h.life);
                tracing::info!("{} finished the daily quest: {name}", h.name);
                h.gain_xp(daily::REWARD_XP);
                let text = format!(
                    "📜 {} — daily quest done: {name}! +{} XP",
                    h.name,
                    daily::REWARD_XP
                );
                announce(&mut self.headline, &mut self.headlines, text);
                self.save_now = true;
            }
            let earned = badges::newly_earned(&h.life, h.level);
            let Some(last) = earned.last() else { continue };
            tracing::info!("{} earned {}", h.name, last.name);
            // A veteran's first load can earn many at once: one banner, not sixteen.
            let text = match earned.len() {
                1 => format!("🏆 {}: {}!", h.name, last.name),
                n => format!("🏆 {} earned {n} badges!", h.name),
            };
            h.life
                .badges
                .extend(earned.iter().map(|b| b.id.to_string()));
            announce(&mut self.headline, &mut self.headlines, text);
            self.save_now = true;
        }
        self.duck.update(dt);
        for h in &self.heroes {
            // The Duck notices long waits and marathon quests (SPEC §6.6).
            let block = format!("block:{}", h.name);
            if h.blocked_ms >= duck::LONG_BLOCK_MS {
                self.duck
                    .quip(Quip::LongBlock, block, self.clock_ms, self.memes);
            } else if h.blocked_ms == 0 {
                self.duck.reset(&block);
            }
            if h.quest
                .as_ref()
                .is_some_and(|q| q.elapsed_ms >= duck::LONG_QUEST_MS)
            {
                let key = format!("quest:{}:{}", h.name, h.life.quests);
                self.duck
                    .quip(Quip::LongQuest, key, self.clock_ms, self.memes);
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
        self.auto_battle(dt);
        for h in &mut self.heroes {
            // A win keeps the fighters there for the death animation; then home.
            // Without a boss, a hero is engaged while it fights the field wave.
            if self.boss.is_none() {
                h.engaged = h.mob_fight_ms > 0;
            }
            h.charge = if h.engaged && !h.class.ranged() {
                (h.charge + dt as f32 / CHARGE_MS).min(1.0)
            } else {
                (h.charge - dt as f32 / RETURN_MS).max(0.0)
            };
        }
    }

    /// Anything animating beyond idle breathing? Drives adaptive FPS (SPEC §8.5).
    pub fn is_busy(&self) -> bool {
        self.boss.is_some()
            || self.headline.is_some()
            || self.heroes.iter().any(|h| {
                h.status != Status::Idle
                    || h.busy()
                    || h.level_up_ms > 0
                    || h.leaving_ms.is_some()
                    || h.rest_ms > 0
                    || h.charge > 0.0
            })
            || self.fireworks_ms > 0
            || self.messages.busy()
            || self.encounter_ms > 0
            || !self.missiles.is_empty()
            || self
                .mobs
                .iter()
                .any(|m| m.hit_ms > 0 || m.dying_ms.is_some())
    }

    /// Auto-battle (M21/M22): a working hero with nothing else to show fights the field
    /// wave. Melee classes charge and swing; ranged ones shoot from home and hit when the
    /// shot lands. Each class has its trait (see `swing`). Real actions and bosses come
    /// first; ranged boss fighters shoot at the boss (for show: the command decides).
    fn auto_battle(&mut self, dt: u64) {
        let standing = self.mobs.iter().any(|m| m.dying_ms.is_none());
        let boss_up = self.boss.is_some();
        let mut swings = Vec::new();
        for (k, h) in self.heroes.iter_mut().enumerate() {
            let seed = self.clock_ms / 97 + k as u64 * 7_919;
            let t = h.anim_ms % SWING_MS;
            let lands = t >= 80 && t < 80 + dt.min(SWING_MS);
            if boss_up {
                if h.engaged && h.class.ranged() && lands {
                    self.missiles.push(Shot {
                        hero: h.name.clone(),
                        kind: h.class.missile(),
                        t_ms: 0,
                        dmg: 0,
                        line: None,
                    });
                }
                h.mob_fight_ms = 0;
                continue;
            }
            let free = h.status == Status::Working
                && !h.busy()
                && h.leaving_ms.is_none()
                && h.rest_ms == 0;
            if !free || !standing {
                if h.mob_fight_ms > 0 {
                    h.mob_fight_ms = 0;
                    h.fight_cooldown_ms = FIGHT_GAP_MS;
                }
                continue;
            }
            if h.mob_fight_ms > 0 {
                h.mob_fight_ms = h.mob_fight_ms.saturating_sub(dt);
                // Melee lands on the renderer's strike frame once in reach; ranged
                // shoots on the same beat from home.
                if lands && (h.charge >= 1.0 || h.class.ranged()) {
                    swings.push((k, seed));
                }
                if h.mob_fight_ms == 0 {
                    h.fight_cooldown_ms = FIGHT_GAP_MS + seed % 1_200;
                }
            } else {
                h.fight_cooldown_ms = h.fight_cooldown_ms.saturating_sub(dt);
                if h.fight_cooldown_ms == 0 {
                    h.mob_fight_ms = FIGHT_MS;
                }
            }
        }
        for (k, seed) in swings {
            self.swing(k, seed);
        }
        // Shots in flight land, hit and narrate.
        let quiet = !self.messages.busy();
        for s in &mut self.missiles {
            s.t_ms += dt;
        }
        let (landed, flying): (Vec<Shot>, Vec<Shot>) =
            self.missiles.drain(..).partition(|s| s.t_ms >= MISSILE_MS);
        self.missiles = flying;
        for s in landed {
            if s.dmg == 0 {
                continue;
            }
            if let Some(hit) = battle::strike(&mut self.mobs, s.dmg)
                && quiet
                && let Some(line) = s.line
            {
                self.messages.say(format!("{line} {hit}"));
            }
        }
        self.buff_ms = self.buff_ms.saturating_sub(dt);
        self.heal_ms = self.heal_ms.saturating_sub(dt);
    }

    /// One swing (or shot) of hero `k` at the field wave, with its class trait.
    fn swing(&mut self, k: usize, seed: u64) {
        use super::hero::Class;
        let quiet = !self.messages.busy();
        let h = &self.heroes[k];
        let (name, class) = (h.name.clone(), h.class);
        let buff = if self.buff_ms > 0 { 2 } else { 0 };
        let base = 2 + h.level / 3 + (seed % 4) as u32 + buff;
        let roll = seed / 5 % 4;
        let say = |w: &mut World, line: String| {
            if quiet {
                w.messages.say(line);
            }
        };
        match class {
            Class::Cleric if roll == 0 => {
                say(self, format!("{name} casts Heal! The party feels better."));
                self.heal_ms = HEAL_MS;
                for h in &mut self.heroes {
                    h.say("+HP".into(), 'G');
                }
                return;
            }
            Class::Bard if roll == 0 => {
                say(
                    self,
                    format!("{name} sings a battle song! Everyone's attack rises!"),
                );
                self.buff_ms = BUFF_MS;
                for h in &mut self.heroes {
                    h.say("ATK UP".into(), 'y');
                }
                return;
            }
            _ => {}
        }
        let (dmg, line) = match class {
            Class::Rogue if roll == 1 => (base * 2, format!("{name} strikes twice! Critical hit!")),
            Class::SahurKnight if roll == 1 => {
                (base * 3 / 2, format!("{name}: TUNG TUNG TUNG! Smash!"))
            }
            Class::Mage => (base * 4 / 3, format!("{name} casts Fireball!")),
            Class::Ranger => (base, format!("{name} shoots an arrow!")),
            Class::Bard => (base, format!("{name} plays a sharp note!")),
            Class::WildCard => {
                let (mult, card) = [
                    (1, "the Two of Clubs"),
                    (2, "the Ace of Hearts"),
                    (3, "the Joker"),
                    (2, "the King of Spades"),
                ][roll as usize];
                let dmg = if card == "the Two of Clubs" {
                    base / 2
                } else {
                    base * mult / 2
                };
                let tag = if mult == 3 { " Critical!" } else { "" };
                (dmg.max(1), format!("{name} slashes with {card}!{tag}"))
            }
            _ => (base, format!("{name} attacks!")),
        };
        if class.ranged() {
            self.missiles.push(Shot {
                hero: name.clone(),
                kind: class.missile(),
                t_ms: 0,
                dmg,
                line: Some(line),
            });
        } else if let Some(hit) = battle::strike(&mut self.mobs, dmg) {
            say(self, format!("{line} {hit}"));
        }
        // One swing in four the monster strikes back (cosmetic: stamina stays real).
        if seed.is_multiple_of(4)
            && let Some(m) = self.mobs.iter().find(|m| m.dying_ms.is_none())
        {
            let mob = m.kind.name();
            if class == Class::Knight {
                say(
                    self,
                    format!("The {mob} attacks! {name} blocks with the shield!"),
                );
                self.heroes[k].say("BLOCK".into(), 's');
            } else {
                let hit = 1 + (seed / 4 % 3) as u32;
                say(
                    self,
                    format!("The {mob} attacks! {name} takes {hit} damage."),
                );
                self.heroes[k].say(format!("-{hit}"), 'r');
            }
        }
    }

    /// Narrate a hero's action in the message window; an edit also hits the first enemy
    /// standing (the boss when there is one, else the field wave) for `damage`.
    pub fn on_action(&mut self, i: usize, action: Action, damage: Option<u32>) {
        let Some(h) = self.heroes.get(i) else { return };
        if h.status != Status::Working {
            return;
        }
        let line = battle::action_line(&h.name, action, h.detail.as_deref());
        if action != Action::Attack {
            self.messages.say(line);
            return;
        }
        // An edit is one line with what it hit: "Claude attacks! The Slime takes 12 damage!"
        let dmg = damage.unwrap_or(8);
        let hit = match &self.boss {
            Some(b) => Some(format!("The {} takes {dmg} damage!", b.name)),
            None => battle::strike(&mut self.mobs, dmg),
        };
        match hit {
            Some(hit) => self.messages.say(format!("{line} {hit}")),
            None => self.messages.say(line),
        }
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
    fn party_level_is_the_mean_with_progress() {
        let mut w = World::new("r", "t");
        assert_eq!(w.party_level(), 0);
        let a = w.add_hero("A", Class::Knight);
        w.add_hero("B", Class::Mage);
        w.heroes[a].level = 2; // (2 + 1) / 2 = 1.5
        assert_eq!(w.party_level(), 1);
        assert!((w.party_progress() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn party_level_up_celebrates_but_a_newcomer_does_not() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        w.update(1);
        w.heroes[a].level = 2;
        w.update(1);
        assert_eq!(w.headline.as_ref().unwrap().0, "🎉 Party Lv 2 — Rookies!");
        assert!(w.fireworks_ms > 0);
        w.headline = None;
        w.headlines.clear();
        let b = w.add_hero("B", Class::Mage);
        w.heroes[b].level = 9; // the mean jumps because someone joined, not a level-up
        w.update(1);
        assert!(w.headline.is_none());
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
        assert_eq!(w.headline.as_ref().unwrap().0, "A — Chapter 2 · New Run");
        w.update(BANNER_MS);
        assert!(w.headline.is_none());
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
    fn summons_rise_sink_and_pay_their_hero() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        w.apply(GameEvent::Status(a, Status::Working));
        w.apply(GameEvent::Summon(a, "t1".into(), false));
        w.apply(GameEvent::Summon(a, "t2".into(), false));
        assert_eq!(
            (w.heroes[a].summons.len(), w.heroes[a].life.shadows),
            (2, 2)
        );
        w.update(20_000);
        w.apply(GameEvent::Dismiss(a, "t1".into()));
        w.update(crate::game::summon::SINK_MS);
        assert_eq!(w.heroes[a].summons.len(), 1);
        assert!(w.heroes[a].xp >= 2, "summon XP went to the hero");
    }

    #[test]
    fn duck_quips_on_a_stubborn_boss_unless_memes_are_off() {
        for memes in [true, false] {
            let mut w = World::new("r", "t");
            w.memes = memes;
            let a = w.add_hero("A", Class::Knight);
            fight(&mut w, a, true);
            assert!(w.duck.says.is_none(), "one failure is not stubborn");
            fight(&mut w, a, true);
            assert_eq!(w.duck.says.is_some(), memes);
        }
    }

    #[test]
    fn duck_notices_a_long_block() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        w.apply(GameEvent::Status(a, Status::Blocked));
        w.update(duck::LONG_BLOCK_MS);
        w.update(1);
        assert!(w.duck.says.is_some());
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
    fn first_boss_win_drops_loot_that_raises_a_stat() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        let before = w.heroes[a].life.stats();
        fight(&mut w, a, false);
        let h = &w.heroes[a];
        assert_eq!(h.life.loot.len(), 1);
        let item = &h.life.loot[0];
        assert!(w.headline.as_ref().unwrap().0.contains(&item.name));
        assert_eq!(h.life.stats()[item.stat], before[item.stat] + item.bonus);
    }

    #[test]
    fn badges_unlock_once_and_queue_behind_the_loot_banner() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        fight(&mut w, a, false);
        w.update(1);
        assert!(w.headline.as_ref().unwrap().0.contains("found"));
        w.update(BANNER_MS);
        assert_eq!(w.headline.as_ref().unwrap().0, "🏆 A earned 2 badges!");
        assert_eq!(w.heroes[a].life.badges, ["first_blood", "flawless"]);
        w.update(BANNER_MS);
        assert!(w.headline.is_none(), "nothing earned twice");
        for _ in 0..3 {
            fight(&mut w, a, true);
        }
        fight(&mut w, a, false);
        w.update(1);
        assert!(w.heroes[a].life.badges.contains(&"never_give_up".into()));
    }

    #[test]
    fn a_second_hero_joins_the_fight_and_both_get_team_xp() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        let b = w.add_hero("B", Class::Mage);
        w.apply(GameEvent::BossFight(a, BossKind::Test, "cargo test".into()));
        w.apply(GameEvent::BossFight(b, BossKind::Test, "cargo test".into()));
        assert_eq!(w.boss.as_ref().unwrap().fighters, ["A", "B"]);
        assert_eq!(w.heroes[b].float.as_ref().unwrap().0, "joins the fight!");
        w.apply(GameEvent::BossResult(b, false));
        let team = boss::victory_xp(0) * 3 / 2;
        for i in [a, b] {
            let h = &w.heroes[i];
            assert_eq!(h.life.bosses_won, 1);
            assert_eq!(
                h.float.as_ref().unwrap().0,
                format!("+{team} XP team attack!")
            );
        }
        let shown: Vec<_> = w
            .headline
            .iter()
            .map(|h| h.0.clone())
            .chain(w.headlines.clone())
            .collect();
        assert!(
            shown.iter().any(|t| t == "⚔ Team attack! A + B"),
            "{shown:?}"
        );
    }

    #[test]
    fn daily_quest_pays_once() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        w.today = 20_000;
        w.update(1);
        let (_, _, target) = daily::progress(&w.heroes[a].life);
        let l = &mut w.heroes[a].life;
        for _ in 0..target {
            match l.daily.kind {
                0 => l.bosses_won += 1,
                1 => l.quests += 1,
                2 => l.shadows += 1,
                3 => l.edits += 1,
                4 => l.reads += 1,
                _ => l.shells += 1,
            }
        }
        let xp = w.heroes[a].xp;
        w.update(1);
        assert!(w.heroes[a].life.daily.done);
        assert!(w.heroes[a].xp > xp || w.heroes[a].level > 1);
        let xp = (w.heroes[a].level, w.heroes[a].xp);
        w.update(1);
        assert_eq!((w.heroes[a].level, w.heroes[a].xp), xp, "paid once");
    }

    #[test]
    fn fighters_charge_in_get_knocked_back_and_go_home_after_the_kill() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        w.apply(GameEvent::BossFight(a, BossKind::Test, "cargo test".into()));
        w.update(CHARGE_MS as u64);
        assert_eq!(w.heroes[a].charge, 1.0);
        assert_eq!(
            w.heroes[a].anim(),
            crate::game::hero::Anim::Attack,
            "in melee"
        );
        w.apply(GameEvent::BossResult(a, true));
        w.update(RETURN_MS as u64);
        assert_eq!(w.heroes[a].charge, 0.0, "knocked back home");
        w.apply(GameEvent::BossFight(a, BossKind::Test, "cargo test".into()));
        w.update(CHARGE_MS as u64);
        w.apply(GameEvent::BossResult(a, false));
        w.update(DYING_MS - 1);
        assert!(w.heroes[a].engaged, "stays for the death animation");
        w.update(1); // boss gone
        w.update(RETURN_MS as u64);
        assert!(!w.heroes[a].engaged);
        assert_eq!(w.heroes[a].charge, 0.0);
    }

    #[test]
    fn battles_are_narrated_and_edits_hit_the_field_wave() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        w.apply(GameEvent::Status(a, Status::Working));
        w.update(1); // the first wave draws near
        assert!(!w.mobs.is_empty());
        assert!(
            w.messages.lines[0].contains("draw"),
            "{:?}",
            w.messages.lines
        );
        let hp = w.mobs[0].hp;
        w.apply(GameEvent::Act(
            a,
            Action::Attack,
            Some("editing src/x.rs".into()),
        ));
        assert!(w.mobs[0].hp < hp || w.mobs[0].dying_ms.is_some());
        w.apply(GameEvent::BossFight(a, BossKind::Test, "cargo test".into()));
        assert!(w.encounter_ms > 0);
        w.apply(GameEvent::BossResult(a, false));
        let mut said: Vec<String> = Vec::new();
        for _ in 0..40 {
            w.update(500);
            said.extend(w.messages.lines.iter().cloned());
        }
        assert!(said.iter().any(|l| l.contains("is defeated!")), "{said:?}");
        assert!(said.iter().any(|l| l.contains("A gains")), "{said:?}");
    }

    #[test]
    fn working_heroes_fight_the_field_wave_until_real_work_comes() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        w.apply(GameEvent::Status(a, Status::Working));
        w.update(1); // the wave draws near
        let start: u32 = w.mobs.iter().map(|m| m.hp).sum();
        for _ in 0..60 {
            w.update(50);
        }
        let h = &w.heroes[a];
        assert!(
            h.mob_fight_ms > 0 || h.fight_cooldown_ms > 0,
            "fought or resting"
        );
        let now: u32 = w.mobs.iter().map(|m| m.hp).sum();
        assert!(now < start || w.waves > 1, "the slimes took hits");
        // A real action ends the field fight; the hero runs back to show it.
        w.heroes[a].mob_fight_ms = FIGHT_MS;
        w.apply(GameEvent::Act(
            a,
            Action::Study,
            Some("reading the big book".into()),
        ));
        w.update(16);
        assert_eq!(w.heroes[a].mob_fight_ms, 0);
        assert!(!w.heroes[a].engaged);
    }

    #[test]
    fn every_class_fights_its_own_way() {
        use super::battle::{Mob, MobKind};
        let slime = || Mob {
            kind: MobKind::Slime,
            hp: 50,
            max: 50,
            hit_ms: 0,
            last_damage: 0,
            dying_ms: None,
        };
        let world = |class| {
            let mut w = World::new("r", "t");
            w.add_hero("H", class);
            w.mobs = vec![slime()];
            w
        };
        // seed 5: roll 1, no counterattack. Ranged: the shot hits on landing.
        let mut w = world(Class::Mage);
        w.swing(0, 5);
        assert_eq!(w.missiles.len(), 1);
        assert_eq!(w.mobs[0].hp, 50, "not yet");
        w.update(MISSILE_MS);
        assert!(w.mobs[0].hp < 50);
        // Rogue critical: double damage.
        let mut w = world(Class::Rogue);
        w.swing(0, 5);
        let base = 3; // 2 + level 1 / 3 + seed 5 % 4
        assert_eq!(w.mobs[0].hp, 50 - base * 2);
        // seed 0: roll 0 -> Cleric heals, Bard sings.
        let mut w = world(Class::Cleric);
        w.swing(0, 0);
        assert!(w.heal_ms > 0);
        assert_eq!(w.mobs[0].hp, 50);
        let mut w = world(Class::Bard);
        w.swing(0, 0);
        assert!(w.buff_ms > 0);
        // seed 4: roll 0, the slime strikes back -> the Knight blocks.
        let mut w = world(Class::Knight);
        w.swing(0, 4);
        assert_eq!(w.heroes[0].float.as_ref().unwrap().0, "BLOCK");
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
