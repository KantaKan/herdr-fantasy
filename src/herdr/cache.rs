//! Background thread that keeps the party in sync with herdr. It polls `session.snapshot`
//! (one ~6 KB request) and sends only real changes to the stage.
//!
//! Why polling instead of `events.subscribe`: herdr's `pane.agent_status_changed`
//! subscription needs a `pane_id` per subscription, so following every agent would mean
//! re-subscribing whenever panes come and go. A local snapshot every 200 ms is cheaper to
//! get right and stays inside the 300 ms "sidebar to stage" budget (SPEC §15 Q4).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::Duration;

use serde_json::json;

use super::actions;
use super::client::Client;
use super::types::{AgentInfo, AgentStatus, Snapshot};
use crate::game::hero::Status;
use crate::game::party::LiveAgent;
use crate::game::repo::{self, Repo};
use crate::transcript::claude::{self, Entry};
use crate::transcript::codex;

/// Turns one transcript line into entries (Claude or Codex format).
type Parser = fn(&str) -> Vec<Entry>;
use crate::transcript::tail::Tail;

const VISIBLE_POLL: Duration = Duration::from_millis(200);
const HIDDEN_POLL: Duration = Duration::from_millis(1000);
const BACKOFF_MIN: Duration = Duration::from_millis(250);
const BACKOFF_MAX: Duration = Duration::from_secs(5);
const TOWN_REFRESH: Duration = Duration::from_secs(60);

/// The stage's own pane, for visibility and "same workspace" filtering.
#[derive(Debug, Clone)]
pub struct Me {
    pub workspace: String,
    pub tab: String,
    pub pane: String,
}

impl Me {
    /// From herdr's injected env; `None` when not running inside a herdr pane.
    pub fn from_env() -> Option<Me> {
        let var = |k| std::env::var(k).ok().filter(|v: &String| !v.is_empty());
        Some(Me {
            workspace: var("HERDR_WORKSPACE_ID")?,
            tab: var("HERDR_TAB_ID")?,
            pane: var("HERDR_PANE_ID")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Update {
    Party {
        agents: Vec<LiveAgent>,
        visible: bool,
    },
    /// New transcript lines for one agent.
    Transcript {
        pane_id: String,
        entries: Vec<Entry>,
        /// Read on first sight of this transcript: state only, no animation.
        history: bool,
        /// The pane switched to a different transcript (new session / `/clear`).
        new_chapter: bool,
    },
    /// A fresh survey of a repo's town (git, at most once a minute per repo).
    Town {
        repo: String,
        town: crate::game::town::Town,
    },
    /// Connection to herdr lost; the thread keeps retrying.
    Lost,
}

/// Requests from the stage, executed on the herdr thread.
#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    Notify {
        title: String,
        body: String,
        sound: &'static str,
    },
    /// Jump to this pane (Enter on a hero). Never sends keys.
    Focus(String),
}

fn status(s: AgentStatus) -> Status {
    match s {
        AgentStatus::Working => Status::Working,
        AgentStatus::Blocked => Status::Blocked,
        AgentStatus::Done => Status::Done,
        AgentStatus::Idle | AgentStatus::Unknown => Status::Idle,
    }
}

fn display_name(a: &AgentInfo, kind: &str) -> String {
    a.display_agent.clone().unwrap_or_else(|| {
        let mut c = kind.chars();
        c.next()
            .map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
    })
}

/// Turn a snapshot into the party the stage should show. `repo_of` resolves a cwd.
pub fn party(
    snap: &Snapshot,
    me: Option<&Me>,
    all_workspaces: bool,
    repo_of: &mut dyn FnMut(&str) -> Repo,
) -> (Vec<LiveAgent>, bool) {
    let visible = me.is_none_or(|m| snap.is_visible(&m.workspace, &m.tab, &m.pane));
    let agents = snap
        .agents
        .iter()
        .filter(|a| all_workspaces || me.is_none_or(|m| a.workspace_id == m.workspace))
        .filter(|a| me.is_none_or(|m| a.pane_id != m.pane))
        .map(|a| {
            let kind = a.agent.clone().unwrap_or_else(|| "agent".into());
            let cwd = a
                .foreground_cwd
                .as_deref()
                .or(a.cwd.as_deref())
                .unwrap_or(".");
            LiveAgent {
                pane_id: a.pane_id.clone(),
                name: display_name(a, &kind),
                kind,
                status: status(a.agent_status),
                title: a.terminal_title_stripped.clone().filter(|t| !t.is_empty()),
                repo: repo_of(cwd),
                focused: a.focused,
            }
        })
        .collect();
    (agents, visible)
}

fn claude_dir() -> PathBuf {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            directories::BaseDirs::new()
                .map(|d| d.home_dir().join(".claude"))
                .unwrap_or_else(|| ".claude".into())
        })
}

