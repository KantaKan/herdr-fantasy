//! Frame loop: adaptive FPS, redraw only when the frame changed, nothing at all while
//! the stage is hidden (SPEC §8.5).

pub mod halfblock;
pub mod layout;
pub mod sprites;

use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::config::Config;
use crate::game::demo;
use crate::game::party;
use crate::game::persist::{QuestRecord, Store};
use crate::game::world::World;
use crate::herdr::cache::{self, Update};

const ACTIVE_FPS: u64 = 15;
const IDLE_FPS: u64 = 4;
/// While hidden we only wake for input/resize or this often to pick up herdr changes.
const HIDDEN_WAKE: Duration = Duration::from_secs(1);
const AUTOSAVE: Duration = Duration::from_secs(30);
const MAX_WAIT: Duration = Duration::from_millis(100);

pub fn run_demo(scenario: &str, seed: u64, speed: f32, config: &Config) -> anyhow::Result<()> {
    let (mut world, mut demo) = demo::scenario(scenario, seed, &config.classes)
        .ok_or_else(|| anyhow::anyhow!("unknown demo scenario `{scenario}` (try: party)"))?;
    sprites::sheets(); // decode before the first frame
    let speed = speed.max(0.0) as f64;
    let mut carry = 0.0;
    with_terminal(|t| {
        // ponytail: `q` quits the demo until M7 gives it the quest log.
        frame_loop(t, &mut world, true, |w, real_ms| {
            let ms = real_ms * speed + carry;
            carry = ms.fract();
            demo.advance(ms as u64, w);
            true
        })
    })
}

/// The real stage: party from herdr, progress saved per repo.
pub fn run_live(config: &Config) -> anyhow::Result<()> {
    let me = cache::Me::from_env();
    let rx = cache::spawn(me.clone(), config.show_all_workspaces || me.is_none());
    let root = crate::logging::state_dir()
        .ok_or_else(|| anyhow::anyhow!("no state directory"))?
        .join("state");
    let mut store = Store::open(root);
    let mut world = World::new("…", "Meadow Hamlet");
    world.banner = Some("looking for herdr…".into());
    sprites::sheets();
    let mut visible = true;
    let mut last_save = Instant::now();
    let result = with_terminal(|t| {
        frame_loop(t, &mut world, false, |w, real_ms| {
            visible = drain(&rx, w, config, &mut store, visible);
            w.update(real_ms as u64);
            persist(w, &mut store, &mut last_save);
            visible
        })
    });
    for h in &world.heroes {
        party::save_hero(h, &mut store);
    }
    store.flush();
    result
}

/// Apply every pending herdr update; returns the latest visibility.
fn drain(
    rx: &Receiver<Update>,
    w: &mut World,
    cfg: &Config,
    store: &mut Store,
    mut visible: bool,
) -> bool {
    loop {
        match rx.try_recv() {
            Ok(Update::Party { agents, visible: v }) => {
                w.banner = None;
                party::sync(w, &agents, &cfg.classes, store);
                visible = v;
            }
            Ok(Update::Lost) => w.banner = Some("reconnecting to herdr…".into()),
            Err(TryRecvError::Empty) => return visible,
            Err(TryRecvError::Disconnected) => {
                w.banner = Some("herdr link stopped".into());
                return visible;
            }
        }
    }
}

/// Write finished quests to the log; save progress on level-up / quest end / every 30 s.
fn persist(w: &mut World, store: &mut Store, last_save: &mut Instant) {
    let ended = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    for f in std::mem::take(&mut w.finished) {
        let Some(seat) = &f.seat else { continue };
        store.append_quest(
            &seat.repo.key,
            &QuestRecord {
                hero: &f.hero,
                title: &f.quest.title,
                duration_ms: f.quest.duration_ms,
                xp: f.quest.xp,
                result: f.quest.result,
                ended_unix_ms: ended,
            },
        );
        let mut p = store.get(&seat.repo.key, &seat.key());
        p.quests += 1;
        store.put(&seat.repo.key, &seat.key(), p);
    }
    if w.save_now || last_save.elapsed() >= AUTOSAVE {
        for h in &w.heroes {
            party::save_hero(h, store);
        }
        if store.is_dirty() {
            store.flush();
        }
        w.save_now = false;
        *last_save = Instant::now();
    }
}

fn with_terminal(
    f: impl FnOnce(&mut ratatui::DefaultTerminal) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let result = f(&mut terminal);
    ratatui::restore();
    result
}

