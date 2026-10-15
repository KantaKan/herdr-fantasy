//! The slice of herdr's API we read (herdr 0.9.3 schema). Unknown fields are ignored.

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Idle,
    Working,
    Blocked,
    Done,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AgentSession {
    /// `"id"` or `"path"`.
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AgentInfo {
    pub pane_id: String,
    pub workspace_id: String,
    pub tab_id: String,
    pub agent: Option<String>,
    pub display_agent: Option<String>,
    pub agent_status: AgentStatus,
    pub cwd: Option<String>,
    pub foreground_cwd: Option<String>,
    pub terminal_title_stripped: Option<String>,
    #[serde(default)]
    pub focused: bool,
    pub agent_session: Option<AgentSession>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WorkspaceInfo {
    pub workspace_id: String,
    pub active_tab_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LayoutInfo {
    pub tab_id: String,
    pub focused_pane_id: Option<String>,
    #[serde(default)]
    pub zoomed: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    pub agents: Vec<AgentInfo>,
    pub focused_workspace_id: Option<String>,
    #[serde(default)]
    pub workspaces: Vec<WorkspaceInfo>,
    #[serde(default)]
    pub layouts: Vec<LayoutInfo>,
}

impl Snapshot {
    /// Is pane `pane` (in `workspace`/`tab`) on screen? Its workspace is focused, its tab
    /// is that workspace's active tab, and no other pane is zoomed over it.
    pub fn is_visible(&self, workspace: &str, tab: &str, pane: &str) -> bool {
        if self.focused_workspace_id.as_deref() != Some(workspace) {
            return false;
        }
        let active = self
            .workspaces
            .iter()
            .find(|w| w.workspace_id == workspace)
            .and_then(|w| w.active_tab_id.as_deref());
        if active != Some(tab) {
            return false;
        }
        match self.layouts.iter().find(|l| l.tab_id == tab) {
            Some(l) if l.zoomed => l.focused_pane_id.as_deref() == Some(pane),
            _ => true,
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

    #[test]
    fn parses_real_snapshot() {
        let s = fixture();
        assert!(!s.agents.is_empty());
        let a = &s.agents[0];
        assert_eq!(a.agent.as_deref(), Some("claude"));
        assert_eq!(a.agent_session.as_ref().unwrap().kind, "id");
        assert!(s.focused_workspace_id.is_some());
    }

    #[test]
    fn parses_real_agent_list() {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/agent_list.json")).unwrap();
        let agents: Vec<AgentInfo> = serde_json::from_value(v["result"]["agents"].clone()).unwrap();
        assert!(
            agents
                .iter()
                .any(|a| a.agent_status == AgentStatus::Working)
        );
    }

    #[test]
    fn unknown_status_does_not_fail() {
        let s: AgentStatus = serde_json::from_str("\"thinking_hard\"").unwrap();
        assert_eq!(s, AgentStatus::Unknown);
    }

    #[test]
    fn visibility() {
        let mut s = fixture();
        s.focused_workspace_id = Some("w".into());
        s.workspaces = vec![WorkspaceInfo {
            workspace_id: "w".into(),
            active_tab_id: Some("t".into()),
        }];
        s.layouts = vec![LayoutInfo {
            tab_id: "t".into(),
            focused_pane_id: Some("other".into()),
            zoomed: false,
        }];
        assert!(s.is_visible("w", "t", "me"));
        assert!(!s.is_visible("w", "t2", "me"));
        assert!(!s.is_visible("w2", "t", "me"));
        s.layouts[0].zoomed = true;
        assert!(!s.is_visible("w", "t", "me"));
        s.layouts[0].focused_pane_id = Some("me".into());
        assert!(s.is_visible("w", "t", "me"));
    }
}
