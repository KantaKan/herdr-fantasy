//! Codex CLI rollout lines → the same agent-neutral entries as Claude (TASKS M17). Pure.
//!
//! A rollout is `~/.codex/sessions/YYYY/MM/DD/rollout-<time>-<session id>.jsonl`; herdr's
//! Codex integration reports the session id. Each line is `{timestamp, type, payload}`
//! (envelope and `session_meta` verified 2026-10-17 on a real file).
//! ⚠ The payload shapes below follow Codex's rollout format but were not yet seen in a
//! real session with tool calls on this machine (SPEC §15):
//! - prompt: `event_msg` / `user_message` `{message}`.
//! - shell: `response_item` / `function_call` `{name: shell|shell_command|exec_command|
//!   local_shell, arguments: "<json>", call_id}`; `command` is an argv array
//!   (`["bash","-lc","ls"]`) or a string (`cmd` / `command`).
//! - edit: `custom_tool_call` (or `function_call`) `{name: apply_patch, input|arguments}`
//!   with `*** Update File: <path>` / `*** Add File: <path>` and `+` lines.
//! - result: `function_call_output` / `custom_tool_call_output` `{call_id, output}`;
//!   the exit code is in `metadata.exit_code` (JSON output) or "Exit code: N" text.
//! - usage: `event_msg` / `token_count` `{info: {last_token_usage, model_context_window}}`.
//! - compaction: `compacted`, or `event_msg` / `context_compacted`.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::claude::Entry;

pub fn parse(line: &str) -> Vec<Entry> {
    let Ok(e) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    let p = &e["payload"];
    let kind = p["type"].as_str().unwrap_or("");
    let mut out = Vec::new();
    match (e["type"].as_str().unwrap_or(""), kind) {
        ("event_msg", "user_message") => {
            let m = p["message"].as_str().unwrap_or("").trim();
            if !m.is_empty() && !m.starts_with('<') && !m.starts_with('/') {
                out.push(Entry::Prompt(m.to_string()));
            }
        }
        ("response_item", "function_call" | "custom_tool_call") => {
            let name = p["name"].as_str().unwrap_or("");
            let id = p["call_id"].as_str().unwrap_or("").to_string();
            // `arguments` is a JSON string for function calls; `input` is raw text for
            // custom tools like apply_patch.
            let args: Value = p["arguments"]
                .as_str()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(Value::Null);
            match name {
                "shell" | "shell_command" | "exec_command" | "local_shell" => {
                    out.push(Entry::Tool {
                        id,
                        name: "Bash".into(),
                        input: json!({ "command": command(&args) }),
                    });
                }
                "apply_patch" => {
                    let patch = p["input"]
                        .as_str()
                        .or_else(|| args["input"].as_str())
                        .or_else(|| p["arguments"].as_str())
                        .unwrap_or("");
                    out.push(Entry::Tool {
                        id,
                        name: "Edit".into(),
                        input: patch_input(patch),
                    });
                }
                "web_search" => out.push(Entry::Tool {
                    id,
                    name: "WebSearch".into(),
                    input: args,
                }),
                _ => {}
            }
        }
        ("response_item", "function_call_output" | "custom_tool_call_output") => {
            let id = p["call_id"].as_str().unwrap_or("").to_string();
            let raw = match &p["output"] {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            let (code, text) = exit_and_text(&raw);
            out.push(Entry::ToolResult {
                id,
                is_error: code.is_some_and(|c| c != 0),
                text,
            });
        }
        ("event_msg", "token_count") => {
            let last = &p["info"]["last_token_usage"];
            if last.is_object() {
                let n = |k: &str| last[k].as_u64().unwrap_or(0);
                out.push(Entry::Usage {
                    context: n("input_tokens") + n("output_tokens"),
                    output: n("output_tokens"),
                    model: None,
                    message: None,
                    window: p["info"]["model_context_window"].as_u64(),
                });
            }
        }
        ("compacted", _) | ("event_msg", "context_compacted") => {
            out.push(Entry::Compact { post_tokens: None });
        }
        _ => {}
    }
    out
}

/// The shell command line from a call's arguments: `bash -lc "<cmd>"` → `<cmd>`.
fn command(args: &Value) -> String {
    if let Some(argv) = args["command"].as_array() {
        let parts: Vec<&str> = argv.iter().filter_map(|v| v.as_str()).collect();
        return match parts.as_slice() {
            [_, flag, cmd] if flag.starts_with('-') && flag.contains('c') => cmd.to_string(),
            _ => parts.join(" "),
        };
    }
    args["cmd"]
        .as_str()
        .or_else(|| args["command"].as_str())
        .unwrap_or("")
        .to_string()
}

/// Edit input for `moves::tool_move`: the first file the patch touches and its added
/// lines (the swing's damage).
fn patch_input(patch: &str) -> Value {
    let file = patch
        .lines()
        .find_map(|l| {
            l.strip_prefix("*** Update File: ")
                .or_else(|| l.strip_prefix("*** Add File: "))
        })
        .unwrap_or("")
        .trim();
    let added: Vec<&str> = patch
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .map(|l| &l[1..])
        .collect();
    json!({ "file_path": file, "new_string": added.join("\n") })
}

/// (exit code if known, output text) from a tool output: JSON `{output, metadata:
/// {exit_code}}`, or text with an "Exit code: N" / "Process exited with code N" line.
fn exit_and_text(raw: &str) -> (Option<i64>, String) {
    if let Ok(v) = serde_json::from_str::<Value>(raw)
        && v.is_object()
    {
        let text = v["output"].as_str().unwrap_or("").to_string();
        return (v["metadata"]["exit_code"].as_i64(), text);
    }
    let code = raw.lines().find_map(|l| {
        let l = l.trim();
        l.strip_prefix("Exit code:")
            .or_else(|| l.strip_prefix("Process exited with code"))
            .and_then(|n| n.trim().parse().ok())
    });
    (code, raw.chars().take(8192).collect())
}

/// Codex's home: `$CODEX_HOME`, else `~/.codex`.
pub fn codex_dir() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            directories::BaseDirs::new()
                .map(|d| d.home_dir().join(".codex"))
                .unwrap_or_else(|| ".codex".into())
        })
}

