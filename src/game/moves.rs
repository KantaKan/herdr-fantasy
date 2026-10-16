//! What a hero does for each agent tool call (SPEC §5.4). Pure.

use serde_json::Value;

use super::hero::Action;

#[derive(Debug, Clone, PartialEq)]
pub struct Move {
    pub action: Action,
    /// Footer activity text, e.g. "editing auth.rs".
    pub detail: String,
    /// Floating text above the hero (text, palette key).
    pub float: Option<(String, char)>,
}

fn mv(action: Action, detail: String) -> Option<Move> {
    Some(Move {
        action,
        detail,
        float: None,
    })
}

/// Last two path components: `src/auth.rs`.
fn short_path(p: &str) -> String {
    let parts: Vec<&str> = p.split(['/', '\\']).filter(|s| !s.is_empty()).collect();
    parts[parts.len().saturating_sub(2)..].join("/")
}

fn lines(v: &Value) -> usize {
    v.as_str().map_or(0, |s| s.lines().count())
}

/// Damage for an edit: lines written, clamped 1–999.
fn damage(name: &str, input: &Value) -> usize {
    let n = match name {
        "Write" => lines(&input["content"]),
        "NotebookEdit" => lines(&input["new_source"]),
        "MultiEdit" => input["edits"]
            .as_array()
            .map_or(0, |es| es.iter().map(|e| lines(&e["new_string"])).sum()),
        _ => lines(&input["new_string"]),
    };
    n.clamp(1, 999)
}

pub fn tool_move(name: &str, input: &Value) -> Option<Move> {
    let file = || short_path(input["file_path"].as_str().unwrap_or(""));
    let s = |k: &str| input[k].as_str().unwrap_or("").to_string();
    match name {
        "Read" => mv(Action::Open, format!("reading {}", file())),
        "Grep" => mv(Action::Study, format!("searching for {}", s("pattern"))),
        "Glob" | "LS" => mv(Action::Scout, format!("looking for {}", s("pattern"))),
        "WebSearch" | "WebFetch" => mv(Action::Scout, "searching the web".into()),
        "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => Some(Move {
            action: Action::Attack,
            detail: format!("editing {}", file()),
            float: Some((format!("-{}", damage(name, input)), 'w')),
        }),
        "Bash" | "PowerShell" => Some(shell_move(&s("command"))),
        "Skill" => {
            let skill = s("skill");
            let short = skill.rsplit(':').next().unwrap_or(&skill).to_string();
            Some(Move {
                action: Action::Spellbook,
                detail: format!("studying {short}"),
                float: Some((format!("learned: {short}"), 'C')),
            })
        }
        "Agent" | "Task" => mv(Action::Cast, format!("summoning {}", s("subagent_type"))),
        _ => None,
    }
}

/// First real command of a shell line: skip `cd`, env assignments, `sudo`/`time`.
pub fn commands(line: &str) -> Vec<Vec<String>> {
    line.split(['|', ';', '&'])
        .map(|seg| {
            seg.split_whitespace()
                .skip_while(|w| w.contains('=') || ["sudo", "time", "env", "&&"].contains(w))
                .map(|w| w.to_lowercase())
                .collect::<Vec<_>>()
        })
        .filter(|ws| !ws.is_empty() && ws[0] != "cd" && ws[0] != "set-location")
        .collect()
}

