//! Background thread that keeps the party in sync with herdr. It polls `session.snapshot`
//! (one ~6 KB request) and sends only real changes to the stage.
//!
//! Why polling instead of `events.subscribe`: herdr's `pane.agent_status_changed`
//! subscription needs a `pane_id` per subscription, so following every agent would mean
//! re-subscribing whenever panes come and go. A local snapshot every 200 ms is cheaper to
//! get right and stays inside the 300 ms "sidebar to stage" budget (SPEC §15 Q4).

use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use serde_json::json;

use super::client::Client;
use super::types::{AgentInfo, AgentStatus, Snapshot};
use crate::game::hero::Status;
use crate::game::party::LiveAgent;
use crate::game::repo::{self, Repo};

const VISIBLE_POLL: Duration = Duration::from_millis(200);
const HIDDEN_POLL: Duration = Duration::from_millis(1000);
const BACKOFF_MIN: Duration = Duration::from_millis(250);
const BACKOFF_MAX: Duration = Duration::from_secs(5);

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
    /// Connection to herdr lost; the thread keeps retrying.
    Lost,
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

pub fn spawn(me: Option<Me>, all_workspaces: bool) -> Receiver<Update> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("herdr".into())
        .spawn(move || run(tx, me, all_workspaces))
        .expect("spawn herdr thread");
    rx
}

fn run(tx: Sender<Update>, me: Option<Me>, all_workspaces: bool) {
    let mut repos: HashMap<String, Repo> = HashMap::new();
    let mut repo_of = |cwd: &str| {
        repos
            .entry(cwd.to_string())
            .or_insert_with(|| repo::identify(Path::new(cwd)))
            .clone()
    };
    let mut backoff = BACKOFF_MIN;
    let mut last: Option<Update> = None;
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
        loop {
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
            if !send(Update::Party { agents, visible }) {
                return; // stage exited
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
    fn outside_herdr_everything_is_visible() {
        let (agents, visible) = party(&fixture(), None, false, &mut repo_of);
        assert!(visible);
        assert_eq!(agents.len(), fixture().agents.len());
    }
}
