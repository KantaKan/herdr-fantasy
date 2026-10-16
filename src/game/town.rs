//! The repo as a village (SPEC §5.7). Planning is pure; `survey` runs git and must stay
//! off the render thread.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Forge,
    Lantern,
    Harbor,
    Serpent,
    Meadow,
}

impl Theme {
    pub fn name(self) -> &'static str {
        match self {
            Theme::Forge => "Forge Village",
            Theme::Lantern => "Lantern City",
            Theme::Harbor => "Harbor Town",
            Theme::Serpent => "Serpent Grove",
            Theme::Meadow => "Meadow Hamlet",
        }
    }

    /// Sprite id part: `town_<id>_<tier>`.
    pub fn id(self) -> &'static str {
        match self {
            Theme::Forge => "forge",
            Theme::Lantern => "lantern",
            Theme::Harbor => "harbor",
            Theme::Serpent => "serpent",
            Theme::Meadow => "meadow",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Building {
    /// Top-level directory name.
    pub name: String,
    pub files: usize,
    /// 0, 1 or 2: grows with the directory's file count.
    pub tier: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Town {
    pub theme: Theme,
    pub buildings: Vec<Building>,
    pub commits: u64,
    /// Unlocked decorations, in unlock order (sprite ids `deco_<name>`).
    pub decorations: Vec<&'static str>,
}

const MAX_BUILDINGS: usize = 6;
/// Commit thresholds → decoration (ART.md §7).
const DECORATIONS: [(u64, &str); 6] = [
    (10, "tree"),
    (50, "well"),
    (100, "lanterns"),
    (250, "windmill"),
    (500, "statue"),
    (1000, "castle"),
];

fn tier(files: usize) -> u8 {
    match files {
        0..20 => 0,
        20..100 => 1,
        _ => 2,
    }
}

/// Dominant language by file extension; nothing recognised → Meadow Hamlet.
pub fn theme(paths: &[String]) -> Theme {
    let mut counts: BTreeMap<u8, usize> = BTreeMap::new();
    for p in paths {
        let ext = p.rsplit_once('.').map_or("", |(_, e)| e);
        let t = match ext {
            "rs" => 0,
            "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => 1,
            "go" => 2,
            "py" => 3,
            _ => continue,
        };
        *counts.entry(t).or_default() += 1;
    }
    match counts
        .iter()
        .max_by_key(|(t, n)| (**n, std::cmp::Reverse(**t)))
    {
        Some((0, _)) => Theme::Forge,
        Some((1, _)) => Theme::Lantern,
        Some((2, _)) => Theme::Harbor,
        Some((3, _)) => Theme::Serpent,
        _ => Theme::Meadow,
    }
}

/// Plan a town from tracked file paths (repo-relative, `/`-separated) and commit count.
pub fn plan(paths: &[String], commits: u64) -> Town {
    let mut dirs: BTreeMap<&str, usize> = BTreeMap::new();
    for p in paths {
        if let Some((top, _)) = p.split_once('/')
            && !top.starts_with('.')
        {
            *dirs.entry(top).or_default() += 1;
        }
    }
    let mut ranked: Vec<(&str, usize)> = dirs.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    Town {
        theme: theme(paths),
        buildings: ranked
            .into_iter()
            .take(MAX_BUILDINGS)
            .map(|(name, files)| Building {
                name: name.to_string(),
                files,
                tier: tier(files),
            })
            .collect(),
        commits,
        decorations: DECORATIONS
            .iter()
            .filter(|(n, _)| commits >= *n)
            .map(|(_, d)| *d)
            .collect(),
    }
}

/// The building a file lives in: the first path component naming a building.
pub fn building_for(file: &str, town: &Town) -> Option<usize> {
    file.split(['/', '\\'])
        .find_map(|part| town.buildings.iter().position(|b| b.name == part))
}

/// Run git in `cwd` for the whole repo's tracked files and commit count.
pub fn survey(cwd: &Path) -> Option<Town> {
    let git = |args: &[&str]| -> Option<String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(cwd)
            .args(args)
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    };
    let files = git(&["ls-files", "--full-name", "--", ":/"])?;
    let paths: Vec<String> = files.lines().map(String::from).collect();
    let commits = git(&["rev-list", "--count", "HEAD"])
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    Some(plan(&paths, commits))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(spec: &[(&str, usize, &str)]) -> Vec<String> {
        spec.iter()
            .flat_map(|(dir, n, ext)| (0..*n).map(move |i| format!("{dir}/f{i}.{ext}")))
            .collect()
    }

    #[test]
    fn ranks_top_six_dirs_by_files() {
        let p = paths(&[
            ("src", 120, "rs"),
            ("tests", 30, "rs"),
            ("docs", 5, "md"),
            ("a", 2, "md"),
            ("b", 2, "md"),
            ("c", 1, "md"),
            ("d", 1, "md"),
            (".github", 50, "yml"),
        ]);
        let t = plan(&p, 0);
        let names: Vec<_> = t.buildings.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(
            names,
            ["src", "tests", "docs", "a", "b", "c"],
            "hidden dirs skipped"
        );
        assert_eq!(
            t.buildings.iter().map(|b| b.tier).collect::<Vec<_>>(),
            [2, 1, 0, 0, 0, 0]
        );
        assert_eq!(t.theme, Theme::Forge);
    }

    #[test]
    fn themes_by_language() {
        assert_eq!(
            theme(&paths(&[("web", 10, "tsx"), ("x", 3, "rs")])),
            Theme::Lantern
        );
        assert_eq!(theme(&paths(&[("cmd", 4, "go")])), Theme::Harbor);
        assert_eq!(theme(&paths(&[("app", 4, "py")])), Theme::Serpent);
        assert_eq!(theme(&paths(&[("docs", 4, "md")])), Theme::Meadow);
        assert_eq!(theme(&[]), Theme::Meadow);
    }

    #[test]
    fn decorations_unlock_by_commits() {
        assert!(plan(&[], 9).decorations.is_empty());
        assert_eq!(plan(&[], 120).decorations, ["tree", "well", "lanterns"]);
        assert_eq!(plan(&[], 5000).decorations.len(), 6);
    }

    #[test]
    fn heroes_find_their_building() {
        let t = plan(&paths(&[("src", 3, "rs"), ("tests", 2, "rs")]), 0);
        assert_eq!(building_for(r"D:\code\app\tests\login.rs", &t), Some(1));
        assert_eq!(building_for("/home/me/app/src/game/hero.rs", &t), Some(0));
        assert_eq!(building_for("README.md", &t), None);
    }

    #[test]
    fn this_repo_surveys() {
        let t = survey(Path::new(env!("CARGO_MANIFEST_DIR"))).expect("git works here");
        assert!(t.buildings.iter().any(|b| b.name == "src"));
        assert_eq!(t.theme, Theme::Forge);
    }
}
