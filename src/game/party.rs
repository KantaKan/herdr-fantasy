//! Live party from herdr agents (SPEC §6.2): who sits in which seat, arrivals load their
//! saved progress, departures save it and walk off.

use std::collections::BTreeMap;

use super::hero::{Class, Hero, Seat, Status};
use super::persist::Store;
use super::repo::Repo;
use super::world::{Finished, World};

/// What the herdr side tells us about one agent, already resolved to a repo.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveAgent {
    pub pane_id: String,
    /// herdr agent kind, e.g. "claude"; drives class and hero key.
    pub kind: String,
    /// Display label, e.g. "Claude".
    pub name: String,
    pub status: Status,
    pub title: Option<String>,
    pub repo: Repo,
    pub focused: bool,
}

fn roman(n: usize) -> String {
    ["", " II", " III", " IV", " V", " VI", " VII", " VIII"]
        .get(n)
        .map_or(format!(" {}", n + 1), |s| s.to_string())
}

/// Save one hero's level and XP (keeps its lifetime counters).
pub fn save_hero(h: &Hero, store: &mut Store) {
    if let Some(seat) = &h.seat {
        let mut p = store.get(&seat.repo.key, &seat.key());
        p.level = h.level;
        p.xp = h.xp;
        store.put(&seat.repo.key, &seat.key(), p);
    }
}

