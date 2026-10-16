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
const COMPACT_SCENE: u16 = 5;
const MAX_DRAWN: usize = 4;

fn footer_rows(w: &World) -> u16 {
    w.heroes.len().clamp(1, MAX_DRAWN) as u16
}

pub fn mode(area: Rect, w: &World) -> Mode {
    let chrome = 2 + footer_rows(w);
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
        Screen::Quests | Screen::Sheet | Screen::Help | Screen::Week
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
            _ => screens::help(body, buf),
        }
        return;
    }
    match m {
        Mode::Mini => mini(w, area, buf),
        m => {
            top_bar(w, area, buf, area.y);
            quest_line(w, area, buf, area.y + 1);
            let footer = footer_rows(w);
            let scene = Rect::new(area.x, area.y + 2, area.width, area.height - 2 - footer);
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
                // Last, so a hero's ❗ or a float never cuts through it.
                banner(w, scene, buf);
            }
            for (i, h) in w.heroes.iter().take(MAX_DRAWN).enumerate() {
                footer_row(h, area, buf, scene.bottom() + i as u16, i == view.selected);
            }
        }
    }
}

fn top_bar(w: &World, area: Rect, buf: &mut Buffer, y: u16) {
    let right = format!(" Party Lv.{} ", w.party_level());
    let rx = area.right().saturating_sub(right.chars().count() as u16);
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
    text(buf, rx, y, area.right(), &right, 'y', 'd');
}

