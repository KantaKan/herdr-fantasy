//! `$HERDR_PLUGIN_CONFIG_DIR/config.toml` (SPEC §9). Invalid values fall back to the
//! default per key with a warning; loading never fails.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoOpen {
    EveryWorkspace,
    FirstWorkspace,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Render {
    Auto,
    Kitty,
    Halfblock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotifySound {
    None,
    Done,
    Request,
}

pub const CLASSES: [&str; 8] = [
    "Knight",
    "Mage",
    "Rogue",
    "Ranger",
    "Cleric",
    "Bard",
    "Sahur Knight",
    "Wild Card",
];

pub const PETS: [&str; 4] = ["cat", "dog", "dragon", "owl"];

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub auto_open: AutoOpen,
    pub render: Render,
    pub reduced_motion: bool,
    pub memes: bool,
    pub notify_on_blocked: bool,
    pub notify_sound: NotifySound,
    pub store_prompt_text: bool,
    pub show_all_workspaces: bool,
    /// agent kind → class name; `"default"` covers unknown kinds.
    pub classes: BTreeMap<String, String>,
    /// model id → context window tokens; `"default"` covers unknown models.
    pub context_window: BTreeMap<String, u64>,
    /// Extra user boss rules, checked before built-ins.
    pub bosses: Bosses,
    /// agent kind → pet (M14); `"default"` covers unknown kinds. None by default.
    pub pets: BTreeMap<String, String>,
    /// Costume recolors unlocked at Lv 5 / 10 / 20 (M14).
    pub skins: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Bosses {
    pub test: Vec<String>,
    pub build: Vec<String>,
    pub lint: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        let classes = [
            ("claude", "Knight"),
            ("codex", "Mage"),
            ("cursor", "Rogue"),
            ("opencode", "Ranger"),
            ("gemini", "Cleric"),
            ("default", "Bard"),
        ];
        Self {
            auto_open: AutoOpen::EveryWorkspace,
            render: Render::Auto,
            reduced_motion: false,
            memes: true,
            notify_on_blocked: true,
            notify_sound: NotifySound::Request,
            store_prompt_text: true,
            show_all_workspaces: false,
            classes: classes
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            context_window: BTreeMap::from([("default".to_string(), 200_000)]),
            bosses: Bosses::default(),
            pets: BTreeMap::new(),
            skins: true,
        }
    }
}

impl Config {
    /// Parse config text. Returns the config plus human-readable warnings.
    pub fn parse(text: &str) -> (Self, Vec<String>) {
        let mut c = Self::default();
        let mut warn = Vec::new();
        let table: toml::Table = match text.parse() {
            Ok(t) => t,
            Err(e) => {
                warn.push(format!("not valid TOML, using defaults: {e}"));
                return (c, warn);
            }
        };
        for (key, value) in table {
            match key.as_str() {
                "auto_open" => set(&mut c.auto_open, &key, value, &mut warn),
                "render" => set(&mut c.render, &key, value, &mut warn),
                "reduced_motion" => set(&mut c.reduced_motion, &key, value, &mut warn),
                "memes" => set(&mut c.memes, &key, value, &mut warn),
                "notify_on_blocked" => set(&mut c.notify_on_blocked, &key, value, &mut warn),
                "notify_sound" => set(&mut c.notify_sound, &key, value, &mut warn),
                "store_prompt_text" => set(&mut c.store_prompt_text, &key, value, &mut warn),
                "show_all_workspaces" => set(&mut c.show_all_workspaces, &key, value, &mut warn),
                "classes" => merge(&mut c.classes, &key, value, &mut warn, |v: &String| {
                    CLASSES.contains(&v.as_str())
                }),
                "context_window" => {
                    merge(&mut c.context_window, &key, value, &mut warn, |v: &u64| {
                        *v > 0
                    })
                }
                "bosses" => set(&mut c.bosses, &key, value, &mut warn),
                "pets" => merge(&mut c.pets, &key, value, &mut warn, |v: &String| {
                    PETS.contains(&v.as_str())
                }),
                "skins" => set(&mut c.skins, &key, value, &mut warn),
                _ => warn.push(format!("unknown key `{key}` ignored")),
            }
        }
        (c, warn)
    }
}

