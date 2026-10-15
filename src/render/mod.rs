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
