//! Bosses (SPEC §6.3): the kind is truthful (test / build / lint), the species is a
//! stable pick from the command text (same command → same nemesis), and the size
//! tier follows the command's scope. Pure functions.

use crate::config::Bosses;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossKind {
    Test,
    Build,
    Lint,
}

/// Built-in rules (SPEC §6.3), checked after the user's. A rule matches a command whose
/// words start with the rule's words (`cargo test` matches `cargo test --workspace`).
const TEST: &[&str] = &[
    "cargo test",
    "cargo nextest",
    "npm test",
    "pnpm test",
    "yarn test",
    "bun test",
    "npm run test",
    "pnpm run test",
    "vitest",
    "jest",
    "pytest",
    "go test",
    "just test",
    "make test",
    "dotnet test",
    "mvn test",
    "gradle test",
    "./gradlew test",
];
const BUILD: &[&str] = &[
    "cargo build",
    "cargo check",
    "tsc",
    "npm run build",
    "pnpm build",
    "pnpm run build",
    "yarn build",
    "bun run build",
    "go build",
    "make",
    "vite build",
    "next build",
    "dotnet build",
    "mvn package",
    "gradle build",
    "./gradlew build",
];
const LINT: &[&str] = &[
    "cargo clippy",
    "cargo fmt --check",
    "eslint",
    "biome",
    "ruff",
    "golangci-lint",
    "prettier --check",
    "npm run lint",
    "pnpm lint",
    "pnpm run lint",
    "yarn lint",
    "mypy",
    "flake8",
    "rubocop",
];

fn matches(words: &[String], rule: &str) -> bool {
    let rule: Vec<String> = rule.split_whitespace().map(str::to_lowercase).collect();
    !rule.is_empty() && words.len() >= rule.len() && words[..rule.len()] == rule[..]
}

/// Which boss a shell command summons, if any: the first command in the line that matches
/// a user rule (test, build, lint), else a built-in one.
pub fn classify(command: &str, user: &Bosses) -> Option<BossKind> {
    let tables: [(BossKind, Vec<&str>); 6] = [
        (
            BossKind::Test,
            user.test.iter().map(String::as_str).collect(),
        ),
        (
            BossKind::Build,
            user.build.iter().map(String::as_str).collect(),
        ),
        (
            BossKind::Lint,
            user.lint.iter().map(String::as_str).collect(),
        ),
        (BossKind::Test, TEST.to_vec()),
        (BossKind::Build, BUILD.to_vec()),
        (BossKind::Lint, LINT.to_vec()),
    ];
    for words in super::moves::commands(command) {
        for (kind, rules) in &tables {
            if rules.iter().any(|r| matches(&words, r)) {
                return Some(*kind);
            }
        }
    }
    None
}

/// Did a boss command fail? Its exit status (`is_error`) — or, because agents often
/// pipe output through `tail`/`head` (which hides the exit code), failure markers in
/// the output itself.
pub fn failed(is_error: bool, output: &str) -> bool {
    const MARKERS: [&str; 10] = [
        "test result: FAILED",
        "error[E",
        "could not compile",
        "error: could not",
        "npm ERR!",
        "Tests failed",
        "FAILED (",
        "--- FAIL",
        "FAIL\t",
        "panicked at",
    ];
    is_error || MARKERS.iter().any(|m| output.contains(m))
}

pub const VICTORY_XP: u64 = 40;
pub const FIRST_TRY_XP: u64 = 30;
pub const PERSISTENCE_XP: u64 = 20;

/// XP for beating a boss after `fails` failed attempts (SPEC §7.1).
pub fn victory_xp(fails: u32) -> u64 {
    VICTORY_XP
        + if fails == 0 { FIRST_TRY_XP } else { 0 }
        + if fails >= 2 { PERSISTENCE_XP } else { 0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Little,
    Normal,
    Big,
}

impl Tier {
    pub fn id(self) -> &'static str {
        match self {
            Tier::Little => "little",
            Tier::Normal => "normal",
            Tier::Big => "big",
        }
    }

    /// One size down, for panes too short to fit this tier.
    pub fn smaller(self) -> Option<Tier> {
        match self {
            Tier::Big => Some(Tier::Normal),
            Tier::Normal => Some(Tier::Little),
            Tier::Little => None,
        }
    }
}

