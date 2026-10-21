//! Stage layouts by pane size (SPEC §5.2–5.3): full / compact / mini.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use super::halfblock::{self, rgb};
use super::motion;
use super::screens;
use super::sprites::sheet;
use super::view::{Screen, View};
use crate::game::hero::{Anim, FLOAT_MS, Fatigue, Hero, Motion, REST_MS, Status};
use crate::game::world::World;
use crate::game::xp::xp_to_next;
use crate::palette;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Full,
    Compact,
    Mini,
}

/// Scene rows needed: 16 px hero = 8 rows + ground; 8 px face = 4 rows + ground.
const FULL_SCENE: u16 = 9;
/// Rows for the compact scene: the 12 px chibi heroes stand in it.
const COMPACT_SCENE: u16 = 6;
const MAX_DRAWN: usize = 4;

/// Rows of the Dragon Quest band at the bottom: party windows + message window.
const BAND: u16 = 4;
/// One party window: `Claude  L12!` / `HP▓▓▓▓▓░ 65`.
const PARTY_W: u16 = 15;

pub fn mode(area: Rect, _w: &World) -> Mode {
    let chrome = 1 + BAND;
    if area.height >= chrome + FULL_SCENE && area.width >= 40 {
        Mode::Full
    } else if area.height >= chrome + COMPACT_SCENE && area.width >= 30 {
        Mode::Compact
    } else {
        Mode::Mini
    }
}

fn c(key: char) -> Color {
    rgb(palette::rgb(key))
}

fn text(buf: &mut Buffer, x: u16, y: u16, max_x: u16, s: &str, fg: char, bg: char) -> u16 {
    if x >= max_x {
        return x;
    }
    let style = Style::default().fg(c(fg)).bg(c(bg));
    buf.set_stringn(x, y, s, (max_x - x) as usize, style).0
}

pub fn render(w: &World, view: &View, area: Rect, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    halfblock::fill(buf, area, c('d'));
    let m = mode(area, w);
    let hero = w.heroes.get(view.selected);
    // Text screens take everything below the top bar, in any layout.
    if matches!(
        view.screen,
        Screen::Quests | Screen::Sheet | Screen::Help | Screen::Week | Screen::Settings
    ) {
        top_bar(w, area, buf, area.y);
        let body = Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(1),
        );
        match view.screen {
            Screen::Quests => screens::quests(w, hero, view.scroll, body, buf),
            Screen::Sheet => screens::character(w, hero, body, buf),
            Screen::Week => screens::week(w, body, buf),
            Screen::Settings => screens::settings(w, view.selected, view.scroll, body, buf),
            _ => screens::help(body, buf),
        }
        return;
    }
    match m {
        Mode::Mini => mini(w, area, buf),
        m => {
            top_bar(w, area, buf, area.y);
            let scene = Rect::new(area.x, area.y + 1, area.width, area.height - 1 - BAND);
            let band = Rect::new(area.x, area.bottom() - BAND, area.width, BAND);
            if view.screen == Screen::Town {
                backdrop(w, scene, buf);
                screens::town_scene(w, hero, scene, buf, m == Mode::Full);
            } else {
                if m == Mode::Full && big(w, scene) {
                    big_scene(w, scene, buf);
                } else if m == Mode::Full {
                    full_scene(w, scene, buf);
                } else {
                    compact_scene(w, scene, buf);
                }
                if m == Mode::Full {
                    enemy_window(w, scene, buf);
                }
                encounter_wipe(w, scene, buf);
                // Last, so a hero's ❗ or a float never cuts through it.
                banner(w, scene, buf);
                if w.welcome && crate::game::settings::first_run(w) {
                    screens::welcome(scene, buf);
                }
            }
            dq_band(w, view, band, buf);
        }
    }
}

/// Top bar: title and repo on the left; the party crest on the right — rank star,
/// level, rank title, and a bar toward the next party level.
fn top_bar(w: &World, area: Rect, buf: &mut Buffer, y: u16) {
    let level = w.party_level();
    let (rank, color) = crate::game::world::party_rank(level);
    const BAR: usize = 6;
    let filled = ((w.party_progress() * BAR as f32) as usize).min(BAR);
    let lv = format!(" Lv {level} ");
    // Narrow panes drop the title, then the bar.
    let full = 3 + lv.chars().count() + rank.chars().count() + 1 + BAR + 1;
    let room = area.width as usize;
    let (show_rank, show_bar) = (room >= full + 30, room >= lv.len() + BAR + 26);
    let width = 2
        + lv.chars().count()
        + if show_rank {
            rank.chars().count() + 1
        } else {
            0
        }
        + if show_bar { BAR + 1 } else { 0 };
    let rx = area.right().saturating_sub(width as u16);
    let x = text(buf, area.x, y, rx, " herdr fantasy", 'w', 'd');
    text(
        buf,
        x,
        y,
        rx,
        &format!(" · {} · {}", w.repo, w.theme),
        's',
        'd',
    );
    let r = area.right();
    let mut x = text(buf, rx, y, r, " ★", color, 'd');
    x = text(buf, x, y, r, &lv, 'y', 'd');
    if show_rank {
        x = text(buf, x, y, r, &format!("{rank} "), color, 'd');
    }
    if show_bar {
        x = text(buf, x, y, r, &"█".repeat(filled), 'y', 'd');
        text(buf, x, y, r, &"░".repeat(BAR - filled), 'S', 'd');
    }
}