/// Transcript file and parser per live Claude or Codex agent in the party (SPEC §3.3).
/// `codex_file` maps a Codex session id to its rollout file (it searches, so the caller
/// caches).
pub fn transcripts(
    snap: &Snapshot,
    me: Option<&Me>,
    all_workspaces: bool,
    claude_dir: &Path,
    codex_file: &mut dyn FnMut(&str) -> Option<PathBuf>,
) -> Vec<(String, PathBuf, Parser)> {
    snap.agents
        .iter()
        .filter(|a| all_workspaces || me.is_none_or(|m| a.workspace_id == m.workspace))
        .filter_map(|a| {
            let s = a.agent_session.as_ref()?;
            let cwd = a.cwd.as_deref().unwrap_or(".");
            let (path, parse): (PathBuf, Parser) = match a.agent.as_deref()? {
                "claude" => (
                    claude::transcript_path(&s.kind, &s.value, cwd, claude_dir),
                    claude::parse,
                ),
                "codex" if s.kind == "path" => (s.value.clone().into(), codex::parse),
                "codex" => (codex_file(&s.value)?, codex::parse),
                _ => return None,
            };
            Some((a.pane_id.clone(), path, parse))
        })
        .collect()
}

/// Codex session id → rollout file, searched at most every 2 s while missing (the file
/// can appear a moment after the session starts).
#[derive(Default)]
struct CodexFiles {
    found: HashMap<String, PathBuf>,
    tried: HashMap<String, std::time::Instant>,
}

impl CodexFiles {
    fn get(&mut self, dir: &Path, id: &str) -> Option<PathBuf> {
        if let Some(p) = self.found.get(id) {
            return Some(p.clone());
        }
        if self
            .tried
            .get(id)
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(2))
        {
            return None;
        }
        self.tried.insert(id.to_string(), std::time::Instant::now());
        let p = codex::find_session(dir, id)?;
        self.found.insert(id.to_string(), p.clone());
        Some(p)
    }
}

/// Follows each agent's transcript; remembers which file each pane was on.
#[derive(Default)]
struct Tails {
    by_pane: HashMap<String, (PathBuf, Tail)>,
}

impl Tails {
    fn poll(&mut self, wanted: Vec<(String, PathBuf, Parser)>, tx: &Sender<Update>) -> bool {
        self.by_pane
            .retain(|pane, _| wanted.iter().any(|(p, ..)| p == pane));
        for (pane, path, parse) in wanted {
            let known = self.by_pane.get(&pane).map(|(p, _)| p.clone());
            if known.as_ref() != Some(&path) {
                // First sight of this file (missing ones are retried next poll).
                let Ok((tail, history)) = Tail::open(&path) else {
                    continue;
                };
                let entries = history.iter().flat_map(|l| parse(l)).collect();
                let u = Update::Transcript {
                    pane_id: pane.clone(),
                    entries,
                    history: true,
                    new_chapter: known.is_some(),
                };
                self.by_pane.insert(pane, (path, tail));
                if tx.send(u).is_err() {
                    return false;
                }
                continue;
            }
            let (_, tail) = self.by_pane.get_mut(&pane).unwrap();
            let lines = match tail.poll() {
                Ok(l) => l,
                Err(e) => {
                    tracing::debug!("transcript {}: {e}", path.display());
                    continue;
                }
            };
            let entries: Vec<Entry> = lines.iter().flat_map(|l| parse(l)).collect();
            if !entries.is_empty() {
                let u = Update::Transcript {
                    pane_id: pane,
                    entries,
                    history: false,
                    new_chapter: false,
                };
                if tx.send(u).is_err() {
                    return false;
                }
            }
        }
        true
    }
}

