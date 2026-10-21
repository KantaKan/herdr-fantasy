//! Dragon Quest-style battle (TASKS M21): the message window and the field enemies.
//! Pure; the world feeds it events and the renderer draws it.

use std::collections::VecDeque;

use super::hero::Action;

/// The message-window line for a hero's action (`detail` is its activity text).
pub fn action_line(hero: &str, action: Action, detail: Option<&str>) -> String {
    let d = detail.unwrap_or("");
    let after = |prefix: &str| {
        d.strip_prefix(prefix)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    match action {
        Action::Attack => format!("{hero} attacks!"),
        Action::Open => match after("reading") {
            Some(f) => format!("{hero} opens a chest: {f}"),
            None => format!("{hero} opens a chest."),
        },
        Action::Study => match after("searching for") {
            Some(p) => format!("{hero} searches the tomes for \"{p}\"..."),
            None => format!("{hero} reads the big book..."),
        },
        Action::Scroll => format!("{hero} reads a scroll."),
        Action::Scout => match after("looking for") {
            Some(p) => format!("{hero} looks around for {p}."),
            None => format!("{hero} scouts ahead."),
        },
        Action::Bird => format!("{hero} sends a messenger bird!"),
        Action::Seal => format!("{hero} seals a commit with wax!"),
        Action::PigeonOut => format!("{hero} sends a carrier pigeon! (git push)"),
        Action::PigeonIn => format!("A pigeon brings {hero} news!"),
        Action::Shop => format!("{hero} shops for supplies."),
        Action::Fireball => format!("{hero} casts Fireball!"),
        Action::Craft => format!("{hero} forges something new."),
        Action::Doze => format!("{hero} dozes off... zzz"),
        Action::Spellbook => match after("studying") {
            Some(s) => format!("{hero} opens the spellbook: {s}!"),
            None => format!("{hero} opens the spellbook!"),
        },
        Action::Cast => match (after("running"), after("summoning")) {
            (Some(c), _) => format!("{hero} casts {c}!"),
            (_, Some(_)) => format!("{hero} calls a shadow from below!"),
            _ => format!("{hero} casts a spell!"),
        },
    }
}

/// Milliseconds per typed character, and how long a finished line stays before the next
/// one starts (DQ text speed "normal").
pub const TYPE_MS: u64 = 25;
pub const HOLD_MS: u64 = 900;
/// Lines waiting beyond this are dropped (oldest first): agents can outrun any reader.
const QUEUE: usize = 3;

/// The message window: two lines on screen, the newest typing out, a short queue.
#[derive(Debug, Clone, Default)]
pub struct Messages {
    pub lines: VecDeque<String>,
    pending: VecDeque<String>,
    /// Time since the newest line started typing.
    pub typing_ms: u64,
}

impl Messages {
    pub fn say(&mut self, text: impl Into<String>) {
        let text = text.into();
        if self.lines.back() == Some(&text) || self.pending.contains(&text) {
            return;
        }
        if self.lines.is_empty() {
            self.lines.push_back(text);
            self.typing_ms = 0;
            return;
        }
        if self.pending.len() == QUEUE {
            self.pending.pop_front();
        }
        self.pending.push_back(text);
    }

    pub fn update(&mut self, dt: u64) {
        self.typing_ms += dt;
        let Some(cur) = self.lines.back() else { return };
        let done = cur.chars().count() as u64 * TYPE_MS + HOLD_MS;
        if self.typing_ms >= done
            && let Some(next) = self.pending.pop_front()
        {
            if self.lines.len() == 2 {
                self.lines.pop_front();
            }
            self.lines.push_back(next);
            self.typing_ms = 0;
        }
    }

    /// The two lines as shown: the older one whole, the newest typed so far.
    pub fn shown(&self) -> Vec<String> {
        let n = self.lines.len();
        self.lines
            .iter()
            .enumerate()
            .map(|(i, l)| {
                if i + 1 == n {
                    l.chars()
                        .take((self.typing_ms / TYPE_MS) as usize)
                        .collect()
                } else {
                    l.clone()
                }
            })
            .collect()
    }

    /// Still typing or with lines waiting (keeps the frame rate up).
    pub fn busy(&self) -> bool {
        !self.pending.is_empty()
            || self
                .lines
                .back()
                .is_some_and(|l| self.typing_ms < l.chars().count() as u64 * TYPE_MS)
    }
}

/// A field enemy: what heroes swing at during normal work.
#[derive(Debug, Clone, PartialEq)]
pub struct Mob {
    pub kind: MobKind,
    pub hp: u32,
    pub max: u32,
    /// Time left of the blink + damage number after a hit.
    pub hit_ms: u64,
    pub last_damage: u32,
    /// Counting up once defeated; removed after `DIE_MS`.
    pub dying_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobKind {
    Slime,
    RedSlime,
    MetalSlime,
}

impl MobKind {
    pub fn name(self) -> &'static str {
        match self {
            MobKind::Slime => "Slime",
            MobKind::RedSlime => "Red Slime",
            MobKind::MetalSlime => "Metal Slime",
        }
    }

    /// Palette swaps applied to the slime sprite.
    pub fn swaps(self) -> &'static [(char, char)] {
        match self {
            MobKind::Slime => &[],
            MobKind::RedSlime => &[('g', 'r'), ('G', 'o'), ('t', 'p')],
            MobKind::MetalSlime => &[('g', 's'), ('G', 'w'), ('t', 'S')],
        }
    }

    fn max_hp(self) -> u32 {
        match self {
            MobKind::Slime => 20,
            MobKind::RedSlime => 35,
            MobKind::MetalSlime => 6,
        }
    }
}

