//! Daily quest (TASKS M13): one goal per hero per day, rolled from the date. Pure.

use serde::{Deserialize, Serialize};

use super::hero::Lifetime;

pub const REWARD_XP: u64 = 50;

/// Today's quest and the counter value it started from.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Daily {
    /// Days since 1970-01-01 (UTC); 0 = never rolled.
    pub day: u32,
    pub kind: usize,
    pub start: u32,
    pub done: bool,
}

pub struct Kind {
    pub name: &'static str,
    pub target: u32,
    count: fn(&Lifetime) -> u32,
}

pub const KINDS: &[Kind] = &[
    Kind {
        name: "Win 3 boss fights",
        target: 3,
        count: |l| l.bosses_won,
    },
    Kind {
        name: "Finish 3 quests",
        target: 3,
        count: |l| l.quests,
    },
    Kind {
        name: "Raise 2 shadows",
        target: 2,
        count: |l| l.shadows,
    },
    Kind {
        name: "Make 20 edits",
        target: 20,
        count: |l| l.edits,
    },
    Kind {
        name: "Read or search 30 times",
        target: 30,
        count: |l| l.reads,
    },
    Kind {
        name: "Run 15 shell commands",
        target: 15,
        count: |l| l.shells,
    },
];

/// ponytail: UTC days, so "today" turns over at UTC midnight; local midnight needs a
/// timezone crate.
pub fn day_of(unix_ms: u64) -> u32 {
    (unix_ms / 86_400_000) as u32
}

fn roll(hero: &str, day: u32) -> usize {
    let h = hero
        .bytes()
        .fold(day as u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64));
    (h % KINDS.len() as u64) as usize
}

/// Bring the daily quest up to `today`: a new day rolls a new quest. Returns true when
/// it was just completed (pay the reward once).
pub fn tick(life: &mut Lifetime, hero: &str, today: u32) -> bool {
    if life.daily.day != today {
        let kind = roll(hero, today);
        life.daily = Daily {
            day: today,
            kind,
            start: (KINDS[kind].count)(life),
            done: false,
        };
    }
    let (_, n, target) = progress(life);
    if !life.daily.done && n >= target {
        life.daily.done = true;
        return true;
    }
    false
}

/// (quest name, progress, target).
pub fn progress(life: &Lifetime) -> (&'static str, u32, u32) {
    let k = &KINDS[life.daily.kind.min(KINDS.len() - 1)];
    let n = (k.count)(life).saturating_sub(life.daily.start);
    (k.name, n.min(k.target), k.target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completes_once_and_rolls_over_each_day() {
        let mut l = Lifetime::default();
        let day = day_of(1_792_000_000_000);
        assert!(!tick(&mut l, "A", day));
        let (_, n, target) = progress(&l);
        assert_eq!(n, 0);
        // Do whatever today's quest asks.
        let k = l.daily.kind;
        for _ in 0..target {
            match k {
                0 => l.bosses_won += 1,
                1 => l.quests += 1,
                2 => l.shadows += 1,
                3 => l.edits += 1,
                4 => l.reads += 1,
                _ => l.shells += 1,
            }
        }
        assert!(tick(&mut l, "A", day), "done today");
        assert!(!tick(&mut l, "A", day), "paid once");
        assert!(!tick(&mut l, "A", day + 1));
        assert!(!l.daily.done, "a new day, a new quest");
        assert_eq!(progress(&l).1, 0, "counts from the new day's start");
    }

    #[test]
    fn heroes_and_days_get_different_quests() {
        let kinds: std::collections::HashSet<_> = (0..30).map(|d| roll("Claude", d)).collect();
        assert!(kinds.len() >= 4);
    }
}