/// Party level-up fireworks: three rockets climb and burst in the sky, one after another.
fn fireworks(w: &World, scene: Rect, buf: &mut Buffer) {
    if w.fireworks_ms == 0 || w.reduced_motion {
        return;
    }
    let t = crate::game::world::FIREWORKS_MS - w.fireworks_ms;
    let sky = (scene.height as i32 * 2 - 6).max(6);
    for i in 0..3u64 {
        let Some(local) = t.checked_sub(i * 800) else {
            continue;
        };
        if local >= 1_200 {
            continue;
        }
        let cx = (scene.width as i32 * [30, 62, 46][i as usize]) / 100;
        let top = sky / 3 + [0, 2, -1][i as usize];
        let colors = [['y', 'o'], ['c', 'w'], ['G', 'y']][i as usize];
        if local < 300 {
            // The rocket climbing, with a spark trail.
            let y = sky - ((sky - top) as u64 * local / 300) as i32;
            pixel(buf, scene, cx, y, 'w');
            pixel(buf, scene, cx, y + 1, 'o');
            continue;
        }
        // The burst: a ring that grows, then fades to scattered sparks.
        let p = local - 300;
        let r = 1 + (p / 90) as i32;
        // 12 spokes (wider than tall: cells are taller than pixels), each a spark with a
        // 2-pixel trail; late in the burst the sparks twinkle out.
        const SPOKES: [(i32, i32); 12] = [
            (3, 0),
            (3, 1),
            (2, 2),
            (1, 3),
            (0, 3),
            (-1, 3),
            (-3, 0),
            (-3, -1),
            (-2, -2),
            (-1, -3),
            (0, -3),
            (1, -3),
        ];
        for (k, (dx, dy)) in SPOKES.iter().enumerate() {
            if p > 600 && (k as u64 + p / 80).is_multiple_of(3) {
                continue;
            }
            for (back, c) in [(0, colors[k % 2]), (1, colors[(k + 1) % 2]), (2, 'p')] {
                let rr = (r - back).max(0);
                pixel(buf, scene, cx + dx * rr / 3, top + dy * rr / 3, c);
            }
        }
        pixel(buf, scene, cx, top, 'w');
    }
}

/// Night sky with twinkling stars, then a strip of grass that scrolls while anyone walks.
fn backdrop(w: &World, scene: Rect, buf: &mut Buffer) {
    let resting = w.heroes.iter().any(|h| h.rest_ms > 0);
    let sky = if resting { 'k' } else { 'n' };
    halfblock::fill(buf, scene, c(sky));
    let gy = scene.bottom() - 1;
    for y in scene.top()..gy {
        for x in scene.left()..scene.right() {
            let h = (x as u64 - scene.x as u64).wrapping_mul(73_856_093)
                ^ (y as u64 - scene.y as u64).wrapping_mul(19_349_663);
            if !h.is_multiple_of(31) || (h / 31 + w.clock_ms / 700).is_multiple_of(7) {
                continue;
            }
            let cell = &mut buf[(x, y)];
            cell.set_symbol("▀");
            cell.set_fg(c(if h.is_multiple_of(3) { 'w' } else { 's' }));
            cell.set_bg(c(sky));
        }
    }
    let walking = w.heroes.iter().any(|h| h.anim() == Anim::Walk);
    let scroll = if walking {
        (w.clock_ms / 120) as u16
    } else {
        0
    };
    for x in scene.left()..scene.right() {
        let tuft = (x + scroll) % 7 == 0;
        let cell = &mut buf[(x, gy)];
        cell.set_symbol("▀");
        cell.set_fg(c(if tuft { 'G' } else { 'g' }));
        cell.set_bg(c('t'));
    }
}

/// Pixel row where a sprite of height `h` stands on the ground.
fn stand_y(scene: Rect, h: u16) -> i32 {
    (scene.height as i32 - 1) * 2 - h as i32
}

fn pixel(buf: &mut Buffer, scene: Rect, x: i32, py: i32, key: char) {
    let one = crate::render::sprites::Frame {
        w: 1,
        h: 1,
        px: vec![Some(palette::rgb(key))],
        ms: 0,
    };
    halfblock::draw(buf, scene, &one, x, py, false);
}

/// Rest scene particles, Well Rested sparkles, sweat drops (SPEC §5.4).
fn condition(w: &World, h: &Hero, scene: Rect, buf: &mut Buffer, x: i32, py: i32, seed: u64) {
    let t = w.clock_ms + seed * 977;
    if h.rest_ms > 0 {
        // Glowing memories drift from the hero's head into a journal by the fire.
        let journal = sheet("prop_book").frame("a", 0);
        halfblock::draw(buf, scene, journal, x + 13, py + 8, false);
        let done = (REST_MS - h.rest_ms) as f32 / REST_MS as f32;
        for k in 0..4u64 {
            let p = ((done * 3.0) + k as f32 * 0.25).fract();
            let (px, ppy) = (
                x + 8 + (p * 9.0) as i32,
                py + 4 + (p * 6.0) as i32 - (k as i32 % 2),
            );
            pixel(buf, scene, px, ppy, if k % 2 == 0 { 'C' } else { 'y' });
        }
        return;
    }
    if h.rested_ms > 0 && (t / 300).is_multiple_of(4) {
        let k = (t / 1200) % 4;
        pixel(
            buf,
            scene,
            x + 3 + (k as i32 * 3),
            py + 1 + (k as i32 % 2) * 3,
            'C',
        );
    }
    let period = match h.fatigue() {
        Fatigue::Fresh => return,
        Fatigue::Tired => 3_000,
        Fatigue::Exhausted => 1_200,
    };
    let phase = t % period;
    if phase < 600 {
        // a drop slides down from the forehead
        pixel(buf, scene, x + 12, py + 4 + (phase / 150) as i32, 'c');
    }
}

/// The playing action's prop (book, pigeon, coin…), placed by how it moves.
fn prop(h: &Hero, scene: Rect, buf: &mut Buffer, x: i32, py: i32) {
    let Some((action, t)) = h.action() else {
        return;
    };
    let Some((name, motion)) = action.prop() else {
        return;
    };
    // Drawn at 2× so a book, a scroll or a pigeon reads at a glance.
    let big = super::sprites::scale2(sheet(&format!("prop_{name}")).frame("a", h.anim_ms));
    let (px, ppy) = match motion {
        Motion::Hand => (x + 11, py),
        Motion::Out => (x + 11 + (t * 30.0) as i32, py - (t * 6.0) as i32),
        Motion::In => (x + 41 - (t * 30.0) as i32, py),
        Motion::Rise => (x, py - 10 - (t * 6.0) as i32),
    };
    halfblock::draw(buf, scene, &big, px, ppy, false);
}