pub const HIT_MS: u64 = 600;
pub const DIE_MS: u64 = 500;
/// Gap between a cleared wave and the next one drawing near.
pub const WAVE_GAP_MS: u64 = 1_500;

/// The wave after `n` waves: 1–3 slimes, sometimes red, rarely a Metal Slime.
pub fn wave(n: u32) -> Vec<Mob> {
    let h = (n as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 33;
    let count = 1 + (h % 3) as usize;
    (0..count)
        .map(|i| {
            let r = (h >> (4 + i * 5)) % 16;
            let kind = match r {
                0 => MobKind::MetalSlime,
                1..=4 => MobKind::RedSlime,
                _ => MobKind::Slime,
            };
            Mob {
                kind,
                hp: kind.max_hp(),
                max: kind.max_hp(),
                hit_ms: 0,
                last_damage: 0,
                dying_ms: None,
            }
        })
        .collect()
}

/// "A Slime draws near!" / "2 Slimes draw near!" / "A Slime and a Red Slime draw near!"
pub fn appear_line(mobs: &[Mob]) -> String {
    let names: Vec<&str> = mobs.iter().map(|m| m.kind.name()).collect();
    if names.len() == 1 {
        return format!("A {} draws near!", names[0]);
    }
    if names.iter().all(|n| *n == names[0]) {
        return format!("{} {}s draw near!", names.len(), names[0]);
    }
    let mut s = names[..names.len() - 1]
        .iter()
        .map(|n| format!("a {n}"))
        .collect::<Vec<_>>()
        .join(", ");
    s.push_str(&format!(" and a {} draw near!", names[names.len() - 1]));
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or(s)
}

/// Hit the first standing mob for `damage` (Metal Slimes shrug most of it off). Returns
/// the lines to narrate.
pub fn strike(mobs: &mut [Mob], damage: u32) -> Vec<String> {
    let Some(m) = mobs.iter_mut().find(|m| m.dying_ms.is_none()) else {
        return Vec::new();
    };
    let dmg = match m.kind {
        MobKind::MetalSlime => damage.clamp(1, 2),
        _ => damage.max(1),
    };
    m.hp = m.hp.saturating_sub(dmg);
    m.hit_ms = HIT_MS;
    m.last_damage = dmg;
    let name = m.kind.name();
    let mut out = vec![format!("The {name} takes {dmg} damage!")];
    if m.hp == 0 {
        m.dying_ms = Some(0);
        out.push(format!("The {name} is defeated!"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_type_out_hold_and_queue() {
        let mut m = Messages::default();
        m.say("Claude attacks!");
        assert_eq!(m.shown(), [""]);
        m.update(5 * TYPE_MS);
        assert_eq!(m.shown(), ["Claud"]);
        m.say("The Slime takes 12 damage!");
        m.say("The Slime is defeated!");
        m.say("A Slime draws near!");
        m.say("4th line");
        m.update(15 * TYPE_MS + HOLD_MS);
        assert_eq!(m.lines.len(), 2);
        assert_eq!(m.lines[0], "Claude attacks!");
        assert!(m.busy());
        for _ in 0..10 {
            m.update(2_000);
        }
        assert_eq!(m.lines.back().unwrap(), "4th line");
        assert!(
            !m.lines.iter().any(|l| l.contains("12 damage")),
            "dropped from the queue"
        );
        assert!(!m.busy());
    }

    #[test]
    fn mobs_take_damage_die_and_waves_vary() {
        let mut mobs = vec![Mob {
            kind: MobKind::Slime,
            hp: 20,
            max: 20,
            hit_ms: 0,
            last_damage: 0,
            dying_ms: None,
        }];
        assert_eq!(strike(&mut mobs, 12), ["The Slime takes 12 damage!"]);
        assert_eq!(
            strike(&mut mobs, 12),
            ["The Slime takes 12 damage!", "The Slime is defeated!"]
        );
        assert!(strike(&mut mobs, 5).is_empty(), "nothing left standing");
        let kinds: std::collections::HashSet<_> =
            (0..200).flat_map(wave).map(|m| m.kind as u8).collect();
        assert_eq!(kinds.len(), 3, "all three slimes show up");
        assert!((0..200).all(|n| (1..=3).contains(&wave(n).len())));
    }

    #[test]
    fn action_lines() {
        assert_eq!(
            action_line("Claude", Action::Cast, Some("running cargo fmt")),
            "Claude casts cargo fmt!"
        );
        assert_eq!(
            action_line("Codex", Action::Study, Some("searching for auth")),
            "Codex searches the tomes for \"auth\"..."
        );
        assert_eq!(
            action_line("Claude", Action::Open, Some("reading src/auth.rs")),
            "Claude opens a chest: src/auth.rs"
        );
    }

    #[test]
    fn appear_lines_read_naturally() {
        let mk = |k| Mob {
            kind: k,
            hp: 1,
            max: 1,
            hit_ms: 0,
            last_damage: 0,
            dying_ms: None,
        };
        assert_eq!(appear_line(&[mk(MobKind::Slime)]), "A Slime draws near!");
        assert_eq!(
            appear_line(&[mk(MobKind::Slime), mk(MobKind::Slime)]),
            "2 Slimes draw near!"
        );
        assert_eq!(
            appear_line(&[mk(MobKind::Slime), mk(MobKind::RedSlime)]),
            "A Slime and a Red Slime draw near!"
        );
    }
}
