//! Bosses (SPEC §6.3): the kind is truthful (test / build / lint), the species is a
//! stable pick from the command text (same command → same nemesis), and the size
//! tier follows the command's scope. Pure functions.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossKind {
    Test,
    Build,
    Lint,
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
