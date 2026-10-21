//! Scripted scenarios (SPEC §12). Deterministic for a given seed and tick sequence.

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

use serde_json::json;

use super::boss::BossKind;
use super::hero::{Action, Class, Status};
use super::moves;
use super::world::{GameEvent as E, World};

/// Config `[classes]`: agent kind → class name.
pub type Classes = std::collections::BTreeMap<String, String>;

pub struct Demo {
    rng: ChaCha8Rng,
    t: u64,
    idx: usize,
    script: Vec<(u64, E)>,
    loop_ms: u64,
    next_act: Vec<u64>,
    loops: usize,
    /// Time between a hero's random actions (ms range).
    act_gap: (u64, u64),
    /// Stress mode: act even while busy (exercises combo collapse).
    flood: bool,
}

/// One boss command per demo loop, so the roster and size tiers all show up.
const BOSSES: [(BossKind, &str); 6] = [
    // The first loop shows off the rare Sahur Knocker (this command rolls it).
    (BossKind::Build, "cargo build --workspace"),
    (BossKind::Test, "cargo test"),
    (BossKind::Lint, "cargo clippy src/auth.rs"),
    (BossKind::Test, "pytest -k login"),
    (BossKind::Build, "tsc"),
    (BossKind::Lint, "eslint --all"),
];

/// Placeholder detail in the script, replaced by "running <this loop's boss command>".
const BOSS_CMD: &str = "{boss}";

/// Shell lines the demo heroes "run" — one per animation in SPEC §5.4.
const SHELL: [&str; 11] = [
    "rg redirect src",
    "cat Cargo.toml",
    "ls -la src",
    "curl -s https://example.com/health",
    "git commit -m 'fix redirect'",
    "git push --force origin main",
    "git pull --rebase",
    "npm install",
    "rm -rf target/tmp",
    "mkdir -p src/routes",
    "cargo fmt",
];

const SKILLS: [&str; 3] = [
    "superpowers:brainstorming",
    "mattpocock-skills:tdd",
    "ponytail:ponytail",
];

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
        "stress" => Some(stress(seed, classes)),
        _ => None,
    }
}

/// 3 heroes; reads, edits, a boss that fails twice then dies, one hero blocked, stamina
/// rising, quests completing, then the whole party camps. Loops every 42 s.
pub fn party(seed: u64, classes: &Classes) -> (World, Demo) {
    let mut w = World::new("demo-repo", "Forge Village");
    let mut files: Vec<String> = Vec::new();
    for (dir, n, ext) in [
        ("src", 120, "rs"),
        ("tests", 30, "rs"),
        ("assets", 40, "txt"),
        ("docs", 8, "md"),
        ("tools", 4, "py"),
    ] {
        files.extend((0..n).map(|i| format!("{dir}/f{i}.{ext}")));
    }
    w.towns.insert(
        super::world::DEMO_REPO.into(),
        super::town::plan(&files, 180),
    );
    let (k, m, r) = (
        w.add_hero("Claude", Class::for_agent(classes, "claude")),
        w.add_hero("Codex", Class::for_agent(classes, "codex")),
        w.add_hero("OpenCode", Class::for_agent(classes, "opencode")),
    );
    // Show off M14: a grown dragon and the Crimson skin, a Verdant Mage with a baby owl.
    for (i, level, pet) in [(k, 12, "dragon"), (m, 6, "owl"), (r, 2, "cat")] {
        w.heroes[i].level = level;
        w.heroes[i].pet = Some(pet.into());
    }
    // Codex is one quest from Lv 7, which lifts the party to Lv 7: fireworks.
    w.heroes[m].xp = super::xp::xp_to_next(6) - 20;
    let test = || Some(BOSS_CMD.to_string());
    let mut script = vec![
        (0, E::Title(k, "fix login redirect bug".into())),
        (0, E::Title(m, "add rate limiter".into())),
        (0, E::Title(r, "write session tests".into())),
        (0, E::Status(k, Status::Working)),
        (0, E::Status(m, Status::Working)),
        (0, E::Status(r, Status::Idle)),
        (0, E::ContextUsed(k, Some(35))),
        // Claude sends two subagents off to explore; they come back at different times.
        (3_000, E::Summon(k, "toolu_demo_scout".into(), false)),
        (3_000, E::Summon(k, "toolu_demo_reader".into(), false)),
        (11_000, E::Dismiss(k, "toolu_demo_scout".into())),
        (13_500, E::Dismiss(k, "toolu_demo_reader".into())),
        (8_000, E::BossFight(k, BossKind::Test, String::new())),
        (8_000, E::Act(k, Action::Cast, test())),
        (9_500, E::BossResult(k, true)),
        (12_000, E::Status(m, Status::Blocked)),
        (14_000, E::BossFight(k, BossKind::Test, String::new())),
        (14_000, E::Act(k, Action::Cast, test())),
        (15_500, E::BossResult(k, true)),
        (
            18_000,
            E::Act(k, Action::Attack, Some("editing src/auth.rs".into())),
        ),
        (20_000, E::BossFight(k, BossKind::Test, String::new())),
        (20_000, E::Act(k, Action::Cast, test())),
        (21_500, E::BossResult(k, false)),
        (22_000, E::Status(m, Status::Working)),
        (24_000, E::Status(r, Status::Working)),
        (24_000, E::ContextUsed(k, Some(72))),
        (23_000, E::ContextUsed(k, Some(91))),
        // Claude compacts: the Rest scene, then Well Rested.
        (25_000, E::Rest(k, Some(12))),
        // Quests complete 10 s after their hero stops working (SPEC §6.4).
        (26_000, E::Status(k, Status::Done)),
        (26_000, E::Status(m, Status::Done)),
        (29_000, E::Status(k, Status::Idle)),
        (29_000, E::Status(m, Status::Idle)),
        (30_000, E::Status(r, Status::Idle)),
        // OpenCode starts a fresh session: a new Chapter, level kept.
        (40_000, E::NewRun(r)),
    ];
    // Script lines are grouped by story above; play them in time order.
    script.sort_by_key(|(at, _)| *at);
    let demo = Demo {
        rng: ChaCha8Rng::seed_from_u64(seed),
        t: 0,
        idx: 0,
        script,
        loop_ms: 42_000,
        next_act: vec![1_000; w.heroes.len()],
        loops: 0,
        act_gap: (1_800, 4_200),
        flood: false,
    };
    (w, demo)
}

