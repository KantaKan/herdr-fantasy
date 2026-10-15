//! Scripted scenarios (SPEC §12). Deterministic for a given seed and tick sequence.

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

use super::boss::BossKind;
use super::hero::{Action, Class, Status};
use super::world::{GameEvent as E, World};

/// Config `[classes]`: agent kind → class name.
pub type Classes = std::collections::BTreeMap<String, String>;

/// SPEC §6.2 class mapping; unknown kinds use `default`.
pub fn class_of(classes: &Classes, agent: &str) -> Class {
    let name = classes.get(agent).or_else(|| classes.get("default"));
    Class::from_name(name.map_or("Bard", String::as_str))
}

pub struct Demo {
    rng: ChaCha8Rng,
    t: u64,
    idx: usize,
    script: Vec<(u64, E)>,
    loop_ms: u64,
    next_act: Vec<u64>,
    loops: usize,
}

/// One boss command per demo loop, so the roster and size tiers all show up.
const BOSSES: [(BossKind, &str); 6] = [
    (BossKind::Test, "cargo test"),
    (BossKind::Build, "cargo build --workspace"),
    (BossKind::Lint, "cargo clippy src/auth.rs"),
    (BossKind::Test, "pytest -k login"),
    (BossKind::Build, "tsc"),
    (BossKind::Lint, "eslint --all"),
];

/// Placeholder detail in the script, replaced by "running <this loop's boss command>".
const BOSS_CMD: &str = "{boss}";

const FILES: [&str; 5] = [
    "src/auth.rs",
    "src/session.rs",
    "src/routes/login.rs",
    "tests/redirect.rs",
    "Cargo.toml",
];

pub fn scenario(name: &str, seed: u64, classes: &Classes) -> Option<(World, Demo)> {
    match name {
        "party" => Some(party(seed, classes)),
        _ => None,
    }
}

/// 3 heroes; reads, edits, a Test Golem that fails twice then dies, one hero blocked,
/// stamina rising, quest complete. Loops every 42 s.
pub fn party(seed: u64, classes: &Classes) -> (World, Demo) {
    let mut w = World::new("demo-repo", "Forge Village");
    let (k, m, r) = (
        w.add_hero("Claude", class_of(classes, "claude")),
        w.add_hero("Codex", class_of(classes, "codex")),
        w.add_hero("OpenCode", class_of(classes, "opencode")),
    );
    let test = || Some(BOSS_CMD.to_string());
    let script = vec![
        (0, E::QuestStart("fix login redirect bug".into())),
        (0, E::Status(k, Status::Working)),
        (0, E::Status(m, Status::Working)),
        (0, E::Status(r, Status::Idle)),
        (0, E::ContextUsed(k, Some(35))),
        (8_000, E::BossEnter(BossKind::Test, String::new())),
        (8_000, E::Act(k, Action::Cast, test())),
        (9_500, E::BossFail),
        (9_500, E::Xp(k, 5)),
        (12_000, E::Status(m, Status::Blocked)),
        (14_000, E::Act(k, Action::Cast, test())),
        (15_500, E::BossFail),
        (
            18_000,
            E::Act(k, Action::Attack, Some("editing src/auth.rs".into())),
        ),
        (20_000, E::Act(k, Action::Cast, test())),
        (21_500, E::BossDefeat),
        (21_500, E::Xp(k, 60)),
        (22_000, E::Status(m, Status::Working)),
        (24_000, E::Status(r, Status::Working)),
        (24_000, E::ContextUsed(k, Some(72))),
        (30_000, E::ContextUsed(k, Some(91))),
        (34_000, E::Status(k, Status::Done)),
        (34_000, E::Status(m, Status::Done)),
        (34_000, E::Xp(k, 25)),
        (34_000, E::Xp(m, 25)),
        (36_000, E::Status(r, Status::Idle)),
        (38_000, E::Status(k, Status::Idle)),
        (38_000, E::Status(m, Status::Idle)),
        (38_000, E::QuestEnd),
        (38_000, E::ContextUsed(k, Some(12))),
    ];
    let demo = Demo {
        rng: ChaCha8Rng::seed_from_u64(seed),
        t: 0,
        idx: 0,
        script,
        loop_ms: 42_000,
        next_act: vec![1_000; w.heroes.len()],
        loops: 0,
    };
    (w, demo)
}

