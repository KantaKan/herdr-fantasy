//! Frame loop: adaptive FPS, redraw only when the frame changed (SPEC §8.5).

pub mod halfblock;
pub mod layout;
pub mod sprites;

use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::game::demo::{self, Demo};
use crate::game::world::World;

const ACTIVE_FPS: u64 = 15;
const IDLE_FPS: u64 = 4;

pub fn run_demo(
    scenario: &str,
    seed: u64,
    speed: f32,
    classes: &demo::Classes,
) -> anyhow::Result<()> {
    let (mut world, mut demo) = demo::scenario(scenario, seed, classes)
        .ok_or_else(|| anyhow::anyhow!("unknown demo scenario `{scenario}` (try: party)"))?;
    sprites::sheets(); // decode before the first frame
    let mut terminal = ratatui::init();
    let result = frame_loop(&mut terminal, &mut world, &mut demo, speed.max(0.0) as f64);
    ratatui::restore();
    result
}

fn frame_loop(
    terminal: &mut ratatui::DefaultTerminal,
    world: &mut World,
    demo: &mut Demo,
    speed: f64,
) -> anyhow::Result<()> {
    let mut last = Instant::now();
    let mut carry = 0.0;
    let mut prev: Option<Buffer> = None;
    loop {
        let fps = if world.is_busy() {
            ACTIVE_FPS
        } else {
            IDLE_FPS
        };
        let frame = Duration::from_millis(1000 / fps);
        if event::poll(frame.saturating_sub(last.elapsed()))? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => {
                    let ctrl_c =
                        k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL);
                    // ponytail: `q` quits the demo until M7 gives it the quest log.
                    if ctrl_c || k.code == KeyCode::Char('q') {
                        return Ok(());
                    }
                }
                Event::Resize(..) => prev = None,
                _ => {}
            }
            continue;
        }
        let now = Instant::now();
        let ms = (now - last).as_secs_f64() * 1000.0 * speed + carry;
        last = now;
        carry = ms.fract();
        demo.advance(ms as u64, world);

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