/// Reconcile the world's heroes with herdr's live agents. Returns the names of heroes
/// that just went `blocked` (not ones that arrived already blocked).
pub fn sync(
    w: &mut World,
    agents: &[LiveAgent],
    classes: &BTreeMap<String, String>,
    store: &mut Store,
) -> Vec<String> {
    let mut newly_blocked = Vec::new();
    // Departures: save, close the quest as unfinished, walk off.
    for h in &mut w.heroes {
        let Some(seat) = &h.seat else { continue };
        if h.leaving_ms.is_some() || agents.iter().any(|a| a.pane_id == seat.pane_id) {
            continue;
        }
        save_hero(h, store);
        if let Some(quest) = h.end_quest("unfinished") {
            w.finished.push(Finished {
                hero: h.name.clone(),
                seat: h.seat.clone(),
                quest,
            });
        }
        h.set_status(Status::Idle);
        h.leaving_ms = Some(0);
        w.save_now = true;
    }

    for a in agents {
        let here = w.heroes.iter_mut().find(|h| {
            h.leaving_ms.is_none() && h.seat.as_ref().is_some_and(|s| s.pane_id == a.pane_id)
        });
        let was = here.as_ref().map(|h| h.status);
        let h = match here {
            Some(h) => h,
            None => {
                // Lowest slot not taken by a live hero of the same kind in the same repo.
                let taken: Vec<usize> = w
                    .heroes
                    .iter()
                    .filter(|h| h.leaving_ms.is_none())
                    .filter_map(|h| h.seat.as_ref())
                    .filter(|s| s.kind == a.kind && s.repo == a.repo)
                    .map(|s| s.slot)
                    .collect();
                let slot = (0..).find(|n| !taken.contains(n)).unwrap();
                let seat = Seat {
                    pane_id: a.pane_id.clone(),
                    repo: a.repo.clone(),
                    kind: a.kind.clone(),
                    slot,
                };
                let p = store.get(&seat.repo.key, &seat.key());
                let mut hero = Hero::new(
                    &format!("{}{}", a.name, roman(slot)),
                    Class::for_agent(classes, &a.kind),
                );
                hero.level = p.level.max(1);
                hero.xp = p.xp;
                hero.seat = Some(seat);
                w.heroes.push(hero);
                w.heroes.last_mut().unwrap()
            }
        };
        if a.status == Status::Blocked && was.is_some_and(|s| s != Status::Blocked) {
            newly_blocked.push(h.name.clone());
        }
        h.set_status(a.status);
        h.focused = a.focused;
        if a.title.is_some() {
            h.title.clone_from(&a.title);
        }
    }

    // Top bar shows the repo most heroes are in (SPEC §5.2).
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for s in w.heroes.iter().filter_map(|h| h.seat.as_ref()) {
        *counts.entry(&s.repo.name).or_default() += 1;
    }
    if let Some((name, _)) = counts.iter().max_by_key(|(_, n)| **n) {
        w.repo = name.to_string();
    }
    newly_blocked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::persist::Progress;

    fn store(name: &str) -> Store {
        let d = std::env::temp_dir().join(format!("hf-party-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        Store::open(d)
    }

    fn agent(pane: &str, kind: &str, status: Status) -> LiveAgent {
        LiveAgent {
            pane_id: pane.into(),
            kind: kind.into(),
            name: kind[..1].to_uppercase() + &kind[1..],
            status,
            title: Some(format!("work in {pane}")),
            repo: Repo::from_identity("github.com/me/app"),
            focused: false,
        }
    }

    fn classes() -> BTreeMap<String, String> {
        crate::config::Config::default().classes
    }

    #[test]
    fn agents_become_heroes_with_slots_and_classes() {
        let mut w = World::new("", "");
        let mut s = store("slots");
        let agents = [
            agent("p1", "claude", Status::Working),
            agent("p2", "claude", Status::Idle),
            agent("p3", "codex", Status::Blocked),
        ];
        sync(&mut w, &agents, &classes(), &mut s);
        let names: Vec<_> = w.heroes.iter().map(|h| h.name.as_str()).collect();
        assert_eq!(names, ["Claude", "Claude II", "Codex"]);
        assert_eq!(w.heroes[2].class, Class::Mage);
        assert_eq!(w.heroes[2].status, Status::Blocked);
        assert_eq!(w.repo, "app");
    }

    #[test]
    fn freed_slot_is_reused_by_the_next_arrival() {
        let mut w = World::new("", "");
        let mut s = store("reuse");
        let both = [
            agent("p1", "claude", Status::Idle),
            agent("p2", "claude", Status::Idle),
        ];
        sync(&mut w, &both, &classes(), &mut s);
        sync(&mut w, &both[1..], &classes(), &mut s); // p1 leaves
        assert!(w.heroes[0].leaving_ms.is_some());
        sync(
            &mut w,
            &[both[1].clone(), agent("p9", "claude", Status::Idle)],
            &classes(),
            &mut s,
        );
        let p9 = w
            .heroes
            .iter()
            .find(|h| h.seat.as_ref().unwrap().pane_id == "p9")
            .unwrap();
        assert_eq!(
            (p9.name.as_str(), p9.seat.as_ref().unwrap().slot),
            ("Claude", 0)
        );
    }

    #[test]
    fn progress_persists_across_departure_and_return() {
        let mut w = World::new("", "");
        let mut s = store("persist");
        let a = agent("p1", "claude", Status::Working);
        sync(&mut w, std::slice::from_ref(&a), &classes(), &mut s);
        w.heroes[0].level = 7;
        w.heroes[0].xp = 42;
        sync(&mut w, &[], &classes(), &mut s); // agent closed
        let key = Seat {
            pane_id: String::new(),
            repo: a.repo.clone(),
            kind: "claude".into(),
            slot: 0,
        }
        .key();
        assert_eq!(
            s.get(&a.repo.key, &key),
            Progress {
                level: 7,
                xp: 42,
                quests: 0
            }
        );
        assert!(w.save_now);

        let mut fresh = World::new("", "");
        sync(
            &mut fresh,
            &[agent("p5", "claude", Status::Idle)],
            &classes(),
            &mut s,
        );
        assert_eq!((fresh.heroes[0].level, fresh.heroes[0].xp), (7, 42));
    }

    #[test]
    fn closing_mid_quest_records_unfinished() {
        let mut w = World::new("", "");
        let mut s = store("abandon");
        sync(
            &mut w,
            &[agent("p1", "claude", Status::Working)],
            &classes(),
            &mut s,
        );
        w.update(1);
        assert!(w.heroes[0].quest.is_some());
        sync(&mut w, &[], &classes(), &mut s);
        assert_eq!(w.finished.len(), 1);
        assert_eq!(w.finished[0].quest.result, "unfinished");
    }

    #[test]
    fn only_transitions_into_blocked_notify() {
        let mut w = World::new("", "");
        let mut s = store("notify");
        let arrive = sync(
            &mut w,
            &[agent("p1", "claude", Status::Blocked)],
            &classes(),
            &mut s,
        );
        assert!(arrive.is_empty(), "already blocked on arrival: no toast");
        sync(
            &mut w,
            &[agent("p1", "claude", Status::Working)],
            &classes(),
            &mut s,
        );
        let blocked = sync(
            &mut w,
            &[agent("p1", "claude", Status::Blocked)],
            &classes(),
            &mut s,
        );
        assert_eq!(blocked, ["Claude"]);
        let again = sync(
            &mut w,
            &[agent("p1", "claude", Status::Blocked)],
            &classes(),
            &mut s,
        );
        assert!(again.is_empty(), "one toast per transition");
    }

    #[test]
    fn roman_suffixes() {
        assert_eq!(roman(0), "");
        assert_eq!(roman(2), " III");
        assert_eq!(roman(20), " 21");
    }
}