impl Demo {
    /// Advance demo time by `dt` ms: fire due script events, sprinkle random actions on
    /// working heroes, then tick the world.
    pub fn advance(&mut self, dt: u64, w: &mut World) {
        self.t += dt;
        while let Some((at, ev)) = self.script.get(self.idx) {
            if *at > self.t {
                break;
            }
            let (kind, cmd) = BOSSES[self.loops % BOSSES.len()];
            let ev = match ev.clone() {
                E::BossEnter(..) => E::BossEnter(kind, cmd.to_string()),
                E::Act(i, a, Some(d)) if d == BOSS_CMD => {
                    E::Act(i, a, Some(format!("running {cmd}")))
                }
                ev => ev,
            };
            w.apply(ev);
            self.idx += 1;
        }
        for i in 0..w.heroes.len() {
            if self.t < self.next_act[i] {
                continue;
            }
            self.next_act[i] = self.t + self.rng.random_range(900..2200);
            if w.heroes[i].status != Status::Working || w.heroes[i].busy() {
                continue;
            }
            let file = FILES[self.rng.random_range(0..FILES.len())];
            let (action, detail) = match self.rng.random_range(0..3) {
                0 => (Action::Attack, format!("editing {file}")),
                1 => (Action::Open, format!("reading {file}")),
                _ => (Action::Cast, "running a command".to_string()),
            };
            w.apply(E::Act(i, action, Some(detail)));
            w.apply(E::Xp(i, 2));
        }
        w.update(dt);
        if self.t >= self.loop_ms {
            self.t -= self.loop_ms;
            self.idx = 0;
            self.loops += 1;
            self.next_act.iter_mut().for_each(|n| *n = self.t + 1_000);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::world::BossState;

    fn run(seed: u64, ticks: u32) -> World {
        let (mut w, mut d) = party(seed, &crate::config::Config::default().classes);
        for _ in 0..ticks {
            d.advance(66, &mut w);
        }
        w
    }

    #[test]
    fn same_seed_same_story() {
        let (a, b) = (run(1, 300), run(1, 300));
        for (x, y) in a.heroes.iter().zip(&b.heroes) {
            assert_eq!((x.xp, x.anim(), &x.detail), (y.xp, y.anim(), &y.detail));
        }
    }

    #[test]
    fn golem_fails_twice_then_dies() {
        let w = run(1, 10_000 / 66);
        assert_eq!(w.boss.as_ref().unwrap().state, BossState::Laugh);
        let w = run(1, 16_000 / 66);
        assert_eq!(w.boss.as_ref().unwrap().fails, 2);
        assert!(run(1, 24_000 / 66).boss.is_none());
    }

    #[test]
    fn class_mapping_uses_config_then_default() {
        let mut c = crate::config::Config::default().classes;
        assert_eq!(class_of(&c, "claude"), Class::Knight);
        assert_eq!(class_of(&c, "grok"), Class::Bard);
        c.insert("default".into(), "Cleric".into());
        assert_eq!(class_of(&c, "grok"), Class::Cleric);
    }

    #[test]
    fn each_loop_brings_a_new_boss() {
        let (mut w, mut d) = party(1, &crate::config::Config::default().classes);
        let mut seen = Vec::new();
        for _ in 0..(3 * 42_000 / 66) {
            d.advance(66, &mut w);
            if let Some(b) = &w.boss
                && seen.last() != Some(&(b.id, b.tier))
            {
                seen.push((b.id, b.tier));
            }
        }
        assert_eq!(seen.len(), 3, "{seen:?}");
    }

    #[test]
    fn loops_forever() {
        let w = run(1, 3 * 42_000 / 66);
        assert!(w.heroes[0].level > 1);
    }
}