/// What the hero is doing, over its head while the action plays (`📖 grep`, `🕊 push`).
fn caption(h: &Hero, scene: Rect, buf: &mut Buffer, x: i32, py: i32) {
    let Some((action, _)) = h.action() else {
        return;
    };
    if h.float.is_some() {
        return; // the float (damage, +XP) says it this moment
    }
    let s = action.caption();
    let y = (scene.y as i32 + py / 2 - 1).max(scene.y as i32 + 1) as u16;
    let cx = (scene.x as i32 + x + 8 - s.chars().count() as i32 / 2).max(scene.x as i32) as u16;
    text_on(buf, cx, y, scene.right(), s, 'y');
}

/// Damage numbers, "learned: …", combos: rise two rows over the head, then vanish.
fn float_text(h: &Hero, scene: Rect, buf: &mut Buffer, x: u16, py: i32) {
    let Some((text, color, left)) = &h.float else {
        return;
    };
    let rise = ((FLOAT_MS - left) / 400).min(2) as i32;
    let y = scene.y as i32 + py / 2 - 1 - rise;
    if y >= scene.y as i32 {
        let tx = (scene.x + x + 8).saturating_sub(text.chars().count() as u16 / 2);
        text_on(buf, tx, y as u16, scene.right(), text, *color);
    }
}

fn text_on(buf: &mut Buffer, x: u16, y: u16, max_x: u16, s: &str, fg: char) {
    let style = Style::default().fg(c(fg)).add_modifier(Modifier::BOLD);
    if x < max_x {
        buf.set_stringn(x, y, s, (max_x - x) as usize, style);
    }
}

/// "Chapter N · New Run" across the top of the scene.
fn banner(w: &World, scene: Rect, buf: &mut Buffer) {
    let Some((text, _)) = &w.headline else {
        return;
    };
    let s = format!(" ~ {text} ~ ");
    let len = s.chars().count() as u16;
    let x = scene.x + scene.width.saturating_sub(len) / 2;
    let style = Style::default()
        .fg(c('k'))
        .bg(c('y'))
        .add_modifier(Modifier::BOLD);
    buf.set_stringn(x, scene.y, s, scene.width as usize, style);
}

/// A departing hero turns around and walks off to the left (SPEC §5.4).
fn walk_off(h: &Hero) -> (i32, bool) {
    h.leaving_ms
        .map_or((0, false), |ms| (-((ms / 40) as i32), true))
}

/// Golden column of light behind the hero, with LEVEL UP! on top (SPEC §7.2).
fn level_up(w: &World, scene: Rect, buf: &mut Buffer, x: u16, left_ms: u64) {
    let pulse = (w.clock_ms / 120).is_multiple_of(2);
    let ground = scene.bottom() - 1;
    for cy in scene.top()..ground {
        for cx in x + 5..x + 11 {
            if cx >= scene.right() {
                continue;
            }
            let edge = cx == x + 5 || cx == x + 10;
            let col = if edge == pulse { 'y' } else { 'w' };
            let cell = &mut buf[(cx, cy)];
            cell.set_symbol(" ");
            cell.set_bg(c(col));
        }
    }
    // Text rises for the first second, then holds.
    let rise = (2_000u64.saturating_sub(left_ms) / 250).min(3) as u16;
    let ty = (scene.y + 3).saturating_sub(rise).max(scene.y);
    text(
        buf,
        scene.x + x + 3,
        ty,
        scene.right(),
        "LEVEL UP!",
        'k',
        'y',
    );
}

/// The whole party is resting: slimes leave and the camp comes out.
fn camp(w: &World) -> bool {
    w.boss.is_none() && w.heroes.iter().all(|h| h.status == Status::Idle)
}

/// Campfire with the Rubber Duck beside it, ending at column `right`.
fn campfire(w: &World, scene: Rect, buf: &mut Buffer, right: i32, duck: bool) {
    let fire = sheet("campfire").frame("burn", w.clock_ms);
    let fx = right - fire.w as i32 - if duck { 12 } else { 2 };
    halfblock::draw(buf, scene, fire, fx, stand_y(scene, fire.h), false);
    if duck {
        let d = sheet("rubber_duck").frame("idle", w.clock_ms);
        halfblock::draw(
            buf,
            scene,
            d,
            right - d.w as i32 - 2,
            stand_y(scene, d.h),
            true,
        );
    }
}

/// Floating z's over a sleeping hero; `head_y` is the cell row of the top of its head.
fn zzz(w: &World, scene: Rect, buf: &mut Buffer, x: u16, head_y: i32, seed: u64) {
    let phase = ((w.clock_ms + seed * 700) / 700 % 3) as i32;
    let y = head_y - 1 - phase;
    if y >= scene.y as i32 {
        let s = if phase == 2 { "Z" } else { "z" };
        text(
            buf,
            scene.x + x + phase as u16,
            y as u16,
            scene.right(),
            s,
            's',
            'n',
        );
    }
}

fn badge(h: &Hero) -> Option<(&'static str, char)> {
    match h.status {
        Status::Blocked => Some(("❗", 'r')),
        Status::Done => Some(("★", 'y')),
        _ => None,
    }
}

fn plus_n(w: &World, scene: Rect, buf: &mut Buffer, x: u16, drawn: usize) {
    if w.heroes.len() > drawn {
        let s = format!("+{}", w.heroes.len() - drawn);
        text(buf, scene.x + x, scene.y, scene.right(), &s, 'w', 'n');
    }
}

/// Big hero mode (M15): a tall pane whose half-size scene still fits the whole party.
pub fn big(w: &World, scene: Rect) -> bool {
    let step = if w.heroes.iter().any(|h| h.pet.is_some()) {
        23
    } else {
        17
    };
    let heroes = w.heroes.len().clamp(1, MAX_DRAWN) as u16;
    scene.height >= 30 && (scene.width / 2).saturating_sub(29) / step >= heroes
}

