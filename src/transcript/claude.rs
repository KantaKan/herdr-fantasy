//! Claude Code transcript lines → agent-neutral entries (SPEC §3.3). Pure.
//!
//! Shapes verified against real transcripts (Claude Code, 2026-10):
//! - human prompt: `type:"user"`, `message.content` is a string; skip `isMeta`,
//!   `isCompactSummary`, sidechains, and command echoes that start with `<`.
//! - tool call: `type:"assistant"`, `message.content[]` has `{type:"tool_use", id, name, input}`.
//! - tool result: `type:"user"`, `message.content[]` has `{type:"tool_result", tool_use_id,
//!   is_error?}`; a non-zero Bash/PowerShell exit sets `is_error: true` ("Exit code N").
//! - usage: `message.usage` on assistant lines; context = input + cache_creation + cache_read.
//! - compaction: `type:"system", subtype:"compact_boundary", compactMetadata.postTokens`.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    /// What the human asked for.
    Prompt(String),
    Tool {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        id: String,
        is_error: bool,
        /// The start of the result text (up to 8 KB), to spot failures a pipe hid.
        text: String,
    },
    /// Current context size in tokens, and the model that reported it.
    Usage { context: u64, model: Option<String> },
    /// The conversation was compacted; new context size if known.
    Compact { post_tokens: Option<u64> },
}

pub fn parse(line: &str) -> Vec<Entry> {
    let Ok(e) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    if e["isSidechain"].as_bool() == Some(true) {
        return Vec::new(); // subagent chatter; the parent's Agent call covers it
    }
    let mut out = Vec::new();
    let msg = &e["message"];
    match e["type"].as_str() {
        Some("user") => {
            if e["isMeta"].as_bool() == Some(true) || e["isCompactSummary"].as_bool() == Some(true)
            {
                return out;
            }
            match &msg["content"] {
                Value::String(s) if !s.trim_start().starts_with('<') && !s.trim().is_empty() => {
                    out.push(Entry::Prompt(s.clone()));
                }
                Value::Array(blocks) => {
                    for b in blocks {
                        match b["type"].as_str() {
                            Some("tool_result") => out.push(Entry::ToolResult {
                                id: b["tool_use_id"].as_str().unwrap_or_default().to_string(),
                                is_error: b["is_error"].as_bool().unwrap_or(false),
                                text: result_text(&b["content"]),
                            }),
                            Some("text") => {
                                let t = b["text"].as_str().unwrap_or_default();
                                if !t.starts_with("[Request interrupted")
                                    && !t.starts_with('<')
                                    && !t.trim().is_empty()
                                {
                                    out.push(Entry::Prompt(t.to_string()));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        Some("assistant") => {
            if let Some(blocks) = msg["content"].as_array() {
                for b in blocks.iter().filter(|b| b["type"] == "tool_use") {
                    out.push(Entry::Tool {
                        id: b["id"].as_str().unwrap_or_default().to_string(),
                        name: b["name"].as_str().unwrap_or_default().to_string(),
                        input: b["input"].clone(),
                    });
                }
            }
            let u = &msg["usage"];
            if u.is_object() {
                let n = |k: &str| u[k].as_u64().unwrap_or(0);
                out.push(Entry::Usage {
                    context: n("input_tokens")
                        + n("cache_creation_input_tokens")
                        + n("cache_read_input_tokens"),
                    model: msg["model"].as_str().map(String::from),
                });
            }
        }
        Some("system") if e["subtype"] == "compact_boundary" => out.push(Entry::Compact {
            post_tokens: e["compactMetadata"]["postTokens"].as_u64(),
        }),
        _ => {}
    }
    out
}

/// Tool result content is a string or a list of `{type:"text", text}` blocks.
fn result_text(v: &Value) -> String {
    let mut s = match v {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| p["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };
    if s.len() > 8192 {
        let mut cut = 8192;
        while !s.is_char_boundary(cut) {
            cut -= 1;
        }
        s.truncate(cut);
    }
    s
}

/// Where Claude Code keeps a session's transcript, given herdr's `agent_session`:
/// a path is used as-is; an id lives at `<claude dir>/projects/<slug(cwd)>/<id>.jsonl`,
/// where slug replaces every non-alphanumeric char of the cwd with `-`.
pub fn transcript_path(
    kind: &str,
    value: &str,
    cwd: &str,
    claude_dir: &std::path::Path,
) -> std::path::PathBuf {
    if kind == "path" {
        return value.into();
    }
    let slug: String = cwd
        .trim_end_matches(['/', '\\'])
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    claude_dir
        .join("projects")
        .join(slug)
        .join(format!("{value}.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<Entry> {
        include_str!("../../tests/fixtures/claude_transcript.jsonl")
            .lines()
            .flat_map(parse)
            .collect()
    }

    #[test]
    fn real_shapes_parse() {
        let all = fixture();
        assert_eq!(
            all.iter().filter(|e| matches!(e, Entry::Prompt(_))).count(),
            1,
            "meta and compact-summary user lines are not prompts: {all:#?}"
        );
        let tools: Vec<_> = all
            .iter()
            .filter_map(|e| match e {
                Entry::Tool { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            tools,
            [
                "Read",
                "Grep",
                "Glob",
                "Edit",
                "Write",
                "Bash",
                "PowerShell",
                "Skill",
                "Agent"
            ]
        );
        assert!(all.iter().any(|e| matches!(e, Entry::ToolResult { id, is_error: true, text } if id == "toolu_bash" && text.starts_with("Exit code"))));
        assert!(all.iter().any(|e| matches!(
            e,
            Entry::ToolResult {
                is_error: false,
                ..
            }
        )));
        assert!(
            all.iter()
                .any(|e| matches!(e, Entry::Usage { context, .. } if *context > 0))
        );
        assert!(all.iter().any(|e| matches!(
            e,
            Entry::Compact {
                post_tokens: Some(_)
            }
        )));
    }

    #[test]
    fn skips_noise() {
        for line in [
            "not json",
            r#"{"type":"user","isSidechain":true,"message":{"content":"sub prompt"}}"#,
            r#"{"type":"user","message":{"content":"<command-name>/clear</command-name>"}}"#,
            r#"{"type":"user","message":{"content":[{"type":"text","text":"[Request interrupted by user]"}]}}"#,
            r#"{"type":"last-prompt","lastPrompt":"x"}"#,
        ] {
            assert!(parse(line).is_empty(), "{line}");
        }
    }

    #[test]
    fn transcript_path_from_session_id() {
        let p = transcript_path(
            "id",
            "abc",
            r"D:\code\hrdr-game",
            std::path::Path::new("/c"),
        );
        assert!(p.ends_with("projects/D--code-hrdr-game/abc.jsonl"), "{p:?}");
        let p = transcript_path("id", "abc", "/home/me/my.app/", std::path::Path::new("/c"));
        assert!(p.ends_with("projects/-home-me-my-app/abc.jsonl"), "{p:?}");
        assert_eq!(
            transcript_path("path", "/x/y.jsonl", "/ignored", std::path::Path::new("/c")),
            std::path::PathBuf::from("/x/y.jsonl")
        );
    }
}