/// Which move a shell command gets (SPEC §5.4 command table); otherwise a plain spell.
pub fn shell_move(line: &str) -> Move {
    let cmds = commands(line);
    for ws in &cmds {
        let head = ws[0].trim_start_matches(['.', '/', '\\']);
        let sub = ws.get(1).map(String::as_str).unwrap_or("");
        let pick = match head {
            "grep" | "rg" | "ag" | "findstr" | "select-string" | "sls" => {
                Some((Action::Study, "reading the big book"))
            }
            "cat" | "less" | "more" | "head" | "tail" | "type" | "get-content" | "gc" | "bat" => {
                Some((Action::Scroll, "reading a scroll"))
            }
            "ls" | "dir" | "find" | "fd" | "tree" | "get-childitem" | "gci" => {
                Some((Action::Scout, "scouting ahead"))
            }
            "curl" | "wget" | "invoke-webrequest" | "iwr" | "invoke-restmethod" | "irm" => {
                Some((Action::Bird, "sending a messenger bird"))
            }
            "git" => match sub {
                "commit" => Some((Action::Seal, "sealing a commit")),
                "push" => Some((Action::PigeonOut, "sending a carrier pigeon")),
                "pull" | "fetch" | "clone" => Some((Action::PigeonIn, "a pigeon brings news")),
                _ => None,
            },
            "npm" | "pnpm" | "yarn" | "bun" | "cargo" | "pip" | "pip3" | "uv" | "poetry"
                if ["install", "add", "i"].contains(&sub) =>
            {
                Some((Action::Shop, "shopping for supplies"))
            }
            "rm" | "del" | "rmdir" | "remove-item" | "ri" => {
                Some((Action::Fireball, "casting fireball"))
            }
            "mkdir" | "md" | "touch" | "cp" | "mv" | "new-item" | "ni" | "copy-item"
            | "move-item" => Some((Action::Craft, "forging")),
            "sleep" | "start-sleep" => Some((Action::Doze, "dozing off")),
            _ => None,
        };
        if let Some((action, detail)) = pick {
            return Move {
                action,
                detail: detail.into(),
                float: None,
            };
        }
    }
    let shown = cmds
        .first()
        .map(|ws| ws.iter().take(3).cloned().collect::<Vec<_>>().join(" "));
    Move {
        action: Action::Cast,
        detail: format!("running {}", shown.unwrap_or_else(|| "a spell".into())),
        float: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn act(cmd: &str) -> Action {
        shell_move(cmd).action
    }

    #[test]
    fn command_table() {
        use Action::*;
        for (cmd, want) in [
            ("grep -rn redirect src", Study),
            ("cd /d/code/app && rg login", Study),
            ("Select-String -Pattern foo *.rs", Study),
            ("cat Cargo.toml", Scroll),
            ("tail -f log.txt", Scroll),
            ("Get-Content README.md", Scroll),
            ("ls -la", Scout),
            ("Get-ChildItem src", Scout),
            ("find . -name '*.rs'", Scout),
            ("curl -s https://example.com", Bird),
            ("git commit -m 'fix'", Seal),
            ("git push origin main", PigeonOut),
            ("git pull --rebase", PigeonIn),
            ("npm install", Shop),
            ("cargo add serde", Shop),
            ("pip install requests", Shop),
            ("rm -rf target", Fireball),
            ("Remove-Item -Recurse dist", Fireball),
            ("mkdir -p src/game", Craft),
            ("Start-Sleep -Seconds 4", Doze),
            ("RUST_LOG=debug cargo run", Cast),
            ("git status", Cast),
            ("npm run build", Cast),
        ] {
            assert_eq!(act(cmd), want, "{cmd}");
        }
    }

    #[test]
    fn default_spell_names_the_command() {
        assert_eq!(
            shell_move("cargo test --workspace").detail,
            "running cargo test --workspace"
        );
        assert_eq!(shell_move("").detail, "running a spell");
    }

    #[test]
    fn tools() {
        let edit = tool_move(
            "Edit",
            &json!({"file_path": "/x/src/auth.rs", "new_string": "a\nb\nc"}),
        )
        .unwrap();
        assert_eq!(
            (edit.action, edit.detail.as_str()),
            (Action::Attack, "editing src/auth.rs")
        );
        assert_eq!(edit.float, Some(("-3".into(), 'w')));
        let big = "x\n".repeat(5000);
        let w = tool_move("Write", &json!({"file_path": "a.rs", "content": big})).unwrap();
        assert_eq!(w.float.unwrap().0, "-999");
        let skill = tool_move("Skill", &json!({"skill": "superpowers:brainstorming"})).unwrap();
        assert_eq!(skill.action, Action::Spellbook);
        assert_eq!(skill.float.unwrap().0, "learned: brainstorming");
        assert_eq!(
            tool_move("Read", &json!({"file_path": r"D:\c\src\main.rs"}))
                .unwrap()
                .detail,
            "reading src/main.rs"
        );
        assert_eq!(
            tool_move("Grep", &json!({"pattern": "x"})).unwrap().action,
            Action::Study
        );
        assert_eq!(
            tool_move("PowerShell", &json!({"command": "git push"}))
                .unwrap()
                .action,
            Action::PigeonOut
        );
        assert!(tool_move("AskUserQuestion", &json!({})).is_none());
        assert!(tool_move("mcp__x__y", &json!({})).is_none());
    }
}