fn set<T: DeserializeOwned>(slot: &mut T, key: &str, value: toml::Value, warn: &mut Vec<String>) {
    match value.try_into() {
        Ok(v) => *slot = v,
        Err(e) => warn.push(format!("`{key}` invalid, using default: {e}")),
    }
}

/// Merge a table entry-by-entry so one bad entry keeps the rest.
fn merge<T: DeserializeOwned>(
    map: &mut BTreeMap<String, T>,
    key: &str,
    value: toml::Value,
    warn: &mut Vec<String>,
    valid: impl Fn(&T) -> bool,
) {
    let toml::Value::Table(table) = value else {
        warn.push(format!("`{key}` must be a table, using defaults"));
        return;
    };
    for (k, v) in table {
        match v.try_into::<T>() {
            Ok(v) if valid(&v) => {
                map.insert(k, v);
            }
            _ => warn.push(format!("`{key}.{k}` invalid, using default")),
        }
    }
}

/// Write one setting into config.toml, keeping everything else (comments too):
/// replaces the key's line, or adds it under its table / before the first table.
pub fn save_edit(
    path: &std::path::Path,
    edit: &crate::game::settings::Edit,
) -> std::io::Result<()> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let out = apply_edit(&text, edit);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, out)
}

fn apply_edit(text: &str, edit: &crate::game::settings::Edit) -> String {
    use crate::game::settings::Edit;
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let is_key = |l: &str, key: &str| {
        let l = l.trim_start();
        let k = l.split('=').next().unwrap_or("").trim().trim_matches('"');
        !l.starts_with('#') && l.contains('=') && k == key
    };
    let first_table = lines
        .iter()
        .position(|l| l.trim_start().starts_with('['))
        .unwrap_or(lines.len());
    match edit {
        Edit::Flag { key, value } => {
            let new = format!("{key} = {value}");
            match lines[..first_table].iter().position(|l| is_key(l, key)) {
                Some(i) => lines[i] = new,
                None => lines.insert(first_table, new),
            }
        }
        Edit::Table { table, key, value } => {
            let header = format!("[{table}]");
            let start = lines.iter().position(|l| l.trim() == header);
            let new = value.as_ref().map(|v| {
                let k = if key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                {
                    key.clone()
                } else {
                    format!("\"{key}\"")
                };
                format!("{k} = \"{v}\"")
            });
            match start {
                Some(s) => {
                    let end = lines[s + 1..]
                        .iter()
                        .position(|l| l.trim_start().starts_with('['))
                        .map_or(lines.len(), |e| s + 1 + e);
                    match (lines[s + 1..end].iter().position(|l| is_key(l, key)), new) {
                        (Some(i), Some(n)) => lines[s + 1 + i] = n,
                        (Some(i), None) => {
                            lines.remove(s + 1 + i);
                        }
                        (None, Some(n)) => lines.insert(s + 1, n),
                        (None, None) => {}
                    }
                }
                None => {
                    if let Some(n) = new {
                        if lines.last().is_some_and(|l| !l.trim().is_empty()) {
                            lines.push(String::new());
                        }
                        lines.push(header);
                        lines.push(n);
                    }
                }
            }
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

pub fn config_path() -> Option<PathBuf> {
    let dir = match std::env::var_os("HERDR_PLUGIN_CONFIG_DIR") {
        Some(d) => PathBuf::from(d),
        None => directories::ProjectDirs::from("", "", "herdr-fantasy")?
            .config_dir()
            .to_path_buf(),
    };
    Some(dir.join("config.toml"))
}

/// Load from disk. A missing file is not a warning.
pub fn load() -> (Config, Vec<String>) {
    match config_path() {
        Some(path) => load_from(&path),
        None => (Config::default(), vec![]),
    }
}

fn load_from(path: &std::path::Path) -> (Config, Vec<String>) {
    match std::fs::read_to_string(path) {
        Ok(text) => Config::parse(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Config::default(), vec![]),
        Err(e) => (
            Config::default(),
            vec![format!("cannot read {}: {e}", path.display())],
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_defaults_without_warning() {
        let (c, w) = load_from(std::path::Path::new("definitely/not/here/config.toml"));
        assert_eq!(c, Config::default());
        assert!(w.is_empty());
    }

    #[test]
    fn empty_file_is_defaults() {
        let (c, w) = Config::parse("");
        assert_eq!(c, Config::default());
        assert!(w.is_empty());
    }

    #[test]
    fn partial_file_keeps_other_defaults() {
        let (c, w) = Config::parse("render = \"halfblock\"\n[classes]\ncodex = \"Rogue\"\n");
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(c.render, Render::Halfblock);
        assert_eq!(c.classes["codex"], "Rogue");
        assert_eq!(c.classes["claude"], "Knight");
        assert_eq!(c.auto_open, AutoOpen::EveryWorkspace);
    }

    #[test]
    fn invalid_values_fall_back_per_key() {
        let text = r#"
            auto_open = "sometimes"
            memes = "yes"
            reduced_motion = true
            nonsense = 1
            [classes]
            claude = "Batman"
            gemini = "Mage"
            [context_window]
            default = 0
            "claude-opus-5-5" = 1000000
        "#;
        let (c, w) = Config::parse(text);
        let d = Config::default();
        assert_eq!(c.auto_open, d.auto_open);
        assert_eq!(c.memes, d.memes);
        assert!(c.reduced_motion);
        assert_eq!(c.classes["claude"], "Knight");
        assert_eq!(c.classes["gemini"], "Mage");
        assert_eq!(c.context_window["default"], 200_000);
        assert_eq!(c.context_window["claude-opus-5-5"], 1_000_000);
        assert_eq!(w.len(), 5, "{w:?}");
    }

    #[test]
    fn broken_toml_is_defaults_with_warning() {
        let (c, w) = Config::parse("render = ");
        assert_eq!(c, Config::default());
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn pets_are_checked() {
        let (c, w) = Config::parse(
            "skins = false
[pets]
claude = \"dragon\"
codex = \"unicorn\"
",
        );
        assert_eq!(c.pets["claude"], "dragon");
        assert!(!c.pets.contains_key("codex"));
        assert!(!c.skins);
        assert_eq!(w.len(), 1, "{w:?}");
    }

    #[test]
    fn edits_keep_comments_and_other_keys() {
        use crate::game::settings::Edit;
        let text = "# my settings\nmemes = true\n\n[classes]\n# me\nclaude = \"Wild Card\"\n";
        let t = apply_edit(
            text,
            &Edit::Flag {
                key: "memes",
                value: false,
            },
        );
        assert!(t.contains("# my settings\nmemes = false"));
        let t = apply_edit(
            &t,
            &Edit::Flag {
                key: "skins",
                value: false,
            },
        );
        assert!(t.find("skins = false").unwrap() < t.find("[classes]").unwrap());
        let t = apply_edit(
            &t,
            &Edit::Table {
                table: "classes",
                key: "claude".into(),
                value: Some("Mage".into()),
            },
        );
        assert!(t.contains("# me\nclaude = \"Mage\""));
        let t = apply_edit(
            &t,
            &Edit::Table {
                table: "pets",
                key: "claude".into(),
                value: Some("owl".into()),
            },
        );
        assert!(t.ends_with("[pets]\nclaude = \"owl\"\n"));
        let t = apply_edit(
            &t,
            &Edit::Table {
                table: "pets",
                key: "claude".into(),
                value: None,
            },
        );
        assert!(!t.contains("owl"));
        let (c, w) = Config::parse(&t);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(c.classes["claude"], "Mage");
        assert!(!c.memes && !c.skins);
    }

    #[test]
    fn bosses_extra_rules() {
        let (c, w) = Config::parse("[bosses]\ntest = [\"just t\"]\n");
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(c.bosses.test, vec!["just t"]);
        assert!(c.bosses.lint.is_empty());
    }
}
