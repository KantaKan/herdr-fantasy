//! Per-hero state machine (SPEC §5.4). herdr status picks the base animation; one-shot
//! actions play on top, one at a time, only while the hero is working.

use std::collections::VecDeque;

use super::xp;

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
        }
    }

    pub fn anim(&self) -> Anim {
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

    pub fn update(&mut self, dt_ms: u64) {
        self.anim_ms += dt_ms;
        self.track(|h| {
            if let Some((_, left)) = &mut h.action {
                *left = left.saturating_sub(dt_ms);
                if *left == 0 {
                    h.action = None;
                }
            }
            h.start_next();
        });
    }

    /// Returns levels gained.
    pub fn gain_xp(&mut self, amount: u64) -> u32 {
        let (level, xp, gained) = xp::add_xp(self.level, self.xp, amount);
        self.level = level;
        self.xp = xp;
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
