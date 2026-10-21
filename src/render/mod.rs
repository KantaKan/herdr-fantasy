//! Frame loop: adaptive FPS, redraw only when the frame changed, nothing at all while
//! the stage is hidden (SPEC §8.5).

pub mod ask;
pub mod halfblock;
pub mod layout;
pub mod motion;
pub mod screens;
pub mod sprites;
pub mod view;

use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::config::{Config, NotifySound};
use crate::game::demo;
use crate::game::party;
use crate::game::persist::{QuestRecord, Store};
use crate::game::world::World;
use crate::herdr::cache::{self, Cmd, Update};

const ACTIVE_FPS: u64 = 15;
const IDLE_FPS: u64 = 4;
/// While hidden we only wake for input/resize or this often to pick up herdr changes.
const HIDDEN_WAKE: Duration = Duration::from_secs(1);
const AUTOSAVE: Duration = Duration::from_secs(30);
const MAX_WAIT: Duration = Duration::from_millis(100);

/// Frame-loop cost, for `stage --demo --bench` (SPEC §8.5).
#[derive(Debug, Default)]
pub struct Bench {
    /// Stop after this long.
    limit: Duration,
    frames: u64,
    drawn: u64,
    busy: Duration,
    worst: Duration,
}

impl Bench {
    fn record(&mut self, took: Duration, drew: bool) {
        self.frames += 1;
        self.drawn += drew as u64;
        self.busy += took;
        self.worst = self.worst.max(took);
    }

    pub fn report(&self, wall: Duration) -> String {
        let avg = self.busy.as_secs_f64() * 1000.0 / self.frames.max(1) as f64;
        format!(
            "bench: {:.0} s, {} ticks ({} redrawn), avg {:.3} ms/tick, worst {:.2} ms, \
             loop busy {:.2}% of one core",
            wall.as_secs_f64(),
            self.frames,
            self.drawn,
            avg,
            self.worst.as_secs_f64() * 1000.0,
            self.busy.as_secs_f64() / wall.as_secs_f64().max(1e-9) * 100.0
        )
    }
}

pub fn run_demo(
    scenario: &str,
    seed: u64,
    speed: f32,
    config: &Config,
    bench: Option<Duration>,
) -> anyhow::Result<()> {
    let (mut world, mut demo) =
        demo::scenario(scenario, seed, &config.classes).ok_or_else(|| {
            anyhow::anyhow!("unknown demo scenario `{scenario}` (try: party, stress)")
        })?;
    sprites::sheets(); // decode before the first frame
    world.reduced_motion = config.reduced_motion;
    world.memes = config.memes;
    world.skins = config.skins;
    let speed = speed.max(0.0) as f64;
    let mut carry = 0.0;
    let mut bench_state = bench.map(|limit| {
        let b = Bench {
            limit,
            ..Bench::default()
        };
        (b, Instant::now())
    });
    let result = with_terminal(|t| {
        frame_loop(
            t,
            &mut world,
            bench_state.as_mut(),
            |w, v, key| match key {
                view::Key::Change(row, d) => {
                    crate::game::settings::change(
                        w,
                        v.selected,
                        crate::game::settings::ROWS[row],
                        d,
                    );
                }
                view::Key::Buy(n) => {
                    w.buy(v.selected, n);
                }
                _ => {}
            },
            |w, real_ms| {
                let ms = real_ms * speed + carry;
                carry = ms.fract();
                w.today = today();
                demo.advance(ms as u64, w);
                w.drain_finished();
                true
            },
        )
    });
    if let Some((b, started)) = &bench_state {
        println!("{}", b.report(started.elapsed()));
    }
    result
}

/// Days since 1970 (UTC) for the daily quest.
fn today() -> u32 {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    crate::game::daily::day_of(ms)
}

