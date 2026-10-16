//! Live party from herdr agents (SPEC §6.2): who sits in which seat, arrivals load their
//! saved progress, departures save it and walk off.

use std::collections::BTreeMap;

use super::boss;
use super::hero::{Action, Class, Hero, Seat, Status};
use super::moves;
use super::persist::Store;
use super::repo::Repo;
use super::world::{Finished, GameEvent, World};
use crate::config::Bosses;
use crate::transcript::claude::Entry;

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
        // A real prompt names the quest better than herdr's terminal title.
        if a.title.is_some() && !h.title_from_prompt {
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

/// First line of a prompt, at most 60 chars (SPEC §5.6).
pub fn prompt_title(prompt: &str) -> String {
    let first = prompt
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let mut t: String = first.chars().take(60).collect();
    if first.chars().count() > 60 {
        t.push('…');
    }
    t
}

/// Context window for a model (SPEC §7.4): config map, else `default`; usage above the
/// assumed size bumps it to the 1M tier instead of showing more than 100%.
pub fn context_pct(context: u64, model: Option<&str>, windows: &BTreeMap<String, u64>) -> u8 {
    let mut window = model
        .and_then(|m| windows.get(m))
        .or_else(|| windows.get("default"))
        .copied()
        .unwrap_or(200_000);
    if context > window {
        window = window.max(1_000_000);
    }
    ((context * 100) / window.max(1)).min(100) as u8
}

/// Things the transcript tells us about one agent.
pub struct Feed<'a> {
    pub pane_id: &'a str,
    pub entries: &'a [Entry],
    /// Lines read on first sight: learn state from them, don't animate them.
    pub history: bool,
    /// The agent started a new session (`/clear`, new run): a new Chapter.
    pub new_chapter: bool,
}