/// The full scene drawn at half size, then every pixel doubled. Text (floats, badges,
/// quips) is redrawn once at normal size so it stays readable.
fn big_scene(w: &World, scene: Rect, buf: &mut Buffer) {
    let small = Rect::new(0, 0, scene.width / 2, scene.height / 2);
    let mut s = Buffer::empty(small);
    full_scene(w, small, &mut s);
    let halves = |c: &ratatui::buffer::Cell| match c.symbol() {
        "▀" => Some((c.fg, c.bg)),
        " " => Some((c.bg, c.bg)),
        _ => None,
    };
    for y in 0..small.height {
        for x in 0..small.width {
            let c = &s[(x, y)];
            let (top, bottom) = halves(c).unwrap_or((c.bg, c.bg));
            for (dy, color) in [(0, top), (1, bottom)] {
                for dx in 0..2 {
                    let (bx, by) = (scene.x + x * 2 + dx, scene.y + y * 2 + dy);
                    if bx < scene.right() && by < scene.bottom() {
                        buf[(bx, by)].reset();
                        buf[(bx, by)].set_bg(color);
                    }
                }
            }
        }
    }
    for y in 0..small.height {
        let mut x = 0;
        while x < small.width {
            if halves(&s[(x, y)]).is_some() {
                x += 1;
                continue;
            }
            let start = x;
            let mut run = String::new();
            let fg = s[(x, y)].fg;
            while x < small.width && halves(&s[(x, y)]).is_none() {
                run.push_str(s[(x, y)].symbol());
                x += 1;
            }
            let style = Style::default().fg(fg).bg(s[(start, y)].bg);
            let bx = scene.x + start * 2;
            if bx < scene.right() {
                buf.set_stringn(
                    bx,
                    scene.y + y * 2,
                    &run,
                    (scene.right() - bx) as usize,
                    style,
                );
            }
        }
    }
}

/// The boss frame and the column it stands at, shrunk a tier at a time until it fits
/// under the sky.
fn boss_sprite(w: &World, scene: Rect) -> Option<(&'static super::sprites::Frame, i32)> {
    let b = w.boss.as_ref()?;
    let room = scene.height as i32 * 2 - if scene.height > 8 { 2 } else { 0 };
    let mut tier = b.tier;
    let mut s = sheet(&format!("boss_{}_{}", b.id, tier.id()));
    while (s.frames[0].h as i32) > room
        && let Some(t) = tier.smaller()
    {
        tier = t;
        s = sheet(&format!("boss_{}_{}", b.id, tier.id()));
    }
    let f = s.frame(b.tag(), b.state_ms);
    Some((f, scene.width as i32 - f.w as i32 - 2))
}

/// Swing rhythm in melee: the 3 attack frames (240 ms), then the wind-up held while
/// it lines up the next blow. Returns the attack-sheet time to show.
use crate::game::world::SWING_MS;
fn swing_ms(h: &Hero) -> u64 {
    let t = h.anim_ms % SWING_MS;
    if t < 240 { t } else { 0 }
}

/// Is a melee blow landing on the boss right now (the strike frame)?
fn blow_lands(h: &Hero) -> bool {
    h.engaged
        && h.mob_fight_ms == 0
        && h.charge >= 1.0
        && (80..160).contains(&(h.anim_ms % SWING_MS))
}

/// Where a charging hero runs to: in front of the boss, or of the front slime of the
/// field wave (`slime_gap` = the columns between slimes, as `draw_mobs` lays them out).
fn charge_target(w: &World, scene: Rect, right: i32, max: usize) -> Option<i32> {
    if let Some((_, bx)) = boss_sprite(w, scene) {
        return Some(bx);
    }
    let n = w.mobs.len().min(max) as i32;
    let front = w.mobs.iter().take(max).position(|m| m.dying_ms.is_none())? as i32;
    let fw = sheet("slime").frames[0].w as i32;
    Some(right - (n - front) * (fw + 3))
}

/// Order of `h` among the heroes charging the same target (they line up).
fn queue_spot(w: &World, h: &Hero) -> i32 {
    match &w.boss {
        Some(b) => b.fighters.iter().position(|n| *n == h.name).unwrap_or(0) as i32,
        None => w
            .heroes
            .iter()
            .take_while(|o| o.name != h.name)
            .filter(|o| o.mob_fight_ms > 0)
            .count() as i32,
    }
}

/// The boss with its hit flash, shake, and melee blows (knock-back + spark). Returns
/// whether a boss was drawn.
fn draw_boss(w: &World, scene: Rect, buf: &mut Buffer) -> bool {
    let (Some(b), Some((f, bx))) = (&w.boss, boss_sprite(w, scene)) else {
        return false;
    };
    // Hit: one white frame, then a ±1 column shake while it lasts (SPEC §8.4).
    let hit = b.hit_ms > 0 && !w.reduced_motion;
    let white;
    let f = if hit && b.hit_ms > crate::game::world::HIT_MS - 60 {
        white = crate::render::sprites::Frame {
            w: f.w,
            h: f.h,
            px: f.px.iter().map(|p| p.map(|_| palette::rgb('w'))).collect(),
            ms: f.ms,
        };
        &white
    } else {
        f
    };
    // A landing melee blow knocks it back a column and throws a spark.
    let blow = !w.reduced_motion && w.heroes.iter().any(blow_lands);
    let shake = if hit {
        [1, -1][((b.hit_ms / 40) % 2) as usize]
    } else if blow {
        1
    } else {
        0
    };
    let by = stand_y(scene, f.h);
    halfblock::draw(buf, scene, f, bx + shake, by, false);
    if blow {
        let cy = by + f.h as i32 / 2;
        for (dx, dy, c) in [
            (0, 0, 'w'),
            (-1, -1, 'y'),
            (1, 1, 'y'),
            (-1, 1, 'o'),
            (1, -1, 'o'),
        ] {
            pixel(buf, scene, bx + 1 + dx, cy + dy, c);
        }
    }
    true
}