pub fn spawn(me: Option<Me>, all_workspaces: bool) -> (Receiver<Update>, Sender<Cmd>) {
    let (tx, rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("herdr".into())
        .spawn(move || run(tx, cmd_rx, me, all_workspaces))
        .expect("spawn herdr thread");
    (rx, cmd_tx)
}

fn run(tx: Sender<Update>, cmds: Receiver<Cmd>, me: Option<Me>, all_workspaces: bool) {
    let mut repos: HashMap<String, Repo> = HashMap::new();
    let mut repo_of = |cwd: &str| {
        repos
            .entry(cwd.to_string())
            .or_insert_with(|| repo::identify(Path::new(cwd)))
            .clone()
    };
    let mut backoff = BACKOFF_MIN;
    let mut last: Option<Update> = None;
    let mut tails = Tails::default();
    let claude_dir = claude_dir();
    let codex_dir = codex::codex_dir();
    let mut codex_files = CodexFiles::default();
    let mut surveyed: HashMap<String, std::time::Instant> = HashMap::new();
    loop {
        let mut send = |u: Update| {
            if last.as_ref() == Some(&u) {
                return true;
            }
            last = Some(u.clone());
            tx.send(u).is_ok()
        };
        let mut client = match Client::connect() {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!("herdr connect: {e:#}");
                if !send(Update::Lost) {
                    return;
                }
                std::thread::sleep(backoff);
                backoff = (backoff * 2).min(BACKOFF_MAX);
                continue;
            }
        };
        backoff = BACKOFF_MIN;
        if let Some(m) = &me {
            actions::tag_stage(&mut client, &m.pane);
        }
        loop {
            loop {
                match cmds.try_recv() {
                    Ok(Cmd::Notify { title, body, sound }) => {
                        let r = client.request(
                            "notification.show",
                            json!({ "title": title, "body": body, "sound": sound }),
                        );
                        if let Err(e) = r {
                            tracing::warn!("notification: {e:#}");
                        }
                    }
                    Ok(Cmd::Focus(pane)) => {
                        if let Err(e) = client.request("agent.focus", json!({ "target": pane })) {
                            tracing::warn!("focus {pane}: {e:#}");
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return,
                }
            }
            let snap = client
                .request("session.snapshot", json!({}))
                .and_then(|mut v| Ok(serde_json::from_value::<Snapshot>(v["snapshot"].take())?));
            let snap = match snap {
                Ok(s) => s,
                Err(e) => {
                    tracing::info!("herdr connection lost: {e:#}");
                    if !send(Update::Lost) {
                        return;
                    }
                    break;
                }
            };
            let (agents, visible) = party(&snap, me.as_ref(), all_workspaces, &mut repo_of);
            if !send(Update::Party {
                agents: agents.clone(),
                visible,
            }) {
                return; // stage exited
            }
            let wanted = transcripts(&snap, me.as_ref(), all_workspaces, &claude_dir, &mut |id| {
                codex_files.get(&codex_dir, id)
            });
            if !tails.poll(wanted, &tx) {
                return;
            }
            // Towns: one git survey per repo, refreshed at most once a minute.
            for a in &agents {
                let fresh = surveyed
                    .get(&a.repo.key)
                    .is_some_and(|t| t.elapsed() < TOWN_REFRESH);
                if fresh {
                    continue;
                }
                surveyed.insert(a.repo.key.clone(), std::time::Instant::now());
                let cwd = snap
                    .agents
                    .iter()
                    .find(|x| x.pane_id == a.pane_id)
                    .and_then(|x| x.foreground_cwd.clone().or_else(|| x.cwd.clone()));
                if let Some(town) = cwd.and_then(|d| crate::game::town::survey(Path::new(&d))) {
                    let u = Update::Town {
                        repo: a.repo.key.clone(),
                        town,
                    };
                    if tx.send(u).is_err() {
                        return;
                    }
                }
            }
            std::thread::sleep(if visible { VISIBLE_POLL } else { HIDDEN_POLL });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Snapshot {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/snapshot.json")).unwrap();
        serde_json::from_value(v["result"]["snapshot"].clone()).unwrap()
    }

    fn repo_of(cwd: &str) -> Repo {
        Repo::from_identity(cwd)
    }

    #[test]
    fn party_is_own_workspace_minus_own_pane() {
        let snap = fixture();
        let a = &snap.agents[0];
        let me = Me {
            workspace: a.workspace_id.clone(),
            tab: a.tab_id.clone(),
            pane: "stage-pane".into(),
        };
        let (agents, _) = party(&snap, Some(&me), false, &mut repo_of);
        assert!(!agents.is_empty());
        let in_ws = snap
            .agents
            .iter()
            .filter(|x| x.workspace_id == me.workspace)
            .count();
        assert_eq!(agents.len(), in_ws);
        assert_eq!(agents[0].name, "Claude");

        let (all, _) = party(&snap, Some(&me), true, &mut repo_of);
        assert_eq!(all.len(), snap.agents.len());

        let me_is_agent = Me {
            pane: a.pane_id.clone(),
            ..me
        };
        let (without_me, _) = party(&snap, Some(&me_is_agent), true, &mut repo_of);
        assert_eq!(without_me.len(), snap.agents.len() - 1);
    }

    #[test]
    fn transcript_paths_for_claude_agents() {
        let snap = fixture();
        let t = transcripts(&snap, None, true, Path::new("/claude"), &mut |_| None);
        let claudes = snap
            .agents
            .iter()
            .filter(|a| a.agent.as_deref() == Some("claude") && a.agent_session.is_some())
            .count();
        assert_eq!(t.len(), claudes);
        assert!(t[0].1.starts_with("/claude/projects"));
        assert_eq!(t[0].1.extension().unwrap(), "jsonl");
    }

    #[test]
    fn tails_send_history_then_new_lines_then_new_chapter() {
        let dir = std::env::temp_dir().join(format!("hf-tails-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (a, b) = (dir.join("a.jsonl"), dir.join("b.jsonl"));
        let prompt =
            |s: &str| format!("{{\"type\":\"user\",\"message\":{{\"content\":\"{s}\"}}}}\n");
        std::fs::write(&a, prompt("old")).unwrap();
        std::fs::write(&b, prompt("fresh")).unwrap();
        let (tx, rx) = mpsc::channel();
        let mut tails = Tails::default();
        assert!(tails.poll(vec![("p".into(), a.clone(), claude::parse)], &tx));
        let first = rx.try_recv().unwrap();
        assert!(
            matches!(&first, Update::Transcript { history: true, new_chapter: false, entries, .. } if entries.len() == 1)
        );
        std::fs::OpenOptions::new().append(true).open(&a).unwrap();
        use std::io::Write;
        std::fs::OpenOptions::new()
            .append(true)
            .open(&a)
            .unwrap()
            .write_all(prompt("new").as_bytes())
            .unwrap();
        tails.poll(vec![("p".into(), a.clone(), claude::parse)], &tx);
        assert!(matches!(
            rx.try_recv().unwrap(),
            Update::Transcript { history: false, .. }
        ));
        tails.poll(vec![("p".into(), b, claude::parse)], &tx);
        assert!(matches!(
            rx.try_recv().unwrap(),
            Update::Transcript {
                new_chapter: true,
                history: true,
                ..
            }
        ));
    }

    #[test]
    fn outside_herdr_everything_is_visible() {
        let (agents, visible) = party(&fixture(), None, false, &mut repo_of);
        assert!(visible);
        assert_eq!(agents.len(), fixture().agents.len());
    }
}
