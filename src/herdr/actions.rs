//! One-shot commands herdr runs for us: `autostart` (startup + workspace.created),
//! the `toggle-stage` and `open-ask` actions (SPEC §5.1, §5.5).

use serde_json::{Value, json};

use super::client::Client;
use super::types::{PaneInfo, Snapshot};
use crate::config::AutoOpen;

pub const PLUGIN_ID: &str = "kantakan.herdr-fantasy";
/// The stage tags its own pane with this metadata token so we never open it twice.
pub const STAGE_TOKEN: &str = "hf_stage";
/// Manifest title; herdr shows it as the pane label right away, before the stage tags.
const STAGE_LABEL: &str = "herdr fantasy";
/// Below this workspace height there's no room for a stage (SPEC §5.1).
const MIN_WORKSPACE_ROWS: u16 = 24;
const STAGE_MIN_ROWS: f32 = 10.0;
const STAGE_SHARE: f32 = 0.25;

fn looks_like_stage(p: &PaneInfo) -> bool {
    p.tokens.contains_key(STAGE_TOKEN) || p.label.as_deref() == Some(STAGE_LABEL)
}

pub fn stage_pane<'a>(snap: &'a Snapshot, workspace: &str) -> Option<&'a PaneInfo> {
    snap.panes
        .iter()
        .find(|p| p.workspace_id == workspace && looks_like_stage(p))
}

/// Is our stage process the pane's foreground program (`pane.process_info` result)?
pub fn runs_stage(process_info: &Value) -> bool {
    process_info["process_info"]["foreground_processes"]
        .as_array()
        .is_some_and(|ps| {
            ps.iter().any(|p| {
                p["name"]
                    .as_str()
                    .is_some_and(|n| n.to_lowercase().starts_with("herdr-fantasy"))
            })
        })
}