fn full_scene(w: &World, scene: Rect, buf: &mut Buffer) {
    backdrop(w, scene, buf);
    fireworks(w, scene, buf);
    let boss_at = charge_target(w, scene, scene.width as i32, 3);
    // Pets stand at their hero's front foot, so a party with pets spreads out.
    let step: u16 = if w.heroes.iter().any(|h| h.pet.is_some()) {
        23
    } else {
        17
    };
    let enemy_zone = 28;
    let slots = (scene.width.saturating_sub(enemy_zone + 1) / step).clamp(1, MAX_DRAWN as u16);
    let drawn = w.heroes.len().min(slots as usize);
    let mut counts = Vec::new();
    for (i, h) in w.heroes.iter().take(drawn).enumerate() {
        let x = 1 + i as u16 * step;
        // Exhausted heroes walk at half speed (SPEC §5.4).
        let slow = h.anim() == Anim::Walk && h.fatigue() == Fatigue::Exhausted;
        let ms = if slow { h.anim_ms / 2 } else { h.anim_ms };
        let ms = if h.engaged && h.anim() == Anim::Attack {
            swing_ms(h)
        } else {
            ms
        };
        let f = sheet(&format!("hero_{}", h.class.id())).frame(h.anim().tag(), ms);
        let skinned;
        let f = match crate::game::skins::for_level(h.level).filter(|_| w.skins) {
            Some(skin) => {
                skinned = super::sprites::recolor(f, skin.swaps);
                &skinned
            }
            None => f,
        };
        let ground = stand_y(scene, f.h);
        if h.level_up_ms > 0 {
            level_up(w, scene, buf, x, h.level_up_ms);
        }
        let (mut dx, mut flip) = walk_off(h);
        // Boss fight: run over to the boss (fighters line up behind each other) and
        // back home afterwards, facing the way it runs.
        if h.charge > 0.0
            && let Some(bx) = boss_at.filter(|_| !w.reduced_motion)
        {
            let k = queue_spot(w, h);
            let target = bx - 12 - k * 7;
            let e = h.charge * h.charge * (3.0 - 2.0 * h.charge); // ease in-out
            dx += ((target - x as i32) as f32 * e).round() as i32;
            flip |= !h.engaged;
        }
        let mut py = ground;
        let running = h.anim() == Anim::Walk && h.leaving_ms.is_none();
        if !w.reduced_motion {
            let seed = i as u64;
            if let Some((action, t)) = h.action() {
                dx += motion::lunge(action, t);
            }
            if running {
                dx += motion::sway(w.clock_ms, seed);
                if !slow {
                    py -= motion::hop(w.clock_ms, seed);
                }
                for &(ddx, ddy) in motion::dust(ms) {
                    pixel(buf, scene, x as i32 + dx + ddx, ground + ddy, 'S');
                }
            }
        }
        // Summons stand behind their hero so the hero is never covered.
        summons(w, h, scene, buf, x as i32 + dx + 11);
        counts.push((h, x as i32 + dx + 11));
        halfblock::draw(buf, scene, f, x as i32 + dx, py, flip);
        pet(w, h, scene, buf, x as i32 + dx + 12, py - ground, flip);
        prop(h, scene, buf, x as i32 + dx, py);
        caption(h, scene, buf, x as i32 + dx, py);
        condition(w, h, scene, buf, x as i32 + dx, py, i as u64);
        float_text(h, scene, buf, x, py);
        if let Some((s, fg)) = badge(h) {
            text(buf, scene.x + x + 12, scene.y, scene.right(), s, fg, 'n');
        }
        if h.anim() == Anim::Sit {
            // The seated sprite's head starts 4 px down.
            zzz(
                w,
                scene,
                buf,
                x + 11,
                scene.y as i32 + (py + 4) / 2,
                i as u64,
            );
        }
    }
    for (h, x) in counts {
        summon_count(h, scene, buf, x);
    }
    plus_n(w, scene, buf, 1 + drawn as u16 * step, drawn);

    let right = scene.width as i32;
    if draw_boss(w, scene, buf) {
    } else if camp(w) {
        campfire(w, scene, buf, right, true);
    } else {
        draw_mobs(w, scene, buf, right, 3);
        // The Rubber Duck watches the fight from just in front of the enemies.
        let d = sheet("rubber_duck").frame(
            if w.duck.says.is_some() {
                "quack"
            } else {
                "idle"
            },
            w.clock_ms,
        );
        let dx = right - 2 * 12 - d.w as i32 - 3;
        halfblock::draw(buf, scene, d, dx, stand_y(scene, d.h), true);
    }
    if let Some(b) = &w.boss
        && b.id == "sahur_knocker"
        && b.state == crate::game::world::BossState::Laugh
        && w.memes
    {
        let s = crate::game::duck::SAHUR_LAUGH;
        let x = scene.right().saturating_sub(s.chars().count() as u16 + 2);
        text_on(buf, x, scene.y, scene.right(), s, 'o');
    }
    duck_bubble(w, scene, buf);
}

/// The Duck's speech bubble, top-right of the scene so it never covers a hero.
fn duck_bubble(w: &World, scene: Rect, buf: &mut Buffer) {
    let Some((said, _)) = &w.duck.says else {
        return;
    };
    let s = format!(" 🦆 {said} ");
    let len = s.chars().count() as u16 + 1;
    let x = scene.right().saturating_sub(len + 1);
    let style = Style::default().fg(c('k')).bg(c('w'));
    buf.set_stringn(x, scene.y + 1, s, (scene.right() - x) as usize, style);
}

