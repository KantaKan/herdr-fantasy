//! "An agent needs you" JRPG dialogue (SPEC §5.5, ART.md §6). Reads only; the one
//! thing it ever does to an agent is `agent.focus`. It never sends keys.

use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Widget};
use serde_json::json;

use super::halfblock::{self, rgb};
use super::sprites::sheet;
use crate::config::Config;
use crate::game::hero::Class;
use crate::herdr::client::Client;
use crate::herdr::types::{AgentStatus, Snapshot};
use crate::palette;

const CHARS_PER_SEC: u64 = 45;
const PORTRAIT_COLS: u16 = 32;

#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub name: String,
    pub class: Class,
    pub pane_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opt {
    Go,
    Next,
    Close,
}

impl Opt {
    fn label(self) -> &'static str {
        match self {
            Opt::Go => "Go to this agent",
            Opt::Next => "Next agent",
            Opt::Close => "Close",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Stay,
    Exit,
    Focus(String),
}

pub struct Ask {
    pub targets: Vec<Target>,
    pub idx: usize,
    pub sel: usize,
    /// Time on the current target; drives the typewriter and the talk animation.
    pub typed_ms: u64,
    pub skip: bool,
    pub clock_ms: u64,
}

impl Ask {
    pub fn new(targets: Vec<Target>) -> Self {
        Ask {
            targets,
            idx: 0,
            sel: 0,
            typed_ms: 0,
            skip: false,
            clock_ms: 0,
        }
    }

    fn target(&self) -> Option<&Target> {
        self.targets.get(self.idx)
    }

    pub fn options(&self) -> Vec<Opt> {
        let mut o = Vec::new();
        if self.target().is_some() {
            o.push(Opt::Go);
        }
        if self.targets.len() > 1 {
            o.push(Opt::Next);
        }
        o.push(Opt::Close);
        o
    }

    fn text_len(&self) -> usize {
        self.target().map_or(0, |t| t.text.chars().count())
    }

    pub fn shown_chars(&self) -> usize {
        let typed = (self.typed_ms * CHARS_PER_SEC / 1000) as usize;
        if self.skip {
            self.text_len()
        } else {
            typed.min(self.text_len())
        }
    }

    pub fn typing(&self) -> bool {
        self.shown_chars() < self.text_len()
    }

    pub fn tick(&mut self, dt: u64) {
        self.typed_ms += dt;
        self.clock_ms += dt;
    }

    pub fn key(&mut self, code: KeyCode) -> Outcome {
        let n = self.options().len();
        match code {
            KeyCode::Esc | KeyCode::Char('q') => return Outcome::Exit,
            KeyCode::Left | KeyCode::Up | KeyCode::BackTab => self.sel = (self.sel + n - 1) % n,
            KeyCode::Right | KeyCode::Down | KeyCode::Tab => self.sel = (self.sel + 1) % n,
            KeyCode::Enter if !self.typing() => match self.options()[self.sel.min(n - 1)] {
                Opt::Go => return Outcome::Focus(self.target().unwrap().pane_id.clone()),
                Opt::Next => {
                    self.idx = (self.idx + 1) % self.targets.len();
                    self.sel = 0;
                    self.typed_ms = 0;
                    self.skip = false;
                }
                Opt::Close => return Outcome::Exit,
            },
            // Any other key (or Enter mid-sentence) finishes the typewriter.
            _ => self.skip = true,
        }
        Outcome::Stay
    }
}

fn c(key: char) -> Color {
    rgb(palette::rgb(key))
}

/// Greedy word wrap to `width` columns.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.lines() {
        let mut line = String::new();
        for word in para.split_whitespace() {
            if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.extend(word.chars().take(width));
        }
        lines.push(line);
    }
    lines
}