fn quest_line(w: &World, area: Rect, buf: &mut Buffer, y: u16) {
    let right = if let Some(b) = &w.boss {
        let filled = (b.hp as usize).div_ceil(25);
        format!(
            " {} {}{} ",
            b.name,
            "█".repeat(filled * 2),
            "░".repeat(8 - filled * 2)
        )
    } else if let Some(q) = w.quest_hero().and_then(|h| h.quest.as_ref()) {
        let s = q.elapsed_ms / 1000;
        format!(" ⏱ {:02}:{:02} ", s / 60, s % 60)
    } else {
        String::new()
    };
    let rx = area.right().saturating_sub(right.chars().count() as u16);
    if let Some(b) = &w.banner {
        text(buf, area.x, y, rx, &format!(" ⚠ {b}"), 'o', 'd');
    } else if let Some(h) = w.quest_hero() {
        let q = h.quest.as_ref().unwrap();
        let x = text(buf, area.x, y, rx, " ⚔ Quest: ", 'w', 'd');
        let x = text(buf, x, y, rx, &q.title, 'w', 'd');
        if w.heroes.len() > 1 {
            text(
                buf,
                x,
                y,
                rx,
                &format!(" · {}", h.name),
                h.class.color(),
                'd',
            );
        }
    } else {
        text(
            buf,
            area.x,
            y,
            rx,
            " ⚔ No quest — the party rests",
            's',
            'd',
        );
    }
    let fg = if w.boss.is_some() { 'r' } else { 's' };
    text(buf, rx, y, area.right(), &right, fg, 'd');
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
    let f = sheet(&format!("prop_{name}")).frame("a", h.anim_ms);
    let (px, ppy) = match motion {
        Motion::Hand => (x + 12, py + 6),
        Motion::Out => (x + 12 + (t * 24.0) as i32, py + 4 - (t * 6.0) as i32),
        Motion::In => (x + 36 - (t * 24.0) as i32, py + 6),
        Motion::Rise => (x + 4, py - (t * 8.0) as i32),
    };
    halfblock::draw(buf, scene, f, px, ppy, false);
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

fn full_scene(w: &World, scene: Rect, buf: &mut Buffer) {
    backdrop(w, scene, buf);
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
        let (mut dx, flip) = walk_off(h);
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
    if let Some(b) = &w.boss {
        // Drop a size tier until the boss fits under the sky.
        let room = (scene.height as i32 - 1) * 2;
        let mut tier = b.tier;
        let mut s = sheet(&format!("boss_{}_{}", b.id, tier.id()));
        while (s.frames[0].h as i32) > room
            && let Some(t) = tier.smaller()
        {
            tier = t;
            s = sheet(&format!("boss_{}_{}", b.id, tier.id()));
        }
        let f = s.frame(b.tag(), b.state_ms);
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
        let shake = if hit {
            [1, -1][((b.hit_ms / 40) % 2) as usize]
        } else {
            0
        };
        halfblock::draw(
            buf,
            scene,
            f,
            right - f.w as i32 - 2 + shake,
            stand_y(scene, f.h),
            false,
        );
    } else if camp(w) {
        campfire(w, scene, buf, right, true);
    } else {
        let hit = w.heroes.iter().any(|h| h.anim() == Anim::Attack);
        slimes(w, scene, buf, right, 2, hit);
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

fn slimes(w: &World, scene: Rect, buf: &mut Buffer, right: i32, n: i32, hit: bool) {
    let s = sheet("slime");
    for i in 0..n {
        let tag = if hit && i == 0 { "hurt" } else { "idle" };
        // Offset each slime's clock so they don't bounce in lockstep.
        let f = s.frame(tag, w.clock_ms + i as u64 * 170);
        let x = right - (n - i) * (f.w as i32 + 2);
        halfblock::draw(buf, scene, f, x, stand_y(scene, f.h), true);
    }
}

fn compact_scene(w: &World, scene: Rect, buf: &mut Buffer) {
    backdrop(w, scene, buf);
    const STEP: u16 = 10;
    let slots = (scene.width.saturating_sub(14) / STEP).clamp(1, MAX_DRAWN as u16);
    let drawn = w.heroes.len().min(slots as usize);
    for (i, h) in w.heroes.iter().take(drawn).enumerate() {
        let x = 1 + i as u16 * STEP;
        // Faces only have `idle`; anything but sitting bobs at walk pace.
        let ms = if h.anim() == Anim::Sit {
            0
        } else {
            h.anim_ms * 2
        };
        let f = sheet(&format!("face_{}", h.class.id())).frame("idle", ms);
        let (dx, flip) = walk_off(h);
        let py = stand_y(scene, f.h);
        halfblock::draw(buf, scene, f, x as i32 + dx, py, flip);
        float_text(h, scene, buf, x.saturating_sub(4), py);
        if let Some((s, fg)) = badge(h) {
            text(buf, scene.x + x + 7, scene.y, scene.right(), s, fg, 'n');
        }
    }
    plus_n(w, scene, buf, 1 + drawn as u16 * STEP, drawn);
    if camp(w) {
        campfire(w, scene, buf, scene.width as i32, false);
    } else {
        let hit = w.heroes.iter().any(|h| h.anim() == Anim::Attack);
        slimes(w, scene, buf, scene.width as i32, 1, hit);
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
fn footer_row(h: &Hero, area: Rect, buf: &mut Buffer, y: u16, selected: bool) {
    let end = area.right();
    let mut x = text(
        buf,
        area.x,
        y,
        end,
        if selected { "▶" } else { " " },
        'y',
        'd',
    );
    x = text(
        buf,
        x,
        y,
        end,
        &format!("{:<9.9}", h.name),
        h.class.color(),
        'd',
    );
    x = text(buf, x, y, end, &format!(" Lv{:<3}", h.level), 'w', 'd');
    let (f, e) = bar(xp_cells(h, 8), 8);
    x = text(buf, x, y, end, &f, 'y', 'd');
    x = text(buf, x, y, end, &e, 'S', 'd');
    x = text(buf, x, y, end, " XP ", 's', 'd');
    if area.width >= 50 {
        x = text(buf, x, y, end, " STA ", 's', 'd');
        match h.context_used {
            Some(used) => {
                let left = 100 - used as usize;
                let col = match used {
                    90.. => 'r',
                    70.. => 'o',
                    _ => 'c',
                };
                let (f, e) = bar(left.div_ceil(20), 5);
                x = text(buf, x, y, end, &f, col, 'd');
                x = text(buf, x, y, end, &e, 'S', 'd');
                x = text(buf, x, y, end, &format!(" {left:>3}% "), 's', 'd');
            }
            None => x = text(buf, x, y, end, "  ?        ", 's', 'd'),
        }
    }
    if !h.summons.is_empty() {
        x = text(buf, x, y, end, &format!(" ☠{}", h.summons.len()), 'p', 'd');
    }
    if area.width >= 36 {
        let fg = if h.status == Status::Blocked {
            'r'
        } else {
            'w'
        };
        text(buf, x, y, end, &format!(" {}", activity(h)), fg, 'd');
    }
}

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
