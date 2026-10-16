//! Loot (TASKS M11): a boss victory may drop an item. Pure and seeded, so the same win
//! always drops the same thing.

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use super::boss::Tier;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Slot {
    Weapon,
    Armor,
    Charm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Rarity {
    Common,
    Rare,
    Epic,
}

impl Rarity {
    /// Palette key for the item name.
    pub fn color(self) -> char {
        match self {
            Rarity::Common => 's',
            Rarity::Rare => 'c',
            Rarity::Epic => 'y',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub slot: Slot,
    pub rarity: Rarity,
    /// Index into STR/INT/DEX/VIT/LUK.
    pub stat: usize,
    pub bonus: u32,
}

pub const STAT_NAMES: [&str; 5] = ["STR", "INT", "DEX", "VIT", "LUK"];

const MATERIAL: [&[&str]; 3] = [
    &["Rusty", "Wooden", "Leather", "Patched"],
    &["Steel", "Silver", "Enchanted", "Polished"],
    &["Mythril", "Dragonbone", "Legendary", "Cursed"],
];
const WEAPONS: &[&str] = &["Sword", "Axe", "Dagger", "Staff", "Hammer", "Bow"];
const ARMOR: &[&str] = &["Cape", "Helm", "Boots", "Gauntlets", "Mail", "Shield"];
const CHARMS: &[&str] = &["Amulet", "Ring", "Rubber Duck", "Lucky Coin", "Talisman"];

/// What the boss leaves behind, by species id.
fn suffix(species: &str) -> &'static str {
    match species {
        "test_golem" => "of Green Tests",
        "snapshot_mimic" => "of Fresh Snapshots",
        "flaky_ghost" => "of Steady Tests",
        "build_wyrm" => "of Clean Builds",
        "linker_kraken" => "of Resolved Symbols",
        "dependency_hydra" => "of Locked Dependencies",
        "lint_imp" => "of Zero Warnings",
        "typo_gremlin" => "of Perfect Spelling",
        "slime_king" => "of the Slime King",
        "sahur_knocker" => "of the Early Sahur",
        _ => "of Victory",
    }
}

/// Stable across runs and platforms.
fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

/// The drop for `hero`'s `win`-th boss victory (1-based), if any. The first win always
/// drops; bigger bosses drop more often and better.
pub fn roll(hero: &str, win: u32, species: &str, tier: Tier) -> Option<Item> {
    let mut rng = ChaCha8Rng::seed_from_u64(fnv1a(hero) ^ (win as u64).wrapping_mul(0x9e37));
    let (chance, epic) = match tier {
        Tier::Little => (0.25, 0.03),
        Tier::Normal => (0.35, 0.06),
        Tier::Big => (0.5, 0.12),
    };
    if win > 1 && rng.random::<f64>() >= chance {
        return None;
    }
    let r = rng.random::<f64>();
    let rarity = if r < epic {
        Rarity::Epic
    } else if r < epic + 0.25 {
        Rarity::Rare
    } else {
        Rarity::Common
    };
    let slot = [Slot::Weapon, Slot::Armor, Slot::Charm][rng.random_range(0..3)];
    let (pieces, stat) = match slot {
        Slot::Weapon => (WEAPONS, 0),
        Slot::Armor => (ARMOR, 3),
        Slot::Charm => (CHARMS, [1, 2, 4][rng.random_range(0..3)]),
    };
    let materials = MATERIAL[rarity as usize];
    let name = format!(
        "{} {} {}",
        materials[rng.random_range(0..materials.len())],
        pieces[rng.random_range(0..pieces.len())],
        suffix(species)
    );
    let bonus = match rarity {
        Rarity::Common => 1,
        Rarity::Rare => rng.random_range(2..=3),
        Rarity::Epic => rng.random_range(4..=6),
    };
    Some(Item {
        name,
        slot,
        rarity,
        stat,
        bonus,
    })
}

/// The best item in each slot (rarity, then bonus), in slot order.
pub fn equipped(items: &[Item]) -> Vec<&Item> {
    [Slot::Weapon, Slot::Armor, Slot::Charm]
        .iter()
        .filter_map(|&s| {
            items
                .iter()
                .filter(|i| i.slot == s)
                .max_by_key(|i| (i.rarity, i.bonus))
        })
        .collect()
}

/// Stat bonus from equipped gear.
pub fn gear_bonus(items: &[Item]) -> [u32; 5] {
    let mut b = [0; 5];
    for i in equipped(items) {
        b[i.stat] += i.bonus;
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_win_always_drops_and_rolls_are_stable() {
        let a = roll("Claude", 1, "test_golem", Tier::Little).expect("first win drops");
        assert_eq!(
            Some(a.clone()),
            roll("Claude", 1, "test_golem", Tier::Little)
        );
        assert!(a.name.ends_with("of Green Tests"), "{}", a.name);
    }

    #[test]
    fn drop_rates_follow_size() {
        let drops = |t| {
            (2..2002)
                .filter(|&w| roll("H", w, "x", t).is_some())
                .count()
        };
        let (little, big) = (drops(Tier::Little), drops(Tier::Big));
        assert!((400..600).contains(&little), "little {little}");
        assert!((900..1100).contains(&big), "big {big}");
    }

    #[test]
    fn best_per_slot_is_equipped() {
        let item = |slot, rarity, stat, bonus| Item {
            name: String::new(),
            slot,
            rarity,
            stat,
            bonus,
        };
        let bag = [
            item(Slot::Weapon, Rarity::Common, 0, 1),
            item(Slot::Weapon, Rarity::Rare, 0, 3),
            item(Slot::Charm, Rarity::Common, 4, 1),
        ];
        assert_eq!(equipped(&bag).len(), 2);
        assert_eq!(gear_bonus(&bag), [3, 0, 0, 0, 1]);
    }
}