/// `step(world, real_ms)` advances the game and returns whether the stage is visible.
fn frame_loop(
    terminal: &mut ratatui::DefaultTerminal,
    world: &mut World,
    quit_on_q: bool,
    mut step: impl FnMut(&mut World, f64) -> bool,
) -> anyhow::Result<()> {
    let mut last = Instant::now();
    let mut prev: Option<Buffer> = None;
    let mut visible = true;
    loop {
        let wait = if !visible {
            HIDDEN_WAKE
        } else {
            let fps = if world.is_busy() {
                ACTIVE_FPS
            } else {
                IDLE_FPS
            };
            // Never sleep longer than MAX_WAIT so a herdr change shows up within ~300 ms
            // even at idle FPS; unchanged frames are not redrawn anyway.
            Duration::from_millis(1000 / fps)
                .min(MAX_WAIT)
                .saturating_sub(last.elapsed())
        };
        if event::poll(wait)? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => {
                    let ctrl_c =
                        k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL);
                    if ctrl_c || (quit_on_q && k.code == KeyCode::Char('q')) {
                        return Ok(());
                    }
                }
                Event::Resize(..) => prev = None,
                _ => {}
            }
            if visible {
                continue;
            }
        }
        let now = Instant::now();
        let real_ms = (now - last).as_secs_f64() * 1000.0;
        last = now;
        visible = step(world, real_ms);
        if !visible {
            prev = None; // repaint fully when we come back
            continue;
        }

        let size = terminal.size()?;
        let area = Rect::new(0, 0, size.width, size.height);
        let mut buf = Buffer::empty(area);
        layout::render(world, area, &mut buf);
        if prev.as_ref() != Some(&buf) {
            terminal.draw(|f| f.buffer_mut().clone_from(&buf))?;
            prev = Some(buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    const TICK_MS: u64 = 1000 / ACTIVE_FPS;

    fn frame_at(ticks: u32, w: u16, h: u16) -> Buffer {
        let (mut world, mut d) = demo::party(1, &crate::config::Config::default().classes);
        for _ in 0..ticks {
            d.advance(TICK_MS, &mut world);
        }
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| layout::render(&world, f.area(), f.buffer_mut()))
            .unwrap();
        t.backend().buffer().clone()
    }

    #[test]
    fn party_seed1_snapshots() {
        for ticks in [0, 50, 200] {
            insta::assert_snapshot!(
                format!("party_tick_{ticks}"),
                format!("{:?}", frame_at(ticks, 90, 15))
            );
        }
    }

    #[test]
    fn layouts_by_size() {
        let (w, _) = demo::party(1, &crate::config::Config::default().classes);
        let m = |cols, rows| layout::mode(Rect::new(0, 0, cols, rows), &w);
        assert_eq!(m(90, 14), layout::Mode::Full);
        assert_eq!(m(90, 13), layout::Mode::Compact);
        assert_eq!(m(90, 10), layout::Mode::Compact);
        assert_eq!(m(90, 9), layout::Mode::Mini);
        assert_eq!(m(39, 30), layout::Mode::Compact);
        assert_eq!(m(29, 30), layout::Mode::Mini);
    }

    #[test]
    fn any_size_renders_without_panic() {
        for (w, h) in [
            (1, 1),
            (5, 2),
            (20, 3),
            (45, 12),
            (70, 12),
            (200, 60),
            (90, 14),
        ] {
            for ticks in [0, 130, 330] {
                frame_at(ticks, w, h);
            }
        }
    }
}

/// Dev helper: `PREVIEW_DIR=<dir> PREVIEW=<cols>x<rows>@<tick>[,…] cargo test preview_png -- --ignored`
/// writes demo frames as PNGs (text cells drawn as solid fg) so layouts can be eyeballed.
#[cfg(test)]
mod preview {
    use super::*;
    use ratatui::style::Color;

    fn px(c: Color) -> image::Rgb<u8> {
        match c {
            Color::Rgb(r, g, b) => image::Rgb([r, g, b]),
            _ => image::Rgb([0, 0, 0]),
        }
    }

    #[test]
    #[ignore]
    fn preview_png() {
        let dir = std::env::var("PREVIEW_DIR").expect("PREVIEW_DIR");
        let spec = std::env::var("PREVIEW").unwrap_or("90x15@140".into());
        for item in spec.split(',') {
            let (size, tick) = item.split_once('@').unwrap();
            let (w, h) = size.split_once('x').unwrap();
            let (w, h, tick): (u16, u16, u32) = (
                w.parse().unwrap(),
                h.parse().unwrap(),
                tick.parse().unwrap(),
            );
            let (mut world, mut d) = demo::party(1, &crate::config::Config::default().classes);
            for _ in 0..tick {
                d.advance(66, &mut world);
            }
            let area = Rect::new(0, 0, w, h);
            let mut buf = Buffer::empty(area);
            layout::render(&world, area, &mut buf);
            let s = 5;
            let mut img = image::RgbImage::new(w as u32 * s, h as u32 * 2 * s);
            for (i, c) in buf.content.iter().enumerate() {
                let (x, y) = (i as u32 % w as u32, i as u32 / w as u32);
                let (top, bot) = match c.symbol() {
                    "▀" => (px(c.fg), px(c.bg)),
                    " " => (px(c.bg), px(c.bg)),
                    _ => (px(c.fg), px(c.bg)),
                };
                for dy in 0..2 * s {
                    for dx in 0..s {
                        let col = if dy < s { top } else { bot };
                        img.put_pixel(x * s + dx, y * 2 * s + dy, col);
                    }
                }
            }
            img.save(format!("{dir}/preview_{item}.png").replace(['@', ','], "_"))
                .unwrap();
        }
    }
}