/// Up to 3 Summons beside their hero, rising out of / sinking into a shadow puddle;
/// `+N` for the rest (SPEC §6.5).
fn summons(w: &World, h: &Hero, scene: Rect, buf: &mut Buffer, x: i32) {
    for (k, s) in h.summons.iter().take(3).enumerate() {
        let sx = x + k as i32 * 5;
        let full = sheet(&format!("joke_{}", s.id())).frame("idle", s.alive_ms);
        let ground = stand_y(scene, full.h);
        let shown = (s.emerged() * full.h as f32).round() as u16;
        if shown < full.h {
            for px in 1..full.w as i32 - 1 {
                let key = if (px + k as i32) % 3 == 0 { 'k' } else { 'p' };
                pixel(buf, scene, sx + px, ground + full.h as i32 - 1, key);
            }
        }
        // Only the emerged rows: the body comes up out of the shadow.
        let cut = full.h - shown;
        let mut f = crate::render::sprites::Frame {
            w: full.w,
            h: full.h,
            px: full.px.clone(),
            ms: full.ms,
        };
        for y in 0..cut {
            for xx in 0..f.w {
                f.px[(y * f.w + xx) as usize] = None;
            }
        }
        halfblock::draw(buf, scene, &f, sx, ground, false);
        if w.memes && s.alive_ms < 2_500 && k == h.summons.len().min(3) - 1 {
            let q = s.quip();
            let ty = (scene.y as i32 + ground / 2 - 1).max(scene.y as i32) as u16;
            text_on(
                buf,
                (scene.x as i32 + sx).max(0) as u16,
                ty,
                scene.right(),
                q,
                'C',
            );
        }
    }
}

/// The hero's pet at its front foot; it grows up at Lv 10 and hops with its hero (M14).
fn pet(w: &World, h: &Hero, scene: Rect, buf: &mut Buffer, x: i32, hop: i32, flip: bool) {
    let Some(id) = &h.pet else { return };
    let stage = if h.level >= 10 { "grown" } else { "baby" };
    // Its trick (fire, stretch, bark, flap) when the hero fights, and now and then on
    // its own; the offset keeps two pets from doing it in sync.
    const TRICK_MS: u64 = 600;
    let own = (w.clock_ms + h.name.len() as u64 * 2_300) % 7_000;
    let (tag, ms) = match h.anim() {
        Anim::Walk => ("walk", w.clock_ms),
        Anim::Sit => ("sit", w.clock_ms),
        Anim::Attack | Anim::Cast | Anim::Victory => ("trick", h.anim_ms),
        _ if own < TRICK_MS => ("trick", own),
        _ => ("idle", w.clock_ms),
    };
    let f = sheet(&format!("pet_{id}_{stage}")).frame(tag, ms);
    halfblock::draw(buf, scene, f, x, stand_y(scene, f.h) + hop, flip);
}

/// `+N` for Summons past the third. Drawn after every hero: the 2nd and 3rd Summon
/// stand behind the next hero, and a label there would be painted over.
fn summon_count(h: &Hero, scene: Rect, buf: &mut Buffer, x: i32) {
    if h.summons.len() > 3 {
        let s = format!("+{}", h.summons.len() - 3);
        let y = scene.y + (stand_y(scene, 12) / 2) as u16;
        text_on(
            buf,
            (scene.x as i32 + x + 5).max(0) as u16,
            y,
            scene.right(),
            &s,
            'p',
        );
    }
}

/// The field wave (M21): slimes recolored by kind; a hit blinks it, slashes it and pops
/// the damage over it; the defeated blink out.
fn draw_mobs(w: &World, scene: Rect, buf: &mut Buffer, right: i32, max: usize) {
    use crate::game::battle::HIT_MS;
    let s = sheet("slime");
    let n = w.mobs.len().min(max) as i32;
    for (i, m) in w.mobs.iter().take(max).enumerate() {
        let fresh = m.hit_ms > HIT_MS - 200;
        let f = s.frame(
            if fresh { "hurt" } else { "idle" },
            w.clock_ms + i as u64 * 170,
        );
        let tinted;
        let f = if m.kind.swaps().is_empty() {
            f
        } else {
            tinted = super::sprites::recolor(f, m.kind.swaps());
            &tinted
        };
        let x = right - (n - i as i32) * (f.w as i32 + 3);
        let y = stand_y(scene, f.h);
        let blink = match m.dying_ms {
            Some(d) => (d / 60).is_multiple_of(2),
            None => m.hit_ms > 0 && (m.hit_ms / 80).is_multiple_of(2) && !w.reduced_motion,
        };
        if !blink {
            halfblock::draw(buf, scene, f, x, y, true);
        }
        if fresh && m.dying_ms.is_none() {
            for k in 0..6 {
                pixel(buf, scene, x + 1 + k, y - 1 + k, 'w'); // the slash
            }
        }
        if m.hit_ms > 0 {
            let rise = ((HIT_MS - m.hit_ms) / 150) as i32;
            let ty = (scene.y as i32 + (y - 2) / 2 - rise).max(scene.y as i32) as u16;
            let tx = (scene.x as i32 + x + 2).max(0) as u16;
            text_on(buf, tx, ty, scene.right(), &m.last_damage.to_string(), 'w');
        }
    }
}

/// Boss encounter: black bands sweep in from both sides, then open on the boss
/// (alternate rows from opposite edges, like the old battle transitions).
fn encounter_wipe(w: &World, scene: Rect, buf: &mut Buffer) {
    use crate::game::world::ENCOUNTER_MS;
    if w.encounter_ms == 0 || w.reduced_motion {
        return;
    }
    let t = 1.0 - w.encounter_ms as f32 / ENCOUNTER_MS as f32;
    let cover = if t < 0.5 { t * 2.0 } else { (1.0 - t) * 2.0 };
    let reach = (scene.width as f32 * cover) as u16;
    let black = c('k');
    for y in scene.top()..scene.bottom() {
        let from_left = (y - scene.top()).is_multiple_of(2);
        for i in 0..reach.min(scene.width) {
            let x = if from_left {
                scene.x + i
            } else {
                scene.right() - 1 - i
            };
            buf[(x, y)].reset();
            buf[(x, y)].set_bg(black);
        }
    }
}

/// Greedy word wrap to `width` columns (a word longer than a row is cut).
fn wrap(s: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut rows = vec![String::new()];
    for word in s.split(' ') {
        let row = rows.last_mut().unwrap();
        let len = row.chars().count();
        if len > 0 && len + 1 + word.chars().count() > width {
            rows.push(word.chars().take(width).collect());
        } else {
            if len > 0 {
                row.push(' ');
            }
            row.push_str(word);
        }
    }
    rows
}