pub fn draw(ask: &Ask, area: Rect, buf: &mut Buffer) {
    halfblock::fill(buf, area, c('d'));
    let block = Block::bordered()
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(c('S')).bg(c('d')))
        .style(Style::default().bg(c('d')));
    let inner = block.inner(area);
    block.render(area, buf);
    if inner.width < 10 || inner.height < 3 {
        return;
    }
    let text_style = Style::default().fg(c('w')).bg(c('d'));
    let Some(t) = ask.target() else {
        buf.set_stringn(
            inner.x + 1,
            inner.y,
            "Nobody needs you right now.",
            inner.width as usize - 1,
            text_style,
        );
        options(ask, inner, buf);
        return;
    };

    let show_portrait = inner.width >= PORTRAIT_COLS + 24;
    let tx = if show_portrait {
        let tag = if ask.typing() { "talk" } else { "idle" };
        let f = sheet(&format!("portrait_{}", t.class.id())).frame(tag, ask.typed_ms);
        let frame = Rect::new(
            inner.x + 1,
            inner.y,
            PORTRAIT_COLS,
            inner.height.saturating_sub(1),
        );
        halfblock::draw(buf, frame, f, 0, 0, false);
        inner.x + PORTRAIT_COLS + 3
    } else {
        inner.x + 1
    };
    let width = inner.right().saturating_sub(tx + 1) as usize;
    let plate = Style::default()
        .fg(c(t.class.color()))
        .bg(c('k'))
        .add_modifier(Modifier::BOLD);
    buf.set_stringn(tx, inner.y, format!(" {} ", t.name), width, plate);

    let shown: String = t.text.chars().take(ask.shown_chars()).collect();
    let rows = inner.height.saturating_sub(4) as usize;
    let lines = wrap(&shown, width.max(1));
    // Keep the newest lines in view as the text types out.
    let start = lines.len().saturating_sub(rows);
    for (i, line) in lines[start..].iter().enumerate() {
        buf.set_stringn(tx, inner.y + 2 + i as u16, line, width, text_style);
    }
    options(ask, inner, buf);
}

fn options(ask: &Ask, inner: Rect, buf: &mut Buffer) {
    let y = inner.bottom() - 1;
    let mut x = inner.x + 1;
    let blink = (ask.clock_ms / 500).is_multiple_of(2);
    for (i, o) in ask.options().into_iter().enumerate() {
        let selected = i == ask.sel;
        let cursor = if selected && blink { "▶ " } else { "  " };
        let fg = if selected { 'y' } else { 's' };
        let s = format!("{cursor}{}   ", o.label());
        if x >= inner.right() {
            break;
        }
        let style = Style::default().fg(c(fg)).bg(c('d'));
        x = buf
            .set_stringn(x, y, s, (inner.right() - x) as usize, style)
            .0;
    }
}

/// Lines of a pane worth showing: drop blank lines and pure decoration (box rules,
/// spinners), keep the last `max`.
pub fn dialogue(raw: &str, max: usize) -> String {
    let lines: Vec<&str> = raw
        .lines()
        .map(str::trim)
        .filter(|l| l.chars().filter(|ch| ch.is_alphanumeric()).count() >= 3)
        .collect();
    lines[lines.len().saturating_sub(max)..].join("\n")
}

fn targets_from(c: &mut Client, snap: &Snapshot, config: &Config) -> Vec<Target> {
    let ws = std::env::var("HERDR_WORKSPACE_ID")
        .ok()
        .or_else(|| snap.focused_workspace_id.clone());
    let mut blocked: Vec<_> = snap
        .agents
        .iter()
        .filter(|a| a.agent_status == AgentStatus::Blocked)
        .filter(|a| ws.as_deref().is_none_or(|w| a.workspace_id == w))
        .collect();
    blocked.sort_by_key(|a| std::cmp::Reverse(a.state_change_seq));
    blocked
        .into_iter()
        .map(|a| {
            let kind = a.agent.clone().unwrap_or_else(|| "agent".into());
            let text = c
                .request(
                    "pane.read",
                    json!({ "pane_id": a.pane_id, "source": "detection", "lines": 40 }),
                )
                .map(|r| dialogue(r["read"]["text"].as_str().unwrap_or(""), 12))
                .unwrap_or_default();
            Target {
                name: a.display_agent.clone().unwrap_or_else(|| {
                    let mut ch = kind.chars();
                    ch.next()
                        .map_or(String::new(), |f| f.to_uppercase().chain(ch).collect())
                }),
                class: Class::for_agent(&config.classes, &kind),
                pane_id: a.pane_id.clone(),
                text: if text.is_empty() {
                    "I need your input!".into()
                } else {
                    text
                },
            }
        })
        .collect()
}

fn demo_targets() -> Vec<Target> {
    vec![
        Target {
            name: "Claude".into(),
            class: Class::Knight,
            pane_id: "demo:p1".into(),
            text: "Do you want to make this edit to src/auth.rs?\n1. Yes\n2. Yes, and don't ask again this session\n3. No, tell Claude what to do differently".into(),
        },
        Target {
            name: "Codex".into(),
            class: Class::Mage,
            pane_id: "demo:p2".into(),
            text: "Allow command `cargo test --workspace`? [y/N]".into(),
        },
    ]
}