/// (sprite id, display name) per kind. Sprites: `boss_<id>_<tier>`.
const ROSTER: [(BossKind, [(&str, &str); 3]); 3] = [
    (
        BossKind::Test,
        [
            ("test_golem", "Test Golem"),
            ("flaky_ghost", "Flaky Ghost"),
            ("snapshot_mimic", "Snapshot Mimic"),
        ],
    ),
    (
        BossKind::Build,
        [
            ("build_wyrm", "Build Wyrm"),
            ("linker_kraken", "Linker Kraken"),
            ("dependency_hydra", "Dependency Hydra"),
        ],
    ),
    (
        BossKind::Lint,
        [
            ("lint_imp", "Lint Imp"),
            ("typo_gremlin", "Typo Gremlin"),
            ("slime_king", "Off-by-One Slime King"),
        ],
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Species {
    pub id: &'static str,
    pub name: &'static str,
}

/// Rare boss of any kind: 1 in `RARE_ODDS` commands summon it instead of the roster.
const RARE: Species = Species {
    id: "sahur_knocker",
    name: "Sahur Knocker",
};
const RARE_ODDS: u64 = 8;

pub fn species(kind: BossKind, command: &str) -> Species {
    let h = fnv1a(command.trim());
    if h.is_multiple_of(RARE_ODDS) {
        return RARE;
    }
    let roster = ROSTER.iter().find(|(k, _)| *k == kind).unwrap().1;
    let (id, name) = roster[((h / RARE_ODDS) % roster.len() as u64) as usize];
    Species { id, name }
}

/// Stable across Rust versions and platforms (unlike `DefaultHasher`).
fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

const BIG_FLAGS: [&str; 6] = [
    "--workspace",
    "--all",
    "--all-targets",
    "--all-features",
    "--recursive",
    "-r",
];

/// Size by scope: whole-repo flags → Big; a narrowed run (test path, `::`, `-k`,
/// `--test`, `-run`, a file) → Little; otherwise Normal.
pub fn tier(command: &str) -> Tier {
    let words: Vec<&str> = command.split_whitespace().collect();
    if words.iter().any(|w| BIG_FLAGS.contains(w)) {
        return Tier::Big;
    }
    let narrowed = words.iter().enumerate().any(|(i, w)| {
        i > 0
            && (w.contains("::")
                || w.contains('/')
                || [".rs", ".py", ".ts", ".js", ".go"]
                    .iter()
                    .any(|e| w.ends_with(e))
                || ["-k", "--test", "-run", "-t", "--grep"].contains(w))
    });
    if narrowed { Tier::Little } else { Tier::Normal }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_command_same_nemesis() {
        let a = species(BossKind::Test, "cargo test");
        assert_eq!(a, species(BossKind::Test, "  cargo test "));
        assert!(ROSTER[0].1.iter().any(|(id, _)| *id == a.id));
    }

    #[test]
    fn species_stays_in_its_kind() {
        for cmd in ["cargo build", "tsc", "make", "go build ./...", "next build"] {
            let s = species(BossKind::Build, cmd);
            assert!(
                s == RARE || ROSTER[1].1.iter().any(|(id, _)| *id == s.id),
                "{cmd}"
            );
        }
    }

    #[test]
    fn every_species_is_reachable() {
        let mut seen = std::collections::HashSet::new();
        for kind in [BossKind::Test, BossKind::Build, BossKind::Lint] {
            for i in 0..200 {
                seen.insert(species(kind, &format!("cmd {i}")).id);
            }
        }
        assert_eq!(seen.len(), 10);
    }

    #[test]
    fn sahur_knocker_is_rare_but_real() {
        assert_eq!(species(BossKind::Build, "cargo build --workspace"), RARE);
        let n = (0..800)
            .filter(|i| species(BossKind::Test, &format!("cmd {i}")) == RARE)
            .count();
        assert!((60..140).contains(&n), "{n} of 800");
    }

    fn none() -> Bosses {
        Bosses::default()
    }

    #[test]
    fn every_built_in_rule_classifies() {
        for (kind, table) in [
            (BossKind::Test, TEST),
            (BossKind::Build, BUILD),
            (BossKind::Lint, LINT),
        ] {
            for rule in table {
                assert_eq!(classify(rule, &none()), Some(kind), "{rule}");
                assert_eq!(
                    classify(&format!("{rule} --some-flag"), &none()),
                    Some(kind),
                    "{rule} + flag"
                );
            }
        }
    }

    #[test]
    fn real_world_lines() {
        let c = |s| classify(s, &none());
        assert_eq!(
            c("cd /d/code/app && cargo test 2>&1 | tail -5"),
            Some(BossKind::Test)
        );
        assert_eq!(
            c("RUST_LOG=debug cargo clippy --all-targets -- -D warnings"),
            Some(BossKind::Lint)
        );
        assert_eq!(
            c("cargo build --release -q 2>&1 | tail -3"),
            Some(BossKind::Build)
        );
        assert_eq!(
            c("make test"),
            Some(BossKind::Test),
            "test rules win over plain make"
        );
        assert_eq!(c("cargo fmt"), None, "formatting is not a lint check");
        assert_eq!(c("git status"), None);
        assert_eq!(c("cargo testy"), None, "whole words only");
        assert_eq!(c("echo cargo test"), None, "only the command itself counts");
    }

    #[test]
    fn user_rules_come_first() {
        let user = Bosses {
            test: vec!["just t".into()],
            build: vec![],
            lint: vec!["cargo test".into()],
        };
        assert_eq!(classify("just t", &user), Some(BossKind::Test));
        assert_eq!(classify("cargo test", &user), Some(BossKind::Lint));
    }

    #[test]
    fn failure_seen_through_pipes() {
        assert!(failed(true, ""));
        assert!(failed(false, "test result: FAILED. 3 passed; 1 failed"));
        assert!(failed(false, "error[E0425]: cannot find value `x`"));
        assert!(failed(
            false,
            "error: could not compile `app` (bin) due to 2 previous errors"
        ));
        assert!(failed(false, "--- FAIL: TestLogin (0.00s)"));
        assert!(!failed(false, "test result: ok. 96 passed; 0 failed"));
        assert!(!failed(false, "Finished `release` profile"));
    }

    #[test]
    fn victory_bonuses() {
        assert_eq!(victory_xp(0), 70, "first try");
        assert_eq!(victory_xp(1), 40);
        assert_eq!(victory_xp(2), 60, "persistence");
    }

    #[test]
    fn tier_by_scope() {
        assert_eq!(tier("cargo test"), Tier::Normal);
        assert_eq!(tier("cargo test auth::redirect"), Tier::Little);
        assert_eq!(tier("pytest tests/test_login.py"), Tier::Little);
        assert_eq!(tier("go test -run TestLogin"), Tier::Little);
        assert_eq!(tier("cargo test --workspace"), Tier::Big);
        assert_eq!(tier("cargo clippy --all-targets -- -D warnings"), Tier::Big);
        assert_eq!(tier("pnpm -r build"), Tier::Big);
        assert_eq!(tier("npm test"), Tier::Normal);
    }
}