/// Perf test (SPEC §12): 8 heroes acting every 80–250 ms, back-to-back bosses, Summons
/// coming and going. Loops every 20 s.
pub fn stress(seed: u64, classes: &Classes) -> (World, Demo) {
    let mut w = World::new("stress-test", "Forge Village");
    let kinds = [
        "claude", "codex", "cursor", "opencode", "gemini", "grok", "claude", "codex",
    ];
    let names = [
        "Claude",
        "Codex",
        "Cursor",
        "OpenCode",
        "Gemini",
        "Grok",
        "Claude II",
        "Codex II",
    ];
    for (k, n) in kinds.iter().zip(names) {
        w.add_hero(n, Class::for_agent(classes, k));
    }
    let mut script = Vec::new();
    for i in 0..w.heroes.len() {
        script.push((0, E::Status(i, Status::Working)));
        script.push((0, E::ContextUsed(i, Some(40 + i as u8 * 7))));
    }
    for round in 0..4u64 {
        let t = round * 5_000;
        let i = round as usize % w.heroes.len();
        script.push((t, E::BossFight(i, BossKind::Test, String::new())));
        script.push((t + 1_500, E::BossResult(i, round % 2 == 0)));
        for s in 0..3u64 {
            let id = format!("toolu_stress_{round}_{s}");
            script.push((t + 200 * s, E::Summon((i + 1) % 8, id.clone(), false)));
            script.push((t + 3_000 + 200 * s, E::Dismiss((i + 1) % 8, id)));
        }
    }
    script.push((12_000, E::Rest(2, Some(10))));
    script.sort_by_key(|(at, _)| *at);
    let n = w.heroes.len();
    let demo = Demo {
        rng: ChaCha8Rng::seed_from_u64(seed),
        t: 0,
        idx: 0,
        script,
        loop_ms: 20_000,
        next_act: vec![100; n],
        loops: 0,
        act_gap: (80, 250),
        flood: true,
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
                E::BossFight(i, ..) => E::BossFight(i, kind, cmd.to_string()),
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
            self.next_act[i] = self.t + self.rng.random_range(self.act_gap.0..self.act_gap.1);
            if w.heroes[i].status != Status::Working || (w.heroes[i].busy() && !self.flood) {
                continue;
            }
            let file = FILES[self.rng.random_range(0..FILES.len())];
            let (tool, input) = match self.rng.random_range(0..SHELL.len() + 3) {
                0 => (
                    "Edit",
                    json!({ "file_path": file, "new_string": "x\n".repeat(self.rng.random_range(1..40)) }),
                ),
                1 => ("Read", json!({ "file_path": file })),
                2 => (
                    "Skill",
                    json!({ "skill": SKILLS[self.rng.random_range(0..SKILLS.len())] }),
                ),
                n => ("Bash", json!({ "command": SHELL[n - 3] })),
            };
            if let Some(f) = input["file_path"].as_str() {
                w.heroes[i].last_file = Some(f.to_string());
            }
            if let Some(m) = moves::tool_move(tool, &input) {
                w.apply(E::Act(i, m.action, Some(m.detail)));
                if let Some((text, color)) = m.float {
                    w.heroes[i].say(text, color);
                }
            }
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
    fn stress_keeps_eight_heroes_busy() {
        let (mut w, mut d) = stress(1, &crate::config::Config::default().classes);
        for _ in 0..(60_000 / 66) {
            d.advance(66, &mut w);
        }
        assert_eq!(w.heroes.len(), 8);
        assert!(w.heroes.iter().all(|h| h.level > 1), "everyone earned XP");
    }

    #[test]
    fn loops_forever() {
        let w = run(1, 3 * 42_000 / 66);
        assert!(w.heroes[0].level > 1);
    }
}