pub fn feed(w: &mut World, f: Feed, windows: &BTreeMap<String, u64>, bosses: &Bosses) {
    let Some(i) = w.heroes.iter().position(|h| {
        h.leaving_ms.is_none() && h.seat.as_ref().is_some_and(|s| s.pane_id == f.pane_id)
    }) else {
        return;
    };
    let finish = |w: &mut World| {
        let h = &mut w.heroes[i];
        if let Some(quest) = h.finish_quest() {
            let done = Finished {
                hero: h.name.clone(),
                seat: h.seat.clone(),
                quest,
            };
            w.finished.push(done);
            w.save_now = true;
        }
    };
    if f.new_chapter {
        finish(w);
        let h = &mut w.heroes[i];
        h.title_from_prompt = false;
        h.boss_tool = None;
        h.say("— New Run —".into(), 'y');
    }
    for e in f.entries {
        match e {
            Entry::Prompt(p) => {
                // A new prompt closes the previous quest; the next work starts a new one.
                if !f.history {
                    finish(w);
                }
                let h = &mut w.heroes[i];
                h.title = Some(prompt_title(p));
                h.title_from_prompt = true;
            }
            Entry::Tool { id, name, input } if !f.history => {
                let command = input["command"].as_str().unwrap_or("");
                let boss = matches!(name.as_str(), "Bash" | "PowerShell")
                    .then(|| boss::classify(command, bosses))
                    .flatten();
                if let Some(kind) = boss {
                    w.apply(GameEvent::BossFight(i, kind, command.to_string()));
                    w.heroes[i].boss_tool = Some(id.clone());
                }
                let boss_name = w.boss.as_ref().filter(|_| boss.is_some()).map(|b| b.name);
                let h = &mut w.heroes[i];
                if let Some(m) = moves::tool_move(name, input) {
                    let detail = match boss_name {
                        Some(b) => format!("fighting the {b}"),
                        None => m.detail,
                    };
                    let action = if boss_name.is_some() {
                        Action::Cast
                    } else {
                        m.action
                    };
                    h.act(action, Some(detail));
                    if let Some((text, color)) = m.float
                        && h.status == Status::Working
                    {
                        h.say(text, color);
                    }
                }
            }
            Entry::ToolResult { id, is_error, text } if !f.history => {
                if w.heroes[i].boss_tool.as_ref() == Some(id) {
                    w.heroes[i].boss_tool = None;
                    w.apply(GameEvent::BossResult(i, boss::failed(*is_error, text)));
                }
            }
            Entry::Usage { context, model } => {
                w.heroes[i].context_used = Some(context_pct(*context, model.as_deref(), windows));
            }
            Entry::Compact { post_tokens } => {
                let h = &mut w.heroes[i];
                if let Some(t) = post_tokens {
                    h.context_used = Some(context_pct(*t, None, windows));
                }
                if !f.history {
                    h.say("Well rested!".into(), 'C');
                }
            }
            _ => {}
        }
    }
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

    fn fed(w: &mut World, entries: &[Entry], history: bool) {
        let cfg = crate::config::Config::default();
        feed(
            w,
            Feed {
                pane_id: "p1",
                entries,
                history,
                new_chapter: false,
            },
            &cfg.context_window,
            &cfg.bosses,
        );
    }

    fn tool(name: &str, input: serde_json::Value) -> Entry {
        Entry::Tool {
            id: "t".into(),
            name: name.into(),
            input,
        }
    }

    #[test]
    fn transcript_drives_moves_titles_and_stamina() {
        let mut w = World::new("", "");
        let mut s = store("feed");
        sync(
            &mut w,
            &[agent("p1", "claude", Status::Working)],
            &classes(),
            &mut s,
        );
        fed(
            &mut w,
            &[
                Entry::Prompt("fix the login redirect bug\nit loops".into()),
                Entry::Usage {
                    context: 100_000,
                    model: None,
                },
                tool(
                    "Edit",
                    serde_json::json!({"file_path": "src/auth.rs", "new_string": "a\nb"}),
                ),
            ],
            false,
        );
        let h = &w.heroes[0];
        assert_eq!(h.title.as_deref(), Some("fix the login redirect bug"));
        assert_eq!(h.context_used, Some(50));
        assert_eq!(h.anim(), crate::game::hero::Anim::Attack);
        assert_eq!(h.float.as_ref().unwrap().0, "-2");
        // herdr's terminal title no longer overrides the prompt title
        sync(
            &mut w,
            &[agent("p1", "claude", Status::Working)],
            &classes(),
            &mut s,
        );
        assert_eq!(
            w.heroes[0].title.as_deref(),
            Some("fix the login redirect bug")
        );
    }

    #[test]
    fn history_sets_state_but_never_animates() {
        let mut w = World::new("", "");
        let mut s = store("history");
        sync(
            &mut w,
            &[agent("p1", "claude", Status::Working)],
            &classes(),
            &mut s,
        );
        fed(
            &mut w,
            &[
                tool("Read", serde_json::json!({"file_path": "a.rs"})),
                Entry::Usage {
                    context: 20_000,
                    model: None,
                },
            ],
            true,
        );
        assert_eq!(
            w.heroes[0].anim(),
            crate::game::hero::Anim::Walk,
            "no replay"
        );
        assert_eq!(w.heroes[0].context_used, Some(10));
    }

    #[test]
    fn new_prompt_completes_the_previous_quest() {
        let mut w = World::new("", "");
        let mut s = store("prompt-quest");
        sync(
            &mut w,
            &[agent("p1", "claude", Status::Working)],
            &classes(),
            &mut s,
        );
        w.update(1);
        assert!(w.heroes[0].quest.is_some());
        fed(&mut w, &[Entry::Prompt("next thing".into())], false);
        assert_eq!(w.finished.len(), 1);
        assert_eq!(w.finished[0].quest.result, "complete");
    }

    fn result(id: &str, is_error: bool) -> Entry {
        Entry::ToolResult {
            id: id.into(),
            is_error,
            text: String::new(),
        }
    }

    #[test]
    fn m5_cargo_test_fail_then_pass_from_the_transcript() {
        let mut w = World::new("", "");
        let mut s = store("boss");
        sync(
            &mut w,
            &[agent("p1", "claude", Status::Working)],
            &classes(),
            &mut s,
        );
        let run = |id: &str| Entry::Tool {
            id: id.into(),
            name: "Bash".into(),
            input: serde_json::json!({"command": "cargo test 2>&1 | tail -5"}),
        };
        fed(&mut w, &[run("t1")], false);
        let b = w.boss.as_ref().expect("Test boss summoned");
        assert_eq!((b.kind, b.hp), (boss::BossKind::Test, 100));
        fed(&mut w, &[result("other", true)], false);
        assert_eq!(
            w.boss.as_ref().unwrap().hp,
            100,
            "unrelated results don't count"
        );
        fed(&mut w, &[result("t1", true)], false);
        assert_eq!(w.boss.as_ref().unwrap().hp, 75);
        assert_eq!(
            w.boss.as_ref().unwrap().state,
            crate::game::world::BossState::Laugh
        );
        let xp = w.heroes[0].xp;
        fed(&mut w, &[run("t2"), result("t2", false)], false);
        assert_eq!(w.boss.as_ref().unwrap().hp, 0);
        assert_eq!(w.heroes[0].xp - xp, boss::VICTORY_XP);
    }

    #[test]
    fn a_pipe_cannot_hide_a_failing_test() {
        let mut w = World::new("", "");
        let mut s = store("pipe");
        sync(
            &mut w,
            &[agent("p1", "claude", Status::Working)],
            &classes(),
            &mut s,
        );
        let entries = [
            Entry::Tool {
                id: "t1".into(),
                name: "Bash".into(),
                input: serde_json::json!({"command": "cargo test 2>&1 | tail -3"}),
            },
            Entry::ToolResult {
                id: "t1".into(),
                is_error: false,
                text: "test result: FAILED. 1 passed; 1 failed".into(),
            },
        ];
        fed(&mut w, &entries, false);
        assert_eq!(w.boss.as_ref().unwrap().hp, 75, "counted as a failure");
        assert!(
            w.heroes[0]
                .detail
                .as_deref()
                .unwrap()
                .starts_with("fighting the ")
        );
    }

    #[test]
    fn context_tiers() {
        let windows = crate::config::Config::default().context_window;
        assert_eq!(context_pct(100_000, None, &windows), 50);
        assert_eq!(
            context_pct(500_000, Some("claude-opus-5-5"), &windows),
            50,
            "bumped to 1M"
        );
        assert_eq!(context_pct(5_000_000, None, &windows), 100);
        assert_eq!(
            prompt_title(&"x".repeat(70)),
            format!("{}…", "x".repeat(60))
        );
    }

    #[test]
    fn roman_suffixes() {
        assert_eq!(roman(0), "");
        assert_eq!(roman(2), " III");
        assert_eq!(roman(20), " 21");
    }
}
