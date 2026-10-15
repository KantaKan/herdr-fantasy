//! Per-hero state machine (SPEC §5.4). herdr status picks the base animation; one-shot
//! actions play on top, one at a time, only while the hero is working.

use std::collections::{BTreeMap, VecDeque};

use super::repo::Repo;
use super::xp;

/// Quest ends after this long idle with no new work (SPEC §6.4).
pub const QUEST_END_IDLE_MS: u64 = 10_000;
pub const QUEST_COMPLETE_XP: u64 = 25;
const LEVEL_UP_MS: u64 = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle,
    Working,
    Blocked,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Edit / Write: sword swing.
    Attack,
    /// Bash / PowerShell: spell.
    Cast,
    /// Read / Grep / Glob: open a chest.
    Open,
}

impl Action {
    /// One pass of the real animation (ART.md §4 frames × ms).
    pub fn duration_ms(self) -> u64 {
        match self {
            Action::Attack => 3 * 80,
            Action::Cast => 4 * 100,
            Action::Open => 4 * 120,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anim {
    Sit,
    Walk,
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
            Anim::Sit => "sit",
            Anim::Walk => "walk",
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
}

/// A finished quest, for the quest log.
#[derive(Debug, Clone, PartialEq)]
pub struct QuestDone {
    pub title: String,
    pub duration_ms: u64,
    pub xp: u64,
    /// "complete" or "unfinished" (pane closed mid-quest).
    pub result: &'static str,
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
        }
    }

    pub fn anim(&self) -> Anim {
        if self.leaving_ms.is_some() {
            return Anim::Walk;
        }
        if let Some((a, _)) = self.action {
            return match a {
                Action::Attack => Anim::Attack,
                Action::Cast => Anim::Cast,
                Action::Open => Anim::Open,
            };
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

    /// Queue a one-shot action. Ignored unless working.
    pub fn act(&mut self, action: Action, detail: Option<String>) {
        if self.status != Status::Working {
            return;
        }
        self.queue.push_back(action);
        if detail.is_some() {
            self.detail = detail;
        }
        self.track(|h| h.start_next());
    }

    fn start_next(&mut self) {
        if self.action.is_none() {
            self.action = self.queue.pop_front().map(|a| (a, a.duration_ms()));
        }
    }

    pub fn busy(&self) -> bool {
        self.action.is_some() || !self.queue.is_empty()
    }

    /// Advance animation and the quest. Returns the quest if it just completed.
    pub fn update(&mut self, dt_ms: u64) -> Option<QuestDone> {
        self.anim_ms += dt_ms;
        self.level_up_ms = self.level_up_ms.saturating_sub(dt_ms);
        if let Some(ms) = &mut self.leaving_ms {
            *ms += dt_ms;
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
                    self.gain_xp(QUEST_COMPLETE_XP);
                    return self.end_quest("complete");
                }
            }
        }
        None
    }

    /// Close the open quest (if any) with `result`.
    pub fn end_quest(&mut self, result: &'static str) -> Option<QuestDone> {
        self.quest.take().map(|q| QuestDone {
            title: q.title,
            duration_ms: q.elapsed_ms,
            xp: q.xp_gained,
            result,
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
