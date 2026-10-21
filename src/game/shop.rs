//! The town shop and inn (TASKS M26): gear for gold, and a night's rest. Pure.

use super::loot::{Item, Rarity, Slot};

pub struct Ware {
    pub name: &'static str,
    pub slot: Slot,
    /// Index into STR/INT/DEX/VIT/LUK.
    pub stat: usize,
    pub bonus: u32,
    pub price: u32,
}

/// What the inn charges for the Well Rested buff.
pub const INN_PRICE: u32 = 30;

/// The three wares of a town's shop, by theme.
pub fn stock(theme: &str) -> [Ware; 3] {
    let w = |name, slot, stat, bonus, price| Ware {
        name,
        slot,
        stat,
        bonus,
        price,
    };
    match theme {
        "Forge Village" => [
            w("Steel Sword", Slot::Weapon, 0, 3, 120),
            w("Iron Mail", Slot::Armor, 3, 3, 140),
            w("Smith's Ring", Slot::Charm, 2, 2, 90),
        ],
        "Lantern City" => [
            w("Silver Dagger", Slot::Weapon, 0, 2, 80),
            w("Lantern Cloak", Slot::Armor, 3, 2, 90),
            w("Moon Amulet", Slot::Charm, 1, 3, 130),
        ],
        "Harbor Town" => [
            w("Harpoon", Slot::Weapon, 0, 3, 110),
            w("Sailor's Coat", Slot::Armor, 3, 2, 80),
            w("Lucky Shell", Slot::Charm, 4, 3, 120),
        ],
        "Serpent Grove" => [
            w("Fang Spear", Slot::Weapon, 0, 4, 160),
            w("Scale Mail", Slot::Armor, 3, 4, 170),
            w("Serpent Ring", Slot::Charm, 2, 3, 130),
        ],
        _ => [
            w("Oak Staff", Slot::Weapon, 0, 2, 60),
            w("Leather Vest", Slot::Armor, 3, 2, 60),
            w("Clover Charm", Slot::Charm, 4, 2, 70),
        ],
    }
}

impl Ware {
    /// The item it becomes once bought (shop gear counts as rare).
    pub fn item(&self) -> Item {
        Item {
            name: self.name.to_string(),
            slot: self.slot,
            rarity: Rarity::Rare,
            stat: self.stat,
            bonus: self.bonus,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_town_sells_three_slots() {
        for theme in [
            "Forge Village",
            "Lantern City",
            "Harbor Town",
            "Serpent Grove",
            "Meadow Hamlet",
        ] {
            let s = stock(theme);
            assert_eq!(
                s.each_ref().map(|w| w.slot),
                [Slot::Weapon, Slot::Armor, Slot::Charm]
            );
            assert!(s.iter().all(|w| w.price > 0 && w.bonus > 0));
        }
    }
}
