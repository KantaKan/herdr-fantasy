//! Per-hero state machine (SPEC §5.4). herdr status picks the base animation; one-shot
//! actions play on top, one at a time, only while the hero is working.

use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};

use super::repo::Repo;
use super::summon::Summon;
use super::xp;

/// Quest ends after this long idle with no new work (SPEC §6.4).
pub const QUEST_END_IDLE_MS: u64 = 10_000;
pub const QUEST_COMPLETE_XP: u64 = 25;
const LEVEL_UP_MS: u64 = 2_000;
/// How long floating text (damage, "learned: …", combos) stays up.
pub const FLOAT_MS: u64 = 1_200;
/// More queued actions than this collapse into a combo (SPEC §5.4).
const MAX_QUEUE: usize = 6;
/// The Rest scene (compaction), then the Well Rested sparkle (SPEC §5.4).
pub const REST_MS: u64 = 4_000;
pub const RESTED_MS: u64 = 120_000;
pub const REST_XP: u64 = 10;
/// Token XP per quest is capped (SPEC §7.1).
const TOKEN_XP_CAP: u64 = 400;

/// Lifetime counters a hero keeps forever (saved per repo). Stats derive from them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Lifetime {
    pub quests: u32,
    pub chapters: u32,
    pub edits: u32,
    pub reads: u32,
    pub shells: u32,
    pub rests: u32,
    pub first_tries: u32,
    pub bosses_won: u32,
    pub bosses_lost: u32,
    /// Boss wins in a row (reset by a failure) and the best run so far.
    pub streak: u32,
    pub best_streak: u32,
    /// Lifetime Summons ("Shadows raised").
    pub shadows: u32,
    /// Level-up points, one per level, each added to the top stat at the time.
    pub bonus: [u32; 5],
    /// Boss drops (M11); the best per slot is equipped.
    pub loot: Vec<super::loot::Item>,
    /// Boss wins after 3+ failures, and `git push --force` runs (badges, M12).
    pub comebacks: u32,
    pub force_pushes: u32,
    /// Unlocked badge ids (M12).
    pub badges: Vec<String>,
    /// Today's daily quest (M13).
    pub daily: super::daily::Daily,
}

impl Lifetime {
    /// STR/INT/DEX/VIT/LUK (SPEC §7.3).
    pub fn stats(&self) -> [u32; 5] {
        let base = [
            self.edits / 10,
            self.reads / 10,
            self.shells / 10,
            self.rests,
            self.first_tries,
        ];
        let gear = super::loot::gear_bonus(&self.loot);
        std::array::from_fn(|i| base[i] + self.bonus[i] + gear[i])
    }
}

/// How tired a hero is from a filling context window (SPEC §7.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fatigue {
    Fresh,
    /// ≥ 70% of the context used.
    Tired,
    /// ≥ 90% used.
    Exhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle,
    Working,
    Blocked,
    Done,
}

/// One thing a hero does for one real agent action (SPEC §5.4 tables).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Edit / Write: sword swing.
    Attack,
    /// Any other shell command: spell.
    Cast,
    /// Read tool: open a chest.
    Open,
    /// grep / rg / Grep tool: read a big book.
    Study,
    /// cat / head / tail: read a scroll.
    Scroll,
    /// ls / find / Glob / web: spyglass.
    Scout,
    /// curl / wget: a messenger bird flies off.
    Bird,
    /// git commit: wax seal.
    Seal,
    /// git push: carrier pigeon flies off.
    PigeonOut,
    /// git pull / fetch: a pigeon lands.
    PigeonIn,
    /// package installs: shopping, a coin pops.
    Shop,
    /// rm / del: fireball.
    Fireball,
    /// mkdir / touch / cp / mv: anvil.
    Craft,
    /// sleep: dozes standing up.
    Doze,
    /// Skill tool: glowing spellbook.
    Spellbook,
}

/// How an action's prop moves while it plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    /// Held at the hero's hands.
    Hand,
    /// Flies away to the right.
    Out,
    /// Flies in from the right and lands at the hands.
    In,
    /// Pops up above the head.
    Rise,
}