/// After a herdr server restart the stage pane comes back as a plain shell that still
/// carries the stage label (SPEC §15 Q3). Close those and forget them, so a real stage
/// gets opened in their place.
fn prune_dead_stages(c: &mut Client, snap: &mut Snapshot) {
    let candidates: Vec<String> = snap
        .panes
        .iter()
        .filter(|p| looks_like_stage(p))
        .map(|p| p.pane_id.clone())
        .collect();
    for pane in candidates {
        let alive = c
            .request("pane.process_info", json!({ "pane_id": pane }))
            .map(|info| runs_stage(&info))
            .unwrap_or(true); // can't tell: leave it alone
        if !alive {
            tracing::info!("closing dead stage pane {pane}");
            if let Err(e) = c.request("pane.close", json!({ "pane_id": pane })) {
                tracing::warn!("close dead stage {pane}: {e:#}");
            }
            snap.panes.retain(|p| p.pane_id != pane);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenStage {
    pub workspace: String,
    /// Split this pane (the focused pane of the workspace's active tab).
    pub target_pane: String,
    /// `pane.resize` amount after the 50/50 open: shrinks the stage to ~25%, ≥ 10 rows.
    pub shrink: f32,
}

/// Where a new stage would go in `workspace`, or why not.
pub fn plan_stage(snap: &Snapshot, workspace: &str) -> Result<OpenStage, String> {
    let ws = snap
        .workspaces
        .iter()
        .find(|w| w.workspace_id == workspace)
        .ok_or("unknown workspace")?;
    let tab = ws.active_tab_id.as_deref().ok_or("no active tab")?;
    let layout = snap
        .layouts
        .iter()
        .find(|l| l.tab_id == tab)
        .ok_or("no layout")?;
    let height = layout.area.map_or(0, |a| a.height);
    if height < MIN_WORKSPACE_ROWS {
        return Err(format!("workspace only {height} rows tall"));
    }
    let target = layout.focused_pane_id.clone().ok_or("no focused pane")?;
    let share = STAGE_SHARE.max(STAGE_MIN_ROWS / height as f32);
    Ok(OpenStage {
        workspace: workspace.to_string(),
        target_pane: target,
        shrink: (0.5 - share).max(0.0),
    })
}

/// Which workspaces get a stage now (SPEC §5.1). Never one that already has a stage.
pub fn plan_autostart(snap: &Snapshot, mode: AutoOpen) -> Vec<OpenStage> {
    let mut targets: Vec<&str> = match mode {
        AutoOpen::Off => vec![],
        AutoOpen::EveryWorkspace => snap
            .workspaces
            .iter()
            .map(|w| w.workspace_id.as_str())
            .collect(),
        AutoOpen::FirstWorkspace => snap
            .workspaces
            .iter()
            .min_by_key(|w| w.number)
            .map(|w| w.workspace_id.as_str())
            .into_iter()
            .collect(),
    };
    targets.retain(|w| stage_pane(snap, w).is_none());
    targets
        .into_iter()
        .filter_map(|w| match plan_stage(snap, w) {
            Ok(p) => Some(p),
            Err(why) => {
                tracing::info!("autostart: skipping {w}: {why}");
                None
            }
        })
        .collect()
}

fn snapshot(c: &mut Client) -> anyhow::Result<Snapshot> {
    let mut v = c.request("session.snapshot", json!({}))?;
    Ok(serde_json::from_value(v["snapshot"].take())?)
}

fn open_stage(c: &mut Client, plan: &OpenStage) -> anyhow::Result<()> {
    let r = c.request(
        "plugin.pane.open",
        json!({
            "plugin_id": PLUGIN_ID,
            "entrypoint": "stage",
            "placement": "split",
            // The target pane picks the workspace; herdr rejects split + workspace_id.
            "target_pane_id": plan.target_pane,
            "direction": "down",
            "focus": false,
        }),
    )?;
    if let Some(pane) = r["plugin_pane"]["pane"]["pane_id"].as_str()
        && plan.shrink > 0.0
    {
        c.request(
            "pane.resize",
            json!({ "pane_id": pane, "direction": "down", "amount": plan.shrink }),
        )?;
    }
    Ok(())
}

/// Workspace the action was invoked from (`HERDR_PLUGIN_CONTEXT_JSON`), else ours.
fn context_workspace(snap: &Snapshot) -> Option<String> {
    let ctx: Value = std::env::var("HERDR_PLUGIN_CONTEXT_JSON")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    ctx["workspace_id"]
        .as_str()
        .map(String::from)
        .or_else(|| std::env::var("HERDR_WORKSPACE_ID").ok())
        .or_else(|| snap.focused_workspace_id.clone())
}

pub fn autostart(mode: AutoOpen) -> anyhow::Result<()> {
    let mut c = Client::connect()?;
    let mut snap = snapshot(&mut c)?;
    prune_dead_stages(&mut c, &mut snap);
    for plan in plan_autostart(&snap, mode) {
        if let Err(e) = open_stage(&mut c, &plan) {
            tracing::warn!("autostart: open stage in {}: {e:#}", plan.workspace);
        }
    }
    Ok(())
}

pub fn toggle_stage() -> anyhow::Result<()> {
    let mut c = Client::connect()?;
    let mut snap = snapshot(&mut c)?;
    prune_dead_stages(&mut c, &mut snap);
    let ws = context_workspace(&snap).ok_or_else(|| anyhow::anyhow!("no workspace"))?;
    match stage_pane(&snap, &ws) {
        Some(p) => {
            // A stage opened by an earlier install (link -> GitHub) is no longer this
            // plugin's pane, so plugin.pane.close refuses it; close it as a plain pane.
            if c.request("plugin.pane.close", json!({ "pane_id": p.pane_id }))
                .is_err()
            {
                c.request("pane.close", json!({ "pane_id": p.pane_id }))?;
            }
        }
        None => open_stage(&mut c, &plan_stage(&snap, &ws).map_err(anyhow::Error::msg)?)?,
    }
    Ok(())
}

/// Popups always open over the active pane (herdr rejects a workspace here).
pub fn open_ask() -> anyhow::Result<()> {
    Client::connect()?.request(
        "plugin.pane.open",
        json!({
            "plugin_id": PLUGIN_ID,
            "entrypoint": "ask",
            "placement": "popup",
            "width": "70%",
            "height": 20,
            "focus": true,
        }),
    )?;
    Ok(())
}

/// Tag our own pane as the stage (see `STAGE_TOKEN`). Best effort.
pub fn tag_stage(c: &mut Client, pane: &str) {
    let r = c.request(
        "pane.report_metadata",
        json!({
            "pane_id": pane,
            "source": format!("plugin:{PLUGIN_ID}"),
            "tokens": { STAGE_TOKEN: "1" },
        }),
    );
    if let Err(e) = r {
        tracing::warn!("tag stage pane: {e:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::{Area, LayoutInfo, WorkspaceInfo};
    use super::*;

    fn snap(heights: &[(u32, u16)]) -> Snapshot {
        let mut s: Snapshot = serde_json::from_value(json!({})).unwrap();
        for (n, h) in heights {
            let (w, t, p) = (format!("w{n}"), format!("w{n}:t1"), format!("w{n}:p1"));
            s.workspaces.push(WorkspaceInfo {
                workspace_id: w.clone(),
                active_tab_id: Some(t.clone()),
                number: *n,
            });
            s.layouts.push(LayoutInfo {
                tab_id: t.clone(),
                workspace_id: w.clone(),
                focused_pane_id: Some(p.clone()),
                zoomed: false,
                area: Some(Area {
                    height: *h,
                    width: 100,
                }),
            });
            s.panes.push(PaneInfo {
                pane_id: p,
                workspace_id: w,
                tab_id: t,
                label: None,
                tokens: Default::default(),
            });
        }
        s
    }

    #[test]
    fn every_workspace_but_never_twice_and_never_too_small() {
        let mut s = snap(&[(1, 40), (2, 40), (3, 20)]);
        s.panes.push(PaneInfo {
            pane_id: "w2:p9".into(),
            workspace_id: "w2".into(),
            tab_id: "w2:t1".into(),
            label: None,
            tokens: [(STAGE_TOKEN.to_string(), "1".to_string())].into(),
        });
        let plans = plan_autostart(&s, AutoOpen::EveryWorkspace);
        let ws: Vec<_> = plans.iter().map(|p| p.workspace.as_str()).collect();
        assert_eq!(ws, ["w1"], "w2 has a stage, w3 is too small");
        assert_eq!(plans[0].target_pane, "w1:p1");
        assert!((plans[0].shrink - 0.25).abs() < 1e-6, "50% → 25%");
    }

    #[test]
    fn label_counts_as_a_stage_before_it_tags_itself() {
        let mut s = snap(&[(1, 40)]);
        s.panes[0].label = Some(STAGE_LABEL.into());
        assert!(plan_autostart(&s, AutoOpen::EveryWorkspace).is_empty());
    }

    #[test]
    fn only_a_running_stage_process_counts() {
        let alive = json!({"process_info": {"foreground_processes": [
            {"name": "herdr-fantasy.exe", "argv": ["herdr-fantasy.EXE", "stage"]}]}});
        let shell = json!({"process_info": {"foreground_processes": [
            {"name": "powershell.exe"}]}});
        assert!(runs_stage(&alive));
        assert!(!runs_stage(&shell));
        assert!(!runs_stage(&json!({})));
    }

    #[test]
    fn short_workspace_keeps_ten_rows() {
        let s = snap(&[(1, 30)]);
        let p = plan_stage(&s, "w1").unwrap();
        assert!((p.shrink - (0.5 - 10.0 / 30.0)).abs() < 1e-6);
    }

    #[test]
    fn first_workspace_and_off() {
        let s = snap(&[(2, 40), (1, 40)]);
        let first = plan_autostart(&s, AutoOpen::FirstWorkspace);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].workspace, "w1");
        assert!(plan_autostart(&s, AutoOpen::Off).is_empty());
    }
}