/// The rollout file of session `id` under `<codex>/sessions/YYYY/MM/DD/`, newest days
/// first. The caller caches the answer; this walks directories.
pub fn find_session(codex_dir: &Path, id: &str) -> Option<PathBuf> {
    let suffix = format!("{id}.jsonl");
    let sorted_dirs = |p: &Path| -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(p)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .collect();
        v.sort();
        v.reverse();
        v
    };
    for year in sorted_dirs(&codex_dir.join("sessions")) {
        for month in sorted_dirs(&year) {
            for day in sorted_dirs(&month) {
                if let Some(f) = sorted_dirs(&day).into_iter().find(|f| {
                    f.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.ends_with(&suffix))
                }) {
                    return Some(f);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(kind: &str, payload: Value) -> String {
        json!({"timestamp": "2026-10-17T00:00:00Z", "type": kind, "payload": payload}).to_string()
    }

    #[test]
    fn prompts_shell_patches_results_usage_and_compaction() {
        let lines = [
            line("session_meta", json!({"id": "x", "cwd": "/r"})),
            line(
                "event_msg",
                json!({"type": "user_message", "message": "fix the login bug"}),
            ),
            line(
                "event_msg",
                json!({"type": "user_message", "message": "<environment_context>"}),
            ),
            line(
                "response_item",
                json!({"type": "function_call", "name": "shell", "call_id": "c1",
                "arguments": "{\"command\":[\"bash\",\"-lc\",\"cargo test\"]}"}),
            ),
            line(
                "response_item",
                json!({"type": "function_call_output", "call_id": "c1",
                "output": "{\"output\":\"test result: FAILED\",\"metadata\":{\"exit_code\":101}}"}),
            ),
            line(
                "response_item",
                json!({"type": "custom_tool_call", "name": "apply_patch", "call_id": "c2",
                "input": "*** Begin Patch\n*** Update File: src/auth.rs\n@@\n-old\n+new\n+lines\n*** End Patch"}),
            ),
            line(
                "response_item",
                json!({"type": "custom_tool_call_output", "call_id": "c2",
                "output": "Exit code: 0\nWall time: 0.1 seconds\nOutput:\nSuccess"}),
            ),
            line(
                "event_msg",
                json!({"type": "token_count", "info": {
                "last_token_usage": {"input_tokens": 9000, "cached_input_tokens": 8000, "output_tokens": 500},
                "model_context_window": 272000}}),
            ),
            line("compacted", json!({"message": "summary"})),
        ];
        let all: Vec<Entry> = lines.iter().flat_map(|l| parse(l)).collect();
        assert_eq!(all[0], Entry::Prompt("fix the login bug".into()));
        assert!(matches!(&all[1], Entry::Tool { name, input, .. }
            if name == "Bash" && input["command"] == "cargo test"));
        assert!(matches!(&all[2], Entry::ToolResult { is_error: true, .. }));
        assert!(matches!(&all[3], Entry::Tool { name, input, .. }
            if name == "Edit" && input["file_path"] == "src/auth.rs" && input["new_string"] == "new\nlines"));
        assert!(matches!(
            &all[4],
            Entry::ToolResult {
                is_error: false,
                ..
            }
        ));
        assert_eq!(
            all[5],
            Entry::Usage {
                context: 9500,
                output: 500,
                model: None,
                message: None,
                window: Some(272000)
            }
        );
        assert_eq!(all[6], Entry::Compact { post_tokens: None });
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn string_commands_and_unknown_lines() {
        let l = line(
            "response_item",
            json!({"type": "function_call", "name": "exec_command",
            "call_id": "c", "arguments": "{\"cmd\":\"git status\"}"}),
        );
        assert!(
            matches!(&parse(&l)[0], Entry::Tool { input, .. } if input["command"] == "git status")
        );
        assert!(parse("not json").is_empty());
        assert!(parse(&line("event_msg", json!({"type": "agent_reasoning"}))).is_empty());
    }

    #[test]
    fn finds_the_rollout_by_session_id() {
        let root = std::env::temp_dir().join(format!("hf-codex-{}", std::process::id()));
        let day = root.join("sessions/2026/10/17");
        std::fs::create_dir_all(&day).unwrap();
        let f = day.join("rollout-2026-10-17T09-00-00-abc-123.jsonl");
        std::fs::write(&f, "").unwrap();
        assert_eq!(find_session(&root, "abc-123"), Some(f));
        assert_eq!(find_session(&root, "nope"), None);
        let _ = std::fs::remove_dir_all(root);
    }
}
