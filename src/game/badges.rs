//! Achievements (TASKS M12): badges a hero unlocks from its lifetime counters. Pure.

use super::hero::Lifetime;
use super::loot::Rarity;

pub struct Badge {
    pub id: &'static str,
    pub name: &'static str,
    /// How to earn it (character sheet).
    pub how: &'static str,
    check: fn(&Lifetime, u32) -> bool,
}

pub const BADGES: &[Badge] = &[
    Badge {
        id: "first_blood",
        name: "First Blood",
        how: "win a boss fight",
        check: |l, _| l.bosses_won >= 1,
    },
    Badge {
        id: "flawless",
        name: "Flawless",
        how: "beat a boss on the first try",
        check: |l, _| l.first_tries >= 1,
    },
    Badge {
        id: "never_give_up",
        name: "Never Give Up",
        how: "win after failing 3 times",
        check: |l, _| l.comebacks >= 1,
    },
    Badge {
        id: "on_a_roll",
        name: "On a Roll",
        how: "win 5 boss fights in a row",
        check: |l, _| l.best_streak >= 5,
    },
    Badge {
        id: "boss_hunter",
        name: "Boss Hunter",
        how: "win 10 boss fights",
        check: |l, _| l.bosses_won >= 10,
    },
    Badge {
        id: "legend",
        name: "Legend",
        how: "win 100 boss fights",
        check: |l, _| l.bosses_won >= 100,
    },
    Badge {
        id: "treasure_hunter",
        name: "Treasure Hunter",
        how: "find an epic item",
        check: |l, _| l.loot.iter().any(|i| i.rarity == Rarity::Epic),
    },
    Badge {
        id: "shadow_lord",
        name: "Shadow Lord",
        how: "raise 10 shadows",
        check: |l, _| l.shadows >= 10,
    },
    Badge {
        id: "bookworm",
        name: "Bookworm",
        how: "read or search 100 times",
        check: |l, _| l.reads >= 100,
    },
    Badge {
        id: "blacksmith",
        name: "Blacksmith",
        how: "make 100 edits",
        check: |l, _| l.edits >= 100,
    },
    Badge {
        id: "shell_shocked",
        name: "Shell Shocked",
        how: "run 100 shell commands",
        check: |l, _| l.shells >= 100,
    },
    Badge {
        id: "well_rested",
        name: "Well Rested",
        how: "rest 5 times",
        check: |l, _| l.rests >= 5,
    },
    Badge {
        id: "adventurer",
        name: "Adventurer",
        how: "finish 50 quests",
        check: |l, _| l.quests >= 50,
    },
    Badge {
        id: "saga",
        name: "Saga",
        how: "reach chapter 10",
        check: |l, _| l.chapters >= 9,
    },
    Badge {
        id: "veteran",
        name: "Veteran",
        how: "reach level 10",
        check: |_, level| level >= 10,
    },
    Badge {
        id: "force_pusher",
        name: "Force Pusher",
        how: "git push --force (the Duck saw)",
        check: |l, _| l.force_pushes >= 1,
    },
];

/// Badges earned but not yet in `life.badges`.
pub fn newly_earned(life: &Lifetime, level: u32) -> Vec<&'static Badge> {
    BADGES
        .iter()
        .filter(|b| !life.badges.iter().any(|id| id == b.id) && (b.check)(life, level))
        .collect()
}

pub fn by_id(id: &str) -> Option<&'static Badge> {
    BADGES.iter().find(|b| b.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earned_once() {
        let mut l = Lifetime {
            bosses_won: 1,
            first_tries: 1,
            ..Lifetime::default()
        };
        let ids: Vec<_> = newly_earned(&l, 1).iter().map(|b| b.id).collect();
        assert_eq!(ids, ["first_blood", "flawless"]);
        l.badges = ids.iter().map(|s| s.to_string()).collect();
        assert!(newly_earned(&l, 1).is_empty());
        assert_eq!(newly_earned(&l, 10)[0].id, "veteran");
    }

    #[test]
    fn ids_are_unique() {
        let ids: std::collections::HashSet<_> = BADGES.iter().map(|b| b.id).collect();
        assert_eq!(ids.len(), BADGES.len());
    }
}