pub fn run(config: &Config, demo: bool) -> anyhow::Result<()> {
    let mut client = if demo { None } else { Some(Client::connect()?) };
    let targets = match &mut client {
        None => demo_targets(),
        Some(c) => {
            let mut v = c.request("session.snapshot", json!({}))?;
            let snap: Snapshot = serde_json::from_value(v["snapshot"].take())?;
            targets_from(c, &snap, config)
        }
    };
    let mut ask = Ask::new(targets);
    super::sprites::sheets();
    let mut terminal = ratatui::init();
    let mut last = Instant::now();
    let result = (|| -> anyhow::Result<()> {
        loop {
            terminal.draw(|f| draw(&ask, f.area(), f.buffer_mut()))?;
            if event::poll(Duration::from_millis(50))?
                && let Event::Key(k) = event::read()?
                && k.kind == KeyEventKind::Press
            {
                match ask.key(k.code) {
                    Outcome::Stay => {}
                    Outcome::Exit => return Ok(()),
                    Outcome::Focus(pane) => {
                        if let Some(c) = &mut client {
                            c.request("agent.focus", json!({ "target": pane }))?;
                        }
                        return Ok(());
                    }
                }
            }
            let now = Instant::now();
            ask.tick((now - last).as_millis() as u64);
            last = now;
        }
    })();
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask() -> Ask {
        Ask::new(demo_targets())
    }

    #[test]
    fn typewriter_then_any_key_skips() {
        let mut a = ask();
        assert_eq!(a.shown_chars(), 0);
        a.tick(1000);
        assert_eq!(a.shown_chars(), CHARS_PER_SEC as usize);
        assert!(a.typing());
        assert_eq!(a.key(KeyCode::Char('x')), Outcome::Stay);
        assert!(!a.typing());
    }

    #[test]
    fn enter_mid_sentence_only_finishes_the_text() {
        let mut a = ask();
        assert_eq!(a.key(KeyCode::Enter), Outcome::Stay);
        assert!(!a.typing());
        assert_eq!(a.key(KeyCode::Enter), Outcome::Focus("demo:p1".into()));
    }

    #[test]
    fn next_cycles_agents_and_restarts_typing() {
        let mut a = ask();
        a.skip = true;
        assert_eq!(a.options(), [Opt::Go, Opt::Next, Opt::Close]);
        a.key(KeyCode::Right);
        assert_eq!(a.key(KeyCode::Enter), Outcome::Stay);
        assert_eq!(a.idx, 1);
        assert!(a.typing());
        a.skip = true;
        assert_eq!(a.key(KeyCode::Enter), Outcome::Focus("demo:p2".into()));
    }

    #[test]
    fn esc_q_and_close_always_exit() {
        for k in [KeyCode::Esc, KeyCode::Char('q')] {
            assert_eq!(ask().key(k), Outcome::Exit);
        }
        let mut a = ask();
        a.skip = true;
        a.key(KeyCode::Left); // wraps to Close
        assert_eq!(a.key(KeyCode::Enter), Outcome::Exit);
        let mut empty = Ask::new(vec![]);
        assert_eq!(empty.options(), [Opt::Close]);
        assert_eq!(empty.key(KeyCode::Enter), Outcome::Exit);
    }

    #[test]
    fn dialogue_drops_decoration() {
        let raw = "✢ Crafting… (3m)\n\n────────\n❯\n────\n  Do you want to proceed?\n 1. Yes\n";
        assert_eq!(
            dialogue(raw, 12),
            "✢ Crafting… (3m)\nDo you want to proceed?\n1. Yes"
        );
        assert_eq!(dialogue(raw, 1), "1. Yes");
    }

    #[test]
    fn wraps_words() {
        assert_eq!(wrap("aaa bbb ccc", 7), ["aaa bbb", "ccc"]);
        assert_eq!(wrap("a\nb", 10), ["a", "b"]);
    }

    #[test]
    fn renders_at_any_size() {
        let mut a = ask();
        a.tick(800);
        for (w, h) in [(1, 1), (20, 5), (70, 18), (120, 20)] {
            let area = Rect::new(0, 0, w, h);
            draw(&a, area, &mut Buffer::empty(area));
        }
        draw(
            &Ask::new(vec![]),
            Rect::new(0, 0, 40, 6),
            &mut Buffer::empty(Rect::new(0, 0, 40, 6)),
        );
    }

    #[test]
    fn ask_demo_snapshot() {
        let mut a = ask();
        a.tick(1200);
        let area = Rect::new(0, 0, 90, 18);
        let mut buf = Buffer::empty(area);
        draw(&a, area, &mut buf);
        insta::assert_snapshot!(format!("{buf:?}"));
    }
}