/// A Dragon Quest window: black, with a double-line border.
fn window(buf: &mut Buffer, r: Rect, border: char) {
    if r.width < 2 || r.height < 2 {
        return;
    }
    let style = Style::default().fg(c(border)).bg(c('k'));
    for y in r.top()..r.bottom() {
        for x in r.left()..r.right() {
            let ch = match (
                x == r.left(),
                x == r.right() - 1,
                y == r.top(),
                y == r.bottom() - 1,
            ) {
                (true, _, true, _) => "╔",
                (_, true, true, _) => "╗",
                (true, _, _, true) => "╚",
                (_, true, _, true) => "╝",
                (_, _, true, _) | (_, _, _, true) => "═",
                (true, _, _, _) | (_, true, _, _) => "║",
                _ => " ",
            };
            buf[(x, y)].reset();
            buf[(x, y)].set_symbol(ch).set_style(style);
        }
    }
}

/// Enemy names (or the boss and its HP) in a small window over the enemies.
fn enemy_window(w: &World, scene: Rect, buf: &mut Buffer) {
    let label = if let Some(b) = &w.boss {
        let filled = (b.hp as usize).div_ceil(25);
        format!(
            "{} {}{}",
            b.name,
            "█".repeat(filled * 2),
            "░".repeat(8 - filled * 2)
        )
    } else {
        let mut groups: Vec<(&str, usize)> = Vec::new();
        for m in w.mobs.iter().filter(|m| m.dying_ms.is_none()) {
            match groups.iter_mut().find(|(n, _)| *n == m.kind.name()) {
                Some(g) => g.1 += 1,
                None => groups.push((m.kind.name(), 1)),
            }
        }
        if groups.is_empty() || camp(w) || scene.height < 9 {
            return;
        }
        groups
            .iter()
            .map(|(n, k)| {
                if *k > 1 {
                    format!("{n} ×{k}")
                } else {
                    n.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("  ")
    };
    let wdt = (label.chars().count() as u16 + 4).min(scene.width);
    let r = Rect::new(scene.right().saturating_sub(wdt + 1), scene.y, wdt, 3);
    window(buf, r, 'w');
    let fg = if w.boss.is_some() { 'r' } else { 'w' };
    text(buf, r.x + 2, r.y + 1, r.right() - 1, &label, fg, 'k');
}

/// The Dragon Quest band: a window per hero (the selected one in yellow, a blocked one
/// blinking red) and the message window typing out what happens.
fn dq_band(w: &World, view: &View, band: Rect, buf: &mut Buffer) {
    let fit = ((band.width.saturating_sub(24)) / PARTY_W).max(1) as usize;
    let shown = w.heroes.len().min(fit).min(MAX_DRAWN);
    for (i, h) in w.heroes.iter().take(shown).enumerate() {
        let r = Rect::new(band.x + i as u16 * PARTY_W, band.y, PARTY_W, BAND);
        let blocked = h.status == Status::Blocked;
        let border = if blocked && (w.clock_ms / 400).is_multiple_of(2) {
            'r'
        } else if i == view.selected {
            'y'
        } else {
            'w'
        };
        window(buf, r, border);
        let (x, y, end) = (r.x + 1, r.y + 1, r.right() - 1);
        let nx = text(
            buf,
            x,
            y,
            end,
            &format!("{:<8.8}", h.name),
            h.class.color(),
            'k',
        );
        let lx = text(buf, nx, y, end, &format!("L{}", h.level), 'w', 'k');
        let (icon, col) = match h.status {
            Status::Blocked => ("!", 'r'),
            Status::Done => ("★", 'y'),
            Status::Idle => ("z", 's'),
            Status::Working => ("", 'w'),
        };
        text(buf, lx.max(end - 1), y, end, icon, col, 'k');
        let hx = text(buf, x, y + 1, end, "HP", 's', 'k');
        match h.context_used {
            Some(used) => {
                let left = 100 - used.min(100) as usize;
                let col = match used {
                    90.. => 'r',
                    70.. => 'o',
                    _ => 'G',
                };
                let (f, e) = bar(left.div_ceil(17), 6);
                let bx = text(buf, hx, y + 1, end, &f, col, 'k');
                let bx = text(buf, bx, y + 1, end, &e, 'S', 'k');
                text(buf, bx, y + 1, end, &format!("{left:>3}"), 'w', 'k');
            }
            None => {
                text(buf, hx, y + 1, end, " ------ ??", 'S', 'k');
            }
        }
    }
    let mx = band.x + shown as u16 * PARTY_W;
    let msg = Rect::new(mx, band.y, band.right().saturating_sub(mx), BAND);
    window(buf, msg, 'w');
    if w.heroes.len() > shown {
        text(
            buf,
            msg.x + 1,
            msg.y,
            msg.right(),
            &format!(" +{} ", w.heroes.len() - shown),
            'y',
            'k',
        );
    }
    let lines = w.messages.shown();
    let fallback;
    let lines: Vec<&str> = if let Some(b) = &w.banner {
        fallback = format!("⚠ {b}");
        vec![fallback.as_str()]
    } else if lines.is_empty() {
        fallback = match w
            .quest_hero()
            .and_then(|h| h.quest.as_ref().map(|q| (h, q)))
        {
            Some((h, q)) => format!("{} is on a quest: {}", h.name, q.title),
            None => "The party rests by the fire.".into(),
        };
        vec![fallback.as_str()]
    } else {
        lines.iter().map(String::as_str).collect()
    };
    // Wrap like the real thing; the window keeps its two newest rows.
    let inner = msg.width.saturating_sub(4) as usize;
    let rows: Vec<String> = lines.iter().flat_map(|l| wrap(l, inner)).collect();
    for (k, l) in rows.iter().skip(rows.len().saturating_sub(2)).enumerate() {
        text(
            buf,
            msg.x + 2,
            msg.y + 1 + k as u16,
            msg.right() - 1,
            l,
            'w',
            'k',
        );
    }
    // The DQ "more" arrow while lines are waiting.
    if w.messages.busy() && (w.clock_ms / 300).is_multiple_of(2) {
        text(
            buf,
            msg.right().saturating_sub(3),
            msg.bottom() - 2,
            msg.right() - 1,
            "▼",
            'w',
            'k',
        );
    }
}

fn compact_scene(w: &World, scene: Rect, buf: &mut Buffer) {
    backdrop(w, scene, buf);
    fireworks(w, scene, buf);
    // Small panes get chibi full-body heroes (12 px) instead of just a head; baby pets
    // walk beside them, and fighters charge the (smallest) boss.
    let pets = w.heroes.iter().any(|h| h.pet.is_some());
    let step: u16 = if pets { 21 } else { 13 };
    let boss_at = charge_target(w, scene, scene.width as i32, 2);
    let slots = (scene.width.saturating_sub(14) / step).clamp(1, MAX_DRAWN as u16);
    let drawn = w.heroes.len().min(slots as usize);
    for (i, h) in w.heroes.iter().take(drawn).enumerate() {
        let x = 1 + i as u16 * step;
        let tag = match h.anim() {
            Anim::Walk => "walk",
            Anim::Sit => "sit",
            Anim::Attack | Anim::Cast => "attack",
            _ => "idle",
        };
        let f = sheet(&format!("mini_{}", h.class.id())).frame(tag, h.anim_ms);
        let skinned;
        let f = match crate::game::skins::for_level(h.level).filter(|_| w.skins) {
            Some(skin) => {
                skinned = super::sprites::recolor(f, skin.swaps);
                &skinned
            }
            None => f,
        };
        let (mut dx, mut flip) = walk_off(h);
        if !w.reduced_motion
            && let Some((action, t)) = h.action()
        {
            dx += motion::lunge(action, t) / 2;
        }
        if h.charge > 0.0
            && let Some(bx) = boss_at.filter(|_| !w.reduced_motion)
        {
            let k = queue_spot(w, h);
            let e = h.charge * h.charge * (3.0 - 2.0 * h.charge);
            dx += ((bx - 9 - k * 6 - x as i32) as f32 * e).round() as i32;
            flip |= !h.engaged;
        }
        let py = stand_y(scene, f.h);
        if let Some(id) = &h.pet {
            let ptag = match h.anim() {
                Anim::Walk => "walk",
                Anim::Sit => "sit",
                Anim::Attack | Anim::Cast | Anim::Victory => "trick",
                _ => "idle",
            };
            let pf = sheet(&format!("pet_{id}_baby")).frame(ptag, w.clock_ms);
            halfblock::draw(
                buf,
                scene,
                pf,
                x as i32 + dx + 10,
                stand_y(scene, pf.h),
                flip,
            );
        }
        halfblock::draw(buf, scene, f, x as i32 + dx, py, flip);
        float_text(h, scene, buf, x.saturating_sub(2), py);
        if let Some((s, fg)) = badge(h) {
            text(buf, scene.x + x + 9, scene.y, scene.right(), s, fg, 'n');
        }
    }
    plus_n(w, scene, buf, 1 + drawn as u16 * step, drawn);
    if draw_boss(w, scene, buf) {
    } else if camp(w) {
        campfire(w, scene, buf, scene.width as i32, false);
    } else {
        draw_mobs(w, scene, buf, scene.width as i32, 2);
    }
    duck_bubble(w, scene, buf);
}

fn bar(filled: usize, total: usize) -> (String, String) {
    let f = filled.min(total);
    ("▓".repeat(f), "░".repeat(total - f))
}

fn activity(h: &Hero) -> String {
    match h.status {
        Status::Blocked => "❗ needs you".into(),
        Status::Done => "★ quest complete!".into(),
        Status::Idle => "resting by the fire".into(),
        Status::Working => h.detail.clone().unwrap_or_else(|| "fighting slimes".into()),
    }
}

fn xp_cells(h: &Hero, cells: usize) -> usize {
    (h.xp as usize * cells) / xp_to_next(h.level) as usize
}

/// `Claude    Lv7  ▓▓▓▓▓▓░░ XP  STA ▓▓▓▓░  62%  editing src/auth.rs`
/// One line per hero; the state icon still animates (SPEC §5.3).
fn mini(w: &World, area: Rect, buf: &mut Buffer) {
    let n = w.heroes.len() as u16;
    let mut y = area.y;
    if area.height > n {
        top_bar(w, area, buf, y);
        y += 1;
    }
    let t = w.clock_ms;
    for h in &w.heroes {
        if y >= area.bottom() {
            break;
        }
        let blink = |ms: u64| (t / ms).is_multiple_of(2);
        let icon = match h.status {
            Status::Working => {
                if blink(250) {
                    "⚔"
                } else {
                    "✦"
                }
            }
            Status::Blocked => {
                if blink(500) {
                    "❗"
                } else {
                    "  "
                }
            }
            Status::Done => {
                if blink(300) {
                    "★"
                } else {
                    "☆"
                }
            }
            Status::Idle => {
                if blink(600) {
                    "z"
                } else {
                    "Z"
                }
            }
        };
        let end = area.right();
        let initial = h.name.chars().next().unwrap_or('?').to_string();
        let mut x = text(buf, area.x, y, end, " ", 'w', 'd');
        x = text(buf, x, y, end, &initial, h.class.color(), 'd');
        x = text(buf, x, y, end, " ", 'w', 'd');
        x = text(buf, x, y, end, &format!("{icon:<2}"), 'y', 'd');
        let (f, e) = bar(xp_cells(h, 6), 6);
        x = text(buf, x, y, end, &f, 'y', 'd');
        x = text(buf, x, y, end, &e, 'S', 'd');
        let style = Style::default().fg(c('w')).bg(c('d'));
        let line = format!(" {} Lv{} {}", h.name, h.level, activity(h));
        if x < end {
            buf.set_stringn(
                x,
                y,
                line,
                (end - x) as usize,
                style.remove_modifier(Modifier::all()),
            );
        }
        y += 1;
    }
}