impl Action {
    /// How long the action plays.
    pub fn duration_ms(self) -> u64 {
        match self {
            Action::Attack => 3 * 80,
            Action::Cast => 4 * 100,
            Action::Open => 4 * 120,
            Action::Craft => 600,
            Action::Fireball | Action::Bird | Action::PigeonOut | Action::PigeonIn => 800,
            Action::Study | Action::Scroll | Action::Spellbook | Action::Doze => 1_000,
            Action::Scout | Action::Seal | Action::Shop => 700,
        }
    }

    /// The hero's pose while it plays.
    pub fn pose(self) -> Anim {
        match self {
            Action::Attack | Action::Craft => Anim::Attack,
            Action::Cast | Action::Fireball | Action::Bird => Anim::Cast,
            Action::Open => Anim::Open,
            Action::Doze => Anim::Idle,
            _ => Anim::Hold,
        }
    }

    /// Prop sprite (`prop_<name>`) and how it moves.
    pub fn prop(self) -> Option<(&'static str, Motion)> {
        Some(match self {
            Action::Attack | Action::Cast | Action::Doze => return None,
            Action::Open => ("chest", Motion::Hand),
            Action::Study => ("book", Motion::Hand),
            Action::Scroll => ("scroll", Motion::Hand),
            Action::Scout => ("spyglass", Motion::Hand),
            Action::Bird => ("bird", Motion::Out),
            Action::Seal => ("seal", Motion::Hand),
            Action::PigeonOut => ("pigeon", Motion::Out),
            Action::PigeonIn => ("pigeon", Motion::In),
            Action::Shop => ("coin", Motion::Rise),
            Action::Fireball => ("fireball", Motion::Out),
            Action::Craft => ("anvil", Motion::Hand),
            Action::Spellbook => ("spellbook", Motion::Hand),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anim {
    /// Standing still (dozing).
    Idle,
    Sit,
    Walk,
    /// Arms forward, holding a prop.
    Hold,
    Attack,
    Cast,
    Open,
    RaiseHand,
    Victory,
}

impl Anim {
    /// Sprite sheet tag name.
    pub fn tag(self) -> &'static str {
        match self {
            Anim::Idle => "idle",
            Anim::Sit => "sit",
            Anim::Walk => "walk",
            Anim::Hold => "hold",
            Anim::Attack => "attack",
            Anim::Cast => "cast",
            Anim::Open => "open",
            Anim::RaiseHand => "raise_hand",
            Anim::Victory => "victory",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Knight,
    Mage,
    Rogue,
    Ranger,
    Cleric,
    Bard,
    /// Special: kentongan-drum helmet, wood armor, a wooden bat (`tools/gen_special.py`).
    SahurKnight,
    /// Special: an original harlequin trickster who throws cards (`tools/gen_special.py`).
    WildCard,
}

impl Class {
    /// SPEC §6.2: config `[classes]` maps agent kind → class; unknown kinds use `default`.
    pub fn for_agent(classes: &BTreeMap<String, String>, agent: &str) -> Class {
        let name = classes.get(agent).or_else(|| classes.get("default"));
        Class::from_name(name.map_or("Bard", String::as_str))
    }

    pub fn from_name(name: &str) -> Class {
        match name {
            "Knight" => Class::Knight,
            "Mage" => Class::Mage,
            "Rogue" => Class::Rogue,
            "Ranger" => Class::Ranger,
            "Cleric" => Class::Cleric,
            "Sahur Knight" | "SahurKnight" => Class::SahurKnight,
            "Wild Card" | "WildCard" => Class::WildCard,
            _ => Class::Bard,
        }
    }

    /// Lowercase id used in sprite sheet names (`hero_<id>`, `face_<id>`).
    pub fn id(self) -> &'static str {
        match self {
            Class::Knight => "knight",
            Class::Mage => "mage",
            Class::Rogue => "rogue",
            Class::Ranger => "ranger",
            Class::Cleric => "cleric",
            Class::Bard => "bard",
            Class::SahurKnight => "sahur_knight",
            Class::WildCard => "wild_card",
        }
    }

    /// Display name, as written in the config.
    pub fn name(self) -> &'static str {
        match self {
            Class::Knight => "Knight",
            Class::Mage => "Mage",
            Class::Rogue => "Rogue",
            Class::Ranger => "Ranger",
            Class::Cleric => "Cleric",
            Class::Bard => "Bard",
            Class::SahurKnight => "Sahur Knight",
            Class::WildCard => "Wild Card",
        }
    }

    /// Palette key of the class's main color (name plates).
    pub fn color(self) -> char {
        match self {
            Class::Knight => 'r',
            Class::Mage => 'c',
            Class::Rogue => 'p',
            Class::Ranger => 'g',
            Class::Cleric => 'y',
            Class::Bard => 'o',
            Class::SahurKnight => 'y',
            Class::WildCard => 'r',
        }
    }
}

/// One unit of work: from the hero starting work until it has been idle for 10 s.
#[derive(Debug, Clone, PartialEq)]
pub struct Quest {
    pub title: String,
    pub elapsed_ms: u64,
    /// Time spent `working` (drives time XP).
    pub working_ms: u64,
    /// XP earned from time so far (capped, SPEC §7.1).
    pub time_xp: u64,
    pub xp_gained: u64,
    idle_ms: u64,
    /// Token XP so far and the tokens behind it (SPEC §7.1).
    token_xp: u64,
    context_tokens: u64,
    output_tokens: u64,
    /// The "running out of stamina" toast went out for this quest.
    pub exhausted_toast: bool,
    /// Boss attempts in this quest: ✓ won, ✗ failed.
    pub bosses: String,
}

/// A finished quest, for the quest log.
#[derive(Debug, Clone, PartialEq)]
pub struct QuestDone {
    pub title: String,
    pub duration_ms: u64,
    pub xp: u64,
    /// "complete" or "unfinished" (pane closed mid-quest).
    pub result: &'static str,
    pub bosses: String,
    /// 1-based chapter of the hero's story this quest belongs to.
    pub chapter: u32,
}

/// Where a live hero sits: its herdr pane and its identity key (SPEC §6.2).
#[derive(Debug, Clone, PartialEq)]
pub struct Seat {
    pub pane_id: String,
    pub repo: Repo,
    /// herdr agent kind, e.g. "claude".
    pub kind: String,
    /// 0-based order among live agents of the same kind in the same repo.
    pub slot: usize,
}

impl Seat {
    /// Key inside the repo's party file.
    pub fn key(&self) -> String {
        format!("{}#{}", self.kind, self.slot)
    }
}

#[derive(Debug, Clone)]
pub struct Hero {
    pub name: String,
    pub class: Class,
    pub level: u32,
    /// XP into the current level.
    pub xp: u64,
    pub status: Status,
    /// Percent of the context window used; `None` = unknown (shows `?`).
    pub context_used: Option<u8>,
    /// Detail of the latest action, e.g. "editing src/auth.rs".
    pub detail: Option<String>,
    action: Option<(Action, u64)>,
    queue: VecDeque<Action>,
    /// Time spent in the current animation (drives the sprite frame).
    pub anim_ms: u64,
    /// Latest work title (herdr's `terminal_title_stripped`); names the next quest.
    pub title: Option<String>,
    pub quest: Option<Quest>,
    /// Remaining time of the LEVEL UP! effect.
    pub level_up_ms: u64,
    /// Is this hero's pane focused in herdr?
    pub focused: bool,
    /// `World::clock_ms` when the hero last worked (picks whose quest the HUD shows).
    pub last_active_ms: u64,
    /// Set when the agent went away: the hero walks off-screen, then is removed.
    pub leaving_ms: Option<u64>,
    /// `None` for demo heroes.
    pub seat: Option<Seat>,
    /// Floating text above the head (text, palette key, ms left).
    pub float: Option<(String, char, u64)>,
    /// Actions collapsed into the current combo.
    combo: u32,
    /// The title came from a real prompt (beats herdr's terminal title).
    pub title_from_prompt: bool,
    /// Tool call id of the boss command this hero is waiting on.
    pub boss_tool: Option<String>,
    pub life: Lifetime,
    /// Rest scene time left, then Well Rested buff time left.
    pub rest_ms: u64,
    pub rested_ms: u64,
    /// Last counted usage: message id (Claude repeats usage on each streamed line) and
    /// context size (token XP counts growth only).
    last_message: Option<String>,
    last_context: Option<u64>,
    /// Latest file the agent touched (places the hero in the Town).
    pub last_file: Option<String>,
    /// Subagents currently out (SPEC §6.5).
    pub summons: Vec<Summon>,
    /// How long the hero has been blocked without a break.
    pub blocked_ms: u64,
}

impl Hero {
    pub fn new(name: &str, class: Class) -> Self {
        Hero {
            name: name.to_string(),
            class,
            level: 1,
            xp: 0,
            status: Status::Idle,
            context_used: None,
            detail: None,
            action: None,
            queue: VecDeque::new(),
            anim_ms: 0,
            title: None,
            quest: None,
            level_up_ms: 0,
            focused: false,
            last_active_ms: 0,
            leaving_ms: None,
            seat: None,
            float: None,
            combo: 0,
            title_from_prompt: false,
            boss_tool: None,
            life: Lifetime::default(),
            rest_ms: 0,
            rested_ms: 0,
            last_message: None,
            last_context: None,
            last_file: None,
            summons: Vec::new(),
            blocked_ms: 0,
        }
    }

    pub fn anim(&self) -> Anim {
        if self.leaving_ms.is_some() {
            return Anim::Walk;
        }
        if self.rest_ms > 0 {
            return Anim::Sit;
        }
        if let Some((a, _)) = self.action {
            return a.pose();
        }
        match self.status {
            Status::Idle => Anim::Sit,
            Status::Working => Anim::Walk,
            Status::Blocked => Anim::RaiseHand,
            Status::Done => Anim::Victory,
        }
    }

    /// Run `f`, restarting the animation clock if the animation changed.
    fn track(&mut self, f: impl FnOnce(&mut Self)) {
        let before = self.anim();
        f(self);
        if self.anim() != before {
            self.anim_ms = 0;
        }
    }

    /// herdr status always wins: leaving `working` drops pending actions.
    pub fn set_status(&mut self, status: Status) {
        self.track(|h| {
            h.status = status;
            if status != Status::Working {
                h.action = None;
                h.queue.clear();
                h.detail = None;
            }
        });
    }

    /// Queue a one-shot action. Ignored unless working. Past `MAX_QUEUE` pending
    /// actions, extra ones collapse into a combo counter instead of piling up.
    pub fn act(&mut self, action: Action, detail: Option<String>) {
        if self.status != Status::Working {
            return;
        }
        if detail.is_some() {
            self.detail = detail;
        }
        if self.queue.len() >= MAX_QUEUE {
            self.combo += 1;
            self.say(format!("x{} combo!", self.combo + MAX_QUEUE as u32), 'y');
            return;
        }
        self.queue.push_back(action);
        self.track(|h| h.start_next());
    }

    /// Show floating text above the hero.
    pub fn say(&mut self, text: String, color: char) {
        self.float = Some((text, color, FLOAT_MS));
    }

    /// The playing action and how far through it is (0.0–1.0).
    pub fn action(&self) -> Option<(Action, f32)> {
        self.action
            .map(|(a, left)| (a, 1.0 - left as f32 / a.duration_ms() as f32))
    }

    fn start_next(&mut self) {
        if self.action.is_none() {
            self.action = self.queue.pop_front().map(|a| (a, a.duration_ms()));
        }
    }

    pub fn busy(&self) -> bool {
        self.action.is_some() || !self.queue.is_empty() || !self.summons.is_empty()
    }

    /// Advance animation and the quest. Returns the quest if it just completed.
    pub fn update(&mut self, dt_ms: u64) -> Option<QuestDone> {
        self.anim_ms += dt_ms;
        self.level_up_ms = self.level_up_ms.saturating_sub(dt_ms);
        if let Some((_, _, left)) = &mut self.float {
            *left = left.saturating_sub(dt_ms);
            if *left == 0 {
                self.float = None;
            }
        }
        // After the float timer, so "Well rested!" isn't aged out in the same tick.
        self.rested_ms = self.rested_ms.saturating_sub(dt_ms);
        if self.rest_ms > 0 {
            self.track(|h| {
                h.rest_ms = h.rest_ms.saturating_sub(dt_ms);
                if h.rest_ms == 0 {
                    h.rested_ms = RESTED_MS;
                    h.say("Well rested!".into(), 'C');
                }
            });
        }
        if self.queue.is_empty() {
            self.combo = 0;
        }
        if let Some(ms) = &mut self.leaving_ms {
            *ms += dt_ms;
        }
        self.blocked_ms = if self.status == Status::Blocked {
            self.blocked_ms + dt_ms
        } else {
            0
        };
        // Summons that sank give their XP to this hero.
        let mut earned = 0;
        self.summons.retain_mut(|s| {
            let alive = s.update(dt_ms);
            if !alive {
                earned += s.xp();
            }
            alive
        });
        if earned > 0 {
            self.gain_xp(earned);
        }
        let done = self.update_quest(dt_ms);
        self.track(|h| {
            if let Some((_, left)) = &mut h.action {
                *left = left.saturating_sub(dt_ms);
                if *left == 0 {
                    h.action = None;
                }
            }
            h.start_next();
        });
        done
    }

    fn update_quest(&mut self, dt: u64) -> Option<QuestDone> {
        let Some(q) = &mut self.quest else {
            if self.status == Status::Working {
                self.quest = Some(Quest {
                    title: self
                        .title
                        .clone()
                        .unwrap_or_else(|| "Untitled quest".into()),
                    elapsed_ms: 0,
                    working_ms: 0,
                    time_xp: 0,
                    xp_gained: 0,
                    idle_ms: 0,
                    token_xp: 0,
                    context_tokens: 0,
                    output_tokens: 0,
                    exhausted_toast: false,
                    bosses: String::new(),
                });
            }
            return None;
        };
        q.elapsed_ms += dt;
        match self.status {
            Status::Working => {
                q.idle_ms = 0;
                q.working_ms += dt;
                if let Some(t) = &self.title {
                    q.title.clone_from(t);
                }
                let earned = xp::time_xp(q.working_ms);
                let new = earned - q.time_xp;
                q.time_xp = earned;
                self.gain_xp(new);
            }
            // Blocked pauses the quest; only idle/done can end it.
            Status::Blocked => q.idle_ms = 0,
            Status::Idle | Status::Done => {
                q.idle_ms += dt;
                if q.idle_ms >= QUEST_END_IDLE_MS {
                    return self.finish_quest();
                }
            }
        }
        None
    }

    /// Complete the open quest (+25 XP), if any.
    pub fn finish_quest(&mut self) -> Option<QuestDone> {
        self.quest.as_ref()?;
        self.gain_xp(QUEST_COMPLETE_XP);
        self.life.quests += 1;
        self.end_quest("complete")
    }

    pub fn fatigue(&self) -> Fatigue {
        match self.context_used {
            Some(90..) => Fatigue::Exhausted,
            Some(70..) => Fatigue::Tired,
            _ => Fatigue::Fresh,
        }
    }

    /// Compaction: the Rest scene, then Well Rested. Never a punishment (SPEC §7.4).
    pub fn rest(&mut self, context_after: Option<u8>) {
        self.track(|h| h.rest_ms = REST_MS);
        if context_after.is_some() {
            self.context_used = context_after;
        }
        self.last_context = None;
        self.life.rests += 1;
        self.gain_xp(REST_XP);
    }

    /// A new session / `/clear`: a new Chapter. Level and stats stay; STA is full.
    pub fn new_run(&mut self) {
        self.life.chapters += 1;
        if self.context_used.is_some() {
            self.context_used = Some(0);
        }
        self.last_context = None;
        self.last_message = None;
        self.title_from_prompt = false;
        self.boss_tool = None;
    }

    /// One usage report. Token XP: +1 per 1,000 new context tokens and +1 per 200 output
    /// tokens, capped at 400 per quest; each message counted once.
    pub fn usage(&mut self, context: u64, output: u64, message: Option<&str>) {
        let grew = self.last_context.map_or(0, |c| context.saturating_sub(c));
        self.last_context = Some(context);
        let new_message = message.is_some() && message != self.last_message.as_deref();
        if new_message {
            self.last_message = message.map(String::from);
        }
        let Some(q) = &mut self.quest else { return };
        q.context_tokens += grew;
        if new_message {
            q.output_tokens += output;
        }
        let earned = (q.context_tokens / 1_000 + q.output_tokens / 200).min(TOKEN_XP_CAP);
        let new = earned - q.token_xp;
        q.token_xp = earned;
        if new > 0 {
            self.gain_xp(new);
        }
    }

    /// Close the open quest (if any) with `result`.
    pub fn end_quest(&mut self, result: &'static str) -> Option<QuestDone> {
        let chapter = self.life.chapters + 1;
        self.quest.take().map(|q| QuestDone {
            title: q.title,
            duration_ms: q.elapsed_ms,
            xp: q.xp_gained,
            result,
            bosses: q.bosses,
            chapter,
        })
    }

    /// Returns levels gained. Starts the LEVEL UP! effect.
    pub fn gain_xp(&mut self, amount: u64) -> u32 {
        let (level, xp, gained) = xp::add_xp(self.level, self.xp, amount);
        self.level = level;
        self.xp = xp;
        if let Some(q) = &mut self.quest {
            q.xp_gained += amount;
        }
        if gained > 0 {
            self.level_up_ms = LEVEL_UP_MS;
        }
        for _ in 0..gained {
            // +1 point to the top stat (SPEC §7.2); ties go to the first.
            let stats = self.life.stats();
            let top = (0..5).max_by_key(|&i| (stats[i], 5 - i)).unwrap();
            self.life.bonus[top] += 1;
        }
        gained
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn working() -> Hero {
        let mut h = Hero::new("Claude", Class::Knight);
        h.set_status(Status::Working);
        h
    }

    #[test]
    fn status_picks_base_anim() {
        let mut h = Hero::new("Claude", Class::Knight);
        assert_eq!(h.anim(), Anim::Sit);
        for (s, a) in [
            (Status::Working, Anim::Walk),
            (Status::Blocked, Anim::RaiseHand),
            (Status::Done, Anim::Victory),
        ] {
            h.set_status(s);
            assert_eq!(h.anim(), a);
        }
    }

    #[test]
    fn actions_play_one_at_a_time_then_return_to_walk() {
        let mut h = working();
        h.act(Action::Attack, None);
        h.act(Action::Open, None);
        assert_eq!(h.anim(), Anim::Attack);
        h.update(Action::Attack.duration_ms());
        assert_eq!(h.anim(), Anim::Open);
        assert_eq!(h.anim_ms, 0);
        h.update(Action::Open.duration_ms());
        assert_eq!(h.anim(), Anim::Walk);
        assert!(!h.busy());
    }

    #[test]
    fn leaving_working_drops_actions() {
        let mut h = working();
        h.act(Action::Cast, Some("cargo test".into()));
        h.act(Action::Cast, None);
        h.set_status(Status::Blocked);
        assert_eq!(h.anim(), Anim::RaiseHand);
        assert!(!h.busy());
        assert_eq!(h.detail, None);
    }

    #[test]
    fn actions_ignored_unless_working() {
        let mut h = Hero::new("Codex", Class::Mage);
        h.act(Action::Attack, None);
        assert_eq!(h.anim(), Anim::Sit);
    }

    #[test]
    fn quest_starts_on_work_and_ends_after_ten_idle_seconds() {
        let mut h = Hero::new("Claude", Class::Knight);
        h.title = Some("fix login".into());
        h.set_status(Status::Working);
        h.update(1);
        h.update(30_000);
        assert_eq!(h.quest.as_ref().unwrap().title, "fix login");
        assert_eq!(h.xp, 3, "1 XP per 10 s working");
        h.set_status(Status::Idle);
        assert!(h.update(9_000).is_none());
        h.set_status(Status::Working); // back within 10 s: same quest
        h.update(1);
        h.set_status(Status::Idle);
        assert!(h.update(9_999).is_none());
        let done = h.update(1).expect("quest complete");
        assert_eq!(
            (done.title.as_str(), done.result),
            ("fix login", "complete")
        );
        assert_eq!(done.xp, 3 + QUEST_COMPLETE_XP);
        assert!(h.quest.is_none());
    }

    #[test]
    fn blocked_pauses_quest_and_earns_nothing() {
        let mut h = Hero::new("Codex", Class::Mage);
        h.set_status(Status::Working);
        h.update(1);
        h.set_status(Status::Blocked);
        h.update(60_000);
        assert!(h.quest.is_some());
        assert_eq!(h.xp, 0);
    }

    #[test]
    fn level_up_starts_effect() {
        let mut h = Hero::new("Claude", Class::Knight);
        assert_eq!(h.gain_xp(100), 1);
        assert_eq!(h.level_up_ms, LEVEL_UP_MS);
        h.update(LEVEL_UP_MS);
        assert_eq!(h.level_up_ms, 0);
    }

    #[test]
    fn class_mapping_uses_config_then_default() {
        let mut c = crate::config::Config::default().classes;
        assert_eq!(Class::for_agent(&c, "claude"), Class::Knight);
        assert_eq!(Class::for_agent(&c, "grok"), Class::Bard);
        c.insert("default".into(), "Cleric".into());
        assert_eq!(Class::for_agent(&c, "grok"), Class::Cleric);
        c.insert("claude".into(), "Sahur Knight".into());
        assert_eq!(Class::for_agent(&c, "claude"), Class::SahurKnight);
        assert_eq!(Class::SahurKnight.name(), "Sahur Knight");
        c.insert("claude".into(), "Wild Card".into());
        assert_eq!(Class::for_agent(&c, "claude"), Class::WildCard);
    }

    #[test]
    fn flood_of_actions_becomes_a_combo() {
        let mut h = working();
        for _ in 0..20 {
            h.act(Action::Attack, None);
        }
        // one playing + MAX_QUEUE waiting; the rest are a combo
        assert_eq!(h.queue.len(), MAX_QUEUE);
        assert_eq!(h.float.as_ref().unwrap().0, "x19 combo!");
        for _ in 0..=MAX_QUEUE {
            h.update(Action::Attack.duration_ms());
        }
        assert!(!h.busy());
        h.update(1);
        assert_eq!(h.combo, 0);
    }

    #[test]
    fn every_action_has_a_pose_and_a_duration() {
        use Action::*;
        for a in [
            Attack, Cast, Open, Study, Scroll, Scout, Bird, Seal, PigeonOut, PigeonIn, Shop,
            Fireball, Craft, Doze, Spellbook,
        ] {
            assert!(a.duration_ms() > 0);
            let mut h = working();
            h.act(a, None);
            assert_eq!(h.anim(), a.pose());
            h.update(a.duration_ms() / 2);
            let (_, p) = h.action().unwrap();
            assert!((0.4..0.6).contains(&p), "{a:?} {p}");
        }
    }

    #[test]
    fn fatigue_thresholds() {
        let mut h = working();
        for (used, want) in [
            (None, Fatigue::Fresh),
            (Some(69), Fatigue::Fresh),
            (Some(70), Fatigue::Tired),
            (Some(90), Fatigue::Exhausted),
        ] {
            h.context_used = used;
            assert_eq!(h.fatigue(), want);
        }
    }

    #[test]
    fn rest_scene_then_well_rested() {
        let mut h = working();
        h.context_used = Some(95);
        h.rest(Some(12));
        assert_eq!(
            (h.anim(), h.context_used, h.life.rests),
            (Anim::Sit, Some(12), 1)
        );
        assert_eq!(h.xp, REST_XP);
        h.update(REST_MS);
        assert_eq!(h.anim(), Anim::Walk);
        assert_eq!(h.rested_ms, RESTED_MS);
        assert_eq!(h.float.as_ref().unwrap().0, "Well rested!");
        assert_eq!(h.life.stats()[3], 1, "VIT");
    }

    #[test]
    fn token_xp_counts_growth_once_per_message_and_caps() {
        let mut h = working();
        h.update(1); // quest starts
        h.usage(10_000, 0, Some("m1")); // first sight: baseline only
        assert_eq!(h.xp, 0);
        h.usage(15_000, 400, Some("m2"));
        assert_eq!(h.xp, 5 + 2);
        h.usage(15_000, 400, Some("m2")); // same message streamed again
        assert_eq!(h.xp, 7);
        h.usage(9_000_000, 0, Some("m3"));
        assert_eq!(h.quest.as_ref().unwrap().token_xp, TOKEN_XP_CAP);
    }

    #[test]
    fn new_run_keeps_level_and_counts_a_chapter() {
        let mut h = working();
        h.gain_xp(500);
        let level = h.level;
        h.context_used = Some(80);
        h.new_run();
        assert_eq!(
            (h.level, h.life.chapters, h.context_used),
            (level, 1, Some(0))
        );
    }

    #[test]
    fn level_up_points_go_to_the_top_stat() {
        let mut h = working();
        h.life.reads = 40; // INT 4 is the top stat
        h.gain_xp(100);
        assert_eq!(h.life.bonus, [0, 1, 0, 0, 0]);
        assert_eq!(h.life.stats()[1], 5);
    }

    #[test]
    fn anim_clock_restarts_only_on_change() {
        let mut h = working();
        h.update(500);
        assert_eq!(h.anim_ms, 500);
        h.set_status(Status::Working);
        assert_eq!(h.anim_ms, 500);
        h.set_status(Status::Idle);
        assert_eq!(h.anim_ms, 0);
    }
}
