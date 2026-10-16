//! Per-repo save files (SPEC §10): `<root>/repos/<repo_key>/party.json` and an append-only
//! `quests.jsonl`. Writes are atomic (temp file + rename). Only the stage writes.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::hero::Lifetime;

pub const SCHEMA_VERSION: u32 = 1;

/// What a hero keeps between sessions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    pub level: u32,
    pub xp: u64,
    /// Flattened so old files (`{"level","xp","quests"}`) still load.
    #[serde(flatten)]
    pub life: Lifetime,
}

impl Default for Progress {
    fn default() -> Self {
        Progress {
            level: 1,
            xp: 0,
            life: Lifetime::default(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct PartyFile {
    #[serde(default)]
    heroes: BTreeMap<String, Progress>,
}

/// One quest-log line.
#[derive(Debug, Serialize)]
pub struct QuestRecord<'a> {
    pub hero: &'a str,
    pub title: &'a str,
    pub duration_ms: u64,
    pub xp: u64,
    pub result: &'a str,
    pub ended_unix_ms: u64,
    pub bosses: &'a str,
    pub chapter: u32,
}

/// A quest-log line read back (older lines lack `bosses`/`chapter`).
#[derive(Debug, Deserialize)]
struct QuestLine {
    hero: String,
    title: String,
    duration_ms: u64,
    xp: u64,
    result: String,
    #[serde(default)]
    bosses: String,
    #[serde(default = "first_chapter")]
    chapter: u32,
}

fn first_chapter() -> u32 {
    1
}

/// Quest title as written to the log. With `store_prompt_text = false` (SPEC §5.6) only
/// a short hash is kept, so prompt text never lands on disk.
pub fn logged_title(title: &str, store_prompts: bool) -> String {
    if store_prompts {
        return title.to_string();
    }
    use sha2::{Digest, Sha256};
    let d = Sha256::digest(title.as_bytes());
    format!(
        "prompt#{}",
        d[..4]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

pub struct Store {
    root: PathBuf,
    parties: HashMap<String, PartyFile>,
    dirty: HashSet<String>,
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

impl Store {
    pub fn open(root: PathBuf) -> Self {
        let version = root.join("version");
        if !version.exists()
            && let Err(e) = write_atomic(&version, SCHEMA_VERSION.to_string().as_bytes())
        {
            tracing::warn!("cannot write {}: {e}", version.display());
        }
        Store {
            root,
            parties: HashMap::new(),
            dirty: HashSet::new(),
        }
    }

    fn party_path(&self, repo: &str) -> PathBuf {
        self.root.join("repos").join(repo).join("party.json")
    }

    /// Load a repo's party file; a corrupt file is moved aside and we start fresh.
    fn party(&mut self, repo: &str) -> &mut PartyFile {
        if !self.parties.contains_key(repo) {
            let path = self.party_path(repo);
            let file = match std::fs::read(&path) {
                Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                    let aside = path.with_extension(format!("json.corrupt-{}", now_ms()));
                    tracing::warn!(
                        "corrupt {} ({e}); moved to {}",
                        path.display(),
                        aside.display()
                    );
                    let _ = std::fs::rename(&path, &aside);
                    PartyFile::default()
                }),
                Err(_) => PartyFile::default(),
            };
            self.parties.insert(repo.to_string(), file);
        }
        self.parties.get_mut(repo).unwrap()
    }

    pub fn get(&mut self, repo: &str, hero: &str) -> Progress {
        self.party(repo)
            .heroes
            .get(hero)
            .cloned()
            .unwrap_or_default()
    }

    pub fn put(&mut self, repo: &str, hero: &str, p: Progress) {
        let heroes = &mut self.party(repo).heroes;
        if heroes.get(hero).cloned().unwrap_or_default() != p {
            heroes.insert(hero.to_string(), p);
            self.dirty.insert(repo.to_string());
        }
    }

    pub fn is_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    /// Write every changed party file. Failures are logged; the data stays dirty.
    pub fn flush(&mut self) {
        for repo in std::mem::take(&mut self.dirty) {
            let path = self.party_path(&repo);
            let json = serde_json::to_vec_pretty(&self.parties[&repo]).unwrap();
            if let Err(e) = write_atomic(&path, &json) {
                tracing::warn!("cannot save {}: {e}", path.display());
                self.dirty.insert(repo);
            }
        }
    }

    /// The saved quest log of a repo (bad lines skipped).
    pub fn read_quests(&self, repo: &str) -> Vec<super::world::LogEntry> {
        let path = self.root.join("repos").join(repo).join("quests.jsonl");
        let Ok(text) = std::fs::read_to_string(path) else {
            return Vec::new();
        };
        text.lines()
            .filter_map(|l| serde_json::from_str::<QuestLine>(l).ok())
            .map(|q| super::world::LogEntry {
                repo: Some(repo.to_string()),
                hero: q.hero,
                title: q.title,
                duration_ms: q.duration_ms,
                xp: q.xp,
                result: q.result,
                bosses: q.bosses,
                chapter: q.chapter,
            })
            .collect()
    }

    pub fn append_quest(&self, repo: &str, rec: &QuestRecord) {
        let path = self.root.join("repos").join(repo).join("quests.jsonl");
        let line = serde_json::to_string(rec).unwrap();
        let res = std::fs::create_dir_all(path.parent().unwrap()).and_then(|_| {
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;
            writeln!(f, "{line}")
        });
        if let Err(e) = res {
            tracing::warn!("cannot append {}: {e}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hf-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn round_trip_survives_restart() {
        let root = tmp("rt");
        let mut s = Store::open(root.clone());
        assert_eq!(s.get("repo1", "claude#0"), Progress::default());
        let mut p = Progress {
            level: 7,
            xp: 12,
            ..Progress::default()
        };
        p.life.quests = 3;
        p.life.bonus[1] = 2;
        p.life.loot.push(
            super::super::loot::roll("Claude", 1, "lint_imp", super::super::boss::Tier::Big)
                .unwrap(),
        );
        s.put("repo1", "claude#0", p.clone());
        assert!(s.is_dirty());
        s.flush();
        assert!(!s.is_dirty());
        let mut again = Store::open(root.clone());
        assert_eq!(again.get("repo1", "claude#0"), p);
        assert_eq!(std::fs::read_to_string(root.join("version")).unwrap(), "1");
    }

    #[test]
    fn corrupt_file_is_moved_aside() {
        let root = tmp("corrupt");
        let path = root.join("repos/r/party.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{not json").unwrap();
        let mut s = Store::open(root.clone());
        assert_eq!(s.get("r", "claude#0"), Progress::default());
        let names: Vec<_> = std::fs::read_dir(root.join("repos/r"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert!(
            names.iter().any(|n| n.starts_with("party.json.corrupt-")),
            "{names:?}"
        );
    }

    #[test]
    fn m2_save_files_still_load() {
        let p: Progress = serde_json::from_str(r#"{"level": 3, "xp": 101, "quests": 5}"#).unwrap();
        assert_eq!((p.level, p.xp, p.life.quests, p.life.rests), (3, 101, 5, 0));
    }

    #[test]
    fn unchanged_put_is_not_dirty() {
        let mut s = Store::open(tmp("clean"));
        s.put("r", "h", Progress::default());
        assert!(!s.is_dirty());
    }

    #[test]
    fn private_titles_are_hashed() {
        assert_eq!(logged_title("fix login", true), "fix login");
        let h = logged_title("fix login", false);
        assert!(
            h.starts_with("prompt#") && h.len() == 15 && !h.contains("login"),
            "{h}"
        );
        assert_eq!(h, logged_title("fix login", false));
    }

    #[test]
    fn quest_log_appends() {
        let root = tmp("quests");
        let s = Store::open(root.clone());
        for title in ["a", "b"] {
            s.append_quest(
                "r",
                &QuestRecord {
                    hero: "Claude",
                    title,
                    duration_ms: 1,
                    xp: 2,
                    result: "complete",
                    ended_unix_ms: 3,
                    bosses: "✗✓",
                    chapter: 2,
                },
            );
        }
        let log = std::fs::read_to_string(root.join("repos/r/quests.jsonl")).unwrap();
        assert_eq!(log.lines().count(), 2);
        let back = s.read_quests("r");
        assert_eq!(back.len(), 2);
        assert_eq!(
            (
                back[1].title.as_str(),
                back[1].bosses.as_str(),
                back[1].chapter
            ),
            ("b", "✗✓", 2)
        );
        // M4-era lines without bosses/chapter still read
        std::fs::write(
            root.join("repos/old/quests.jsonl"),
            "{\"hero\":\"C\",\"title\":\"t\",\"duration_ms\":1,\"xp\":1,\"result\":\"complete\",\"ended_unix_ms\":1}\nnot json\n",
        )
        .ok();
        std::fs::create_dir_all(root.join("repos/old")).unwrap();
        std::fs::write(
            root.join("repos/old/quests.jsonl"),
            "{\"hero\":\"C\",\"title\":\"t\",\"duration_ms\":1,\"xp\":1,\"result\":\"complete\",\"ended_unix_ms\":1}\nnot json\n",
        )
        .unwrap();
        let old = s.read_quests("old");
        assert_eq!((old.len(), old[0].chapter), (1, 1));
    }
}