/// The real stage: party from herdr, progress saved per repo.
pub fn run_live(config: &Config) -> anyhow::Result<()> {
    let me = cache::Me::from_env();
    let (rx, cmds) = cache::spawn(me.clone(), config.show_all_workspaces || me.is_none());
    let root = crate::logging::state_dir()
        .ok_or_else(|| anyhow::anyhow!("no state directory"))?
        .join("state");
    let mut store = Store::open(root);
    let mut world = World::new("…", "Meadow Hamlet");
    world.banner = Some("looking for herdr…".into());
    world.reduced_motion = config.reduced_motion;
    world.memes = config.memes;
    world.pets.clone_from(&config.pets);
    world.classes.clone_from(&config.classes);
    world.skins = config.skins;
    sprites::sheets();
    let mut visible = true;
    let mut last_save = Instant::now();
    let result = with_terminal(|t| {
        let focus = |w: &mut World, v: &view::View, key: view::Key| match key {
            view::Key::Focus(i) => {
                if let Some(seat) = w.heroes.get(i).and_then(|h| h.seat.as_ref()) {
                    let _ = cmds.send(Cmd::Focus(seat.pane_id.clone()));
                }
            }
            view::Key::Change(row, d) => {
                let row = crate::game::settings::ROWS[row];
                if let Some(edit) = crate::game::settings::change(w, v.selected, row, d)
                    && let Some(path) = crate::config::config_path()
                    && let Err(e) = crate::config::save_edit(&path, &edit)
                {
                    tracing::warn!("cannot save {}: {e}", path.display());
                }
            }
            view::Key::Buy(n) => {
                w.buy(v.selected, n); // sets save_now
            }
            view::Key::Stay => {}
        };
        frame_loop(t, &mut world, None, focus, |w, real_ms| {
            visible = drain(&rx, &cmds, w, config, &mut store, visible);
            for title in std::mem::take(&mut w.toasts) {
                if config.notify_on_blocked {
                    let _ = cmds.send(Cmd::Notify {
                        title,
                        body: "Context is almost full; a rest (compaction) is near".into(),
                        sound: "none",
                    });
                }
            }
            w.today = today();
            w.update(real_ms as u64);
            persist(w, &mut store, &mut last_save, config.store_prompt_text);
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
    cmds: &std::sync::mpsc::Sender<Cmd>,
    w: &mut World,
    cfg: &Config,
    store: &mut Store,
    mut visible: bool,
) -> bool {
    loop {
        match rx.try_recv() {
            Ok(Update::Party { agents, visible: v }) => {
                w.banner = None;
                let classes = w.classes.clone();
                for name in party::sync(w, &agents, &classes, store) {
                    if cfg.notify_on_blocked {
                        let _ = cmds.send(Cmd::Notify {
                            title: format!("{name} needs you"),
                            body: "Press prefix+a to talk".into(),
                            sound: match cfg.notify_sound {
                                NotifySound::None => "none",
                                NotifySound::Done => "done",
                                NotifySound::Request => "request",
                            },
                        });
                    }
                }
                visible = v;
            }
            Ok(Update::Transcript {
                pane_id,
                entries,
                history,
                new_chapter,
            }) => party::feed(
                w,
                party::Feed {
                    pane_id: &pane_id,
                    entries: &entries,
                    history,
                    new_chapter,
                },
                &cfg.context_window,
                &cfg.bosses,
            ),
            Ok(Update::Town { repo, town }) => {
                if repo == w.repo_key {
                    w.theme = town.theme.name().to_string();
                }
                w.towns.insert(repo, town);
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
fn persist(w: &mut World, store: &mut Store, last_save: &mut Instant, store_prompts: bool) {
    let ended = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    for f in w.drain_finished() {
        let Some(seat) = &f.seat else { continue };
        store.append_quest(
            &seat.repo.key,
            &QuestRecord {
                hero: &f.hero,
                title: &crate::game::persist::logged_title(&f.quest.title, store_prompts),
                duration_ms: f.quest.duration_ms,
                xp: f.quest.xp,
                result: f.quest.result,
                ended_unix_ms: ended,
                bosses: &f.quest.bosses,
                chapter: f.quest.chapter,
            },
        );
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
/// `on_key(world, view, key)` runs for Enter on a hero and settings changes. With `bench`,
/// the loop measures itself and stops after the bench duration.
fn frame_loop(
    terminal: &mut ratatui::DefaultTerminal,
    world: &mut World,
    mut bench: Option<&mut (Bench, Instant)>,
    mut on_key: impl FnMut(&mut World, &view::View, view::Key),
    mut step: impl FnMut(&mut World, f64) -> bool,
) -> anyhow::Result<()> {
    let mut last = Instant::now();
    let mut prev: Option<Buffer> = None;
    let mut visible = true;
    let mut view = view::View::default();
    // Screen changes fade in from the night (tachyonfx, SPEC §8.4).
    let mut fx: Option<tachyonfx::Effect> = None;
    let mut shown = view.screen;
    loop {
        let wait = if !visible {
            HIDDEN_WAKE
        } else {
            let fps = if world.is_busy() || fx.is_some() {
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
                    if ctrl_c {
                        return Ok(());
                    }
                    world.welcome = false; // any key closes the welcome card
                    let key = view.key(k.code, world.heroes.len());
                    if key != view::Key::Stay {
                        on_key(world, &view, key);
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
        let tick_start = Instant::now();
        visible = step(world, real_ms);
        view.clamp(world.heroes.len());
        if !visible {
            prev = None; // repaint fully when we come back
            continue;
        }

        let size = terminal.size()?;
        let area = Rect::new(0, 0, size.width, size.height);
        let mut buf = Buffer::empty(area);
        layout::render(world, &view, area, &mut buf);
        if view.screen != shown {
            shown = view.screen;
            if !world.reduced_motion {
                let night = halfblock::rgb(crate::palette::rgb('k'));
                fx = Some(tachyonfx::fx::fade_from(
                    night,
                    night,
                    (250, tachyonfx::Interpolation::QuadOut),
                ));
            }
        }
        if let Some(e) = &mut fx {
            e.process(
                tachyonfx::Duration::from_millis(real_ms as _),
                &mut buf,
                area,
            );
            if e.done() {
                fx = None;
            }
        }
        let drew = prev.as_ref() != Some(&buf);
        if drew {
            terminal.draw(|f| f.buffer_mut().clone_from(&buf))?;
            prev = Some(buf);
        }
        if let Some((b, started)) = bench.as_deref_mut() {
            b.record(tick_start.elapsed(), drew);
            if started.elapsed() >= b.limit {
                return Ok(());
            }
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
        t.draw(|f| layout::render(&world, &view::View::default(), f.area(), f.buffer_mut()))
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
        assert_eq!(
            m(90, 11),
            layout::Mode::Compact,
            "6 scene rows for the chibis"
        );
        assert_eq!(m(90, 10), layout::Mode::Mini);
        assert_eq!(m(39, 30), layout::Mode::Compact);
        assert_eq!(m(29, 30), layout::Mode::Mini);
        // Big hero mode: tall, and the half-size scene fits the party (3 heroes with pets).
        let big = |cols, rows| layout::big(&w, Rect::new(0, 0, cols, rows));
        assert!(big(200, 30));
        assert!(!big(200, 29), "not tall enough");
        assert!(!big(150, 40), "the party wouldn't fit at half size");
    }

    #[test]
    fn duck_bubble_shows_in_full_and_compact() {
        for rows in [15, 11] {
            let (mut world, _) = demo::party(1, &crate::config::Config::default().classes);
            world
                .duck
                .quip(crate::game::duck::Quip::ForcePush, "k".into(), 0, true);
            let mut t = Terminal::new(TestBackend::new(90, rows)).unwrap();
            t.draw(|f| layout::render(&world, &view::View::default(), f.area(), f.buffer_mut()))
                .unwrap();
            let screen: String = t
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect();
            assert!(screen.contains("No fear."), "{rows} rows");
        }
    }

    #[test]
    fn summon_storm_shows_three_and_a_count() {
        let (mut world, mut d) = demo::party(1, &crate::config::Config::default().classes);
        for n in 0..10 {
            world.apply(crate::game::world::GameEvent::Summon(
                0,
                format!("toolu_storm_{n}"),
                false,
            ));
        }
        for _ in 0..40 {
            d.advance(TICK_MS, &mut world);
        }
        let mut t = Terminal::new(TestBackend::new(120, 18)).unwrap();
        t.draw(|f| layout::render(&world, &view::View::default(), f.area(), f.buffer_mut()))
            .unwrap();
        let screen: String = t
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(screen.contains("+7"));
    }

    #[test]
    fn dq_band_shows_party_windows_and_messages() {
        let mut world = World::new("r", "t");
        let a = world.add_hero("Claude", crate::game::hero::Class::Knight);
        world.heroes[a].level = 3;
        world.welcome = false;
        world.messages.say("Claude attacks!");
        world.messages.update(2_000);
        let mut t = Terminal::new(TestBackend::new(90, 14)).unwrap();
        t.draw(|f| layout::render(&world, &view::View::default(), f.area(), f.buffer_mut()))
            .unwrap();
        let screen: String = t
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(screen.contains("Claude  L3"), "party window");
        assert!(screen.contains("Claude attacks!"), "message window");
        assert!(screen.contains("╔"), "DQ window border");
    }

    #[test]
    fn welcome_card_for_a_new_party_until_a_key() {
        let mut world = World::new("r", "t");
        world.add_hero("Claude", crate::game::hero::Class::Knight);
        let screen = |w: &World| {
            let mut t = Terminal::new(TestBackend::new(90, 18)).unwrap();
            t.draw(|f| layout::render(w, &view::View::default(), f.area(), f.buffer_mut()))
                .unwrap();
            t.backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>()
        };
        assert!(screen(&world).contains("Welcome to herdr fantasy!"));
        world.welcome = false;
        assert!(!screen(&world).contains("Welcome"));
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
            let mut classes = crate::config::Config::default().classes;
            if let Ok(c) = std::env::var("PREVIEW_CLAUDE") {
                classes.insert("claude".into(), c);
            }
            let (mut world, mut d) = demo::party(1, &classes);
            for _ in 0..tick {
                d.advance(66, &mut world);
                world.drain_finished();
            }
            let area = Rect::new(0, 0, w, h);
            let mut buf = Buffer::empty(area);
            if std::env::var("PREVIEW_ASK").is_ok() {
                let mut a = ask::Ask::new(vec![ask::Target {
                    name: "Claude".into(),
                    class: crate::game::hero::Class::Knight,
                    pane_id: "p".into(),
                    text: "Do you want to make this edit to src/auth.rs?
1. Yes
2. No"
                        .into(),
                }]);
                a.tick(tick as u64 * 66);
                ask::draw(&a, area, &mut buf);
            } else {
                let v = view::View {
                    screen: match std::env::var("PREVIEW_SCREEN").as_deref() {
                        Ok("quests") => view::Screen::Quests,
                        Ok("sheet") => view::Screen::Sheet,
                        Ok("town") => view::Screen::Town,
                        Ok("help") => view::Screen::Help,
                        Ok("week") => view::Screen::Week,
                        Ok("settings") => view::Screen::Settings,
                        _ => view::Screen::Battle,
                    },
                    ..Default::default()
                };
                layout::render(&world, &v, area, &mut buf);
            }
            if std::env::var("PREVIEW_JSON").is_ok() {
                // cells as [symbol, fg, bg] for tools/make_gif.py
                let hex = |c: Color| match c {
                    Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
                    _ => "#000000".into(),
                };
                let cells: Vec<serde_json::Value> = buf
                    .content
                    .iter()
                    .map(|c| serde_json::json!([c.symbol(), hex(c.fg), hex(c.bg)]))
                    .collect();
                let doc = serde_json::json!({ "w": w, "h": h, "cells": cells });
                let name = format!("{dir}/frame_{tick:05}.json");
                std::fs::write(name, doc.to_string()).unwrap();
                continue;
            }
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

#[cfg(test)]
mod perf {
    use super::*;

    /// `cargo test --release perf_split -- --ignored --nocapture`: where a stress frame goes.
    #[test]
    #[ignore]
    fn perf_split() {
        let (mut world, mut d) = demo::stress(1, &crate::config::Config::default().classes);
        let area = Rect::new(0, 0, 120, 30);
        let (mut step, mut draw) = (Duration::ZERO, Duration::ZERO);
        for _ in 0..2_000 {
            let t = Instant::now();
            d.advance(66, &mut world);
            world.drain_finished();
            step += t.elapsed();
            let t = Instant::now();
            let mut buf = Buffer::empty(area);
            layout::render(&world, &view::View::default(), area, &mut buf);
            draw += t.elapsed();
        }
        println!(
            "step {:?}/frame, render {:?}/frame",
            step / 2_000,
            draw / 2_000
        );
    }

    /// Which rows change between frames (each changed cell is terminal output).
    #[test]
    #[ignore]
    fn perf_changed_cells() {
        let (mut world, mut d) = demo::stress(1, &crate::config::Config::default().classes);
        let area = Rect::new(0, 0, 120, 30);
        let mut prev = Buffer::empty(area);
        let mut per_row = vec![0u64; 30];
        let frames = 600;
        for _ in 0..frames {
            d.advance(66, &mut world);
            world.drain_finished();
            let mut buf = Buffer::empty(area);
            layout::render(&world, &view::View::default(), area, &mut buf);
            for (i, (a, b)) in buf.content.iter().zip(prev.content.iter()).enumerate() {
                if a != b {
                    per_row[i / 120] += 1;
                }
            }
            prev = buf;
        }
        let total: u64 = per_row.iter().sum();
        println!("changed cells/frame: {}", total / frames);
        for (y, n) in per_row.iter().enumerate() {
            println!("row {y:2}: {}", n / frames);
        }
    }
}
