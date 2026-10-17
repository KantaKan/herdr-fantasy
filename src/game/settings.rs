//! In-game settings (TASKS M18): the rows of the `s` screen and what changing one does.
//! Pure: changes apply to the world and return the config edit to save.

use super::hero::Class;
use super::world::World;
use crate::config::{CLASSES, PETS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Class,
    Pet,
    Skins,
    Memes,
    ReducedMotion,
}

pub const ROWS: [Row; 5] = [
    Row::Class,
    Row::Pet,
    Row::Skins,
    Row::Memes,
    Row::ReducedMotion,
];

/// A change to write to config.toml.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// `[table] key = "value"`; `None` removes the key.
    Table {
        table: &'static str,
        key: String,
        value: Option<String>,
    },
    /// A top-level `key = true|false`.
    Flag { key: &'static str, value: bool },
}

/// The agent kind whose class/pet the selected hero's rows change (demo heroes: their
/// name, lowercased).
pub fn kind_of(w: &World, hero: usize) -> Option<String> {
    let h = w.heroes.get(hero)?;
    Some(
        h.seat
            .as_ref()
            .map_or_else(|| h.name.to_lowercase(), |s| s.kind.clone()),
    )
}

/// (label, value) for a row, as shown.
pub fn show(w: &World, hero: usize, row: Row) -> (String, String) {
    let on = |b: bool| if b { "on" } else { "off" }.to_string();
    let h = w.heroes.get(hero);
    let kind = kind_of(w, hero).unwrap_or_else(|| "—".into());
    match row {
        Row::Class => (
            format!("Class ({kind})"),
            h.map_or("—".into(), |h| h.class.name().to_string()),
        ),
        Row::Pet => (
            format!("Pet ({kind})"),
            h.and_then(|h| h.pet.clone())
                .unwrap_or_else(|| "none".into()),
        ),
        Row::Skins => ("Skins".into(), on(w.skins)),
        Row::Memes => ("Memes (Duck quips)".into(), on(w.memes)),
        Row::ReducedMotion => ("Reduced motion".into(), on(w.reduced_motion)),
    }
}

fn cycle<'a>(list: &[&'a str], current: &str, delta: i32) -> &'a str {
    let i = list.iter().position(|c| *c == current).unwrap_or(0) as i32;
    list[(i + delta).rem_euclid(list.len() as i32) as usize]
}

/// Change `row` by `delta` (±1) for the selected hero: applied to every hero of the same
/// agent kind, returned as the edit to save.
pub fn change(w: &mut World, hero: usize, row: Row, delta: i32) -> Option<Edit> {
    let kind = kind_of(w, hero)?;
    let same_kind = |w: &World, i: usize| kind_of(w, i).as_deref() == Some(kind.as_str());
    match row {
        Row::Class => {
            let next = cycle(&CLASSES, w.heroes[hero].class.name(), delta);
            w.classes.insert(kind.clone(), next.to_string());
            for i in 0..w.heroes.len() {
                if same_kind(w, i) {
                    w.heroes[i].class = Class::from_name(next);
                }
            }
            Some(Edit::Table {
                table: "classes",
                key: kind,
                value: Some(next.into()),
            })
        }
        Row::Pet => {
            let mut list = vec!["none"];
            list.extend(PETS);
            let cur = w.heroes[hero].pet.clone().unwrap_or_else(|| "none".into());
            let next = cycle(&list, &cur, delta);
            let pet = (next != "none").then(|| next.to_string());
            for i in 0..w.heroes.len() {
                if same_kind(w, i) {
                    w.heroes[i].pet.clone_from(&pet);
                }
            }
            w.pets.remove(&kind);
            if let Some(p) = &pet {
                w.pets.insert(kind.clone(), p.clone());
            }
            Some(Edit::Table {
                table: "pets",
                key: kind,
                value: pet,
            })
        }
        Row::Skins => {
            w.skins = !w.skins;
            Some(Edit::Flag {
                key: "skins",
                value: w.skins,
            })
        }
        Row::Memes => {
            w.memes = !w.memes;
            Some(Edit::Flag {
                key: "memes",
                value: w.memes,
            })
        }
        Row::ReducedMotion => {
            w.reduced_motion = !w.reduced_motion;
            Some(Edit::Flag {
                key: "reduced_motion",
                value: w.reduced_motion,
            })
        }
    }
}

/// A brand-new party (every hero Lv 1 with no quests yet): show the welcome card.
pub fn first_run(w: &World) -> bool {
    !w.heroes.is_empty()
        && w.heroes
            .iter()
            .all(|h| h.level <= 1 && h.xp == 0 && h.life.quests == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_and_pet_cycle_for_the_whole_kind() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("Claude", Class::Knight);
        let e = change(&mut w, a, Row::Class, 1).unwrap();
        assert_eq!(w.heroes[a].class, Class::Mage);
        assert_eq!(
            e,
            Edit::Table {
                table: "classes",
                key: "claude".into(),
                value: Some("Mage".into())
            }
        );
        change(&mut w, a, Row::Class, -2);
        assert_eq!(w.heroes[a].class, Class::WildCard, "wraps around");
        change(&mut w, a, Row::Pet, 1);
        assert_eq!(w.heroes[a].pet.as_deref(), Some("cat"));
        let e = change(&mut w, a, Row::Pet, -1).unwrap();
        assert_eq!(w.heroes[a].pet, None);
        assert!(matches!(e, Edit::Table { value: None, .. }));
    }

    #[test]
    fn flags_toggle_and_first_run() {
        let mut w = World::new("r", "t");
        let a = w.add_hero("A", Class::Knight);
        assert!(first_run(&w));
        assert_eq!(
            change(&mut w, a, Row::Memes, 1),
            Some(Edit::Flag {
                key: "memes",
                value: false
            })
        );
        assert!(!w.memes);
        w.heroes[a].life.quests = 1;
        assert!(!first_run(&w));
    }
}
